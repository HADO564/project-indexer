//! The views a project list is seen through — All, Favourites, a group,
//! Ungrouped, the Bin — and the search applied within them.
//!
//! These rules used to live in the GUI's `views.ts`. They moved here so the
//! GUI, the CLI and the TUI give the same answer to what "Favourites",
//! "Ungrouped" or `client: acme` means: one implementation of the search, not
//! one per frontend with nothing checking they agree.
//!
//! Pure functions over slices the caller already holds — no repository, no
//! I/O. How a view is written down (a localStorage key, a CLI flag) and what
//! it is called on screen stay with each frontend.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::domain::{Group, Project};

/// One selectable view. Groups are navigation, not sections: selecting one
/// changes what the list shows, and the selection stays visible.
///
/// Serialized as `{ "kind": "group", "id": "…" }`, the shape of the
/// frontend's `View` union. `Group` has a named field rather than being
/// `Group(String)` because an internally tagged enum has nowhere to put an
/// unnamed string beside `kind` — serde rejects that at runtime, not compile
/// time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum View {
    All,
    Favorites,
    Group { id: String },
    Ungrouped,
    Bin,
}

/// How many projects each view holds, for the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ViewCounts {
    pub all: usize,
    pub favorites: usize,
    pub ungrouped: usize,
    pub bin: usize,
    /// Keyed by group id. Every group has an entry, including an empty one.
    pub groups: BTreeMap<String, usize>,
}

/// The shortest property name a query may use. A one-letter name is refused
/// because a bare drive letter (`D:\Games`) is far more common than a
/// one-letter property.
const MIN_PROPERTY_NAME_CHARS: usize = 2;

/// Splits a `name: value` query into its name and value, or `None` when the
/// query is free text.
///
/// The name is everything before the first `:`, trimmed. It must be at least
/// [`MIN_PROPERTY_NAME_CHARS`] long and hold no whitespace — which is what
/// keeps `D:\Games` and `build at 12:30` as ordinary text searches rather
/// than queries for a property called `D` or `build at 12`. Splitting once on
/// `:` does the job without a regex, the house style `domain::matching` sets.
fn parse_property_query(query: &str) -> Option<(&str, &str)> {
    let (name, value) = query.split_once(':')?;
    let name = name.trim();
    if name.chars().count() < MIN_PROPERTY_NAME_CHARS || name.chars().any(char::is_whitespace) {
        return None;
    }
    Some((name, value.trim()))
}

/// Whether `query` uses the `name: value` syntax, so a frontend can confirm
/// the syntax as it is typed.
pub fn is_property_query(query: &str) -> bool {
    parse_property_query(query).is_some()
}

/// Every distinct property name across `projects`, folded to lower case and
/// sorted. Backs the search hint — the syntax is only discoverable if the app
/// says which names are actually in use.
///
/// Takes an iterator so a caller can chain the live and binned lists without
/// building a combined one.
pub fn property_keys<'a>(projects: impl IntoIterator<Item = &'a Project>) -> Vec<String> {
    let keys: BTreeSet<String> = projects
        .into_iter()
        .flat_map(|p| p.properties.keys())
        .map(|key| key.to_lowercase())
        .collect();
    keys.into_iter().collect()
}

/// Whether `project` has the property `name`, ignoring case, with a value
/// containing `value`.
fn matches_property(project: &Project, name: &str, value: &str) -> bool {
    let wanted = name.to_lowercase();
    let needle = value.to_lowercase();
    project
        .properties
        .iter()
        .find(|(key, _)| key.trim().to_lowercase() == wanted)
        // An empty value asks "does this project have the property at all?",
        // which is the natural reading of typing `client:` and pausing.
        .is_some_and(|(_, stored)| needle.is_empty() || stored.to_lowercase().contains(&needle))
}

/// Whether `project` matches a search query. Two modes, both ignoring case:
///
/// - `name: value` asks about one property and matches nothing else — a
///   project without that property never matches, however its text reads.
/// - Anything else is free text over the name, directory, tags and property
///   *values*. Property names are not searched.
///
/// An empty query matches everything, so callers need no special case.
pub fn matches_query(project: &Project, query: &str) -> bool {
    if let Some((name, value)) = parse_property_query(query) {
        return matches_property(project, name, value);
    }

    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let contains = |text: &str| text.to_lowercase().contains(&q);
    contains(&project.name)
        || contains(&project.directory)
        || project.tags.iter().any(|tag| contains(tag))
        || project.properties.values().any(|value| contains(value))
}

/// The projects `view` shows, narrowed by `query`.
///
/// `live` is the non-deleted list and `deleted` the binned one. Filtering
/// keeps the order it was given, so whatever sort the caller applied still
/// holds inside every view — which is why this does not sort.
///
/// Favourites is derived from `live` rather than fetched separately: every
/// comparator in `sorting` ends in the unique id, so filtering a sorted list
/// and sorting a filtered one give the same list. Deriving it also means the
/// sidebar count and the list cannot disagree.
pub fn resolve_view<'a>(
    view: &View,
    live: &'a [Project],
    deleted: &'a [Project],
    query: &str,
) -> Vec<&'a Project> {
    let in_view = |p: &Project| match view {
        View::All | View::Bin => true,
        View::Favorites => p.favorite,
        View::Group { id } => p.group_id.as_deref() == Some(id.as_str()),
        View::Ungrouped => p.group_id.is_none(),
    };
    let source = if *view == View::Bin { deleted } else { live };
    source
        .iter()
        .filter(|p| in_view(p) && matches_query(p, query))
        .collect()
}

/// How many projects each view holds.
///
/// Counts ignore the search query: the sidebar reports what each view holds,
/// not what a transient filter leaves of it. No view may hide projects
/// without saying so, so an empty group shows `0` rather than vanishing —
/// hence seeding the map from every group before counting.
pub fn view_counts(live: &[Project], deleted: &[Project], groups: &[Group]) -> ViewCounts {
    let mut counts = ViewCounts {
        all: live.len(),
        bin: deleted.len(),
        groups: groups.iter().map(|g| (g.id.clone(), 0)).collect(),
        ..ViewCounts::default()
    };

    for project in live {
        if project.favorite {
            counts.favorites += 1;
        }
        match &project.group_id {
            None => counts.ungrouped += 1,
            // A project naming a group that no longer exists counts towards
            // neither its group nor Ungrouped. It is transient — deleting a
            // group clears membership in one transaction — and the next
            // refetch resolves it.
            Some(id) => {
                if let Some(count) = counts.groups.get_mut(id) {
                    *count += 1;
                }
            }
        }
    }
    counts
}
