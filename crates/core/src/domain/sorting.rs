use crate::domain::Project;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Which field to order projects by. Lives here (not in a Tauri command) so
/// a future CLI can reuse the exact same sorting logic the app uses —
/// nothing about "how to sort" should only exist behind the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    Alphabetical,
    LastOpened,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SortOptions {
    pub by: SortBy,
    pub direction: SortDirection,
}

/// Sorts `projects` by `options.by`, then flips the result if
/// `options.direction` asks for the opposite of that sort's natural order.
///
/// `sort_alphabetically`'s natural order is ascending (A→Z); "most recently
/// opened first" — [`sort_projects_by_recents`]'s whole purpose — is
/// naturally descending. Reversing a fully tie-broken slice (every
/// comparator here ends in the unique `id`) is exactly equivalent to
/// resorting with the comparison negated, so this is a cheap way to support
/// both directions without duplicating each comparator.
pub fn sort_projects(projects: &mut [Project], options: SortOptions) {
    let natural_direction = match options.by {
        SortBy::Alphabetical => {
            sort_alphabetically(projects);
            SortDirection::Ascending
        }
        SortBy::LastOpened => {
            sort_projects_by_recents(projects);
            SortDirection::Descending
        }
    };

    if options.direction != natural_direction {
        projects.reverse();
    }
}

/// Orders projects for the list view: most recently opened first, then
/// never-opened ones.
///
/// Ties fall back to `created_at` because the projects arrive from a
/// `HashMap`, whose iteration order is arbitrary and reseeded every run.
/// Without a tiebreaker, `sort_by` would faithfully preserve that random
/// order and never-opened projects would shuffle between launches.
pub fn sort_projects_by_recents(projects: &mut [Project]) {
    projects.sort_by(|a, b| {
        match (&a.last_opened_at, &b.last_opened_at) {
            // Reversed: the later timestamp sorts first.
            (Some(a_opened), Some(b_opened)) => b_opened.cmp(a_opened),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
        .then_with(|| a.created_at.cmp(&b.created_at))
        .then_with(|| a.id.cmp(&b.id))
    });
}

/// Returns favorited projects only, ordered per `options`
/// (alphabetical/last-opened, ascending/descending).
pub fn filter_favorites(projects: &[Project], options: SortOptions) -> Vec<Project> {
    let mut favorites: Vec<Project> = projects.iter().filter(|p| p.favorite).cloned().collect();

    sort_projects(&mut favorites, options);
    favorites
}

/// Returns soft-deleted projects only, ordered per `options`
/// (alphabetical/last-opened, ascending/descending).
pub fn filter_deleted(projects: &[Project], options: SortOptions) -> Vec<Project> {
    let mut deleted: Vec<Project> = projects.iter().filter(|p| p.is_deleted).cloned().collect();

    sort_projects(&mut deleted, options);
    deleted
}

/// Orders projects by name, case-insensitively (`apple` sorts before
/// `Zebra`, matching how a person reads the list rather than byte order).
///
/// Uses `sort_by_key` with a precomputed `(lowercase name, id)` key instead
/// of lowercasing inside a `sort_by` comparator: the key is computed once
/// per element up front rather than on every comparison the sort makes
/// (clippy flags the naive `sort_by` version for the same reason elsewhere
/// in this codebase — see `platform/app_discovery.rs`'s installed-apps sort). The `id`
/// tiebreaker keeps ordering deterministic across runs when two projects
/// share a name once case is ignored, since this also runs on
/// `HashMap`-sourced input with no inherent order — same reasoning as
/// `sort_projects_by_recents` above.
pub fn sort_alphabetically(projects: &mut [Project]) {
    projects.sort_by_key(|p| (p.name.to_lowercase(), p.id.clone()));
}
