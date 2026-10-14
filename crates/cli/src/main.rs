//! The `dexily` binary.
//!
//! No arguments opens the TUI. A known subcommand parses into a [`Command`]
//! and runs once. Anything else is a command to run and observe.

mod appearance;
mod commands;
mod confirm;
mod context;
mod editor;
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

use crate::commands::{Command, Failure};
use crate::confirm::StdinConfirmer;
use crate::context::Context;
use crate::editor::TerminalEditor;
use crate::observe::flags::ObserveFlags;
use crate::output::color::Color;
use crate::output::{Format, Look};

#[derive(Debug, Parser)]
#[command(
    name = "dexily",
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

    /// With an observed command (`dexily mkdir thing`): none of dexily's own
    /// lines — never the command's output, and never a warning.
    #[arg(long, short, global = true)]
    quiet: bool,

    /// The colour used to highlight each project's folder name, for this run
    /// only. Without it, the default set by `dexily config folder-color`
    /// applies, else cyan. `dexily config folder-color --help` lists the colours.
    #[arg(long, value_enum, global = true, hide_possible_values = true)]
    folder_color: Option<Color>,

    /// The colour of table headers, for this run only. Without it, the default
    /// set by `dexily config header-color` applies, else magenta.
    #[arg(long, value_enum, global = true, hide_possible_values = true)]
    header_color: Option<Color>,

    /// dexily's flags for the project an observed command creates. They go
    /// before the command; everything after its name is the command's own.
    #[command(flatten)]
    observe: ObserveFlags,

    #[command(subcommand)]
    invocation: Option<Invocation>,
}

// `Command` is far larger than `Observe`, which clippy flags because every
// value takes the larger size. There is exactly one, parsed once per run, so
// the bytes do not matter; boxing it would only add a `*` to every match.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum Invocation {
    #[command(flatten)]
    Command(Command),

    /// Not a dexily command: run it, then record what it created.
    #[command(external_subcommand)]
    Observe(Vec<OsString>),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let format = Format::from_json_flag(cli.json);
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            // A usage error is prose with exit code 2, as clap's own are —
            // even under `--json`, which the contract reserves for results
            // and for failures a script can act on.
            if let Some(Failure::Usage { message }) = e.downcast_ref::<Failure>() {
                // An empty message is already-reported: the observer's override
                // prompt showed the reason, the user declined, and there is
                // nothing to add — just the exit code.
                if !message.is_empty() {
                    eprintln!("dexily: {message}");
                }
                return ExitCode::from(2);
            }
            // The whole cause chain is printed, so core's own message — the
            // version-skew guard above all — reaches the user intact.
            output::print_error(&e, format);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    // The project flags before a command belong to an observed one: on
    // dexily's own commands there is no new project for them to describe.
    if !cli.observe.is_empty() && !matches!(cli.invocation, Some(Invocation::Observe(_))) {
        return Err(Failure::Usage {
            message: "flags like --add-tag or --group before a command describe the project \
                      an observed command creates (dexily --add-tag rust mkdir thing); to \
                      change a project, use `dexily edit <project>`"
                .to_string(),
        }
        .into());
    }
    match cli.invocation {
        None => {
            let ctx = Context::open(
                Box::new(StdinConfirmer::new(cli.yes)),
                Box::new(TerminalEditor::new(cli.json)),
            )?;
            tui::run(&ctx)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Invocation::Command(command)) => {
            let ctx = Context::open(
                Box::new(StdinConfirmer::new(cli.yes)),
                Box::new(TerminalEditor::new(cli.json)),
            )?;
            let outcome = commands::run(command, &ctx)?;
            let look = resolve_look(cli.folder_color, cli.header_color);
            output::print(&outcome, Format::from_json_flag(cli.json), look)?;
            Ok(ExitCode::SUCCESS)
        }
        // The observer opens the database itself, *after* the wrapped command
        // has run: a broken database must never stop `git init` from running.
        Some(Invocation::Observe(argv)) => observe::run(
            &argv,
            observe::Options {
                yes: cli.yes,
                json: cli.json,
                quiet: cli.quiet,
                flags: cli.observe,
            },
        ),
    }
}

/// Each colour's flag if given, else its saved default, else the built-in
/// one; the icons as saved, else off.
///
/// A broken settings file is reported and skipped: a look preference must
/// never stop `dexily list` from listing.
fn resolve_look(folder: Option<Color>, header: Option<Color>) -> Look {
    let saved = settings::load().unwrap_or_else(|e| {
        eprintln!("dexily: ignoring settings: {e:#}");
        settings::Settings::default()
    });
    Look {
        icons: saved.icons.unwrap_or_default(),
        folder: folder
            .or(saved.folder_color)
            .unwrap_or(Color::DEFAULT_FOLDER),
        header: header
            .or(saved.header_color)
            .unwrap_or(Color::DEFAULT_HEADER),
    }
}
