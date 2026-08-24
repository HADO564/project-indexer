use crate::models::Project;
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
fn apply_sort(projects: &mut [Project], options: SortOptions) {
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

    apply_sort(&mut favorites, options);
    favorites
}

/// Returns soft-deleted projects only, ordered per `options`
/// (alphabetical/last-opened, ascending/descending).
pub fn filter_deleted(projects: &[Project], options: SortOptions) -> Vec<Project> {
    let mut deleted: Vec<Project> = projects.iter().filter(|p| p.is_deleted).cloned().collect();

    apply_sort(&mut deleted, options);
    deleted
}

/// Orders projects by name, case-insensitively (`apple` sorts before
/// `Zebra`, matching how a person reads the list rather than byte order).
///
/// Uses `sort_by_key` with a precomputed `(lowercase name, id)` key instead
/// of lowercasing inside a `sort_by` comparator: the key is computed once
/// per element up front rather than on every comparison the sort makes
/// (clippy flags the naive `sort_by` version for the same reason elsewhere
/// in this codebase — see `system.rs`'s installed-apps sort). The `id`
/// tiebreaker keeps ordering deterministic across runs when two projects
/// share a name once case is ignored, since this also runs on
/// `HashMap`-sourced input with no inherent order — same reasoning as
/// `sort_projects_by_recents` above.
pub fn sort_alphabetically(projects: &mut [Project]) {
    projects.sort_by_key(|p| (p.name.to_lowercase(), p.id.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn project(id: &str, favorite: bool, last_opened_at: Option<i64>) -> Project {
        let now = Utc::now();
        Project {
            is_deleted: false,
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            directory: format!("/tmp/{id}"),
            created_at: now,
            updated_at: now,
            last_opened_at: last_opened_at.map(|mins_ago| now - Duration::minutes(mins_ago)),
            tags: Vec::new(),
            favorite,
            open_with: None,
            notes: None,
            client: None,
            tracker: None,
        }
    }

    #[test]
    fn keeps_only_favorites() {
        let projects = vec![
            project("a", true, None),
            project("b", false, None),
            project("c", true, None),
        ];

        let favorites = filter_favorites(&projects, SortOptions::default());

        let ids: Vec<&str> = favorites.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["a", "c"]);
    }

    #[test]
    fn orders_favorites_by_most_recently_opened() {
        let projects = vec![
            project("older", true, Some(60)),
            project("newer", true, Some(5)),
            project("never-opened", true, None),
            project("not-a-favorite", false, Some(1)),
        ];
        let options = SortOptions {
            by: SortBy::LastOpened,
            direction: SortDirection::Descending,
        };

        let favorites = filter_favorites(&projects, options);

        let ids: Vec<&str> = favorites.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["newer", "older", "never-opened"]);
    }

    #[test]
    fn defaults_to_alphabetical_ascending() {
        assert_eq!(SortOptions::default().by, SortBy::Alphabetical);
        assert_eq!(SortOptions::default().direction, SortDirection::Ascending);
    }

    #[test]
    fn last_opened_ascending_reverses_the_most_recent_first_order() {
        let projects = vec![
            project("older", true, Some(60)),
            project("newer", true, Some(5)),
            project("never-opened", true, None),
        ];
        let options = SortOptions {
            by: SortBy::LastOpened,
            direction: SortDirection::Ascending,
        };

        let favorites = filter_favorites(&projects, options);

        let ids: Vec<&str> = favorites.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["never-opened", "older", "newer"]);
    }

    #[test]
    fn alphabetical_descending_reverses_a_to_z() {
        let projects = vec![
            project_named("1", "Zebra"),
            project_named("2", "apple"),
            project_named("3", "Mango"),
        ];
        let options = SortOptions {
            by: SortBy::Alphabetical,
            direction: SortDirection::Descending,
        };

        let mut sorted = projects;
        apply_sort(&mut sorted, options);

        let ids: Vec<&str> = sorted.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["1", "3", "2"]);
    }

    #[test]
    fn filter_deleted_keeps_only_soft_deleted_projects() {
        let mut active = project("kept", false, None);
        active.is_deleted = false;
        let mut removed = project("gone", false, None);
        removed.is_deleted = true;

        let deleted = filter_deleted(&[active, removed], SortOptions::default());

        let ids: Vec<&str> = deleted.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["gone"]);
    }

    fn project_named(id: &str, name: &str) -> Project {
        Project {
            name: name.to_string(),
            ..project(id, false, None)
        }
    }

    #[test]
    fn sorts_names_case_insensitively() {
        let mut projects = vec![
            project_named("1", "Zebra"),
            project_named("2", "apple"),
            project_named("3", "Mango"),
        ];

        sort_alphabetically(&mut projects);

        let ids: Vec<&str> = projects.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["2", "3", "1"]);
    }

    #[test]
    fn breaks_ties_on_same_name_by_id() {
        let mut projects = vec![
            project_named("b", "Same"),
            project_named("a", "same"),
        ];

        sort_alphabetically(&mut projects);

        let ids: Vec<&str> = projects.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
    }
}