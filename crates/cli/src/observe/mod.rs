//! The observer: `indexer <cmd> [args…]` runs `<cmd>` with inherited stdio,
//! then matches argv, working directory and exit code against recognizers and
//! records what it inferred through core services.
//!
//! Rules: the wrapped command's exit code always wins, and a failure to record
//! never changes it. Recording goes through `ProjectService` directly, not
//! through `Command` — it is facts inferred after a command ran, not a command.

mod recognizers;
mod spawn;

use std::ffi::OsString;
use std::process::ExitCode;

use anyhow::bail;

pub fn run(argv: &[OsString]) -> anyhow::Result<ExitCode> {
    let Some(program) = argv.first() else {
        bail!("no command to run");
    };
    bail!(
        "`{}` is not an indexer command, and observing commands is not implemented yet",
        program.to_string_lossy()
    )
}
