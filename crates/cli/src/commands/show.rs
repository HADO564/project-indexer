use clap::Args;

use indexer_core::domain::matching::{resolve, Resolution};
use indexer_core::domain::sorting::SortOptions;

use super::{Failure, Outcome};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// The project to show.
    pub project: String,
}

pub fn run(args: ShowArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let projects = ctx.projects.list(SortOptions::default())?;
    let result = resolve(&projects, &args.project);

    match result {
        Resolution::Found(project) => Ok(Outcome::Project(Box::new(project.clone()))),
        Resolution::NotFound => Err(Failure::NotFound {
            query: args.project,
        }
        .into()),
        Resolution::Ambiguous(matches) => Err(Failure::Ambiguous {
            query: args.project,
            matches: matches.into_iter().cloned().collect(),
        }
        .into()),
    }
}
