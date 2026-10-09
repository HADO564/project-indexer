//! The observer: `dexily <cmd> [args…]` runs `<cmd>` with inherited stdio,
//! then matches argv, working directory and exit code against recognizers and
//! records what it inferred through core services.
//!
//! Rules: the wrapped command's exit code always wins, and a failure to record
//! never changes it. Recording goes through `ProjectService` directly, not
//! through `Command` — it is facts inferred after a command ran, not a command.

pub(crate) mod recognizers;
pub(crate) mod spawn;

use std::ffi::OsString;
use std::process::ExitCode;

pub fn run(argv: &[OsString]) -> anyhow::Result<ExitCode> {
    let code = spawn::run(argv)?;
    Ok(ExitCode::from(code as u8))
}
