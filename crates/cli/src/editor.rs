//! Interactive editing for `edit`, supplied by the caller the way
//! [`crate::confirm`] supplies confirmation: a full-screen form in a shell,
//! and a stand-in in tests.
//!
//! `edit::run` calls it only when no field flag was given, so the form is a way
//! of typing an `edit` command rather than a second path to the database —
//! whatever it returns is saved by the same one `update` the flags make.

use std::io::{stderr, stdin, IsTerminal};

use indexer_core::{Project, UpdateProject};

use crate::commands::Failure;
use crate::settings;
use crate::tui;
use crate::tui::form::{Action, FormState};

pub trait ProjectEditor {
    /// Lets the user edit `project`: `Some` with the changes to save, or
    /// `None` if they cancelled. An editor that cannot run here — no terminal,
    /// or `--json` — returns [`Failure::Usage`] rather than guessing.
    fn edit(&self, project: &Project) -> anyhow::Result<Option<UpdateProject>>;
}

/// The shell's editor: a full-screen form drawn on stderr, when there is a
/// terminal to draw it on and nobody asked for JSON.
pub struct TerminalEditor {
    json: bool,
}

impl TerminalEditor {
    pub fn new(json: bool) -> Self {
        Self { json }
    }
}

impl ProjectEditor for TerminalEditor {
    fn edit(&self, project: &Project) -> anyhow::Result<Option<UpdateProject>> {
        // stdin to read keys from and stderr to draw on — stdout is never
        // needed, so `indexer edit app > out` still gets the form. Under
        // `--json` a script is driving, and a form nobody can see would hang
        // it, so that is refused as well.
        if self.json || !stdin().is_terminal() || !stderr().is_terminal() {
            return Err(Failure::Usage {
                message: "edit needs at least one field flag (--description, --add-tag, \
                          --remove-tag, --set, --unset) when it cannot open the form in a \
                          terminal"
                    .to_string(),
            }
            .into());
        }
        // A broken settings file must never stop the form from opening: it
        // is reported and the form wraps, as `main` does for the colours.
        let wrap = match settings::load() {
            Ok(saved) => saved.form_wrap.unwrap_or(true),
            Err(e) => {
                eprintln!("indexer: ignoring settings: {e:#}");
                true
            }
        };
        let mut state = FormState::new(project, wrap);
        // The session is dropped at the end of this block, which puts the
        // terminal back before anything is printed about the edit.
        let action = {
            let mut session = tui::terminal::enter()?;
            tui::run_form(&mut session.terminal, &mut state)?
        };
        Ok(match action {
            Action::Save => Some(state.changes()),
            Action::Cancel | Action::Continue => None,
        })
    }
}
