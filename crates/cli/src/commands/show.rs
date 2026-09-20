use clap::Args;

use super::{find_one, unique_kinds, Outcome, TrackerKind};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// The project to show.
    pub project: String,

    /// Only include projects with one of these trackers, before the query is
    /// matched. Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,
}

pub fn run(args: ShowArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let tracker = unique_kinds(&args.tracker);
    // Cloned because `find_one` keeps the kinds for the table it puts in an
    // ambiguous failure, and `show` needs them again for its detail sections.
    let project = find_one(ctx, args.project, tracker.clone())?;
    Ok(Outcome::Project {
        project: Box::new(project),
        tracker,
    })
}
