//! Tests for [`crate::domain::matching`].

use std::collections::BTreeMap;

use chrono::{Duration, Utc};

use crate::domain::matching::*;
use crate::domain::Project;

fn project(id: &str, name: &str, directory: &str, opened_mins_ago: Option<i64>) -> Project {
    let now = Utc::now();
    Project {
        is_deleted: false,
        id: id.to_string(),
        name: name.to_string(),
        description: String::new(),
        directory: directory.to_string(),
        created_at: now,
        updated_at: now,
        last_opened_at: opened_mins_ago.map(|mins| now - Duration::minutes(mins)),
        tags: Vec::new(),
        favorite: false,
        open_with: None,
        notes: None,
        properties: BTreeMap::new(),
        trackers: Vec::new(),
        group_id: None,
        color: None,
        icon: None,
    }
}

/// The found project's id, or a panic naming what came back instead.
fn found<'a>(resolution: Resolution<'a>) -> &'a str {
    match resolution {
        Resolution::Found(p) => &p.id,
        Resolution::NotFound => panic!("expected one project, found none"),
        Resolution::Ambiguous(ps) => panic!("expected one project, found {}", ps.len()),
    }
}

/// The ids of several matches, in order.
fn ambiguous<'a>(resolution: Resolution<'a>) -> Vec<&'a str> {
    match resolution {
        Resolution::Ambiguous(ps) => ps.iter().map(|p| p.id.as_str()).collect(),
        Resolution::Found(p) => panic!("expected several projects, found only {}", p.id),
        Resolution::NotFound => panic!("expected several projects, found none"),
    }
}

fn not_found(resolution: Resolution) -> bool {
    matches!(resolution, Resolution::NotFound)
}

#[test]
fn an_empty_query_matches_nothing() {
    let projects = [project("a", "app", "/work/app", None)];

    assert!(not_found(resolve(&projects, "")));
}

#[test]
fn a_query_in_no_name_matches_nothing() {
    let projects = [project("a", "app", "/work/app", None)];

    assert!(not_found(resolve(&projects, "zebra")));
}

#[test]
fn part_of_a_name_finds_the_project_ignoring_case() {
    let projects = [
        project("a", "project-indexer", "/hobby/project-indexer", None),
        project("b", "website", "/work/website", None),
    ];

    assert_eq!(found(resolve(&projects, "INDEX")), "a");
}

#[test]
fn an_exact_name_wins_over_names_containing_it() {
    let projects = [
        project("gateway", "api-gateway", "/work/api-gateway", Some(1)),
        project("api", "api", "/work/api", None),
    ];

    assert_eq!(found(resolve(&projects, "API")), "api");
}

#[test]
fn projects_sharing_an_exact_name_are_all_returned_most_recently_opened_first() {
    let projects = [
        project("play", "app", "/home/me/play/app", None),
        project("work", "app", "/home/me/work/app", Some(5)),
        project(
            "gateway",
            "app-gateway",
            "/home/me/infra/app-gateway",
            Some(1),
        ),
    ];

    assert_eq!(ambiguous(resolve(&projects, "app")), ["work", "play"]);
}

#[test]
fn several_names_containing_the_query_are_ambiguous() {
    let projects = [
        project("a", "web-app", "/work/web-app", None),
        project("b", "mobile-app", "/work/mobile-app", None),
        project("c", "notes", "/work/notes", None),
    ];

    let mut ids = ambiguous(resolve(&projects, "app"));
    ids.sort();
    assert_eq!(ids, ["a", "b"]);
}

#[test]
fn names_starting_with_the_query_rank_first() {
    let projects = [
        project("contains", "my-app", "/work/my-app", Some(1)),
        project("starts", "app-server", "/work/app-server", None),
    ];

    assert_eq!(ambiguous(resolve(&projects, "app")), ["starts", "contains"]);
}

#[test]
fn ties_rank_the_most_recently_opened_first_and_never_opened_last() {
    let projects = [
        project("never", "app-one", "/work/app-one", None),
        project("older", "app-two", "/work/app-two", Some(60)),
        project("newer", "app-three", "/work/app-three", Some(5)),
    ];

    assert_eq!(
        ambiguous(resolve(&projects, "app")),
        ["newer", "older", "never"]
    );
}

#[test]
fn a_path_ending_picks_the_project_in_that_folder() {
    let projects = [
        project("work", "app", "/home/me/work/app", None),
        project("play", "app", "/home/me/play/app", None),
    ];

    assert_eq!(found(resolve(&projects, "work/app")), "work");
}

#[test]
fn a_path_ending_ignores_case() {
    let projects = [project("a", "app", "/Users/me/Work/App", None)];

    assert_eq!(found(resolve(&projects, "work/APP")), "a");
}

#[test]
fn a_path_ending_needs_whole_folder_names() {
    let projects = [project("a", "app", "/home/me/work/app", None)];

    assert!(not_found(resolve(&projects, "rk/app")));
}

