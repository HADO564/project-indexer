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

use crate::commands::Failure;

pub fn run(argv: &[OsString]) -> anyhow::Result<ExitCode> {
    // The gatekeeper: only a command with a recognizer runs. Anything else is
    // refused here, before it starts — it would gain nothing from dexily, and
    // dexily must not be a way to run arbitrary programs.
    let cwd = std::env::current_dir()?;
    let Some(_expectation) = recognizers::expect(argv, &cwd) else {
        let program = argv
            .first()
            .map(|p| p.to_string_lossy())
            .unwrap_or_default();
        return Err(Failure::Usage {
            message: format!(
                "`{program}` isn't a command dexily observes (it knows: mkdir). Run it directly."
            ),
        }
        .into());
    };
    let code = spawn::run(argv)?;
    Ok(ExitCode::from(code as u8))
}
