//! Recognizers: given argv and the working directory, decide before a command
//! runs whether dexily knows it and what it should leave behind; and after it
//! has run, whether it did.
//!
//! One recognizer per command, each with its project-folder rule beside it:
//! `mkdir`, `git init`, `git clone`. A tracker with no command of its own needs
//! none — an Unreal project is made in the editor, and core's detectors find
//! its `.uproject` in whichever folder is tracked.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The commands dexily observes, for the message that refuses any other. One
/// entry per recognizer in [`expect`].
pub const KNOWN: &[&str] = &["mkdir", "git init", "git clone"];

/// What a command should leave behind, worked out before it runs.
#[derive(Debug, PartialEq, Eq)]
pub struct Expectation {
    pub folders: Vec<Expected>,
}

/// One folder a command should leave a project in, and what would prove it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected {
    pub folder: PathBuf,
    pub proof: Proof,
}

/// The evidence that a command made a project of a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proof {
    /// The folder itself is new: absent before the command, there after it.
    /// `mkdir`'s proof.
    NewFolder,
    /// The folder now holds this entry — `.git`. The folder may well have been
    /// there already: `git init` is usually run inside one. Git's proof.
    Holds(&'static str),
}

impl Expectation {
    /// Whether each folder's proof already held, taken before the command
    /// runs. Afterwards a folder just made looks exactly like one that was
    /// always there.
    pub fn before(&self) -> Vec<bool> {
        self.folders
            .iter()
            .map(|expected| match expected.proof {
                // Anything at that path, a file included: only a path that
                // was free can become a folder the command made.
                Proof::NewFolder => expected.folder.exists(),
                Proof::Holds(_) => expected.holds(),
            })
            .collect()
    }
}

impl Expected {
    /// Whether the proof holds on disk right now.
    fn holds(&self) -> bool {
        match self.proof {
            Proof::NewFolder => self.folder.is_dir(),
            Proof::Holds(entry) => self.folder.join(entry).exists(),
        }
    }

    /// Whether the command made a project of this folder, given whether the
    /// proof already held before it ran. A new folder must not have been
    /// there (`mkdir -p work/new` must not track an old `work`); a `.git` may
    /// have been — `git init` in a repository is still a project to track —
    /// and what matters is that it is there now.
    fn proven(&self, held_before: bool) -> bool {
        match self.proof {
            Proof::NewFolder => !held_before && self.holds(),
            Proof::Holds(_) => self.holds(),
        }
    }
}

/// Why dexily will not run a command.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No recognizer: not a command dexily observes.
    Unknown,
    /// A command dexily observes, but with a word it does not pass on: an
    /// option it does not know, or one that would make the command run some
    /// other program — which would make the allowlist mean nothing. The text
    /// is the message.
    Refused(String),
}

/// The program's name as the recognizers match it: as typed, except that on
/// Windows names ignore case and a program may be given with its `.exe`.
/// One rule for [`expect_on`] and [`label`], so they cannot name a command
/// differently.
fn program_name(argv: &[OsString], windows: bool) -> Option<String> {
    let program = argv.first()?.to_str()?;
    Some(if windows {
        let lower = program.to_ascii_lowercase();
        lower.strip_suffix(".exe").unwrap_or(&lower).to_string()
    } else {
        program.to_string()
    })
}

/// The command as a person would name it — `mkdir`, `git clone` — for
/// dexily's own lines.
pub fn label(argv: &[OsString]) -> String {
    let Some(program) = program_name(argv, cfg!(windows)) else {
        return argv
            .first()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
    };
    if program == "git" {
        // A line refused for an option before the subcommand has none to
        // name: it is just `git`.
        if let Some(subcommand) = git_line(&argv[1..]).ok().and_then(|line| line.subcommand) {
            return format!("git {subcommand}");
        }
    }
    program
}

/// What `argv` should leave behind, or why dexily will not run it — the
/// allowlist: only a command with a recognizer here ever starts.
///
/// A known command that names no folder (`mkdir`, `mkdir --help`) is still
/// known: it runs, says whatever it says, and there is nothing to track.
///
/// The program must be named bare (`mkdir`), as found on `PATH`: a path such as
/// `/tmp/x/mkdir` is some other program that happens to share the name, so it
/// is unknown.
pub fn expect(argv: &[OsString], cwd: &Path) -> Result<Expectation, Refusal> {
    expect_on(argv, cwd, cfg!(windows))
}

