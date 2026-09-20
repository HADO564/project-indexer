//! `add [directory]`: start tracking a directory, defaulting to the current one.

use std::path::PathBuf;

use anyhow::{bail, Context as _};
use clap::Args;

use super::Outcome;
use crate::context::Context;

#[derive(Debug, Args)]
pub struct AddArgs {
    /// The directory to track. Defaults to the current directory.
    pub directory: Option<PathBuf>,
}

pub fn run(args: AddArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let directory = absolute(args.directory)?;

    // Asked before ensuring, because `ensure_project` is a get-or-create and
    // will not say which of the two it did. "already tracked" and "now tracked"
    // are different answers to the user, and one indexed lookup is what telling
    // them apart costs.
    let already_tracked = ctx
        .projects
        .find_by_directory(&directory)?
        .is_some_and(|p| !p.is_deleted);

    let project = ctx.projects.ensure_project(&directory)?;
    Ok(Outcome::Added {
        project: Box::new(project),
        already_tracked,
    })
}

/// The directory as an absolute path, defaulting to the current one.
///
/// **Resolved here rather than stored as typed**, which is what makes this
/// more than a `to_string_lossy`. `add .` run in `~/code/app` must store
/// `~/code/app`: the database outlives the shell that wrote to it, and the GUI
/// reads it from a different working directory, where a stored `.` means a
/// folder that does not exist. `normalize_directory` in core tidies separators
/// but cannot help — only the CLI knows what the working directory was.
///
/// No argument becomes the argument `"."` rather than a second branch calling
/// `current_dir`, so both spellings of "here" resolve through the same call and
/// cannot disagree about a symlinked folder — which would otherwise let one
/// directory into the database twice under two paths.
///
/// Canonicalising touches the disk, so a missing directory fails here. That is
/// the intended behaviour: `add ~/typo` should say so rather than track a
/// folder that is not there.
pub(crate) fn absolute(directory: Option<PathBuf>) -> anyhow::Result<String> {
    let directory = directory.unwrap_or_else(|| PathBuf::from("."));
    let resolved = std::fs::canonicalize(&directory)
        .with_context(|| format!("cannot track {}", directory.display()))?;
    // Canonicalising succeeds for a file too, and a tracked file would break
    // every detector downstream.
    if !resolved.is_dir() {
        bail!("{} is not a directory", resolved.display());
    }
    Ok(resolved.to_string_lossy().into_owned())
}
