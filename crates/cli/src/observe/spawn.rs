//! Runs the wrapped command with inherited stdio and hands back its exit
//! status untouched.

use std::process::{Command, ExitStatus};

use anyhow::{Context, Result};

pub fn run(command: &Command, args: &[String]) -> Result<ExitStatus> {
    let mut child = command.spawn()?;
    let all_args = args.join(" ");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(all_args.as_bytes())?;
    child.wait()
}
