//! The terminal UI. So far only the `edit` form (`form`): a bare `indexer edit`
//! opens it, and saving it sends the changes the way the flags would. The TUI
//! proper — panes, vim-style keybinds, and a `:` command line that parses into
//! the same `Command` the shell uses — is still to come.

mod app;
mod cmdline;
// Nothing outside the tests calls the form or its drawing until
// `TerminalEditor` runs them (step 5 of the edit-form brief); drop these then.
#[allow(dead_code)]
pub mod form;
mod keys;
#[allow(dead_code)]
pub mod ui;

use anyhow::bail;

use crate::context::Context;

pub fn run(_ctx: &Context) -> anyhow::Result<()> {
    bail!("the TUI is not implemented yet; see `indexer --help` for commands")
}
