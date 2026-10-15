//! Renders an [`Outcome`] for a shell. stdout carries the result and nothing
//! else; prose and errors go to stderr.

pub mod color;
pub mod human;
pub mod json;

use std::io::Write;

use crate::appearance::IconStyle;
use crate::commands::Outcome;
use crate::output::color::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Human,
    Json,
}

impl Format {
    pub fn from_json_flag(json: bool) -> Self {
        if json {
            Self::Json
        } else {
            Self::Human
        }
    }
}

/// How human output looks, each part already resolved from its flag, the
/// settings file, or the built-in default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Look {
    pub folder: Color,
    pub header: Color,
    pub icons: IconStyle,
}

/// The result goes to stdout; dexily's own prose goes to `err`, supplied by the
/// caller — the real stderr in a shell, the message line in the TUI. `look`
/// only affects human output; `--json` is never coloured.
pub fn print(
    outcome: &Outcome,
    format: Format,
    look: Look,
    err: &mut dyn Write,
) -> anyhow::Result<()> {
    let mut out = std::io::stdout().lock();
    match format {
        Format::Human => human::write(&mut out, err, outcome, look)?,
        Format::Json => json::write(&mut out, outcome)?,
    }
    out.flush()?;
    Ok(())
}

/// Reports a failed command: prose, or under `--json` the
/// `{"schema": 1, "error": …}` document, so a script can tell a query that
/// matched nothing from one that matched several without parsing prose. Writes
/// to `err` — the real stderr in a shell, the TUI's message line otherwise.
pub fn print_error(error: &anyhow::Error, format: Format, err: &mut dyn Write) {
    match format {
        Format::Human => {
            let _ = writeln!(err, "dexily: {error:#}");
        }
        Format::Json => {
            if json::write_error(err, error).is_err() {
                let _ = writeln!(err, "dexily: {error:#}");
            }
        }
    }
}
