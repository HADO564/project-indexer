use clap::Args;

use super::{not_implemented, Outcome};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct OpenArgs {
    /// The project to open.
    pub project: String,
}

pub fn run(_args: OpenArgs, _ctx: &Context) -> anyhow::Result<Outcome> {
    Err(not_implemented("open"))
}
