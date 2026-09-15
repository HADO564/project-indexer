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

/// `folder_color` only affects human output; `--json` is never coloured.
pub fn print(outcome: &Outcome, format: Format, folder_color: Color) -> anyhow::Result<()> {
    let mut out = std::io::stdout().lock();
    match format {
        Format::Human => human::write(&mut out, outcome, folder_color)?,
        Format::Json => json::write(&mut out, outcome)?,
    }
    out.flush()?;
    Ok(())
}
