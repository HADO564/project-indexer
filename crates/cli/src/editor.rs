//! Interactive editing for `edit`, supplied by the caller the way
//! [`crate::confirm`] supplies confirmation: a full-screen form in a shell,
//! and a stand-in in tests.
//!
//! `edit::run` calls it only when no field flag was given, so the form is a way
//! of typing an `edit` command rather than a second path to the database —
//! whatever it returns is saved by the same one `update` the flags make.

use std::io::{stderr, stdin, IsTerminal};
use std::path::PathBuf;

use anyhow::bail;
use indexer_core::{Project, UpdateProject};

use crate::commands::{add, Failure};
use crate::settings;
use crate::tui;
use crate::tui::form::{Action, FormState};

/// Which form `edit` opens: the compact one, or every field with `--full`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormKind {
    Compact,
    Full,
}

pub trait ProjectEditor {
    /// Lets the user edit `project`: `Some` with the changes to save, or
    /// `None` if they cancelled. An editor that cannot run here — no terminal,
    /// or `--json` — returns [`Failure::Usage`] rather than guessing.
    fn edit(&self, project: &Project, kind: FormKind) -> anyhow::Result<Option<UpdateProject>>;
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
    fn edit(&self, project: &Project, kind: FormKind) -> anyhow::Result<Option<UpdateProject>> {
        // stdin to read keys from and stderr to draw on — stdout is never
        // needed, so `indexer edit app > out` still gets the form. Under
        // `--json` a script is driving, and a form nobody can see would hang
        // it, so that is refused as well.
        if self.json || !stdin().is_terminal() || !stderr().is_terminal() {
            // `--full` cannot sit beside a field flag, so the compact form's
            // advice — give one — would be wrong for it.
            let message = match kind {
                FormKind::Compact => {
                    "edit needs at least one field flag (--description, --add-tag, \
                     --remove-tag, --set, --unset) when it cannot open the form in a \
                     terminal"
                }
                FormKind::Full => {
                    "edit --full opens a form, so it needs a terminal and cannot be used \
                     with --json; to change a field without one, give its flag instead"
                }
            };
            return Err(Failure::Usage {
                message: message.to_string(),
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
        let mut state = FormState::new(project, wrap, kind);
        // The session is dropped at the end of this block, which puts the
        // terminal back before anything is printed about the edit.
        let changes = {
            let mut session = tui::terminal::enter()?;
            loop {
                match tui::run_form(&mut session.terminal, &mut state)? {
                    Action::Save => {
                        let mut changes = state.changes();
                        // A directory that does not resolve keeps the form
                        // open with the reason, rather than closing it and
                        // losing every other edit over a typo.
                        match resolve_directory(&mut changes) {
                            Ok(()) => break Some(changes),
                            Err(e) => state.show_error(format!("{e:#}")),
                        }
                    }
                    Action::Cancel | Action::Continue => break None,
                }
            }
        };
        Ok(changes)
    }
}

/// Turns the directory typed into the form into the one to store: `~` for
/// the home folder, as a shell would have expanded it for `--directory`, then
/// absolute with symlinks resolved, as `add` stores a path. An emptied box is
/// refused here, with a reason, rather than as a missing folder named "".
pub(crate) fn resolve_directory(changes: &mut UpdateProject) -> anyhow::Result<()> {
    let Some(typed) = changes.directory.take() else {
        return Ok(());
    };
    let typed = typed.trim();
    if typed.is_empty() {
        bail!("a project needs a directory");
    }
    changes.directory = Some(add::absolute(
        Some(expand_home(typed, dirs::home_dir())),
        "move to",
    )?);
    Ok(())
}

/// A leading `~` or `~/` as the home folder. `~name` — another user's home —
/// is left alone, as is everything when the home folder is unknown.
pub(crate) fn expand_home(typed: &str, home: Option<PathBuf>) -> PathBuf {
    match (typed.strip_prefix('~'), home) {
        (Some(""), Some(home)) => home,
        (Some(rest), Some(home)) if rest.starts_with(['/', '\\']) => home.join(&rest[1..]),
        _ => PathBuf::from(typed),
    }
}
