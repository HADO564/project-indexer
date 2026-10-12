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
//!    the command should leave a project in, what would prove it, and whether
//!    that already holds. Nothing is written.
//! 2. **Run.** The command, untouched; its exit code.
//! 3. **After.** The evidence — it succeeded, and the proof holds: a folder
//!    that is new, or one that now has a `.git` — and only then the write:
//!    `ensure_project`, or for a folder already tracked, its trackers detected
//!    again.
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
use recognizers::Refusal;

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
    // A shell sees an exit code as one byte. Windows allows more: one that
    // does not fit must not be cut down to something else — 256 would read
    // as 0, success — so it becomes a plain failure.
    Ok(ExitCode::from(u8::try_from(code).unwrap_or(1)))
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
    let command = recognizers::label(argv);
    let expectation = match recognizers::expect(argv, cwd) {
        Ok(expectation) => expectation,
        Err(Refusal::Unknown) => {
            return Err(usage(format!(
                "`{command}` isn't a command dexily observes (it knows: {}). Run it directly.",
                recognizers::KNOWN.join(", ")
            )))
        }
        Err(Refusal::Unsafe(why)) => return Err(usage(why)),
    };

    // dexily's own flags, checked before the command runs: a group that does
    // not exist must stop everything now, not after a folder has been made.
    let mut open = Some(open);
    let mut ctx = None;
    if options.flags.need_the_database() {
        ctx = Some(open.take().expect("not opened yet")()?);
    }
    let fields = options
        .flags
        .check(ctx.as_ref(), cwd, expectation.folders.len())
        .map_err(|e| usage(format!("{e:#}")))?;

    let held_before = expectation.before();

    // Phase 2.
    let code = spawn::run(argv, cwd)?;

    // Phase 3. From here on nothing may change `code`: every problem is a
    // warning, never an error returned.
    let proven = recognizers::confirm(&expectation, &held_before, code);
    let mut report = Report {
        tracked: Vec::new(),
        failed: Vec::new(),
    };
    let say = !options.quiet && !options.json;

    if proven.is_empty() {
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
                let ask = fields.is_empty() && !options.json;
                for folder in &proven {
                    match track(&ctx, folder, &fields, ask) {
                        Ok(Some(tracked)) => {
                            if !options.json {
                                tracked.say(say, err);
                            }
                            report.tracked.push(tracked.project);
                        }
                        Ok(None) => {}
                        Err(e) => report.fail(folder, &e, options.json, err),
                    }
                }
            }
            Err(e) => {
                for folder in &proven {
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

/// A folder now tracked, and how it came to be, for the line that says so.
struct Tracked {
    project: Project,
    /// It was tracked before this command: its trackers were detected again
    /// rather than a project created.
    already: bool,
    /// A `[Y/n/e]` prompt was shown and answered.
    asked: bool,
    /// The project is tracked, but its details — the flags, the form — could
    /// not be saved, and why.
    details: Option<String>,
}

impl Tracked {
    fn say(&self, say: bool, err: &mut dyn Write) {
        let Project {
            name, directory, ..
        } = &self.project;
        // After a prompt the user has just answered it; a line would only
        // repeat them.
        if say && self.already {
            let _ = writeln!(
                err,
                "dexily: \"{name}\" was already tracked; detected its trackers again"
            );
        } else if say && !self.asked {
            let _ = writeln!(err, "dexily: tracking \"{name}\" at {directory}");
        }
        // A warning, so even under `--quiet`. Not "could not track": it is
        // tracked, and `add` would be the wrong thing to retry.
        if let Some(why) = &self.details {
            let _ = writeln!(
                err,
                "dexily: tracked \"{name}\", but could not save its details: {why}"
            );
            let _ = writeln!(err, "        run `dexily edit \"{name}\"` to set them");
        }
    }
}

/// Tracks one folder the command made a project of.
///
/// A folder not yet tracked: asks when `ask`, then `ensure_project`. One that
/// already is — `dexily git init` inside a project made with `dexily mkdir` —
/// is not asked about or created again: its trackers are detected again, which
/// is what the command changed. Either way the flags follow as one update, and
/// the edit form if the answer was `e`.
///
/// `None` when the user said no. `Err` only if the folder could not be tracked
/// at all: once it is, a later problem is a warning carried in [`Tracked`].
fn track(
    ctx: &Context,
    folder: &Path,
    fields: &Fields,
    ask: bool,
) -> anyhow::Result<Option<Tracked>> {
    // The real path, as `add` stores one, so a symlink or `..` cannot point
    // dexily somewhere other than where the folder is.
    let directory = add::absolute(Some(folder.to_path_buf()), "track")?;

    let existing = ctx
        .projects
        .find_by_directory(&directory)?
        .filter(|p| !p.is_deleted);
    let already = existing.is_some();

    // `None` from the confirmer: there was nobody to ask (`--yes`, a pipe),
    // so the prefix is the answer and no prompt was shown.
    let answer = if ask && !already {
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

    let mut project = match existing {
        Some(project) => ctx.projects.refresh_trackers(&project.id)?,
        None => ctx.projects.ensure_project(&directory)?,
    };

    // Tracked. From here a failure is about the details only.
    let details = (|| -> anyhow::Result<()> {
        let update = fields.update(&project);
        if !update.is_empty() {
            project = ctx.projects.update(&project.id, update)?;
        }
        if answer == Track::Edit {
            // Esc in the form keeps the project and skips the details: it
            // was already tracked on the way in.
            let groups = ctx.groups.list()?;
            if let Some(changes) = ctx.editor.edit(&project, groups, FormKind::Full)? {
                if !changes.is_empty() {
                    project = ctx.projects.update(&project.id, changes)?;
                }
            }
        }
        Ok(())
    })()
    .err()
    .map(|e| format!("{e:#}"));

    Ok(Some(Tracked {
        project,
        already,
        asked,
        details,
    }))
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
