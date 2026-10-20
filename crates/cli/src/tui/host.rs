//! The [`Confirmer`] and [`ProjectEditor`] the TUI gives its [`Context`], in
//! place of the shell's `StdinConfirmer` / `TerminalEditor`.
//!
//! Both of those read the real stdin or take over the terminal themselves —
//! which the TUI already owns, in raw mode, on the alternate screen — so the TUI
//! supplies its own: a confirmer that trusts the `y/N` gate it already showed,
//! and an editor that opens the form on the screen as the TUI holds it.
//!
//! [`Context`]: crate::context::Context

use std::io::stderr;

use indexer_core::{Group, Project, UpdateProject};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::confirm::{Confirmer, Track};
use crate::editor::{self, FormKind, ProjectEditor};
use crate::settings;
use crate::tui::form::FormState;

/// Approves confirmation. The TUI gates every destructive command with its own
/// `y/N` prompt *before* running it (see `App`'s Confirm mode and
/// `cmdline::needs_confirm`), so by the time a command asks here the user has
/// already said yes.
pub(crate) struct TuiConfirmer;

impl Confirmer for TuiConfirmer {
    fn confirm(&self, _prompt: &str) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn track(&self, _prompt: &str) -> anyhow::Result<Option<Track>> {
        // The observer never runs inside the TUI, so this is never reached.
        Ok(None)
    }
}

/// Opens the edit form over the browser — the same form `dexily edit` opens in
/// a shell, run by the same loop (`editor::run_until_done`). The TUI already
/// holds raw mode and the alternate screen, so this draws on the screen as it
/// stands instead of entering both again: the shell's editor leaves them when
/// it finishes, which here would drop the browser back to the shell. The screen
/// is cleared first so no pane shows through, and the loop repaints the browser
/// afterwards (see `cmdline::opens_form`).
pub(crate) struct TuiEditor;

impl ProjectEditor for TuiEditor {
    fn edit(
        &self,
        project: &Project,
        groups: Vec<Group>,
        kind: FormKind,
    ) -> anyhow::Result<Option<UpdateProject>> {
        // A broken settings file never stops the form, which then wraps — and
        // nothing is printed about it, as stderr is the browser's screen.
        let wrap = settings::load()
            .ok()
            .and_then(|saved| saved.form_wrap)
            .unwrap_or(true);
        let mut state = FormState::new(project, wrap, kind).with_groups(groups);
        let mut terminal = Terminal::new(CrosstermBackend::new(stderr()))?;
        terminal.clear()?;
        Ok(editor::run_until_done(&mut terminal, &mut state)?)
    }
}
