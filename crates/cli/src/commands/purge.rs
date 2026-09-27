//! `purge <project>`: permanently delete a binned project's record.
//!
//! The command-line form of the GUI's two-click purge from the bin. Only a
//! binned project can be purged — it is resolved against the bin, and core's
//! `ProjectService::delete` refuses anything else as a second line of defence.
//! Its folder is already gone (that is how it reached the bin), so this deletes
//! the last trace of it. There is no undo, so it asks first.

use clap::Args;

use super::{find_in, Corpus, Outcome, TrackerKind};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct PurgeArgs {
    /// The binned project — an exact name, an id, a parent/folder path ending,
    /// or part of a name, matched only against the bin.
    pub project: String,

    /// Only consider projects with one of these trackers when matching.
    /// Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,
}

pub fn run(args: PurgeArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let project = find_in(
        ctx,
        Corpus::Bin,
        args.project,
        super::unique_kinds(&args.tracker),
    )?;

    // Through the confirmer, as `untrack` is: piped stdin without `--yes`
    // refuses rather than reading a script's input as consent, and answering
    // no is `Cancelled`, exit 0. The prompt names the directory too, because a
    // query that matched one of several similar names is exactly when the user
    // wants to see which one they are about to lose.
    let prompt = format!(
        "permanently delete the record of \"{}\" (was at {})? this cannot be undone",
        project.name, project.directory
    );
    if !ctx.confirmer.confirm(&prompt)? {
        return Ok(Outcome::Cancelled);
    }

    ctx.projects.delete(&project.id)?;
    Ok(Outcome::Purged {
        project: Box::new(project),
    })
}
