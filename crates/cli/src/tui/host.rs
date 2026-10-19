//! The [`Confirmer`] and [`ProjectEditor`] the TUI gives its [`Context`], in
//! place of the shell's `StdinConfirmer` / `TerminalEditor`.
//!
//! Both of those read the real stdin or draw a full-screen form on stderr —
//! which the TUI already owns, in raw mode, on the alternate screen — so they
//! cannot run while the browser is up. Their interactive flows are later
//! phases: a `y/n` on the command line for a destructive command (phase 4), and
//! opening the edit form inside the TUI (phase 3). Until then these refuse with
//! a message, so `:untrack x` or a bare `:edit x` says why rather than hanging
//! or painting over the interface. A command that needs neither — `open`,
//! `favorite`, `edit x --name y`, `add`, `group create` — runs normally.
//!
//! [`Context`]: crate::context::Context

use anyhow::bail;
use indexer_core::{Group, Project, UpdateProject};

use crate::confirm::{Confirmer, Track};
use crate::editor::{FormKind, ProjectEditor};

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

/// Refuses to open the form: editing in the TUI is phase 3. A field flag still
/// works, since that path never opens the form.
pub(crate) struct TuiEditor;

impl ProjectEditor for TuiEditor {
    fn edit(
        &self,
        _project: &Project,
        _groups: Vec<Group>,
        _kind: FormKind,
    ) -> anyhow::Result<Option<UpdateProject>> {
        bail!("opening the edit form in the TUI isn't available yet — give a field flag, e.g. `edit <project> --name <name>`");
    }
}
