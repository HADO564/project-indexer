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
    /// A command dexily observes, but with an option that makes it run some
    /// other program — which would make the allowlist mean nothing. The text
    /// says which, for the message.
    Unsafe(String),
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
        if let Some(subcommand) = git_line(&argv[1..]).subcommand {
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

/// Git's line before the subcommand's own arguments.
struct GitLine<'a> {
    /// Where `-C` moves git to, relative to the working folder. It may repeat.
    base: PathBuf,
    /// `init`, `clone`, … or `None` when the line has none (`git --version`).
    subcommand: Option<String>,
    /// The arguments after the subcommand.
    rest: &'a [OsString],
    /// A global option that makes git run another program, if one was given.
    unsafe_option: Option<String>,
}

fn git_line(args: &[OsString]) -> GitLine<'_> {
    let mut line = GitLine {
        base: PathBuf::new(),
        subcommand: None,
        rest: &[],
        unsafe_option: None,
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
        // `--name=value` carries its value; `--name value` takes the next
        // argument, which must then not be read as the subcommand.
        let (option, attached) = match text.split_once('=') {
            Some((option, _)) => (option, true),
            None => (text.as_ref(), false),
        };
        match option {
            "-C" => {
                if let Some(path) = args.get(at) {
                    line.base = line.base.join(path);
                }
                at += 1;
            }
            // A setting can name a program for git to run: `core.sshCommand`,
            // `core.fsmonitor`, an alias. `-c` always takes the next argument.
            "-c" => {
                line.unsafe_option.get_or_insert_with(|| option.to_string());
                at += 1;
            }
            // The same through the environment, in either spelling.
            "--config-env" => {
                line.unsafe_option.get_or_insert_with(|| option.to_string());
                if !attached {
                    at += 1;
                }
            }
            // With a value it swaps git's own programs; bare, it only prints
            // where they are.
            "--exec-path" if attached => {
                line.unsafe_option.get_or_insert_with(|| option.to_string());
            }
            "--git-dir" | "--work-tree" | "--namespace" | "--super-prefix" if !attached => at += 1,
            _ => {}
        }
    }
    line
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

    /// Its options whose value is the next argument, so that value is never
    /// read as the repository or the folder.
    fn valued(self) -> &'static [&'static str] {
        match self {
            Self::Init => &[
                "--template",
                "--separate-git-dir",
                "--object-format",
                "--ref-format",
                "-b",
                "--initial-branch",
            ],
            Self::Clone => &[
                "-b",
                "--branch",
                "-o",
                "--origin",
                "--reference",
                "--reference-if-able",
                "--separate-git-dir",
                "--depth",
                "--shallow-since",
                "--shallow-exclude",
                "-j",
                "--jobs",
                "--filter",
                "--server-option",
                "--bundle-uri",
                "--revision",
            ],
        }
    }

    /// Whether `option` makes this subcommand run another program.
    fn runs_a_program(self, option: &str) -> bool {
        match self {
            Self::Init => false,
            // `--upload-pack` names the program outright, a `-c` setting can
            // (`core.fsmonitor`), and a clone checks the new repository out,
            // which runs the template's hooks — `init` only copies them.
            Self::Clone => matches!(
                option,
                "-u" | "--upload-pack" | "-c" | "--config" | "--template"
            ),
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
    /// An option that makes it run another program, if one was given.
    unsafe_option: Option<String>,
}

fn git_args(command: GitCommand, args: &[OsString]) -> GitArgs<'_> {
    let mut parsed = GitArgs {
        positionals: Vec::new(),
        bare: false,
        unsafe_option: None,
    };
    let mut options_done = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        if options_done || !text.starts_with('-') || text == "-" {
            parsed.positionals.push(arg);
            continue;
        }
        let (option, attached) = match text.split_once('=') {
            Some((option, _)) => (option, true),
            None => (text.as_ref(), false),
        };
        if option == "--" {
            options_done = true;
        } else if matches!(option, "--bare" | "--mirror") {
            parsed.bare = true;
        } else if command.runs_a_program(option) {
            parsed
                .unsafe_option
                .get_or_insert_with(|| option.to_string());
        } else if command.valued().contains(&option) && !attached {
            args.next();
        }
    }
    parsed.positionals.truncate(command.positionals());
    parsed
}

/// Git: only `init` and `clone` make a project, so only they run. Their proof
/// is a `.git` in the folder, which may have existed already.
///
/// In two steps, in this order: first every reason to refuse the line, then —
/// only for a line that will run — the folder it should leave a project in.
/// Nothing about the folder can therefore let a refused line through.
fn git(args: &[OsString], cwd: &Path) -> Result<Vec<Expected>, Refusal> {
    let line = git_line(args);
    // Which subcommand comes first: `git push` is unknown whatever its
    // options are.
    let command = GitCommand::parse(line.subcommand.as_deref()).ok_or(Refusal::Unknown)?;
    let parsed = git_args(command, line.rest);
    refuse_git(command, &line, &parsed)?;

    let base = cwd.join(&line.base);
    Ok(git_folder(command, &base, &parsed)
        .map(|folder| Expected {
            folder,
            proof: Proof::Holds(".git"),
        })
        .into_iter()
        .collect())
}

/// Every reason not to run a git line: the options and addresses that make
/// git run another program. dexily adds nothing to those, and an allowlist
/// that let them through would not be one.
fn refuse_git(command: GitCommand, line: &GitLine, parsed: &GitArgs) -> Result<(), Refusal> {
    if let Some(option) = line
        .unsafe_option
        .as_ref()
        .or(parsed.unsafe_option.as_ref())
    {
        return Err(refused(option));
    }
    if command == GitCommand::Clone {
        if let Some(repository) = parsed.positionals.first() {
            let repository = repository.to_string_lossy();
            // Transports that are a command line or an open file handle.
            if repository.starts_with("ext::") || repository.starts_with("fd::") {
                return Err(refused(&repository));
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

fn refused(option: &str) -> Refusal {
    Refusal::Unsafe(format!(
        "`{option}` makes git run another program, so it is not passed on. Run git directly."
    ))
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
