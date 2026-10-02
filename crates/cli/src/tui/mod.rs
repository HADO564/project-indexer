//! The terminal UI. So far only the `edit` form (`form`): a bare `indexer edit`
//! opens it, and saving it sends the changes the way the flags would. The TUI
//! proper — panes, vim-style keybinds, and a `:` command line that parses into
//! the same `Command` the shell uses — is still to come.

mod app;
mod cmdline;
pub mod form;
mod keys;
pub mod terminal;
pub mod ui;

use std::io;

use anyhow::bail;
use ratatui::crossterm::event::{self, Event};

use self::form::{Action, FormState};
use self::terminal::FormTerminal;
use crate::context::Context;

pub fn run(_ctx: &Context) -> anyhow::Result<()> {
    bail!("the TUI is not implemented yet; see `indexer --help` for commands")
}

/// Runs the edit form until the user saves or cancels: draw, wait for an
/// event, hand a key to `handle`. Any other event — a resize — just redraws.
pub fn run_form(terminal: &mut FormTerminal, state: &mut FormState) -> io::Result<Action> {
    loop {
        terminal.draw(|frame| ui::form::draw(frame, state))?;
        let event = event::read()?;
        if let Event::Key(key) = event {
            match state.handle(key) {
                Action::Continue => {}
                action => return Ok(action),
            }
        }
    }
}
