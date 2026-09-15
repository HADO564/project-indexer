use clap::Args;

use super::Outcome;
use crate::context::Context;
use indexer_core::domain::sorting::SortOptions;

#[derive(Debug, Args)]
pub struct ListArgs {}

pub fn run(_args: ListArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let projects = ctx.projects.list(SortOptions::default())?;
    Ok(Outcome::Projects(projects))
}
