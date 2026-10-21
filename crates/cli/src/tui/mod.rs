//! The terminal UI. A bare `dexily` opens the browser (`app` + `ui::app`): the
//! sidebar of views and groups, the project list, and the selected project's
//! detail, navigated with vim-style keys. A bare `dexily edit` opens the `edit`
//! form (`form`). A `:` line runs commands and `/` filters the list, and the
//! panes refresh when another process writes to the database (`watch`).

pub(crate) mod app;
pub(crate) mod cmdline;
pub mod form;
pub(crate) mod host;
pub(crate) mod images;
mod keys;
pub mod terminal;
pub mod ui;
pub(crate) mod watch;

use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::bail;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use self::app::{Action as AppAction, App};
use self::form::{Action, FormState};
use self::images::IconImages;
use self::terminal::FormTerminal;
use self::watch::Watch;
use crate::context::Context;
use crate::output::color::Color;
use crate::output::Look;
use crate::paths;

/// How long the loop waits for a key before checking whether the database has
/// changed. A quarter second reads as instant, and each check is one tiny read.
const REFRESH_TICK: Duration = Duration::from_millis(250);

/// Opens the browser: read the database, then draw and handle keys until the
/// user quits, re-reading whenever another process writes. The terminal is put
/// back by `Session`'s `Drop` on every way out, panic included. Without a
/// terminal to draw on there is no interface to open, so this says so rather
/// than hanging — a pipe or a script wants a command, not a full-screen app.
pub fn run(ctx: &Context) -> anyhow::Result<()> {
    if !std::io::stderr().is_terminal() {
        bail!("dexily's interface needs a terminal; run a command instead (see `dexily --help`)");
    }
    let mut app = App::load(ctx)?;
    let mut watch = open_watch(&mut app);
    let mut session = terminal::enter()?;
    // With the screen up and before the first key is read, as the question to
    // the terminal needs.
    let mut images = IconImages::detect(&ctx.icons);
    let mut redraw = true;
    while !app.quit() {
        if redraw {
            session
                .terminal
                .draw(|frame| ui::app::draw(frame, &app, images.as_mut()))?;
        }
        // No key within the tick: check for another process's write, and only
        // draw again if the panes changed.
        if !event::poll(REFRESH_TICK)? {
            redraw = refresh_if_changed(&mut app, ctx, watch.as_mut(), images.as_mut());
            continue;
        }
        // Any event redraws — a resize must, and a key usually changes the view.
        redraw = true;
        // Only key presses act. Some terminals also send release and repeat
        // events; acting on a release would run every key twice.
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.handle(key) {
                    // A destructive command waits for a `y/N` gate first;
                    // anything else runs straight away.
                    Some(AppAction::Run(line)) => match cmdline::needs_confirm(&line) {
                        Some(prompt) => app.begin_confirm(prompt, line),
                        None => run_and_reload(
                            &mut session.terminal,
                            &mut app,
                            ctx,
                            watch.as_mut(),
                            images.as_mut(),
                            &line,
                        )?,
                    },
                    // Already confirmed by the gate — run it.
                    Some(AppAction::RunConfirmed(line)) => run_and_reload(
                        &mut session.terminal,
                        &mut app,
                        ctx,
                        watch.as_mut(),
                        images.as_mut(),
                        &line,
                    )?,
                    // The folder browser's directory reads happen here, not in
                    // the pure App state.
                    Some(AppAction::OpenBrowser) => {
                        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        let entries = read_subdirs(&dir);
                        app.set_browse(dir, entries);
                    }
                    // `n`: the `:` line, holding `add` and the folder dexily
                    // was started in.
                    Some(AppAction::AddHere) => {
                        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        app.begin_command(cmdline::add_line(&dir));
                    }
                    Some(AppAction::BrowseTo(dir)) => {
                        let entries = read_subdirs(&dir);
                        app.set_browse(dir, entries);
                    }
                    None => {}
                }
            }
        }
    }
    Ok(())
}

/// The live-refresh watch on the database `ctx` opened. If it can't be opened
/// the browser still works — it just doesn't notice other processes' writes
/// until the next command reloads it — so the status line says so, rather than
/// stderr, which the browser is about to draw over.
fn open_watch(app: &mut App) -> Option<Watch> {
    let watch = paths::database_path().and_then(|path| Ok(Watch::open(&path)?));
    match watch {
        Ok(watch) => Some(watch),
        Err(error) => {
            app.set_message(format!("live refresh is off: {error:#}"));
            None
        }
    }
}

/// On an idle tick: if another process has committed since the last look,
/// re-read the database, keeping the cursor where it was. Returns whether the
/// panes need drawing again. A failed check — the database briefly busy — is
/// skipped, since the next tick asks again; a failed reload is shown on the
/// status line, not fatal.
fn refresh_if_changed(
    app: &mut App,
    ctx: &Context,
    watch: Option<&mut Watch>,
    images: Option<&mut IconImages>,
) -> bool {
    let Some(watch) = watch else {
        return false;
    };
    if !watch.changed().unwrap_or(false) {
        return false;
    }
    if let Err(error) = app.reload(ctx) {
        app.set_message(format!("dexily: refresh failed: {error:#}"));
    }
    // A project may now wear an icon added since the icons were last read.
    if let Some(images) = images {
        images.reload(&ctx.icons);
    }
    true
}

/// Run one `:` line against the database, then reload so the panes reflect any
/// change and show the result on the status line. A failed reload is shown, not
/// fatal. After an `edit`, whose form may have drawn over the browser, the
/// whole screen is repainted on the next draw, not only what changed.
fn run_and_reload(
    terminal: &mut FormTerminal,
    app: &mut App,
    ctx: &Context,
    watch: Option<&mut Watch>,
    images: Option<&mut IconImages>,
    line: &str,
) -> io::Result<()> {
    let look = Look {
        folder: Color::DEFAULT_FOLDER,
        header: Color::DEFAULT_HEADER,
        icons: app.icons(),
    };
    let message = cmdline::run_line(line, ctx, look);
    // The command's own write moves the watch's counter too. Take it as seen
    // before the reload below picks it up, so the next tick doesn't read the
    // same data twice; anything written after this point is still noticed.
    if let Some(watch) = watch {
        let _ = watch.changed();
    }
    match app.reload(ctx) {
        Ok(()) => app.set_message(message),
        Err(error) => app.set_message(format!("dexily: reload failed: {error:#}")),
    }
    // `:icon import`, or an `edit --icon`, may have changed what to draw.
    if let Some(images) = images {
        images.reload(&ctx.icons);
    }
    if cmdline::opens_form(line) {
        terminal.clear()?;
    }
    Ok(())
}

/// The subdirectories of `dir`, by name, sorted — what the folder browser
/// lists. An unreadable directory (or a file) yields an empty list rather than
/// an error; the browser then just has nothing to descend into.
fn read_subdirs(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
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
