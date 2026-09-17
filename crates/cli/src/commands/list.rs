use clap::Args;

use super::Outcome;
use crate::context::Context;
use indexer_core::domain::matching::filter;
use indexer_core::domain::sorting::SortOptions;

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Only list projects whose name contains this, or, when it has a `/`,
    /// whose path ends with it (`work/app`). Ignores case.
    pub query: Option<String>,
}

pub fn run(args: ListArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let projects = ctx.projects.list(SortOptions::default())?;
    let projects = match &args.query {
        Some(query) => filter(&projects, query).into_iter().cloned().collect(),
        None => projects,
    };
    Ok(Outcome::Projects {
        projects,
        query: args.query,
    })
}
