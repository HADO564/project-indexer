//! `edit <project> --description …`: change the fields of a project that take
//! a value.
//!
//! The other half of the hybrid rule `favorite.rs` follows: if it takes a
//! value, it is an `edit` flag. Every flag joins the `change` group, which
//! requires at least one, so a bare `edit app` is a usage error rather than a
//! write that changes nothing. (A bare `edit` in a terminal is later to open a
//! full-screen form instead — `docs/cli/checklist.md`.) All the flags given
//! become one `update`, one read and one write, however many there are.

use clap::{ArgGroup, Args};
use indexer_core::domain::UpdateProject;

use super::{find_one, Outcome, TrackerKind};
use crate::context::Context;

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("change").required(true).multiple(true)))]
pub struct EditArgs {
    /// The project — an exact name, an id, a parent/folder path ending, or
    /// part of a name.
    pub project: String,

    /// Replace the description. An empty string clears it.
    #[arg(long, group("change"))]
    pub description: Option<String>,

    /// Only consider projects with one of these trackers when matching.
    /// Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,
}

pub fn run(args: EditArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let project = find_one(ctx, args.project, super::unique_kinds(&args.tracker))?;
    // Every other field is `None`, which `update` reads as "leave alone". It
    // returns the project as saved — with its new `updated_at` — so that copy
    // replaces the one `find_one` returned.
    let project = ctx.projects.update(
        &project.id,
        UpdateProject {
            description: args.description,
            ..Default::default()
        },
    )?;
    Ok(Outcome::Edited {
        project: Box::new(project),
    })
}
