//! Runs the wrapped command with inherited stdio and hands back its exit
//! status untouched.

use std::{
    ffi::OsString,
    process::{Command, ExitStatus},
    sync::Once,
};

use anyhow::{bail, Context};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

/// Runs `argv` — a program and its arguments — with this terminal's stdin,
/// stdout and stderr, waits for it, and returns its exit code.
///
/// Each argument is handed over on its own, with no shell between: a folder
/// named `x; rm -rf ~` is one odd name, never a second command.
pub fn run(argv: &[OsString]) -> anyhow::Result<i32> {
    let Some((program, rest)) = argv.split_first() else {
        bail!("no command to run");
    };
    ignore_interrupts();
    let status = Command::new(program)
        .args(rest)
        .status()
        .with_context(|| format!("could not run `{}`", program.to_string_lossy()))?;
    Ok(exit_code(status))
}

/// The number a shell would report for `status`: the command's own exit code,
/// or for one ended by a signal (Unix only), 128 + the signal's number — Ctrl+C
/// is 130 — so a script can tell "cancelled" from "failed".
pub fn exit_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        return 128 + signal;
    }
    1
}

/// Ctrl+C reaches dexily as well as the command, since they share the
/// terminal. Without this dexily would die on the spot, before it could report
/// the command's exit code or record anything; with it, the command alone
/// decides what Ctrl+C means, and dexily carries on once it has exited.
///
/// Installed once: a process may hold only one handler. If it cannot be
/// installed, Ctrl+C simply ends dexily too, as it would without it — never a
/// reason to refuse to run the command.
fn ignore_interrupts() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = ctrlc::set_handler(|| {});
    });
}
