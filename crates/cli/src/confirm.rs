//! Confirmation for destructive commands, supplied by the caller: a stdin
//! prompt (or `--yes`) in a shell, a `y/n` on the command line in the TUI.

use std::io::{BufRead, IsTerminal, Write};

use anyhow::bail;

pub trait Confirmer {
    /// Whether the user agreed to `prompt`.
    fn confirm(&self, prompt: &str) -> anyhow::Result<bool>;
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
}
