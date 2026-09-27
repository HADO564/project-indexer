//! `edit <project> --description … --add-tag … --remove-tag …`: change the
//! fields of a project that take a value.
//!
//! The other half of the hybrid rule `favorite.rs` follows: if it takes a
//! value, it is an `edit` flag. Every flag joins the `change` group, which
//! requires at least one, so a bare `edit app` is a usage error rather than a
//! write that changes nothing. (A bare `edit` in a terminal is later to open a
//! full-screen form instead — `docs/cli/checklist.md`.) All the flags given
//! become one `update`, one read and one write, however many there are.

use clap::{ArgGroup, Args};
use indexer_core::domain::normalize::normalize_tag;
use indexer_core::domain::UpdateProject;

use super::{find_one, Outcome, TrackerKind};
use crate::context::Context;

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("change").required(true).multiple(true)))]
pub struct EditArgs {
    /// The project — an exact name, an id, a parent/folder path ending, or
    /// part of a name.
    pub project: String,

    /// Replace the description. An empty string clears it.
    #[arg(long, group = "change")]
    pub description: Option<String>,

    /// Add a tag. Repeat the flag or separate tags with commas. Adding a tag
    /// the project already has changes nothing.
    #[arg(long, group = "change", value_delimiter = ',')]
    pub add_tag: Vec<String>,

    /// Remove a tag, ignoring case. Repeat the flag or separate tags with
    /// commas. Removals happen before additions, so a tag both removed and
    /// added ends up present.
    #[arg(long, group = "change", value_delimiter = ',')]
    pub remove_tag: Vec<String>,

    /// Only consider projects with one of these trackers when matching.
    /// Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,
}

pub fn run(args: EditArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let project = find_one(ctx, args.project, super::unique_kinds(&args.tracker))?;

    // `None` unless a tag flag was given, so an edit of other fields never
    // writes the tag list back at all.
    //
    // Core replaces the whole list, so the new one is built here from the copy
    // `find_one` read: a read-modify-write. A write from the GUI between that
    // read and this `update` is lost without an error. The window is
    // milliseconds and accepted for now; the fix, an optimistic check on
    // `updated_at`, is planned in `docs/architecture.md` → *Quality backlog* →
    // *Later — concurrent edits*.
    let tags = if args.add_tag.is_empty() && args.remove_tag.is_empty() {
        None
    } else {
        Some(edited_tags(&project.tags, args.add_tag, &args.remove_tag))
    };

    // Every other field is `None`, which `update` reads as "leave alone". It
    // returns the project as saved — with its new `updated_at` — so that copy
    // replaces the one `find_one` returned.
    let project = ctx.projects.update(
        &project.id,
        UpdateProject {
            description: args.description,
            tags,
            ..Default::default()
        },
    )?;
    Ok(Outcome::Edited {
        project: Box::new(project),
    })
}

/// The tag list after `--remove-tag` and then `--add-tag`, built from the
/// project's current tags. Kept out of `run` so it can be tested without a
/// database, as `add::absolute` is.
///
/// Stored tags are already normalized (`Rust`), so each removal is normalized
/// the same way before comparing — otherwise `--remove-tag rust` would match
/// nothing and quietly succeed. Additions are appended as typed: core's
/// `update` runs `normalize_tags` over the whole list, which title-cases them
/// and drops a tag the project already has.
pub fn edited_tags(current: &[String], add: Vec<String>, remove: &[String]) -> Vec<String> {
    let removals: Vec<String> = remove.iter().map(|t| normalize_tag(t)).collect();
    let mut list = current.to_vec();
    list.retain(|tag| !removals.contains(tag));
    list.extend(add);
    list
}
