//! The terminal UI. A bare `dexily` opens the browser (`app` + `ui::app`): the
//! sidebar of views and groups, the project list, and the selected project's
//! detail, navigated with vim-style keys. A bare `dexily edit` opens the `edit`
//! form (`form`). Running commands from a `:` line, search and live refresh are
//! the later phases in `docs/superpowers/specs/2026-10-15-tui-design.md`.

pub(crate) mod app;
mod cmdline;
pub mod form;
mod keys;
pub mod terminal;
pub mod ui;

use std::io::{self, IsTerminal};

use anyhow::bail;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use self::app::App;
use self::form::{Action, FormState};
use self::terminal::FormTerminal;
use crate::context::Context;

/// Opens the browser: read the database once, then draw and handle keys until
/// the user quits. The terminal is put back by `Session`'s `Drop` on every way
/// out, panic included. Without a terminal to draw on there is no interface to
/// open, so this says so rather than hanging — a pipe or a script wants a
/// command, not a full-screen app.
pub fn run(ctx: &Context) -> anyhow::Result<()> {
    if !std::io::stderr().is_terminal() {
        bail!("dexily's interface needs a terminal; run a command instead (see `dexily --help`)");
    }
    let mut app = App::load(ctx)?;
    let mut session = terminal::enter()?;
    while !app.quit() {
        session.terminal.draw(|frame| ui::app::draw(frame, &app))?;
        // Only key presses act. Some terminals also send release and repeat
        // events; acting on a release would run every key twice.
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                app.handle(key);
            }
        }
    }
    Ok(())
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
