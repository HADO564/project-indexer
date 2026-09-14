//! The view-only TUI: read-only panes, keybinds for movement, and a `:` command
//! line that parses into the same `Command` the shell uses. Every change goes
//! through a command.

mod app;
mod cmdline;
mod keys;
mod ui;

use anyhow::bail;

use crate::context::Context;

pub fn run(_ctx: &Context) -> anyhow::Result<()> {
    bail!("the TUI is not implemented yet; see `indexer --help` for commands")
}
