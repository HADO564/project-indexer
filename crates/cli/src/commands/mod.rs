//! The command layer: one module per command, each holding its arguments and
//! its `run`.
//!
//! The shell and the TUI's `:` line both parse into [`Command`], and [`run`]
//! returns an [`Outcome`] without printing anything — rendering belongs to the
//! caller (`output/` for a shell, the message line for the TUI).

pub mod add;
mod config;
mod failure;
pub mod list;
mod open;
pub mod scan;
mod show;
mod untrack;

use clap::{Subcommand, ValueEnum};
use indexer_core::application::ImportReport;
use indexer_core::domain::matching::{resolve, Resolution};
use indexer_core::domain::scan::ScanReport;
use indexer_core::domain::sorting::SortOptions;
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
    /// and `view` is the set they were drawn from — between them an empty
    /// list can say which emptiness it is, rather than always claiming
    /// nothing is tracked.
    Projects {
        projects: Vec<Project>,
        query: Option<String>,
        tracker: Vec<TrackerKind>,
        view: View,
    },
    /// One project's details. `tracker` is the kinds `--tracker` asked to
    /// expand, so the view can show a section per kind.
    Project {
        project: Box<Project>,
        tracker: Vec<TrackerKind>,
        group: Option<GroupLabel>,
    },
    /// A directory now tracked. `already_tracked` distinguishes the two things
    /// `ensure_project` does, so the message can say which happened.
    Added {
        project: Box<Project>,
        already_tracked: bool,
    },
    /// A project whose metadata was forgotten. Its directory is untouched.
    Untracked { project: Box<Project> },
    /// A project handed to its application, with `last_opened_at` now set.
    Opened { project: Box<Project> },
    /// The user answered no to a confirmation. Not a failure: exit 0, because
    /// nothing went wrong and a script should not treat it as an error.
    Cancelled,
    /// What a scan registered: the projects created, how many directories
    /// were already tracked, and the rows that failed.
    Imported { report: ImportReport },
    /// What a scan found, and the folder it looked in. Nothing is registered:
    /// importing is a second, deliberate run.
    Scanned { root: String, report: ScanReport },
    /// A colour setting now in effect, after `config` showed or changed it.
    Color { setting: ColorSetting, color: Color },
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

/// Which set of projects `list` draws from.
///
/// One flag taking a value rather than a `--favorite` and a `--binned` flag:
/// the sets are alternatives, so a value makes choosing two of them
/// unsayable instead of an error to check for. It also leaves room for the
/// groups the GUI already has — `src/lib/views.ts` models exactly this as one
/// `View` type, with groups as a further variant carrying an id.
///
/// `Ungrouped` is deliberately absent: it is the only view with no core
/// method behind it, and inventing its rule here is what the `views.ts`
/// migration exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum View {
    /// Every tracked project.
    All,
    /// Only favourites. Binned projects are excluded even when favourited.
    Favorites,
    /// Only binned projects. They appear in no other view, so a project
    /// favourited and then binned is here rather than under `favorites` —
    /// restoring it puts it back.
    Binned,
}

impl View {
    /// The spelling clap parses, which is also what `--help` prints as the
    /// default — the two must agree, as they must for [`list::SortByKind`].
    pub fn name(self) -> &'static str {
        match self {
            View::All => "all",
            View::Favorites => "favorites",
            View::Binned => "binned",
        }
    }
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
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

/// The group a project belongs to, as a view needs it.
///
/// Resolved by the command, because `Project` stores only a `group_id` and a
/// renderer has no database to look it up in. `None` on [`Outcome::Project`]
/// means Ungrouped — or a `group_id` whose group has since gone, which is not
/// worth refusing to show a project over.
///
/// Every field is kept exactly as core stored it. Deciding what `"violet"`
/// looks like on this terminal, or whether there is a glyph for `"gamepad"`,
/// is the renderer's job — which is what lets the TUI answer differently from
/// the shell.
#[derive(Debug)]
pub struct GroupLabel {
    pub name: String,
    /// One of `domain::palette::SWATCHES` (`"violet"`), or `#rrggbb` from the
    /// colour picker.
    ///
    /// A `String` rather than an [`output::Color`](crate::output::color::Color):
    /// that enum derives `ValueEnum` so it can be a `--folder-color` value, so
    /// every variant is a bare name with no payload — it cannot represent an
    /// arbitrary hex literal at all.
    ///
    /// Carried before anything renders it: painting the group name needs
    /// `Color::from_swatch` and a hex parser, which are their own checklist
    /// item. Reaching that item should not mean reopening the command layer.
    #[allow(dead_code)]
    pub color: String,
    /// A bundled icon name (`"gamepad"`), or `custom:…` for an uploaded image
    /// that no terminal can draw and every renderer has to fall back on.
    ///
    /// Carried for the same reason as `color`, and further off: icons need a
    /// `config icons` setting and `text_width` counting display columns first.
    #[allow(dead_code)]
    pub icon: String,
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

/// The single project `query` names, or the [`Failure`] explaining why there
/// isn't one.
///
/// Shared by every command that acts on one project — `show`, `open`,
/// `untrack` — so a query resolves identically whichever verb is in front of
/// it. Three copies of this `match` was the point at which they could start to
/// drift.
///
/// `tracker` narrows the corpus first, for the reason [`with_tracker`] gives:
/// `open app -t git` should find the git `app` rather than report an ambiguity
/// with an Unreal one.
pub fn find_one(
    ctx: &Context,
    query: String,
    tracker: Vec<TrackerKind>,
) -> anyhow::Result<Project> {
    let projects = ctx.projects.list(SortOptions::default())?;
    let projects = with_tracker(projects, &tracker);
    match resolve(&projects, &query) {
        Resolution::Found(project) => Ok(project.clone()),
        Resolution::NotFound => Err(Failure::NotFound { query }.into()),
        Resolution::Ambiguous(matches) => Err(Failure::Ambiguous {
            query,
            matches: matches.into_iter().cloned().collect(),
            tracker,
        }
        .into()),
    }
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
