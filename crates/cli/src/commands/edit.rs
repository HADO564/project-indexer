//! `edit <project> --description … --add-tag/--remove-tag … --set/--unset …`:
//! change the fields of a project that take a value.
//!
//! The other half of the hybrid rule `favorite.rs` follows: if it takes a
//! value, it is an `edit` flag. All the flags given become one `update`, one
//! read and one write, however many there are.
//!
//! With no field flag at all, `edit` asks `ctx.editor` instead — the
//! full-screen form in a terminal — and saves what it returns through the same
//! `update`. Where no form can be opened (piped, or `--json`) the editor
//! refuses with a usage error, exit 2, so a bare `edit` is never a write that
//! changes nothing.

use clap::{ArgGroup, Args};
use indexer_core::domain::normalize::normalize_tag;
use indexer_core::domain::UpdateProject;
use std::collections::BTreeMap;

use super::{find_one, Outcome, TrackerKind};
use crate::context::Context;
use crate::editor::FormKind;
use std::path::PathBuf;

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("change").multiple(true)))]
pub struct EditArgs {
    /// The project — an exact name, an id, a parent/folder path ending, or
    /// part of a name.
    pub project: String,

    /// Replace the description. An empty string clears it.
    #[arg(long, group = "change")]
    pub description: Option<String>,

    /// Rename the project.
    #[arg(long, group = "change")]
    pub name: Option<String>,

    /// Replace the notes. An empty string clears them.
    #[arg(long, group = "change")]
    pub notes: Option<String>,
    /// Add a tag. Repeat the flag or separate tags with commas. Adding a tag
    /// the project already has changes nothing.
    #[arg(long, group = "change", value_delimiter = ',')]
    pub add_tag: Vec<String>,

    /// Remove a tag, ignoring case. Repeat the flag or separate tags with
    /// commas. Removals happen before additions, so a tag both removed and
    /// added ends up present.
    #[arg(long, group = "change", value_delimiter = ',')]
    pub remove_tag: Vec<String>,

    /// Set a property, replacing one with the same name whatever its case.
    /// Repeat the flag for several; the value is everything after the first
    /// `=`, kept exactly as typed, commas included.
    #[arg(long, group = "change", value_name = "KEY=VALUE", value_parser = parse_property)]
    pub set: Vec<(String, String)>,

    /// Remove a property, ignoring case. Repeat the flag for several. Unsets
    /// happen before sets, so a property both unset and set ends up set.
    #[arg(long, group = "change", value_name = "KEY")]
    pub unset: Vec<String>,

    /// Only consider projects with one of these trackers when matching.
    /// Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,

    /// Open the form with every field, not just the description, tags and
    /// properties. It only chooses the form, so it cannot be combined with a
    /// field flag.
    #[arg(long, conflicts_with = "change")]
    pub full: bool,

    /// Point the project at another folder — after moving or renaming it on
    /// disk. It must exist, and no other project may have it. A relative path
    /// is resolved from where you run the command.
    #[arg(long, group = "change")]
    pub directory: Option<PathBuf>,

    /// The app to open the project with: its name, or where it is installed.
    /// Stored as typed, less any spaces around it, and only checked when you
    /// `open` the project. An empty string clears it, so the system's default
    /// app opens it again.
    #[arg(long, group = "change")]
    pub open_with: Option<String>,

    /// Put the project in this group, matched ignoring case. A project is in
    /// one group at a time, so this moves it out of any other. An empty string
    /// takes it out, as `--ungroup` does.
    #[arg(long, group = "change")]
    pub group: Option<String>,

    /// Take the project out of its group.
    #[arg(long, group = "change", conflicts_with = "group")]
    pub ungroup: bool,
}

pub fn run(args: EditArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    // Asked before `find_one` takes `args.project`: once one field has been
    // moved out of `args`, the struct can no longer be lent whole.
    let from_form = !has_field_flag(&args);
    let kind = if args.full {
        FormKind::Full
    } else {
        FormKind::Compact
    };
    let project = find_one(ctx, args.project, super::unique_kinds(&args.tracker))?;

    let update = if from_form {
        match ctx.editor.edit(&project, kind)? {
            Some(update) => update,
            None => return Ok(Outcome::Cancelled),
        }
    } else {
        // Absolute, and with symlinks resolved, as `add` stores a path: a
        // relative one means nothing once the command has finished.
        let directory = args
            .directory
            .map(|d| super::add::absolute(Some(d), "move to"))
            .transpose()?;
        // Every field without a flag stays `None`, which `update` reads as
        // "leave alone".
        UpdateProject {
            name: args.name,
            directory,
            description: args.description,
            notes: cleared_if_empty(args.notes),
            tags: tags(&project.tags, args.add_tag, &args.remove_tag),
            properties: properties(&project.properties, args.set, &args.unset),
            open_with: cleared_if_blank(args.open_with),
            group_id: group_id(ctx, args.group, args.ungroup)?,
            ..Default::default()
        }
    };

    // A form saved untouched sends every field `None`. Writing that would
    // still move `updated_at` and report an edit that never happened.
    if update.is_empty() {
        return Ok(Outcome::Unchanged {
            project: Box::new(project),
        });
    }

    // It returns the project as saved — with its new `updated_at` — so that
    // copy replaces the one `find_one` returned.
    let project = ctx.projects.update(&project.id, update)?;
    Ok(Outcome::Edited {
        project: Box::new(project),
    })
}

