//! `favorite <project>` and `unfavorite <project>`: set or clear a project's
//! favourite flag.
//!
//! Two verbs rather than an `edit --favorite` flag, by the rule in the
//! write-commands handoff: a state change with no value is a verb. That also
//! makes favouriting a command the TUI can bind `f` to. One `run` serves both,
//! taking the flag to store, so the two cannot drift apart.

use clap::Args;
use indexer_core::domain::UpdateProject;

use super::{find_one, Outcome, TrackerKind};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct FavoriteArgs {
    /// The project — an exact name, an id, a parent/folder path ending, or
    /// part of a name.
    pub project: String,

    /// Only consider projects with one of these trackers when matching.
    /// Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,
}

pub fn run(args: FavoriteArgs, ctx: &Context, favorite: bool) -> anyhow::Result<Outcome> {
    let project = find_one(ctx, args.project, super::unique_kinds(&args.tracker))?;
    // Every other field is `None`, which `update` reads as "leave alone". It
    // returns the project as saved — with its new `updated_at` — so that copy
    // replaces the one `find_one` returned.
    let project = ctx.projects.update(
        &project.id,
        UpdateProject {
            favorite: Some(favorite),
            ..Default::default()
        },
    )?;
    Ok(Outcome::Favorited {
        project: Box::new(project),
        favorite,
    })
}
