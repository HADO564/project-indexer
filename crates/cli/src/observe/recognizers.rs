//! Recognizers: given argv, the working directory and the exit status, decide
//! whether a command created a project and which directory it is.
//!
//! Candidates for the first set: `git init`, `git clone`, `mkdir`. Each one's
//! project-directory rule is written down beside it.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The commands dexily observes, for the message that refuses any other. One
/// name per recognizer in [`expect`].
pub const KNOWN: &[&str] = &["mkdir"];

/// What a command should leave behind, worked out before it runs.
#[derive(Debug, PartialEq, Eq)]
pub struct Expectation {
    pub folders: Vec<PathBuf>,
}

/// The folders `argv` should create, or `None` when dexily has no recognizer
/// for it — which is what keeps unknown commands from running at all.
///
/// A known command that names no folder (`mkdir`, `mkdir --help`) is still
/// known: it runs, says whatever it says, and there is nothing to track.
///
/// The program must be named bare (`mkdir`), as found on `PATH`: a path such as
/// `/tmp/x/mkdir` is some other program that happens to share the name, so it
/// matches nothing.
pub fn expect(argv: &[OsString], cwd: &Path) -> Option<Expectation> {
    let (program, rest) = argv.split_first()?;
    let program = program.to_str()?;
    let folders = if cfg!(windows) {
        // cmd's names ignore case, and `md` is its other name for mkdir.
        match program.to_ascii_lowercase().as_str() {
            "mkdir" | "md" => cmd_mkdir(rest),
            _ => return None,
        }
    } else {
        match program {
            "mkdir" => mkdir(rest),
            _ => return None,
        }
    };
    Some(Expectation {
        // `join` leaves an absolute path as it is and puts a relative one
        // under `cwd`, where mkdir itself would make it.
        folders: folders.iter().map(|name| cwd.join(name)).collect(),
    })
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

/// The folders to track: those the command was expected to create, that were
/// not there before it ran, and are there now — and only if it succeeded.
///
/// `existed_before` is one flag per expected folder, in the same order, taken
/// before the command ran: afterwards a folder just made looks exactly like
/// one that was always there (`mkdir -p work/new` must not track an old
/// `work`).
pub fn confirm(expected: &Expectation, existed_before: &[bool], exit_code: i32) -> Vec<PathBuf> {
    if exit_code != 0 {
        // The command failed: whatever it left behind, it is not a project.
        return Vec::new();
    }
    let mut confirmed = Vec::new();
    for (folder, &existed) in expected.folders.iter().zip(existed_before) {
        if !existed && folder.is_dir() {
            confirmed.push(folder.clone());
        }
    }
    confirmed
}