/// [`expect`] for a named system rather than the one this was built for, so
/// both sets of rules are tested wherever the tests run: `windows` is cmd's
/// `mkdir`, otherwise the Unix one.
pub fn expect_on(argv: &[OsString], cwd: &Path, windows: bool) -> Result<Expectation, Refusal> {
    let name = program_name(argv, windows).ok_or(Refusal::Unknown)?;
    let rest = &argv[1..];
    let folders = match name.as_str() {
        "mkdir" if !windows => new_folders(mkdir(rest), cwd),
        // `md` is cmd's other name for it.
        "mkdir" | "md" if windows => new_folders(cmd_mkdir(rest), cwd),
        "git" => git(rest, cwd)?,
        _ => return Err(Refusal::Unknown),
    };
    // A folder named twice (`mkdir -p a a`) is one folder: one question, one
    // project.
    let mut once: Vec<Expected> = Vec::new();
    for expected in folders {
        if !once.contains(&expected) {
            once.push(expected);
        }
    }
    Ok(Expectation { folders: once })
}

/// `join` leaves an absolute path as it is and puts a relative one under
/// `cwd`, where the command itself would make it.
fn new_folders(names: Vec<PathBuf>, cwd: &Path) -> Vec<Expected> {
    names
        .iter()
        .map(|name| Expected {
            folder: cwd.join(name),
            proof: Proof::NewFolder,
        })
        .collect()
}

/// mkdir's folders: every argument that is not an option. With `-p` the named
/// path is still the one folder meant — `work/clients/acme` is `acme` — and
/// the parents it makes on the way are not projects.
///
/// `-m` takes a value (`-m 755`, `-m755`, `--mode 755`, `--mode=755`), which
/// is not a folder. After `--` every argument is a name, even one starting
/// with `-`.
fn mkdir(args: &[OsString]) -> Vec<PathBuf> {
    let mut folders = Vec::new();
    let mut args = args.iter();
    let mut options_done = false;
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        if options_done || !text.starts_with('-') || text == "-" {
            folders.push(PathBuf::from(arg));
        } else if text == "--" {
            options_done = true;
        } else if text == "--mode" {
            args.next();
        } else if !text.starts_with("--") && text.ends_with('m') {
            // A cluster of short options ending in `m` (`-m`, `-pm`): the
            // mode is the next argument. `-m755` carries its own.
            args.next();
        }
    }
    folders
}

/// cmd's `mkdir` on Windows: it has no `-p` or `-m` — it makes parents by
/// itself — and its one switch is `/?`. Every other argument is a folder, a
/// name starting with `-` included.
fn cmd_mkdir(args: &[OsString]) -> Vec<PathBuf> {
    args.iter()
        .filter(|arg| !arg.to_string_lossy().starts_with('/'))
        .map(PathBuf::from)
        .collect()
}

/// What an option takes, so that its value is never read as something else —
/// the subcommand, the repository, the folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Takes {
    /// Nothing: `--quiet`.
    Nothing,
    /// A value, as the next word or after `=`: `--depth 1`, `--depth=1`.
    Value,
    /// A value only after `=`, or none at all: `--recurse-submodules[=path]`.
    Optional,
}

/// The options dexily passes on to git, before the subcommand. Every other is
/// refused: a git line runs only if dexily understands each word of it.
///
/// Failing closed is the point. Git takes more spellings than any list of bad
/// ones can hold — any unique abbreviation of a long option (`--upl` is
/// `--upload-pack`), short options run together (`-qu`), global options whose
/// value is the next word (`--shallow-file init …` makes `init` a file name
/// and the word after it the real subcommand) — and each of those ran another
/// program while the list of refusals looked complete. A list of what is
/// *passed* has no such gap: what it does not name does not run.
const GIT_GLOBALS: &[(&str, Takes)] = &[
    ("-C", Takes::Value),
    ("--no-pager", Takes::Nothing),
    ("--version", Takes::Nothing),
    ("--help", Takes::Nothing),
];

/// Options that make git run another program: a setting can name one
/// (`core.sshCommand`, `core.fsmonitor`, an alias), `--exec-path` swaps git's
/// own, `--upload-pack` names one outright, and a template's hooks run on a
/// clone's checkout and on every commit after an `init`. They are in no table
/// of options passed on, so they are refused like any other; this only gives
/// them the message that says why.
const RUNS_A_PROGRAM: &[&str] = &[
    "-c",
    "--config",
    "--config-env",
    "--exec-path",
    "-u",
    "--upload-pack",
    "--template",
];

