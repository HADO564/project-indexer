//! The terminal UI. A bare `dexily` opens the browser (`app` + `ui::app`): the
//! sidebar of views and groups, the project list, and the selected project's
//! detail, navigated with vim-style keys. A bare `dexily edit` opens the `edit`
//! form (`form`). Running commands from a `:` line, search and live refresh are
//! the later phases in `docs/superpowers/specs/2026-10-15-tui-design.md`.

pub(crate) mod app;
pub(crate) mod cmdline;
pub mod form;
pub(crate) mod host;
mod keys;
pub mod terminal;
pub mod ui;

use std::io::{self, IsTerminal};

use anyhow::bail;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use self::app::{Action as AppAction, App};
use self::form::{Action, FormState};
use self::terminal::FormTerminal;
use crate::context::Context;
use crate::output::color::Color;
use crate::output::Look;

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
                match app.handle(key) {
                    // A destructive command waits for a `y/N` gate first;
                    // anything else runs straight away.
                    Some(AppAction::Run(line)) => match cmdline::needs_confirm(&line) {
                        Some(prompt) => app.begin_confirm(prompt, line),
                        None => run_and_reload(&mut app, ctx, &line),
                    },
                    // Already confirmed by the gate — run it.
                    Some(AppAction::RunConfirmed(line)) => run_and_reload(&mut app, ctx, &line),
                    None => {}
                }
            }
        }
    }
    Ok(())
}

/// Run one `:` line against the database, then reload so the panes reflect any
/// change and show the result on the status line. A failed reload is shown, not
/// fatal.
fn run_and_reload(app: &mut App, ctx: &Context, line: &str) {
    let look = Look {
        folder: Color::DEFAULT_FOLDER,
        header: Color::DEFAULT_HEADER,
        icons: app.icons(),
    };
    let message = cmdline::run_line(line, ctx, look);
    match app.reload(ctx) {
        Ok(()) => app.set_message(message),
        Err(error) => app.set_message(format!("dexily: reload failed: {error:#}")),
    }
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
