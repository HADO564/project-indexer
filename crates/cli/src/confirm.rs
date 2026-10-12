//! Confirmation for destructive commands, supplied by the caller: a stdin
//! prompt (or `--yes`) in a shell, a `y/n` on the command line in the TUI.

use std::io::{BufRead, IsTerminal, Write};

use anyhow::bail;

pub trait Confirmer {
    /// Whether the user agreed to `prompt`.
    fn confirm(&self, prompt: &str) -> anyhow::Result<bool>;

    /// What to do with a folder an observed command just made: the answer to
    /// `prompt`, which ends in `[Y/n/e]`. Unlike [`Confirmer::confirm`] the
    /// default is yes — the `dexily` prefix already said "track this" — so
    /// this is less a question than a chance to say no, or to add details.
    ///
    /// `None` when there is nobody to ask — `--yes`, a pipe, a script — and
    /// nothing was shown: the caller tracks, and says so itself.
    fn track(&self, prompt: &str) -> anyhow::Result<Option<Track>>;
}

/// An answer to "track this folder?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    /// Track it.
    Yes,
    /// Leave it an ordinary folder.
    No,
    /// Track it, then open the edit form on it.
    Edit,
}

/// The shell's confirmer: `--yes` agrees to everything, otherwise ask on
/// stderr and read the answer from stdin.
pub struct StdinConfirmer {
    assume_yes: bool,
}

impl StdinConfirmer {
    pub fn new(assume_yes: bool) -> Self {
        Self { assume_yes }
    }
}

impl Confirmer for StdinConfirmer {
    fn confirm(&self, prompt: &str) -> anyhow::Result<bool> {
        if self.assume_yes {
            return Ok(true);
        }
        // A script piping into us cannot answer, and reading its input as
        // consent would be worse than refusing.
        if !std::io::stdin().is_terminal() {
            bail!("{prompt} — pass --yes to confirm when stdin is not a terminal");
        }
        eprint!("{prompt} [y/N] ");
        std::io::stderr().flush()?;
        let mut answer = String::new();
        std::io::stdin().lock().read_line(&mut answer)?;
        Ok(matches!(
            answer.trim().to_ascii_lowercase().as_str(),
            "y" | "yes"
        ))
    }

    fn track(&self, prompt: &str) -> anyhow::Result<Option<Track>> {
        // Nobody to ask — `--yes`, a pipe, a script: the prefix is the
        // answer. Unlike `confirm`, refusing here would lose a folder the
        // command has already made.
        if self.assume_yes || !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
            return Ok(None);
        }
        loop {
            eprint!("{prompt} [Y/n/e] ");
            std::io::stderr().flush()?;
            let mut answer = String::new();
            // End of input (Ctrl+D) is not a yes: nothing was typed at all.
            if std::io::stdin().lock().read_line(&mut answer)? == 0 {
                eprintln!();
                return Ok(Some(Track::No));
            }
            match answer.trim().to_ascii_lowercase().as_str() {
                "" | "y" | "yes" => return Ok(Some(Track::Yes)),
                "n" | "no" => return Ok(Some(Track::No)),
                "e" | "edit" => return Ok(Some(Track::Edit)),
                // Anything else is a slip of the finger: ask again rather
                // than guess which way it was meant.
                _ => eprintln!("dexily: y to track, n to skip, e to track and edit"),
            }
        }
    }
}
