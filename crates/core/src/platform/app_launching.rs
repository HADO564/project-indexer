//! Whether an `open_with` target can be launched, and how to launch it.
//!
//! Split out of `app_discovery`, which held both. Finding the applications on
//! a machine and running one are different jobs — and every test for this half
//! was sitting in a file named for the other.

#[cfg(target_os = "linux")]
use super::desktop_entry::{is_path_field_code, quote_arg, split_with};

/// Checks whether the app an `open_with` value points at can still be
/// launched, so a project configured to open with an app that's since been
/// uninstalled or moved can be caught before actually trying — and reported
/// as a specific "app is missing" error rather than a generic launch
/// failure.
///
/// On Linux `open_with` is a full command line (as stored by the picker, or
/// hand-typed), so only the program — the first token — is resolved. On
/// Windows/macOS it's just a path or bare command name.
pub fn open_with_app_available(open_with: &str) -> bool {
    let program = program_from_open_with(open_with);
    let program = program.trim();
    if program.is_empty() {
        return false;
    }
    command_exists(program)
}

#[cfg(target_os = "linux")]
fn program_from_open_with(open_with: &str) -> String {
    split_command(open_with)
        .into_iter()
        .next()
        .unwrap_or_default()
}

#[cfg(not(target_os = "linux"))]
fn program_from_open_with(open_with: &str) -> String {
    open_with.to_string()
}

/// Checks whether `program` exists: as a path on its own when it looks like
/// one (absolute, or containing a separator), otherwise by searching `PATH`
/// the way the OS would when launching a bare command name.
fn command_exists(program: &str) -> bool {
    use std::path::Path;

    let path = Path::new(program);
    if path.is_absolute() || program.contains(std::path::MAIN_SEPARATOR) {
        return path.is_file();
    }

    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    let extensions = windows_path_extensions();

    std::env::split_paths(&path_var).any(|dir| {
        dir.join(program).is_file()
            || extensions
                .iter()
                .any(|ext| dir.join(format!("{program}{ext}")).is_file())
    })
}

/// `PATHEXT` suffixes (`.EXE`, `.CMD`, …) that Windows tries in turn against
/// a bare command name when resolving it through `PATH`. Empty on other
/// platforms, where a bare name must match a `PATH` entry exactly.
#[cfg(windows)]
fn windows_path_extensions() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .filter(|ext| !ext.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(not(windows))]
fn windows_path_extensions() -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "linux")]
/// Splits a stored command line — one we produced with [`exec_command`],
/// or one the user typed into the "open with" field by hand.
///
/// Percent escaping is deliberately not applied here: the stored form
/// still contains real field codes like `%u` for [`open_with_command`] to
/// substitute, and leaving `%` alone keeps the round trip exact.
pub(crate) fn split_command(command: &str) -> Vec<String> {
    split_with(command, false)
}

#[cfg(target_os = "linux")]
/// Resolves a stored command line and a directory into the program and
/// argument list to spawn.
pub(crate) fn build_launch_args(
    command: &str,
    directory: &str,
) -> Result<(String, Vec<String>), String> {
    let mut args = split_command(command);
    if args.is_empty() {
        return Err(format!("'{}' is not a runnable command", command));
    }
    let program = args.remove(0);

    // Substitute the directory for the entry's file placeholder, keeping
    // any surrounding markers intact. Commands without a placeholder —
    // including anything the user typed by hand, like a bare `code` —
    // just take it as a trailing argument.
    let mut substituted = false;
    for arg in &mut args {
        if is_path_field_code(arg) {
            *arg = directory.to_string();
            substituted = true;
        }
    }
    if !substituted {
        args.push(directory.to_string());
    }

    Ok((program, args))
}

#[cfg(target_os = "linux")]
/// Launches `directory` with a specific application.
///
/// `open::with_detached`, which the opener plugin uses, runs the whole
/// `open_with` string as a single program name, so it can't launch the
/// multi-argument commands real `.desktop` entries use. Splitting the
/// stored command line ourselves and spawning it directly is what makes
/// Flatpak, Snap and Wine entries work.
pub fn open_with_command(directory: &str, command: &str) -> Result<(), String> {
    use std::os::unix::process::CommandExt as _;
    use std::process::{Command, Stdio};

    let (program, args) = build_launch_args(command, directory)?;

    let mut child = Command::new(&program)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // Own process group, so the editor we launch isn't taken down by
        // a Ctrl-C or terminal hangup aimed at this app.
        .process_group(0)
        .spawn()
        .map_err(|e| format!("Failed to launch '{}': {}", program, e))?;

    // Reap in the background: the child outlives this call, and without a
    // wait it would linger as a zombie for the lifetime of the app.
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}
