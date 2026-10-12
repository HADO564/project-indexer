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

/// The database, opened the first time something needs it: after the command
/// has run, unless a flag has to be checked against it before. Whoever needs
/// it asks, so no caller has to know in advance whether it will be open.
pub(crate) struct Database<'a> {
    state: State<'a>,
}

enum State<'a> {
    Closed(Box<dyn FnOnce() -> anyhow::Result<Context> + 'a>),
    Open(Context),
    /// It could not be opened, and why — kept, so asking again says the same
    /// rather than trying a second time.
    Failed(String),
}

impl<'a> Database<'a> {
    fn new(open: impl FnOnce() -> anyhow::Result<Context> + 'a) -> Self {
        Self {
            state: State::Closed(Box::new(open)),
        }
    }

    pub(crate) fn get(&mut self) -> anyhow::Result<&Context> {
        if matches!(self.state, State::Closed(_)) {
            let State::Closed(open) =
                std::mem::replace(&mut self.state, State::Failed(String::new()))
            else {
                unreachable!("just matched as closed");
            };
            self.state = match open() {
                Ok(ctx) => State::Open(ctx),
                Err(e) => State::Failed(format!("{e:#}")),
            };
        }
        match &self.state {
            State::Open(ctx) => Ok(ctx),
            State::Failed(why) => Err(anyhow::anyhow!("{why}")),
            State::Closed(_) => unreachable!("opened or failed above"),
        }
    }
}

/// Who dexily's own lines are for, decided once from the flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Voice {
    /// A person: every line.
    Prose,
    /// `--quiet`: warnings only — quiet hides success, never a problem.
    Quiet,
    /// `--json`: no prose at all; one document at the end.
    Json,
}

impl Voice {
    fn of(options: &Options) -> Self {
        if options.json {
            Voice::Json
        } else if options.quiet {
            Voice::Quiet
        } else {
            Voice::Prose
        }
    }

    /// A line saying what happened.
    fn say(self, err: &mut dyn Write, line: std::fmt::Arguments) {
        if self == Voice::Prose {
            let _ = writeln!(err, "{line}");
        }
    }

    /// A line saying something went wrong.
    fn warn(self, err: &mut dyn Write, line: std::fmt::Arguments) {
        if self != Voice::Json {
            let _ = writeln!(err, "{line}");
        }
    }
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
    let voice = Voice::of(&options);

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
    // Flags also mean the user has already decided and described the project,
    // and `--json` has nobody to ask: either way, no question.
    let mut database = Database::new(open);
    let ask = options.flags.is_empty() && voice != Voice::Json;
    let fields = options
        .flags
        .check(&mut database, cwd, expectation.folders.len())
        .map_err(|e| usage(format!("{e:#}")))?;

    let held_before = expectation.before();

    // Phase 2.
    let code = spawn::run(argv, cwd)?;

    // Phase 3. From here on nothing may change `code`: every problem is a
    // warning, never an error returned, and nothing here may panic.
    let proven = recognizers::confirm(&expectation, &held_before, code);
    let mut report = Report::default();

    if proven.is_empty() {
        // Only worth a line when a folder was expected: `mkdir --help` is not
        // a failure to track anything.
        if !expectation.folders.is_empty() {
            if code == 0 {
                voice.say(
                    err,
                    format_args!("dexily: {command} made nothing new, nothing tracked"),
                );
            } else {
                voice.say(
                    err,
                    format_args!("dexily: {command} failed (exit {code}), nothing tracked"),
                );
            }
        }
    }
    for folder in &proven {
        // The database is opened only here, after the command has run, unless
        // a flag needed it earlier — so a broken one never stands in the way
        // of the command itself.
        let tracked = database
            .get()
            .and_then(|ctx| track(ctx, folder, &fields, ask));
        match tracked {
            Ok(Some(tracked)) => {
                tracked.say(voice, err);
                report.tracked.push(tracked.project);
            }
            Ok(None) => {}
            Err(e) => report.fail(folder, &e, voice, err),
        }
    }

    if voice == Voice::Json {
        // One line, last: the command's own errors share stderr.
        let _ = json::write_observed(err, code, &report.tracked, &report.failed);
    }
    Ok(code)
}

/// A folder now tracked, and how it came to be, for the line that says so.
struct Tracked {
    project: Project,
    /// It was tracked before this command, so no project was created.
    already: bool,
    /// A `[Y/n/e]` prompt was shown and answered.
    asked: bool,
    /// Things that went wrong once it was tracked — its trackers, its details
    /// — each with the command that puts it right. It is tracked all the
    /// same, so none of them is "could not track".
    problems: Vec<Problem>,
}

