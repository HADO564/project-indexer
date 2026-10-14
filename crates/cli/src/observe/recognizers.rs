//! Recognizers: given argv and the working directory, decide before a command
//! runs whether dexily knows it and what it should leave behind; and after it
//! has run, whether it did.
//!
//! One recognizer per command, each with its project-folder rule beside it:
//! `mkdir`, `git init`, `git clone`, `cargo new`, `cargo init`, `go mod init`.
//! A tracker with no command of its own needs none — an Unreal project is made
//! in the editor, and core's detectors find its `.uproject` in whichever folder
//! is tracked.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The commands dexily observes, for the message that refuses any other. One
/// entry per recognizer in [`expect`].
pub const KNOWN: &[&str] = &[
    "mkdir",
    "git init",
    "git clone",
    "cargo new",
    "cargo init",
    "go mod init",
    "gh repo create",
    "npm init",
    "pnpm init",
    "yarn init",
    "poetry init",
];

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
    ///
    /// The expectation rides along: a recognized git line, where dexily can
    /// still place the repo it would make despite the refused word. It is
    /// never read to decide whether the command runs — only after a person at
    /// a terminal has overridden the refusal — so it takes nothing away from
    /// failing closed. `None` when there is no placing it (an abbreviation or
    /// short-option run that might eat the folder, or a `mkdir` with no
    /// recognizer for the word at all).
    Refused(String, Option<Expectation>),
}

impl Refusal {
    /// A refusal as the low-level parse makes one: a reason, and no expectation
    /// yet. [`expect_on`] is where a git line's expectation, if any, is added.
    fn refused(why: String) -> Self {
        Refusal::Refused(why, None)
    }
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
    // cargo and go name the subcommand straight after the program: one word
    // for cargo (`cargo new`, `cargo build`), two for go (`go mod init`,
    // `go mod tidy`). The folder argument is not part of the name, so the walk
    // stops at the first option — a line that names no subcommand is just the
    // program.
    if program == "cargo" {
        return with_subcommand(&program, &argv[1..], 1);
    }
    if program == "go" {
        return with_subcommand(&program, &argv[1..], 2);
    }
    // gh names two subcommand words: `gh repo create`, `gh pr list`.
    if program == "gh" {
        return with_subcommand(&program, &argv[1..], 2);
    }
    // A package manager names one: `npm init`, `poetry init`, `npm install`.
    if matches!(program.as_str(), "npm" | "pnpm" | "yarn" | "poetry") {
        return with_subcommand(&program, &argv[1..], 1);
    }
    program
}

