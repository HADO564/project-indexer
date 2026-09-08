//! Tests for [`crate::domain::sorting`].

use crate::domain::Project;

use std::collections::BTreeMap;

use crate::domain::sorting::*;
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
        properties: BTreeMap::new(),
        trackers: Vec::new(),
        group_id: None,
        color: None,
        icon: None,
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
    sort_projects(&mut sorted, options);

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
    let mut projects = vec![project_named("b", "Same"), project_named("a", "same")];

    sort_alphabetically(&mut projects);

    let ids: Vec<&str> = projects.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
}
