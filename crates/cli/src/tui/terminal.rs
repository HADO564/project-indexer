//! The terminal the `edit` form draws on: raw mode, so every key reaches
//! the form as it is pressed, and the alternate screen, so the user's
//! scrollback is untouched when it closes. On stderr, never stdout, which is
//! for data: `dexily edit app > out` still shows the form.
//!
//! Putting the terminal back is not optional — a program that exits in raw
//! mode leaves a shell with no echo and a dead Enter key — so it happens in
//! `Drop`, on every way out.

use std::io::{self, stderr, Stderr};

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;

/// A ratatui terminal drawing on stderr.
pub type FormTerminal = Terminal<CrosstermBackend<Stderr>>;

/// Leaves raw mode and the alternate screen.
fn leave() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stderr(), LeaveAlternateScreen)?;
    Ok(())
}

/// Puts the terminal back before a panic's message is printed. Rust runs the
/// hook before unwinding, so without this the message lands on the alternate
/// screen in raw mode — garbled, then wiped when `Session` drops.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = leave();
        previous(info);
    }));
}

/// Holds the terminal in raw mode on the alternate screen; dropping it puts
/// both back, however the form ends.
pub struct Session {
    pub terminal: FormTerminal,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = leave();
    }
}

/// Raw mode and the alternate screen on stderr, held until the returned
/// `Session` is dropped. If entering fails partway, what was already done is
/// undone before the error is returned: no `Session` exists yet to do it.
pub fn enter() -> io::Result<Session> {
    enable_raw_mode()?;
    let terminal = (|| {
        execute!(stderr(), EnterAlternateScreen)?;
        Terminal::new(CrosstermBackend::new(stderr()))
    })()
    .inspect_err(|_| {
        let _ = leave();
    })?;
    install_panic_hook();
    Ok(Session { terminal })
}
