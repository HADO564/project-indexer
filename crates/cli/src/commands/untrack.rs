//! `untrack <project>`: forget a project's metadata, leaving its files alone.

use clap::Args;

use super::{find_one, Outcome, TrackerKind};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct UntrackArgs {
    /// The project to stop tracking.
    pub project: String,

    /// Only consider projects with one of these trackers when matching.
    /// Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,
}

pub fn run(args: UntrackArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let project = find_one(ctx, args.project, super::unique_kinds(&args.tracker))?;

    // The first caller of the confirmer, which has been wired into `Context`
    // since the module skeleton. The prompt names the directory as well as the
    // project, because a query that matched one of several similar names is
    // exactly when the user wants to see which one they are about to lose.
    let prompt = format!(
        "stop tracking \"{}\" at {}? its files are left alone",
        project.name, project.directory
    );
    if !ctx.confirmer.confirm(&prompt)? {
        return Ok(Outcome::Cancelled);
    }

    ctx.projects.untrack(&project.id)?;
    Ok(Outcome::Untracked {
        project: Box::new(project),
    })
}
