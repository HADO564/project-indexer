use std::path::PathBuf;

use clap::Args;

use super::{not_implemented, Outcome};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct AddArgs {
    /// The directory to track.
    pub directory: PathBuf,
}

pub fn run(_args: AddArgs, _ctx: &Context) -> anyhow::Result<Outcome> {
    Err(not_implemented("add"))
}
