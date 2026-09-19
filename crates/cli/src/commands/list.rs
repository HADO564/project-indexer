use clap::Args;

use crate::context::Context;
use indexer_core::domain::matching::filter;
use indexer_core::domain::sorting::SortOptions;

use super::{unique_kinds, with_tracker, Outcome, TrackerKind};

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Only list projects whose name contains this, or, when it has a `/`,
    /// whose path ends with it (`work/app`). Ignores case.
    pub query: Option<String>,

    /// Only include projects with one of these trackers, before the query is
    /// matched. Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,
}

pub fn run(args: ListArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let projects = ctx.projects.list(SortOptions::default())?;
    let tracker = unique_kinds(&args.tracker);
    let projects = with_tracker(projects, &tracker);
    let projects = match &args.query {
        Some(query) => filter(&projects, query).into_iter().cloned().collect(),
        None => projects,
    };
    Ok(Outcome::Projects {
        projects,
        query: args.query,
        tracker,
    })
}
