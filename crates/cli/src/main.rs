//! The `indexer` binary.
//!
//! No arguments opens the TUI. A known subcommand parses into a [`Command`]
//! and runs once. Anything else is a command to run and observe.

// Scaffolding: most items are stubs nothing calls yet. Remove this once the
// first commands are implemented, so real dead code shows up again.
#![allow(dead_code)]

mod commands;
mod confirm;
mod context;
mod launcher;
mod observe;
mod output;
mod paths;
mod settings;
mod tui;

#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::commands::Command;
use crate::confirm::StdinConfirmer;
use crate::context::Context;
use crate::output::color::Color;
use crate::output::Format;

#[derive(Debug, Parser)]
#[command(
    name = "indexer",
    version,
    about = "Track your projects from the terminal"
)]
struct Cli {
    /// Print a versioned JSON document instead of human-readable output.
    #[arg(long, global = true)]
    json: bool,

    /// Answer yes to every confirmation prompt.
    #[arg(long, short, global = true)]
    yes: bool,

    /// The colour used to highlight each project's folder name, for this run
    /// only. Without it, the default set by `indexer config folder-color`
    /// applies, else cyan. `indexer config folder-color --help` lists the colours.
    #[arg(long, value_enum, global = true, hide_possible_values = true)]
    folder_color: Option<Color>,

    #[command(subcommand)]
    invocation: Option<Invocation>,
}

#[derive(Debug, Subcommand)]
enum Invocation {
    #[command(flatten)]
    Command(Command),

    /// Not an indexer command: run it, then record what it created.
    #[command(external_subcommand)]
    Observe(Vec<OsString>),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            // `{:#}` prints the whole cause chain, so core's own message —
            // the version-skew guard above all — reaches the user intact.
            eprintln!("indexer: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    match cli.invocation {
        None => {
            let ctx = Context::open(Box::new(StdinConfirmer::new(cli.yes)))?;
            tui::run(&ctx)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Invocation::Command(command)) => {
            let ctx = Context::open(Box::new(StdinConfirmer::new(cli.yes)))?;
            let outcome = commands::run(command, &ctx)?;
            let folder_color = resolve_folder_color(cli.folder_color);
            output::print(&outcome, Format::from_json_flag(cli.json), folder_color)?;
            Ok(ExitCode::SUCCESS)
        }
        // The observer opens the database itself, *after* the wrapped command
        // has run: a broken database must never stop `git init` from running.
        Some(Invocation::Observe(argv)) => observe::run(&argv),
    }
}

/// The flag if given, else the saved default, else `Color::DEFAULT`.
///
/// A broken settings file is reported and skipped: a colour preference must
/// never stop `indexer list` from listing.
fn resolve_folder_color(flag: Option<Color>) -> Color {
    if let Some(color) = flag {
        return color;
    }
    let saved = settings::load().unwrap_or_else(|e| {
        eprintln!("indexer: ignoring settings: {e:#}");
        settings::Settings::default()
    });
    saved.folder_color.unwrap_or(Color::DEFAULT)
}
