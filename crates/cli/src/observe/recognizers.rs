//! Recognizers: given argv, the working directory and the exit status, decide
//! whether a command created a project and which directory it is.
//!
//! Candidates for the first set: `git init`, `git clone`, `mkdir`. Each one's
//! project-directory rule is written down beside it.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// What a command should leave behind, worked out before it runs.
#[derive(Debug, PartialEq, Eq)]
pub struct Expectation {
    pub folders: Vec<PathBuf>,
}

/// The folders `argv` should create, or `None` when dexily has no recognizer
/// for it — which is what keeps unknown commands from running at all.
///
/// The program must be named bare (`mkdir`), as found on `PATH`: a path such as
/// `/tmp/x/mkdir` is some other program that happens to share the name, so it
/// matches nothing.
pub fn expect(argv: &[OsString], cwd: &Path) -> Option<Expectation> {
    let (program, rest) = argv.split_first()?;
    let folders = match program.to_str()? {
        "mkdir" => mkdir(rest),
        _ => return None,
    };
    if folders.is_empty() {
        return None;
    }
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