/// One option word looked up in `table`: its name, what it takes, and whether
/// the value came attached with `=`. Refused unless it is there exactly as
/// spelled — no abbreviation, no run of short options — and takes what it was
/// given.
fn passed_on<'a>(
    text: &'a str,
    table: &[(&'static str, Takes)],
    to: &str,
) -> Result<(&'a str, Takes, bool), Refusal> {
    let (name, attached) = match text.split_once('=') {
        Some((name, _)) => (name, true),
        None => (text, false),
    };
    let known = table.iter().find(|(known, _)| *known == name);
    match known {
        Some((_, Takes::Nothing)) if attached => Err(Refusal::Refused(format!(
            "`{name}` takes no value, so `{text}` is not passed on to {to}. Run git directly."
        ))),
        Some((_, takes)) => Ok((name, *takes, attached)),
        None if RUNS_A_PROGRAM.contains(&name) => Err(Refusal::Refused(format!(
            "`{name}` makes git run another program, so it is not passed on. Run git directly."
        ))),
        None => Err(Refusal::Refused(format!(
            "`{name}` is not an option dexily passes on to {to}: options go in full and one at \
             a time (`--depth 1 -q`, not `--dep 1` or `-q1`). Run git directly for anything else."
        ))),
    }
}

/// Git's line before the subcommand's own arguments.
struct GitLine<'a> {
    /// Where `-C` moves git to, relative to the working folder. It may repeat.
    base: PathBuf,
    /// `init`, `clone`, … or `None` when the line has none (`git --version`).
    subcommand: Option<String>,
    /// The arguments after the subcommand.
    rest: &'a [OsString],
}

fn git_line(args: &[OsString]) -> Result<GitLine<'_>, Refusal> {
    let mut line = GitLine {
        base: PathBuf::new(),
        subcommand: None,
        rest: &[],
    };
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        let text = arg.to_string_lossy();
        if !text.starts_with('-') {
            line.subcommand = Some(text.into_owned());
            line.rest = &args[at + 1..];
            break;
        }
        at += 1;
        let (name, takes, attached) = passed_on(&text, GIT_GLOBALS, "git")?;
        if takes == Takes::Value && !attached {
            if name == "-C" {
                if let Some(path) = args.get(at) {
                    line.base = line.base.join(path);
                }
            }
            at += 1;
        }
    }
    Ok(line)
}

/// The git subcommands that make a project — the only ones that run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitCommand {
    Init,
    Clone,
}

impl GitCommand {
    fn parse(subcommand: Option<&str>) -> Option<Self> {
        match subcommand? {
            "init" => Some(Self::Init),
            "clone" => Some(Self::Clone),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Init => "git init",
            Self::Clone => "git clone",
        }
    }

    /// The options dexily passes on to it, each exactly as spelled. An option
    /// that is not here is refused — which is how the ones that run another
    /// program are kept out, along with any git adds later.
    fn options(self) -> &'static [(&'static str, Takes)] {
        use Takes::{Nothing, Optional, Value};
        match self {
            Self::Init => &[
                ("-q", Nothing),
                ("--quiet", Nothing),
                ("--bare", Nothing),
                ("-b", Value),
                ("--initial-branch", Value),
                ("--object-format", Value),
                ("--ref-format", Value),
                ("--separate-git-dir", Value),
                ("--shared", Optional),
            ],
            Self::Clone => &[
                ("-q", Nothing),
                ("--quiet", Nothing),
                ("-v", Nothing),
                ("--verbose", Nothing),
                ("--progress", Nothing),
                ("-n", Nothing),
                ("--no-checkout", Nothing),
                ("--bare", Nothing),
                ("--mirror", Nothing),
                ("-l", Nothing),
                ("--local", Nothing),
                ("--no-hardlinks", Nothing),
                ("-s", Nothing),
                ("--shared", Nothing),
                ("--dissociate", Nothing),
                ("--single-branch", Nothing),
                ("--no-single-branch", Nothing),
                ("--tags", Nothing),
                ("--no-tags", Nothing),
                ("--sparse", Nothing),
                ("--recursive", Nothing),
                ("--recurse-submodules", Optional),
                ("--shallow-submodules", Nothing),
                ("--no-shallow-submodules", Nothing),
                ("--remote-submodules", Nothing),
                ("--also-filter-submodules", Nothing),
                ("--reject-shallow", Nothing),
                ("--no-reject-shallow", Nothing),
                ("-4", Nothing),
                ("--ipv4", Nothing),
                ("-6", Nothing),
                ("--ipv6", Nothing),
                ("-b", Value),
                ("--branch", Value),
                ("-o", Value),
                ("--origin", Value),
                ("--depth", Value),
                ("--shallow-since", Value),
                ("--shallow-exclude", Value),
                ("-j", Value),
                ("--jobs", Value),
                ("--filter", Value),
                ("--revision", Value),
                ("--reference", Value),
                ("--reference-if-able", Value),
                ("--separate-git-dir", Value),
            ],
        }
    }

    /// How many arguments it reads: `init [folder]`, `clone <repository>
    /// [folder]`.
    fn positionals(self) -> usize {
        match self {
            Self::Init => 1,
            Self::Clone => 2,
        }
    }
}