#[test]
fn several_projects_ending_in_the_same_path_are_ambiguous() {
    let projects = [
        project("2024", "app", "/archive/2024/client/app", None),
        project("2025", "app", "/archive/2025/client/app", None),
    ];

    let mut ids = ambiguous(resolve(&projects, "client/app"));
    ids.sort();
    assert_eq!(ids, ["2024", "2025"]);
}

#[test]
fn a_full_id_finds_the_project() {
    let projects = [
        project(
            "6c5d931e-45f9-4a0b-b2aa-539f435197e9",
            "app",
            "/work/app",
            None,
        ),
        project(
            "0a1b2c3d-0000-4000-8000-000000000000",
            "notes",
            "/work/notes",
            None,
        ),
    ];

    assert_eq!(
        found(resolve(&projects, "6C5D931E-45F9-4A0B-B2AA-539F435197E9")),
        "6c5d931e-45f9-4a0b-b2aa-539f435197e9"
    );
}

#[test]
fn a_short_id_finds_the_project() {
    let projects = [
        project(
            "6c5d931e-45f9-4a0b-b2aa-539f435197e9",
            "app",
            "/work/app",
            None,
        ),
        project(
            "0a1b2c3d-0000-4000-8000-000000000000",
            "notes",
            "/work/notes",
            None,
        ),
    ];

    assert_eq!(
        found(resolve(&projects, "6c5d931e")),
        "6c5d931e-45f9-4a0b-b2aa-539f435197e9"
    );
}

#[test]
fn an_id_prefix_shorter_than_a_short_id_is_not_an_id() {
    let projects = [project(
        "6c5d931e-45f9-4a0b-b2aa-539f435197e9",
        "app",
        "/work/app",
        None,
    )];

    assert!(not_found(resolve(&projects, "6c5d931")));
}

#[test]
fn a_short_id_shared_by_several_projects_is_ambiguous() {
    let projects = [
        project(
            "6c5d931e-0000-4000-8000-000000000001",
            "one",
            "/work/one",
            None,
        ),
        project(
            "6c5d931e-0000-4000-8000-000000000002",
            "two",
            "/work/two",
            None,
        ),
    ];

    assert_eq!(ambiguous(resolve(&projects, "6c5d931e")).len(), 2);
}

#[test]
fn a_hex_looking_name_is_still_found_by_name() {
    let projects = [project(
        "6c5d931e-45f9-4a0b-b2aa-539f435197e9",
        "cafe-bead",
        "/work/cafe-bead",
        None,
    )];

    assert_eq!(
        found(resolve(&projects, "cafe")),
        "6c5d931e-45f9-4a0b-b2aa-539f435197e9"
    );
}

/// The ids `filter` returned, in order.
fn filtered<'a>(projects: &'a [Project], query: &str) -> Vec<&'a str> {
    filter(projects, query)
        .into_iter()
        .map(|p| p.id.as_str())
        .collect()
}

#[test]
fn filter_lists_names_containing_the_query_not_just_the_exact_name() {
    let projects = [
        project("work", "app", "/home/me/work/app", Some(30)),
        project("gateway", "app-gateway", "/home/me/work/app-gateway", None),
        project("mine", "my-app", "/home/me/play/my-app", None),
        project("notes", "notes", "/home/me/docs/notes", None),
    ];

    assert_eq!(filtered(&projects, "app"), ["work", "gateway", "mine"]);
}

#[test]
fn filter_ranks_exact_names_then_prefixes_then_the_rest_over_recency() {
    let projects = [
        project("contains", "my-app", "/work/my-app", Some(1)),
        project("starts", "app-gateway", "/work/app-gateway", Some(2)),
        project("exact", "app", "/work/app", Some(60)),
    ];

    assert_eq!(filtered(&projects, "app"), ["exact", "starts", "contains"]);
}

#[test]
fn filter_ignores_case() {
    let projects = [project(
        "a",
        "Project-Indexer",
        "/hobby/project-indexer",
        None,
    )];

    assert_eq!(filtered(&projects, "INDEXER"), ["a"]);
}

#[test]
fn filter_with_a_slash_matches_the_end_of_the_path_only() {
    let projects = [
        project("work", "app", "/home/me/work/app", None),
        project("play", "app", "/home/me/play/app", None),
        project("billing", "billing", "/home/me/work/app/billing", None),
    ];

    assert_eq!(filtered(&projects, "work/app"), ["work"]);
}

#[test]
fn filter_without_a_slash_does_not_match_folder_names() {
    let projects = [project("billing", "billing", "/home/me/app", None)];

    assert!(filtered(&projects, "app").is_empty());
}

#[test]
fn filter_returns_nothing_for_an_empty_query_or_no_match() {
    let projects = [project("a", "app", "/work/app", None)];

    assert!(filtered(&projects, "").is_empty());
    assert!(filtered(&projects, "zebra").is_empty());
}
