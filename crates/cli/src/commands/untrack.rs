use clap::Args;

use super::{not_implemented, Outcome};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct UntrackArgs {
    /// The project to stop tracking.
    pub project: String,
}

pub fn run(_args: UntrackArgs, _ctx: &Context) -> anyhow::Result<Outcome> {
    Err(not_implemented("untrack"))
}
