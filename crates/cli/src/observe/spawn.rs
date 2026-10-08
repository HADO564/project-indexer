//! Runs the wrapped command with inherited stdio and hands back its exit
//! status untouched.

use std::{ffi::OsString, process::Command};

use anyhow::{bail, Context};

/// Runs `argv` — a program and its arguments — with this terminal's stdin,
/// stdout and stderr, waits for it, and returns its exit code.
///
/// Each argument is handed over on its own, with no shell between: a folder
/// named `x; rm -rf ~` is one odd name, never a second command.
pub fn run(argv: &[OsString]) -> anyhow::Result<i32> {
    let Some((program, rest)) = argv.split_first() else {
        bail!("no command to run");
    };
    let status = Command::new(program)
        .args(rest)
        .status()
        .with_context(|| format!("could not run `{}`", program.to_string_lossy()))?;
    // `None` when a signal ended it (Unix only), which has no exit code.
    Ok(status.code().unwrap_or(1))
}