/// Something that failed about a folder that *is* tracked, and why.
enum Problem {
    /// Its trackers could not be detected again. The CLI has no command that
    /// does only that, so no retry is offered: the reason is the lead.
    Trackers(String),
    /// The flags, or what the form gave, could not be saved.
    Details(String),
}

impl Tracked {
    fn say(&self, voice: Voice, err: &mut dyn Write) {
        let Project {
            name, directory, ..
        } = &self.project;
        let detected = !self
            .problems
            .iter()
            .any(|problem| matches!(problem, Problem::Trackers(_)));
        if self.already && detected {
            voice.say(
                err,
                format_args!("dexily: \"{name}\" was already tracked; detected its trackers again"),
            );
        } else if self.already {
            voice.say(err, format_args!("dexily: \"{name}\" was already tracked"));
        } else if !self.asked {
            // After a prompt the user has just answered it; a line would only
            // repeat them.
            voice.say(
                err,
                format_args!("dexily: tracking \"{name}\" at {directory}"),
            );
        }
        for problem in &self.problems {
            match problem {
                Problem::Trackers(why) => voice.warn(
                    err,
                    format_args!(
                        "dexily: \"{name}\" is tracked, but its trackers could not be detected again: {why}"
                    ),
                ),
                Problem::Details(why) => {
                    voice.warn(
                        err,
                        format_args!(
                            "dexily: tracked \"{name}\", but could not save its details: {why}"
                        ),
                    );
                    voice.warn(
                        err,
                        format_args!("        run `dexily edit \"{name}\"` to set them"),
                    );
                }
            }
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
/// at all: once it is, a later failure is a [`Problem`] carried in [`Tracked`].
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

    let mut problems = Vec::new();
    let mut project = match existing {
        // Already tracked, whatever detection says next. `refresh_trackers`
        // is all or nothing — one malformed `.uproject` fails it — where
        // `ensure_project` on a new folder keeps what it could detect, so a
        // failure here must not read as the folder not being tracked.
        Some(project) => match ctx.projects.refresh_trackers(&project.id) {
            Ok(refreshed) => refreshed,
            Err(e) => {
                problems.push(Problem::Trackers(e.to_string()));
                project
            }
        },
        None => ctx.projects.ensure_project(&directory)?,
    };

    if let Err(e) = save_details(ctx, &mut project, fields, answer == Track::Edit) {
        problems.push(Problem::Details(format!("{e:#}")));
    }

    Ok(Some(Tracked {
        project,
        already,
        asked,
        problems,
    }))
}

/// The flags as one update on a project that is already tracked, then the
/// edit form when the answer was `e`. `project` is left as last saved, so a
/// failure half-way still reports what did go in.
fn save_details(
    ctx: &Context,
    project: &mut Project,
    fields: &Fields,
    form: bool,
) -> anyhow::Result<()> {
    let update = fields.update(project);
    if !update.is_empty() {
        *project = ctx.projects.update(&project.id, update)?;
    }
    if form {
        // Esc in the form keeps the project and skips the details: it was
        // already tracked on the way in.
        let groups = ctx.groups.list()?;
        if let Some(changes) = ctx.editor.edit(project, groups, FormKind::Full)? {
            if !changes.is_empty() {
                *project = ctx.projects.update(&project.id, changes)?;
            }
        }
    }
    Ok(())
}

/// What phase 3 did, for `--json`.
#[derive(Default)]
struct Report {
    tracked: Vec<Project>,
    failed: Vec<Failed>,
}

/// A folder the command made a project of, which dexily could not track.
pub(crate) struct Failed {
    pub directory: String,
    pub message: String,
}

impl Report {
    /// A folder the command made but dexily could not track. Never an error:
    /// the command succeeded, and its exit code stands. A warning, so it is
    /// said even under `--quiet`, with the command that retries it.
    fn fail(&mut self, folder: &Path, error: &anyhow::Error, voice: Voice, err: &mut dyn Write) {
        let directory = folder.display().to_string();
        let message = format!("{error:#}");
        voice.warn(
            err,
            format_args!(
                "dexily: could not track \"{}\": {message}",
                folder_name(folder)
            ),
        );
        voice.warn(
            err,
            format_args!("        run `dexily add {directory}` to try again"),
        );
        self.failed.push(Failed { directory, message });
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
