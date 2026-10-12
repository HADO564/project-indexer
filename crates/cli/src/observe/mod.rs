//! The observer: `dexily <cmd> [args…]` runs `<cmd>` with inherited stdio,
//! then matches argv, working directory and exit code against recognizers and
//! records what it inferred through core services.
//!
//! Rules: the wrapped command's exit code always wins, and a failure to record
//! never changes it. Recording goes through `ProjectService` directly, not
//! through `Command` — it is facts inferred after a command ran, not a command.
//!
//! Three phases, in order, and no threads — the command is its own process, so
//! dexily only waits while it runs:
//!
//! 1. **Before.** The gatekeeper — only a command with a recognizer runs — then
//!    dexily's own flags are checked, and an *expectation* formed: the folders
//!    the command should make, and which of them are already there. Nothing is
//!    written.
//! 2. **Run.** The command, untouched; its exit code.
//! 3. **After.** The evidence — it succeeded, the folder is there now and was
//!    not before — and only then the one write, `ensure_project`.
//!
//! dexily sees the command's *outcome*, never its output, which goes straight
//! to the terminal.

pub(crate) mod flags;
pub(crate) mod recognizers;
pub(crate) mod spawn;

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use indexer_core::Project;

use crate::commands::{add, Failure};
use crate::confirm::{StdinConfirmer, Track};
use crate::context::Context;
use crate::editor::{FormKind, TerminalEditor};
use crate::output::json;

use flags::{Fields, ObserveFlags};

/// How an observed command was asked for: dexily's global flags, and its own
/// flags for the project the command creates.
#[derive(Debug, Default)]
pub struct Options {
    /// `--yes`: track without asking.
    pub yes: bool,
    /// `--json`: a document on stderr instead of prose, and no questions.
    pub json: bool,
    /// `--quiet`: none of dexily's own lines, though never a warning.
    pub quiet: bool,
    pub flags: ObserveFlags,
}

pub fn run(argv: &[OsString], options: Options) -> anyhow::Result<ExitCode> {
    let cwd = std::env::current_dir()?;
    let (yes, json) = (options.yes, options.json);
    let code = observe(
        argv,
        &cwd,
        options,
        // The database, opened when it is first needed: after the command has
        // run, unless a flag has to be checked against it first.
        || {
            Context::open(
                Box::new(StdinConfirmer::new(yes)),
                Box::new(TerminalEditor::new(json)),
            )
        },
        &mut std::io::stderr().lock(),
    )?;
    // A shell sees an exit code as one byte, so this keeps what it would see.
    Ok(ExitCode::from(code as u8))
}

/// [`run`] with its surroundings passed in — the working folder, how to open
/// the database, where dexily's own lines go — so a test can supply them.
/// Returns the command's exit code.
pub(crate) fn observe(
    argv: &[OsString],
    cwd: &Path,
    options: Options,
    open: impl FnOnce() -> anyhow::Result<Context>,
    err: &mut dyn Write,
) -> anyhow::Result<i32> {
    // Phase 1. The gatekeeper: only a command with a recognizer runs. Anything
    // else is refused here, before it starts — it would gain nothing from
    // dexily, and dexily must not be a way to run arbitrary programs.
    let Some(expectation) = recognizers::expect(argv, cwd) else {
        let program = argv
            .first()
            .map(|p| p.to_string_lossy())
            .unwrap_or_default();
        return Err(usage(format!(
            "`{program}` isn't a command dexily observes (it knows: {}). Run it directly.",
            recognizers::KNOWN.join(", ")
        )));
    };

    // dexily's own flags, checked before the command runs: a group that does
    // not exist must stop everything now, not after a folder has been made.
    let mut open = Some(open);
    let mut ctx = None;
    let asked_for_fields = !options.flags.is_empty();
    if options.flags.need_the_database() {
        ctx = Some(open.take().expect("not opened yet")()?);
    }
    let fields = options
        .flags
        .check(ctx.as_ref(), cwd, expectation.folders.len())
        .map_err(|e| usage(format!("{e:#}")))?;

    let existed_before: Vec<bool> = expectation.folders.iter().map(|f| f.exists()).collect();

    // Phase 2.
    let code = spawn::run(argv, cwd)?;

    // Phase 3. From here on nothing may change `code`: every problem is a
    // warning, never an error returned.
    let created = recognizers::confirm(&expectation, &existed_before, code);
    let mut report = Report {
        tracked: Vec::new(),
        failed: Vec::new(),
    };
    let command = argv[0].to_string_lossy();
    let say = !options.quiet && !options.json;

    if created.is_empty() {
        // Only worth a line when a folder was expected: `mkdir --help` is not
        // a failure to track anything.
        if say && !expectation.folders.is_empty() {
            if code == 0 {
                let _ = writeln!(err, "dexily: {command} made nothing new, nothing tracked");
            } else {
                let _ = writeln!(
                    err,
                    "dexily: {command} failed (exit {code}), nothing tracked"
                );
            }
        }
    } else {
        // The database is opened only now, after the command has run, unless
        // a flag needed it earlier — so a broken one never stands in the way
        // of the command itself.
        let ctx = match ctx {
            Some(ctx) => Ok(ctx),
            None => open.take().expect("not opened yet")(),
        };
        match ctx {
            Ok(ctx) => {
                // Flags mean the user has already decided and described the
                // project, and `--json` has nobody to ask.
                let ask = !asked_for_fields && !options.json;
                for folder in &created {
                    match track(&ctx, folder, &fields, ask) {
                        Ok(Some((project, asked))) => {
                            // After a prompt the user has just answered it;
                            // a line would only repeat them.
                            if say && !asked {
                                let _ = writeln!(
                                    err,
                                    "dexily: tracking \"{}\" at {}",
                                    project.name, project.directory
                                );
                            }
                            report.tracked.push(project);
                        }
                        Ok(None) => {}
                        Err(e) => report.fail(folder, &e, options.json, err),
                    }
                }
            }
            Err(e) => {
                for folder in &created {
                    report.fail(folder, &e, options.json, err);
                }
            }
        }
    }

    if options.json {
        // One line, last: the command's own errors share stderr.
        let _ = json::write_observed(err, code, &report.tracked, &report.failed);
    }
    Ok(code)
}

