//! Renders an [`Outcome`] for a shell. stdout carries the result and nothing
//! else; prose and errors go to stderr.

pub mod color;
pub mod human;
mod json;

use std::io::Write;

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

/// The colours human output uses, each already resolved from its flag, the
/// settings file, or the built-in default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    pub folder: Color,
    pub header: Color,
}

/// `colors` only affects human output; `--json` is never coloured.
pub fn print(outcome: &Outcome, format: Format, colors: Colors) -> anyhow::Result<()> {
    let mut out = std::io::stdout().lock();
    match format {
        Format::Human => human::write(&mut out, outcome, colors)?,
        Format::Json => json::write(&mut out, outcome)?,
    }
    out.flush()?;
    Ok(())
}