/// `program` followed by up to `depth` leading words that are not options — the
/// command as a person would name it in dexily's own lines, whether dexily runs
/// it or refuses it. The walk stops at the first option, since an option is
/// never part of the name.
fn with_subcommand(program: &str, rest: &[OsString], depth: usize) -> String {
    let mut name = program.to_string();
    for arg in rest.iter().take(depth) {
        let word = arg.to_string_lossy();
        if word.starts_with('-') {
            break;
        }
        name.push(' ');
        name.push_str(&word);
    }
    name
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
        // A refused git line (a known subcommand with a word dexily will not
        // pass on) still knows the repo it would make, when the refused word
        // is not one that could eat the folder: carry that expectation so a
        // terminal override tracks it. A line that is not `init` or `clone` is
        // `Unknown` and carries nothing — there is no telling what it leaves.
        "git" => match git(rest, cwd) {
            Ok(folders) => folders,
            Err(Refusal::Refused(why, _)) => {
                return Err(Refusal::Refused(why, git_expectation(rest, cwd)))
            }
            Err(unknown) => return Err(unknown),
        },
        "cargo" => cargo(rest, cwd)?,
        "go" => go(rest, cwd)?,
        "gh" => gh(rest, cwd)?,
        "npm" | "pnpm" | "yarn" => package_manager_init(&name, rest, cwd)?,
        "poetry" => poetry_init(rest, cwd)?,
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
/// of options passed on, so they are refused like any other; this gives them
/// the message that says why, and — so a lenient walk past a refusal can step
/// over the value — what each one takes, the same arity git reads them with.
const RUNS_A_PROGRAM: &[(&str, Takes)] = &[
    ("-c", Takes::Value),
    ("--config", Takes::Value),
    ("--config-env", Takes::Value),
    ("--exec-path", Takes::Optional),
    ("-u", Takes::Value),
    ("--upload-pack", Takes::Value),
    ("--template", Takes::Value),
];

/// An option word split into its name and whether a value came attached with
/// `=` — `--depth=1` is `("--depth", true)`, `--depth` is `("--depth", false)`.
fn split_option(text: &str) -> (&str, bool) {
    match text.split_once('=') {
        Some((name, _)) => (name, true),
        None => (text, false),
    }
}

/// One option word looked up in `table`: its name, what it takes, and whether
/// the value came attached with `=`. Refused unless it is there exactly as
/// spelled — no abbreviation, no run of short options — and takes what it was
/// given.
fn passed_on<'a>(
    text: &'a str,
    table: &[(&'static str, Takes)],
    to: &str,
) -> Result<(&'a str, Takes, bool), Refusal> {
    let (name, attached) = split_option(text);
    let known = table.iter().find(|(known, _)| *known == name);
    match known {
        Some((_, Takes::Nothing)) if attached => Err(Refusal::refused(format!(
            "`{name}` takes no value, so `{text}` is not passed on to {to}. Run git directly."
        ))),
        Some((_, takes)) => Ok((name, *takes, attached)),
        None if RUNS_A_PROGRAM.iter().any(|(known, _)| *known == name) => {
            Err(Refusal::refused(format!(
                "`{name}` makes git run another program, so it is not passed on. Run git directly."
            )))
        }
        None => Err(Refusal::refused(format!(
            "`{name}` is not an option dexily passes on to {to}: options go in full and one at \
             a time (`--depth 1 -q`, not `--dep 1` or `-q1`). Run git directly for anything else."
        ))),
    }
}

/// What an option takes, found across the tables that could name it: the ones
/// passed on to this part of the line, and the ones refused for running a
/// program, which git reads a value for all the same. `None` when no table
/// knows it — an abbreviation, a run of short options, or an option git grew
/// since — any of which might swallow the next word, so a lenient walk cannot
/// know where the folder is and offers no expectation at all.
fn option_arity(name: &str, passed: &[(&'static str, Takes)]) -> Option<Takes> {
    passed
        .iter()
        .chain(RUNS_A_PROGRAM)
        .find(|(known, _)| *known == name)
        .map(|&(_, takes)| takes)
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

/// The repo a refused git line would still make, worked out while ignoring
/// every reason it was refused — the same two steps as [`git`], but stepping
/// over each option by its known arity ([`option_arity`]) instead of refusing
/// it. `None` the moment a word cannot be placed: not `init` or `clone`, or an
/// option no table knows, where the folder after it is a guess, so an override
/// tracks nothing rather than the wrong thing.
///
/// This is read only after a person at a terminal has overridden the refusal,
/// never to decide whether the command runs: it cannot let a refused line
/// through the gate, only say what to look for once a human has let it run.
fn git_expectation(args: &[OsString], cwd: &Path) -> Option<Expectation> {
    // The line before the subcommand: step over each global, following `-C`,
    // until the first non-option word names the subcommand.
    let mut base = PathBuf::new();
    let mut at = 0;
    let subcommand = loop {
        let text = args.get(at)?.to_string_lossy();
        at += 1;
        if !text.starts_with('-') {
            break text.into_owned();
        }
        let (name, attached) = split_option(&text);
        if option_arity(name, GIT_GLOBALS)? == Takes::Value && !attached {
            if name == "-C" {
                if let Some(path) = args.get(at) {
                    base = base.join(path);
                }
            }
            at += 1;
        }
    };
    let command = GitCommand::parse(Some(&subcommand))?;

    // The subcommand's own words: the same step-over, collecting the
    // positionals and whether it is a bare repository nobody works in.
    let mut parsed = GitArgs {
        positionals: Vec::new(),
        bare: false,
    };
    let mut options_done = false;
    let mut rest = args[at..].iter();
    while let Some(arg) = rest.next() {
        let text = arg.to_string_lossy();
        if options_done || !text.starts_with('-') || text == "-" {
            parsed.positionals.push(arg);
            continue;
        }
        if text == "--" {
            options_done = true;
            continue;
        }
        let (name, attached) = split_option(&text);
        if matches!(name, "--bare" | "--mirror") {
            parsed.bare = true;
        }
        if option_arity(name, command.options())? == Takes::Value && !attached {
            rest.next();
        }
    }
    parsed.positionals.truncate(command.positionals());

    let folder = git_folder(command, &cwd.join(&base), &parsed)?;
    Some(Expectation {
        folders: vec![Expected {
            folder,
            proof: Proof::Holds(".git"),
        }],
    })
}

/// A clone's address can itself be a command line (`ext::`) or an open file
/// handle (`fd::`): refused, bare clone or not.
fn refuse_address(command: GitCommand, parsed: &GitArgs) -> Result<(), Refusal> {
    if command == GitCommand::Clone {
        if let Some(repository) = parsed.positionals.first() {
            let repository = repository.to_string_lossy();
            if repository.starts_with("ext::") || repository.starts_with("fd::") {
                return Err(Refusal::refused(format!(
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

/// cargo's subcommands that make a project — the only ones that run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CargoCommand {
    New,
    Init,
}

impl CargoCommand {
    fn parse(subcommand: Option<&str>) -> Option<Self> {
        match subcommand? {
            "new" => Some(Self::New),
            "init" => Some(Self::Init),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::New => "cargo new",
            Self::Init => "cargo init",
        }
    }

    /// Its proof: `cargo new`'s folder is new — absent before, there after,
    /// like `mkdir`; `cargo init`'s folder may have been there already (it is
    /// usually run inside one), and is proven by the `Cargo.toml` it drops in,
    /// like `git init`'s `.git`.
    fn proof(self) -> Proof {
        match self {
            Self::New => Proof::NewFolder,
            Self::Init => Proof::Holds("Cargo.toml"),
        }
    }
}

/// The options dexily models for `cargo new` and `cargo init` — one set, since
/// the two share a shape. Each is a safe switch or a plain value (a template, a
/// quiet/verbose level, an edition, a package name, a VCS to initialise, a
/// registry name, a colour choice); none of them makes cargo build, fetch or
/// run anything, so scaffolding cannot become a way to run downloaded code. An
/// option not here — `--config`, `-Z`, a later addition — is refused, not
/// guessed at: the same fail-closed stance as git, for the same reason.
const CARGO_OPTIONS: &[(&str, Takes)] = &[
    ("--bin", Takes::Nothing),
    ("--lib", Takes::Nothing),
    ("-q", Takes::Nothing),
    ("--quiet", Takes::Nothing),
    ("-v", Takes::Nothing),
    ("--verbose", Takes::Nothing),
    ("--frozen", Takes::Nothing),
    ("--locked", Takes::Nothing),
    ("--offline", Takes::Nothing),
    ("-h", Takes::Nothing),
    ("--help", Takes::Nothing),
    ("--edition", Takes::Value),
    ("--name", Takes::Value),
    ("--vcs", Takes::Value),
    ("--registry", Takes::Value),
    ("--color", Takes::Value),
];

/// `go mod init` takes no options of its own, so its table is empty and any
/// word starting with `-` is refused.
const GO_OPTIONS: &[(&str, Takes)] = &[];

/// One option word for a cargo or go line, looked up in `table` — the analogue
/// of git's [`passed_on`], without git's "runs a program" table: these
/// scaffolders have no such option, so a word not in `table` is simply refused.
/// Returns the option as spelled, what it takes, and whether a value came
/// attached with `=`.
fn modelled<'a>(
    text: &'a str,
    table: &[(&'static str, Takes)],
    command: &str,
) -> Result<(&'a str, Takes, bool), Refusal> {
    let (name, attached) = match text.split_once('=') {
        Some((name, _)) => (name, true),
        None => (text, false),
    };
    match table.iter().find(|(known, _)| *known == name) {
        Some((_, Takes::Nothing)) if attached => Err(Refusal::refused(format!(
            "`{name}` takes no value, so `{text}` is not understood. Run `{command}` directly."
        ))),
        Some((_, takes)) => Ok((name, *takes, attached)),
        None => Err(Refusal::refused(format!(
            "`{name}` is not an option dexily models for `{command}`: options go in full and one \
             at a time (`--edition 2021`, not `--ed 2021` or `-qv`). Run `{command}` directly."
        ))),
    }
}

/// A cargo or go subcommand's own words, read once: an unknown option refused,
/// a `Value` option's value skipped so it is never read as a folder, and `--`
/// ending the options. The positionals are returned in order — the folder each
/// command wants is worked out from them afterwards.
fn scaffold_args<'a>(
    command: &str,
    table: &[(&'static str, Takes)],
    args: &'a [OsString],
) -> Result<Vec<&'a OsString>, Refusal> {
    let mut positionals = Vec::new();
    let mut options_done = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        if options_done || !text.starts_with('-') || text == "-" {
            positionals.push(arg);
            continue;
        }
        if text == "--" {
            options_done = true;
            continue;
        }
        let (_, takes, attached) = modelled(&text, table, command)?;
        // `Optional` takes a value only after `=`, never the next word; cargo
        // and go model none, but the scanner handles it as git's does.
        if takes == Takes::Value && !attached {
            args.next();
        }
    }
    Ok(positionals)
}

/// cargo: only `new` and `init` make a project, so only they run, and only with
/// options dexily models. `cargo new` makes the folder it names; `cargo init`
/// writes a `Cargo.toml` into the folder it names, or the working one when it
/// names none. Anything else is unknown, like any `cargo build`.
///
/// Like git's, in two steps: the subcommand and its options first, every reason
/// to refuse the line decided before the folder is worked out.
fn cargo(args: &[OsString], cwd: &Path) -> Result<Vec<Expected>, Refusal> {
    let subcommand = args.first().map(|a| a.to_string_lossy());
    let command = CargoCommand::parse(subcommand.as_deref()).ok_or(Refusal::Unknown)?;
    let positionals = scaffold_args(command.name(), CARGO_OPTIONS, &args[1..])?;
    Ok(cargo_folder(command, cwd, &positionals)
        .map(|folder| Expected {
            folder,
            proof: command.proof(),
        })
        .into_iter()
        .collect())
}

/// The folder a cargo line that will run should leave a project in, from its
/// positionals: `cargo new`'s path, or `cargo init`'s path or the working
/// folder when it names none. `cargo new` with no path only prints its usage,
/// so there is nothing to track.
fn cargo_folder(command: CargoCommand, cwd: &Path, positionals: &[&OsString]) -> Option<PathBuf> {
    match command {
        CargoCommand::New => positionals.first().map(|path| cwd.join(path)),
        CargoCommand::Init => Some(match positionals.first() {
            Some(path) => cwd.join(path),
            None => cwd.to_path_buf(),
        }),
    }
}

/// go: the one shape dexily knows is `go mod init`, which writes a `go.mod` into
/// the working folder — proven like `git init`'s `.git`. Any other `go` line is
/// not a project command, so it is unknown. `go mod init` takes no options, so
/// a word starting with `-` after it is refused, not ignored; its `[module]`
/// argument names the module path, never a folder, so the folder tracked is
/// always the working one.
fn go(args: &[OsString], cwd: &Path) -> Result<Vec<Expected>, Refusal> {
    let word = |at: usize| args.get(at).map(|a| a.to_string_lossy());
    if word(0).as_deref() != Some("mod") || word(1).as_deref() != Some("init") {
        return Err(Refusal::Unknown);
    }
    // The empty table refuses any option; the module path is a positional we
    // never read as a folder.
    scaffold_args("go mod init", GO_OPTIONS, &args[2..])?;
    Ok(vec![Expected {
        folder: cwd.to_path_buf(),
        proof: Proof::Holds("go.mod"),
    }])
}

/// The options dexily models for `gh repo create` — the long forms and the
/// short forms gh gives them. Each is a safe switch or a plain value (a
/// description, a `.gitignore` or licence template, a GitHub template repo, a
/// team, a homepage, a remote or source name). None runs a local program —
/// gh's `--template` names a GitHub template *repository*, not git's local
/// hook-running one — so recognising `gh repo create` does not reopen the
/// allowlist. Anything not here is refused, fail-closed, as git's and cargo's.
const GH_REPO_CREATE_OPTIONS: &[(&str, Takes)] = &[
    ("--private", Takes::Nothing),
    ("--public", Takes::Nothing),
    ("--internal", Takes::Nothing),
    ("-c", Takes::Nothing),
    ("--clone", Takes::Nothing),
    ("--push", Takes::Nothing),
    ("--add-readme", Takes::Nothing),
    ("--disable-issues", Takes::Nothing),
    ("--disable-wiki", Takes::Nothing),
    ("--include-all-branches", Takes::Nothing),
    ("--help", Takes::Nothing),
    ("-d", Takes::Value),
    ("--description", Takes::Value),
    ("-g", Takes::Value),
    ("--gitignore", Takes::Value),
    ("-l", Takes::Value),
    ("--license", Takes::Value),
    ("--homepage", Takes::Value),
    ("-r", Takes::Value),
    ("--remote", Takes::Value),
    ("-s", Takes::Value),
    ("--source", Takes::Value),
    ("-t", Takes::Value),
    ("--team", Takes::Value),
    ("-p", Takes::Value),
    ("--template", Takes::Value),
];

/// `gh repo create`'s words, read once like [`scaffold_args`], but also noting
/// whether `--clone` was asked for — a copy lands on disk only then. An unknown
/// option is refused, a `Value` option's value skipped so it is never read as
/// the repository name, and `--` ends the options.
fn gh_repo_create_args(args: &[OsString]) -> Result<(Vec<&OsString>, bool), Refusal> {
    let mut positionals = Vec::new();
    let mut clone = false;
    let mut options_done = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        if options_done || !text.starts_with('-') || text == "-" {
            positionals.push(arg);
            continue;
        }
        if text == "--" {
            options_done = true;
            continue;
        }
        let (name, takes, attached) = modelled(&text, GH_REPO_CREATE_OPTIONS, "gh repo create")?;
        if name == "--clone" || name == "-c" {
            clone = true;
        }
        if takes == Takes::Value && !attached {
            args.next();
        }
    }
    Ok((positionals, clone))
}

/// gh: the one shape dexily knows is `gh repo create`. With `--clone` gh clones
/// the new repository into a folder named after it — the name without any
/// `owner/` prefix — proven by a `.git`, as `git clone` is. Without `--clone`
/// nothing lands locally, so there is nothing to track; and gh needs a name to
/// clone, so a line that gives none tracks nothing either. Any other `gh` line
/// is not a project command. Fail-closed on options, like git and cargo.
fn gh(args: &[OsString], cwd: &Path) -> Result<Vec<Expected>, Refusal> {
    let word = |at: usize| args.get(at).map(|a| a.to_string_lossy());
    if word(0).as_deref() != Some("repo") || word(1).as_deref() != Some("create") {
        return Err(Refusal::Unknown);
    }
    let (positionals, clone) = gh_repo_create_args(&args[2..])?;
    if !clone {
        return Ok(Vec::new());
    }
    let Some(name) = positionals.first() else {
        return Ok(Vec::new());
    };
    let name = name.to_string_lossy();
    let folder = name.rsplit('/').next().unwrap_or(&name);
    Ok(vec![Expected {
        folder: cwd.join(folder),
        proof: Proof::Holds(".git"),
    }])
}

/// The options dexily models for `npm` / `pnpm` / `yarn init` — a few safe
/// switches and values; none runs code. An unknown flag is refused, like git's.
const NODE_INIT_OPTIONS: &[(&str, Takes)] = &[
    ("-y", Takes::Nothing),
    ("--yes", Takes::Nothing),
    ("-p", Takes::Nothing),
    ("--private", Takes::Nothing),
    ("--workspaces", Takes::Nothing),
    ("-w", Takes::Value),
    ("--workspace", Takes::Value),
    ("--scope", Takes::Value),
    ("-h", Takes::Nothing),
    ("--help", Takes::Nothing),
];

/// The options dexily models for `poetry init` — it writes a `pyproject.toml`
/// and installs nothing, so its flags are safe; an unknown one is refused.
const POETRY_INIT_OPTIONS: &[(&str, Takes)] = &[
    ("--name", Takes::Value),
    ("--description", Takes::Value),
    ("--author", Takes::Value),
    ("--python", Takes::Value),
    ("--dependency", Takes::Value),
    ("--dev-dependency", Takes::Value),
    ("--license", Takes::Value),
    ("-n", Takes::Nothing),
    ("--no-interaction", Takes::Nothing),
    ("-q", Takes::Nothing),
    ("--quiet", Takes::Nothing),
    ("-h", Takes::Nothing),
    ("--help", Takes::Nothing),
];

/// A package manager's `init` that writes a manifest in the working folder and
/// nothing else. **Only the bare form runs:** a positional argument to
/// `npm init` / `yarn init` names an *initializer* package, which npm downloads
/// and runs (`npm init vite` is `npx create-vite`) — the very thing the
/// allowlist exists to stop — so a line that has one is refused. The bare form
/// only writes a file (and at most runs the user's own local init config, as
/// `git init` may run their global template), so it is safe to pass on. Options
/// fail closed, like git's.
fn manifest_init(
    command: &str,
    table: &[(&'static str, Takes)],
    manifest: &'static str,
    args: &[OsString],
    cwd: &Path,
) -> Result<Vec<Expected>, Refusal> {
    let positionals = scaffold_args(command, table, args)?;
    if let Some(extra) = positionals.first() {
        return Err(Refusal::refused(format!(
            "`{command} {}` runs a downloaded package, so dexily does not pass it on — only a \
             bare `{command}`, which just writes a {manifest}. Run it directly.",
            extra.to_string_lossy()
        )));
    }
    Ok(vec![Expected {
        folder: cwd.to_path_buf(),
        proof: Proof::Holds(manifest),
    }])
}

/// `npm` / `pnpm` / `yarn`: only their bare `init` runs — see [`manifest_init`].
/// Anything else (`npm install`, `npm run`, `npm create`) is unknown.
fn package_manager_init(
    tool: &str,
    args: &[OsString],
    cwd: &Path,
) -> Result<Vec<Expected>, Refusal> {
    if args.first().map(|a| a.to_string_lossy()).as_deref() != Some("init") {
        return Err(Refusal::Unknown);
    }
    manifest_init(
        &format!("{tool} init"),
        NODE_INIT_OPTIONS,
        "package.json",
        &args[1..],
        cwd,
    )
}

/// `poetry`: only `poetry init` runs — it writes a `pyproject.toml` and installs
/// nothing. `poetry new`, `poetry add` and the rest are unknown. (`pip` has no
/// project-init command of its own, so none is recognised for it.)
fn poetry_init(args: &[OsString], cwd: &Path) -> Result<Vec<Expected>, Refusal> {
    if args.first().map(|a| a.to_string_lossy()).as_deref() != Some("init") {
        return Err(Refusal::Unknown);
    }
    manifest_init(
        "poetry init",
        POETRY_INIT_OPTIONS,
        "pyproject.toml",
        &args[1..],
        cwd,
    )
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
