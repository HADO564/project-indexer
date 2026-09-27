//! `restore <project>`: bring a project back from the bin.
//!
//! The bin holds projects whose folder was deleted from disk with the record
//! kept (`ProjectService::delete_directory`), so this recovers the record, not
//! the files: the restored project points at a directory that is usually gone,
//! and `open` on it fails its health check, correctly. Reversible, so it does
//! not ask first.

use clap::Args;

use super::{find_in, Corpus, Outcome, TrackerKind};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct RestoreArgs {
    /// The binned project — an exact name, an id, a parent/folder path ending,
    /// or part of a name, matched only against the bin.
    pub project: String,

    /// Only consider projects with one of these trackers when matching.
    /// Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,
}

pub fn run(args: RestoreArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    // Resolved against the bin: the live list filters binned projects out, so
    // `find_one` would answer "no project matches" for the very project named.
    let project = find_in(
        ctx,
        Corpus::Bin,
        args.project,
        super::unique_kinds(&args.tracker),
    )?;
    let project = ctx.projects.restore(&project.id)?;
    Ok(Outcome::Restored {
        project: Box::new(project),
    })
}