/// A git subcommand's own arguments, read once.
struct GitArgs<'a> {
    positionals: Vec<&'a OsString>,
    /// `--bare` or `--mirror`: a repository nobody works in.
    bare: bool,
}

fn git_args(command: GitCommand, args: &[OsString]) -> Result<GitArgs<'_>, Refusal> {
    let mut parsed = GitArgs {
        positionals: Vec::new(),
        bare: false,
    };
    let mut options_done = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        if options_done || !text.starts_with('-') || text == "-" {
            parsed.positionals.push(arg);
            continue;
        }
        if text == "--" {
            options_done = true;
            continue;
        }
        let (name, takes, attached) = passed_on(&text, command.options(), command.name())?;
        if matches!(name, "--bare" | "--mirror") {
            parsed.bare = true;
        }
        if takes == Takes::Value && !attached {
            args.next();
        }
    }
    parsed.positionals.truncate(command.positionals());
    Ok(parsed)
}

/// Git: only `init` and `clone` make a project, so only they run, and only
/// with options dexily knows. Their proof is a `.git` in the folder, which may
/// have existed already.
///
/// In two steps, in this order: first every reason to refuse the line, then —
/// only for a line that will run — the folder it should leave a project in.
/// Nothing about the folder can therefore let a refused line through.
fn git(args: &[OsString], cwd: &Path) -> Result<Vec<Expected>, Refusal> {
    let line = git_line(args)?;
    let command = GitCommand::parse(line.subcommand.as_deref()).ok_or(Refusal::Unknown)?;
    let parsed = git_args(command, line.rest)?;
    refuse_address(command, &parsed)?;

    let base = cwd.join(&line.base);
    Ok(git_folder(command, &base, &parsed)
        .map(|folder| Expected {
            folder,
            proof: Proof::Holds(".git"),
        })
        .into_iter()
        .collect())
}

/// A clone's address can itself be a command line (`ext::`) or an open file
/// handle (`fd::`): refused, bare clone or not.
fn refuse_address(command: GitCommand, parsed: &GitArgs) -> Result<(), Refusal> {
    if command == GitCommand::Clone {
        if let Some(repository) = parsed.positionals.first() {
            let repository = repository.to_string_lossy();
            if repository.starts_with("ext::") || repository.starts_with("fd::") {
                return Err(Refusal::Refused(format!(
                    "`{repository}` makes git run another program, so it is not passed on. Run git directly."
                )));
            }
        }
    }
    Ok(())
}

/// The folder a git line that will run should leave a repository in, or
/// `None` when there is nothing to track: a bare repository is not a folder
/// anyone works in, and `git clone` alone only prints its usage.
fn git_folder(command: GitCommand, base: &Path, parsed: &GitArgs) -> Option<PathBuf> {
    if parsed.bare {
        return None;
    }
    match command {
        GitCommand::Init => Some(match parsed.positionals.first() {
            Some(directory) => base.join(directory),
            None => base.to_path_buf(),
        }),
        GitCommand::Clone => {
            let repository = parsed.positionals.first()?.to_string_lossy();
            match parsed.positionals.get(1) {
                Some(directory) => Some(base.join(directory)),
                None => humanish(&repository).map(|name| base.join(name)),
            }
        }
    }
}

/// The folder `git clone <repository>` makes when none is named, as git works
/// it out: the last part of the address, without a trailing `/`, `/.git` or
/// `.git` — `https://host/you/thing.git` is `thing`, `host:you/thing` too.
fn humanish(repository: &str) -> Option<String> {
    let trimmed = repository.trim_end_matches(['/', '\\']);
    let trimmed = trimmed.strip_suffix("/.git").unwrap_or(trimmed);
    let name = trimmed.rsplit(['/', ':', '\\']).next()?;
    let name = name.strip_suffix(".git").unwrap_or(name);
    let name = name.strip_suffix(".bundle").unwrap_or(name);
    (!name.is_empty()).then(|| name.to_string())
}

/// The folders to track: those whose proof holds now that the command has
/// run — and only if it succeeded.
///
/// `held_before` is [`Expectation::before`], taken before the command ran, one
/// flag per expected folder in the same order.
pub fn confirm(expected: &Expectation, held_before: &[bool], exit_code: i32) -> Vec<PathBuf> {
    debug_assert_eq!(
        expected.folders.len(),
        held_before.len(),
        "one before-flag per expected folder"
    );
    if exit_code != 0 {
        // The command failed: whatever it left behind, it is not a project.
        return Vec::new();
    }
    expected
        .folders
        .iter()
        .zip(held_before)
        .filter(|(expected, &held)| expected.proven(held))
        .map(|(expected, _)| expected.folder.clone())
        .collect()
}