/// Whether any field flag was given. Every flag in the `change` group belongs
/// here: one left out would open the form even though the user asked for a
/// change on the command line.
fn has_field_flag(args: &EditArgs) -> bool {
    args.description.is_some()
        || !args.add_tag.is_empty()
        || !args.remove_tag.is_empty()
        || !args.set.is_empty()
        || !args.unset.is_empty()
        || args.name.is_some()
        || args.notes.is_some()
        || args.directory.is_some()
        || args.open_with.is_some()
        || args.group.is_some()
        || args.ungroup
}

/// A box in a box, as `UpdateProject.notes` takes it: the outer one says
/// whether to touch the field at all, the inner one what to store. `--notes ""`
/// clears them — `Some(None)` — as an emptied notes box does in the app.
fn cleared_if_empty(text: Option<String>) -> Option<Option<String>> {
    text.map(|text| if text.is_empty() { None } else { Some(text) })
}

/// The same, trimmed, for `--open-with`: spaces around an app's name are never
/// part of it, and the launcher would read a blank one as no app anyway.
fn cleared_if_blank(text: Option<String>) -> Option<Option<String>> {
    text.map(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// The group `--group` or `--ungroup` asks for, as `UpdateProject.group_id`
/// takes it: `None` to leave it alone, `Some(None)` to take the project out,
/// `Some(Some(id))` to put it in. The groups are read only when a name has to
/// be looked up, so no other edit touches them.
fn group_id(
    ctx: &Context,
    group: Option<String>,
    ungroup: bool,
) -> anyhow::Result<Option<Option<String>>> {
    if ungroup {
        return Ok(Some(None));
    }
    let Some(name) = group else {
        return Ok(None);
    };
    if name.trim().is_empty() {
        return Ok(Some(None));
    }
    let groups = ctx.groups.list()?;
    // `find` lends a group out of `groups`; the update keeps its own copy of
    // the id, since `groups` is gone when this returns.
    Ok(Some(Some(super::group::find(&groups, &name)?.id.clone())))
}

/// `None` unless a tag flag was given, so an edit of other fields never writes
/// the tag list back at all.
///
/// Core replaces the whole list, so the new one is built here from the copy
/// `find_one` read: a read-modify-write. A write from the GUI between that read
/// and the `update` is lost without an error. The window is milliseconds and
/// accepted for now; the fix, an optimistic check on `updated_at`, is planned
/// in `docs/architecture.md` → *Quality backlog* → *Later — concurrent edits*.
fn tags(current: &[String], add: Vec<String>, remove: &[String]) -> Option<Vec<String>> {
    if add.is_empty() && remove.is_empty() {
        None
    } else {
        Some(edited_tags(current, add, remove))
    }
}

/// The same for properties: `None` unless `--set` or `--unset` was given, and
/// otherwise the whole new map, because core replaces that too. The same
/// lost-update window applies.
fn properties(
    current: &BTreeMap<String, String>,
    set: Vec<(String, String)>,
    unset: &[String],
) -> Option<BTreeMap<String, String>> {
    if set.is_empty() && unset.is_empty() {
        None
    } else {
        Some(edited_properties(current, set, unset))
    }
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

/// Splits a `--set` value at its first `=`, so a value may contain `=` itself
/// (`url=https://x.com/?a=b`). The name is not checked here: core refuses an
/// empty one, or one containing `:`, with its own message, and a second copy
/// of that rule could drift from the first. An `Err` becomes clap's usage
/// error, exit 2, before anything is looked up.
fn parse_property(s: &str) -> Result<(String, String), String> {
    match s.split_once("=") {
        Some((key, value)) => Ok((key.to_string(), value.to_string())),
        None => Err("expected KEY=VALUE, with an `=` between the name and the value".to_string()),
    }
}

/// The property map after `--unset` and then `--set`, built from the
/// project's current properties. Kept out of `run` so it can be tested without
/// a database, like [`edited_tags`].
///
/// Names are matched ignoring case, as the search bar matches them
/// (`core::domain::views`), even though core stores them as typed: otherwise
/// `--unset client` would miss a `Client` the GUI stored and still exit 0, and
/// `--set client=x` would add a second property the search cannot tell apart
/// from the first. A set therefore drops any name equal ignoring case before
/// inserting its own spelling. Names are compared trimmed and otherwise left
/// to core, which trims and validates them; values are inserted exactly as
/// typed, which is core's rule too.
pub fn edited_properties(
    current: &BTreeMap<String, String>,
    set: Vec<(String, String)>,
    unset: &[String],
) -> BTreeMap<String, String> {
    let mut map = current.clone();
    let unset: Vec<String> = unset.iter().map(|k| k.trim().to_lowercase()).collect();
    map.retain(|k, _| !unset.contains(&k.trim().to_lowercase()));
    for (k, v) in set {
        let wanted = k.trim().to_lowercase();
        map.retain(|existing, _| existing.trim().to_lowercase() != wanted);
        map.insert(k, v);
    }
    map
}
