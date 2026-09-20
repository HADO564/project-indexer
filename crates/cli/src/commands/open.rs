//! `open <project>`: hand a project's directory to its application.

use clap::Args;

use super::{find_one, Outcome, TrackerKind};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct OpenArgs {
    /// The project to open.
    pub project: String,

    /// Only consider projects with one of these trackers when matching.
    /// Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,
}

pub fn run(args: OpenArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let project = find_one(ctx, args.project, super::unique_kinds(&args.tracker))?;

    // Core does the rest: it checks the directory still exists, checks the
    // project's `open_with` app can still be launched (a specific "app is
    // missing" error beats a generic launch failure), launches it, and stamps
    // `last_opened_at`. The CLI supplies only the launcher.
    let project = ctx.projects.open(&project.id)?;
    Ok(Outcome::Opened {
        project: Box::new(project),
    })
}
