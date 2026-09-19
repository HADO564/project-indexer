use clap::Args;

use indexer_core::domain::matching::{resolve, Resolution};
use indexer_core::domain::sorting::SortOptions;

use super::{unique_kinds, with_tracker, Failure, Outcome, TrackerKind};
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
    let projects = ctx.projects.list(SortOptions::default())?;
    let tracker = unique_kinds(&args.tracker);
    let projects = with_tracker(projects, &tracker);
    let result = resolve(&projects, &args.project);

    match result {
        Resolution::Found(project) => Ok(Outcome::Project {
            project: Box::new(project.clone()),
            tracker,
        }),
        Resolution::NotFound => Err(Failure::NotFound {
            query: args.project,
        }
        .into()),
        Resolution::Ambiguous(matches) => Err(Failure::Ambiguous {
            query: args.project,
            matches: matches.into_iter().cloned().collect(),
            tracker,
        }
        .into()),
    }
}