/// Tracks one folder the command made: asks when `ask`, then `ensure_project`,
/// the flags as one update, and the edit form if the answer was `e`.
///
/// `None` when the user said no. Otherwise the project, and whether a prompt
/// was actually shown.
fn track(
    ctx: &Context,
    folder: &Path,
    fields: &Fields,
    ask: bool,
) -> anyhow::Result<Option<(Project, bool)>> {
    // The real path, as `add` stores one, so a symlink or `..` cannot point
    // dexily somewhere other than where the folder is.
    let directory = add::absolute(Some(folder.to_path_buf()), "track")?;

    // `None` from the confirmer: there was nobody to ask (`--yes`, a pipe),
    // so the prefix is the answer and no prompt was shown.
    let answer = if ask {
        ctx.confirmer.track(&format!(
            "dexily: track \"{}\" at {directory}?",
            folder_name(folder)
        ))?
    } else {
        None
    };
    let asked = answer.is_some();
    let answer = answer.unwrap_or(Track::Yes);
    if answer == Track::No {
        return Ok(None);
    }

    let mut project = ctx.projects.ensure_project(&directory)?;
    let update = fields.update();
    if !update.is_empty() {
        project = ctx.projects.update(&project.id, update)?;
    }
    if answer == Track::Edit {
        // Esc in the form keeps the project and skips the details: it was
        // already tracked on the way in.
        let groups = ctx.groups.list()?;
        if let Some(changes) = ctx.editor.edit(&project, groups, FormKind::Full)? {
            if !changes.is_empty() {
                project = ctx.projects.update(&project.id, changes)?;
            }
        }
    }
    Ok(Some((project, asked)))
}

/// What phase 3 did, for `--json`.
struct Report {
    tracked: Vec<Project>,
    /// A folder that could not be tracked, and why.
    failed: Vec<(String, String)>,
}

impl Report {
    /// A folder the command made but dexily could not track. Never an error:
    /// the command succeeded, and its exit code stands. Said even under
    /// `--quiet` — quiet hides success, not a problem — with the command that
    /// retries it.
    fn fail(&mut self, folder: &Path, error: &anyhow::Error, json: bool, err: &mut dyn Write) {
        let directory = folder.display().to_string();
        let message = format!("{error:#}");
        if !json {
            let _ = writeln!(
                err,
                "dexily: could not track \"{}\": {message}",
                folder_name(folder)
            );
            let _ = writeln!(err, "        run `dexily add {directory}` to try again");
        }
        self.failed.push((directory, message));
    }
}

fn folder_name(folder: &Path) -> String {
    folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| folder.display().to_string())
}

fn usage(message: String) -> anyhow::Error {
    Failure::Usage { message }.into()
}
