use anyhow::bail;
use clap::Args;

use indexer_core::domain::matching::{resolve, Resolution};
use indexer_core::domain::sorting::SortOptions;

use super::Outcome;
use crate::context::Context;
use crate::output::human;

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
        Resolution::NotFound => bail!("no project matches \"{}\"", args.project),
       Resolution::Ambiguous(matches) => bail!(
    "{} projects match \"{}\"; add a folder from the path, like parent/folder, to narrow it down:\n{}",
    matches.len(),
    args.project,
    human::project_table(&matches, None)
),
    }
}
