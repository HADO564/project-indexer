use clap::Args;

use super::Outcome;
use crate::context::Context;

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// The project to show.
    pub project: String,
}

pub fn run(args: ShowArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let project = ctx.projects.get(&args.project)?;
    Ok(Outcome::Project(Box::new(project)))
}
