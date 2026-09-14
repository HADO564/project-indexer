//! The command layer: one module per command, each holding its arguments and
//! its `run`.
//!
//! The shell and the TUI's `:` line both parse into [`Command`], and [`run`]
//! returns an [`Outcome`] without printing anything — rendering belongs to the
//! caller (`output/` for a shell, the message line for the TUI).

mod add;
mod config;
mod list;
mod open;
mod show;
mod untrack;

use anyhow::anyhow;
use clap::Subcommand;
use indexer_core::Project;

use crate::context::Context;
use crate::output::color::Color;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List tracked projects.
    List(list::ListArgs),
    /// Show one project's details.
    Show(show::ShowArgs),
    /// Start tracking a directory.
    Add(add::AddArgs),
    /// Open a project in its application.
    Open(open::OpenArgs),
    /// Stop tracking a project, leaving its files alone.
    Untrack(untrack::UntrackArgs),
    /// Show or change the CLI's settings.
    Config(config::ConfigArgs),
}

/// What a command produced. Add variants as commands need them.
#[derive(Debug)]
pub enum Outcome {
    Projects(Vec<Project>),
    Project(Box<Project>),
    /// The default folder colour now in effect.
    FolderColor(Color),
    /// Succeeded with nothing to show.
    Done,
}

pub fn run(command: Command, ctx: &Context) -> anyhow::Result<Outcome> {
    match command {
        Command::List(args) => list::run(args, ctx),
        Command::Show(args) => show::run(args, ctx),
        Command::Add(args) => add::run(args, ctx),
        Command::Open(args) => open::run(args, ctx),
        Command::Untrack(args) => untrack::run(args, ctx),
        Command::Config(args) => config::run(args, ctx),
    }
}

/// The error every stub returns until its command is written.
fn not_implemented(name: &str) -> anyhow::Error {
    anyhow!("`{name}` is not implemented yet")
}
