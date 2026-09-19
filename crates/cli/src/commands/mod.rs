//! The command layer: one module per command, each holding its arguments and
//! its `run`.
//!
//! The shell and the TUI's `:` line both parse into [`Command`], and [`run`]
//! returns an [`Outcome`] without printing anything — rendering belongs to the
//! caller (`output/` for a shell, the message line for the TUI).

mod add;
mod config;
mod failure;
mod list;
mod open;
pub mod scan;
mod show;
mod untrack;

use anyhow::anyhow;
use clap::{Subcommand, ValueEnum};
use indexer_core::application::ImportReport;
use indexer_core::domain::scan::ScanReport;
use indexer_core::Project;

use crate::context::Context;
use crate::output::color::Color;

pub use failure::Failure;

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
    /// Find projects under a directory, and optionally register them.
    Scan(scan::ScanArgs),
    /// Show or change the CLI's settings.
    Config(config::ConfigArgs),
}

/// What a command produced. Add variants as commands need them.
#[derive(Debug)]
pub enum Outcome {
    /// Projects to list. `query` is what they were filtered by, if anything,
    /// so an empty list can say "nothing matched" rather than "nothing tracked".
    Projects {
        projects: Vec<Project>,
        query: Option<String>,
        tracker: Vec<TrackerKind>,
    },
    /// One project's details. `tracker` is the kinds `--tracker` asked to
    /// expand, so the view can show a section per kind.
    Project {
        project: Box<Project>,
        tracker: Vec<TrackerKind>,
    },
    /// What a scan registered: the projects created, how many directories
    /// were already tracked, and the rows that failed.
    Imported { report: ImportReport },
    /// What a scan found, and the folder it looked in. Nothing is registered:
    /// importing is a second, deliberate run.
    Scanned { root: String, report: ScanReport },
    /// A colour setting now in effect, after `config` showed or changed it.
    Color { setting: ColorSetting, color: Color },
    /// Succeeded with nothing to show.
    Done,
}

/// The colours a user can set with `indexer config`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSetting {
    /// Each project's folder name in a table.
    Folder,
    /// A table's header row.
    Header,
}

impl ColorSetting {
    /// The key in `cli-settings.json`, and in `--json` output.
    pub fn key(self) -> &'static str {
        match self {
            ColorSetting::Folder => "folder_color",
            ColorSetting::Header => "header_color",
        }
    }

    /// The colour used when nothing has been set.
    pub fn default_color(self) -> Color {
        match self {
            ColorSetting::Folder => Color::DEFAULT_FOLDER,
            ColorSetting::Header => Color::DEFAULT_HEADER,
        }
    }
}

/// The tracker kinds `--tracker` accepts, as `git` and `unreal` on the
/// command line.
///
/// One flag taking a value, rather than a `--git` and a `--unreal` flag, so a
/// new detector adds a variant here instead of a flag and the rules for
/// combining it with the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TrackerKind {
    Git,
    Unreal,
}

impl TrackerKind {
    /// The kind as [`indexer_core::Tracker::is`] compares it — lowercase, the
    /// spelling the user types and `--json` prints.
    pub fn kind(self) -> &'static str {
        match self {
            TrackerKind::Git => "git",
            TrackerKind::Unreal => "unreal",
        }
    }
}

/// Keeps the projects carrying any of `kinds`, or every project when no
/// `--tracker` was given (an empty `kinds`).
///
/// Any rather than all: `--tracker git,unreal` reads as "these are the kinds
/// I care about", so it widens the view. Requiring every kind would instead
/// hide all but the projects that happen to be both.
///
/// Commands narrow the corpus with this *before* matching a query, so
/// `show app --tracker git` finds the git `app` rather than reporting it as
/// ambiguous with an Unreal one and then dropping half the answer.
pub fn with_tracker(projects: Vec<Project>, kinds: &[TrackerKind]) -> Vec<Project> {
    if kinds.is_empty() {
        return projects;
    }
    projects
        .into_iter()
        .filter(|p| {
            p.trackers
                .iter()
                .any(|t| kinds.iter().any(|kind| t.is(kind.kind())))
        })
        .collect()
}

/// The kinds in the order given, without repeats — `-t git -t git` must not
/// print BRANCH and CHANGES twice.
pub fn unique_kinds(kinds: &[TrackerKind]) -> Vec<TrackerKind> {
    let mut unique: Vec<TrackerKind> = Vec::new();
    for kind in kinds {
        if !unique.contains(kind) {
            unique.push(*kind);
        }
    }
    unique
}

pub fn run(command: Command, ctx: &Context) -> anyhow::Result<Outcome> {
    match command {
        Command::List(args) => list::run(args, ctx),
        Command::Show(args) => show::run(args, ctx),
        Command::Add(args) => add::run(args, ctx),
        Command::Open(args) => open::run(args, ctx),
        Command::Untrack(args) => untrack::run(args, ctx),
        Command::Scan(args) => scan::run(args, ctx),
        Command::Config(args) => config::run(args, ctx),
    }
}

/// The error every stub returns until its command is written.
fn not_implemented(name: &str) -> anyhow::Error {
    anyhow!("`{name}` is not implemented yet")
}
