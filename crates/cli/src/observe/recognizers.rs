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
        self.folders.iter().map(Expected::holds).collect()
    }
}

impl Expected {
    fn holds(&self) -> bool {
        match self.proof {
            Proof::NewFolder => self.folder.exists(),
            Proof::Holds(entry) => self.folder.join(entry).exists(),
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

/// The command as a person would name it — `mkdir`, `git clone` — for
/// dexily's own lines.
pub fn label(argv: &[OsString]) -> String {
    let program = argv
        .first()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    if program == "git" {
        if let Some(subcommand) = git_line(&argv[1..], Path::new("")).subcommand {
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
    let (program, rest) = argv.split_first().ok_or(Refusal::Unknown)?;
    let program = program.to_str().ok_or(Refusal::Unknown)?;
    // cmd's names ignore case on Windows, and `md` is its other name for
    // mkdir.
    let name = if windows {
        program.to_ascii_lowercase()
    } else {
        program.to_string()
    };
    let folders = match name.as_str() {
        "mkdir" if !windows => new_folders(mkdir(rest), cwd),
        "mkdir" | "md" if windows => new_folders(cmd_mkdir(rest), cwd),
        "git" | "git.exe" => git(rest, cwd)?,
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
    /// The folder git will work in: `-C` moves it, and may repeat.
    base: PathBuf,
    /// `init`, `clone`, … or `None` when the line has none (`git --version`).
    subcommand: Option<String>,
    /// The arguments after the subcommand.
    rest: &'a [OsString],
    /// A global option that makes git run another program, if one was given.
    unsafe_option: Option<String>,
}

fn git_line<'a>(args: &'a [OsString], cwd: &Path) -> GitLine<'a> {
    let mut line = GitLine {
        base: cwd.to_path_buf(),
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
        let option = text.split('=').next().unwrap_or_default();
        match option {
            "-C" => {
                if let Some(path) = args.get(at) {
                    line.base = line.base.join(path);
                }
                at += 1;
            }
            // A setting can name a program for git to run (`core.sshCommand`,
            // `core.fsmonitor`, an alias), and `--exec-path` swaps git's own.
            "-c" | "--config-env" | "--exec-path" if option == "-c" || text.contains('=') => {
                line.unsafe_option.get_or_insert_with(|| option.to_string());
                if option == "-c" {
                    at += 1;
                }
            }
            // Options whose value is the next argument.
            "--git-dir" | "--work-tree" | "--namespace" | "--config-env" | "--super-prefix"
                if !text.contains('=') =>
            {
                at += 1
            }
            _ => {}
        }
    }
    line
}

/// Git: only `init` and `clone` make a project, so only they run. Their proof
/// is a `.git` in the folder, which may have existed already.
///
/// Refused outright: the options that make git run another program — a
/// `-c`/`--config` setting (`core.sshCommand`, `core.fsmonitor`, an alias),
/// `--upload-pack`, a `--template` whose hooks a clone would run, and the
/// `ext::` and `fd::` transports. dexily adds nothing to those, and an
/// allowlist that let them through would not be one.
fn git(args: &[OsString], cwd: &Path) -> Result<Vec<Expected>, Refusal> {
    let GitLine {
        base,
        subcommand,
        rest,
        unsafe_option,
    } = git_line(args, cwd);
    // Which subcommand comes first: `git push` is unknown whatever its
    // options are.
    if !matches!(subcommand.as_deref(), Some("init" | "clone")) {
        return Err(Refusal::Unknown);
    }
    if let Some(option) = unsafe_option {
        return Err(refused(&option));
    }
    let (valued, positionals_wanted): (&[&str], usize) = match subcommand.as_deref() {
        Some("init") => (
            &[
                "--template",
                "--separate-git-dir",
                "--object-format",
                "--ref-format",
                "-b",
                "--initial-branch",
            ],
            1,
        ),
        Some("clone") => (
            &[
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
            2,
        ),
        _ => unreachable!("only init and clone reach here"),
    };
    let clone = subcommand.as_deref() == Some("clone");

    let mut positionals: Vec<&OsString> = Vec::new();
    let mut bare = false;
    let mut options_done = false;
    let mut rest = rest.iter();
    while let Some(arg) = rest.next() {
        let text = arg.to_string_lossy();
        if options_done || !text.starts_with('-') || text == "-" {
            positionals.push(arg);
            continue;
        }
        let option = text.split('=').next().unwrap_or_default();
        match option {
            "--" => options_done = true,
            "--bare" | "--mirror" => bare = true,
            "-u" | "--upload-pack" | "-c" | "--config" => return Err(refused(option)),
            // A clone checks the new repository out, which runs the
            // template's hooks; `init` only copies them.
            "--template" if clone => return Err(refused(option)),
            _ if valued.contains(&option) && !text.contains('=') => {
                rest.next();
            }
            _ => {}
        }
    }
    positionals.truncate(positionals_wanted);

    // A bare repository is not a folder anyone works in: it runs, and there
    // is nothing to track.
    if bare {
        return Ok(Vec::new());
    }
    let folder = if clone {
        let Some(repository) = positionals.first() else {
            return Ok(Vec::new()); // `git clone` alone: git prints its usage
        };
        let repository = repository.to_string_lossy();
        if repository.starts_with("ext::") || repository.starts_with("fd::") {
            return Err(refused(&repository));
        }
        match positionals.get(1) {
            Some(directory) => base.join(directory),
            None => match humanish(&repository) {
                Some(name) => base.join(name),
                None => return Ok(Vec::new()),
            },
        }
    } else {
        match positionals.first() {
            Some(directory) => base.join(directory),
            None => base,
        }
    };
    Ok(vec![Expected {
        folder,
        proof: Proof::Holds(".git"),
    }])
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
/// `held_before` is [`Expectation::before`], taken before the command ran. A
/// new folder must not have been there (`mkdir -p work/new` must not track an
/// old `work`); a `.git` may have been — `git init` in a repository is still a
/// project to track — and what matters is that it is there now.
pub fn confirm(expected: &Expectation, held_before: &[bool], exit_code: i32) -> Vec<PathBuf> {
    if exit_code != 0 {
        // The command failed: whatever it left behind, it is not a project.
        return Vec::new();
    }
    let mut confirmed = Vec::new();
    for (expected, &held) in expected.folders.iter().zip(held_before) {
        let proven = match expected.proof {
            Proof::NewFolder => !held && expected.folder.is_dir(),
            Proof::Holds(entry) => expected.folder.join(entry).exists(),
        };
        if proven {
            confirmed.push(expected.folder.clone());
        }
    }
    confirmed
}
