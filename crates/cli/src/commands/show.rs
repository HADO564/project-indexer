use clap::Args;

use super::{find_one, unique_kinds, GroupLabel, Outcome, TrackerKind};
use crate::context::Context;

use indexer_core::domain::Project;

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// The project to show.
    pub project: String,

    /// Only include projects with one of these trackers, before the query is
    /// matched. Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,
}

pub fn run(args: ShowArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let tracker = unique_kinds(&args.tracker);

    // Cloned because `find_one` keeps the kinds for the table it puts in an
    // ambiguous failure, and `show` needs them again for its detail sections.
    let project = find_one(ctx, args.project, tracker.clone())?;

    let group = label(ctx, &project)?;

    Ok(Outcome::Project {
        project: Box::new(project),
        tracker,
        group,
    })
}

/// The group `project` belongs to, or `None` when it belongs to none.
///
/// The groups are fetched only when there is an id to look up, so an ungrouped
/// project — the common case — costs no query at all.
///
/// A `group_id` whose group has since gone is `None` as well, not an error.
/// The schema's `ON DELETE SET NULL` should prevent it, but "should" is not
/// "cannot", and a dangling reference is no reason to refuse to show a project
/// that is otherwise fine.
fn label(ctx: &Context, project: &Project) -> anyhow::Result<Option<GroupLabel>> {
    let Some(group_id) = project.group_id.as_deref() else {
        return Ok(None);
    };

    // `into_iter`, not `iter`: an owned `Group` lets its strings move straight
    // into the label. `groups` is not read again, so consuming it costs nothing.
    let groups = ctx.groups.list()?;
    Ok(groups
        .into_iter()
        .find(|g| g.id == group_id)
        .map(|g| GroupLabel {
            name: g.name,
            color: g.color,
            icon: g.icon,
        }))
}
