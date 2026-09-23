//! Tests for [`crate::domain::views`].
//!
//! Ported case for case from the GUI's `views.test.ts`, which pinned these
//! rules while they lived in TypeScript, plus the wire shape the GUI now
//! relies on.

use std::collections::BTreeMap;

use chrono::Utc;
use serde_json::json;

use crate::domain::views::*;
use crate::domain::{Group, Project};

fn project(id: &str, name: &str) -> Project {
    let now = Utc::now();
    Project {
        is_deleted: false,
        id: id.to_string(),
        name: name.to_string(),
        description: String::new(),
        directory: r"D:\Games\friction-engine".to_string(),
        created_at: now,
        updated_at: now,
        last_opened_at: None,
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

fn in_group(mut p: Project, group_id: &str) -> Project {
    p.group_id = Some(group_id.to_string());
    p
}

fn favourite(mut p: Project) -> Project {
    p.favorite = true;
    p
}

fn with_properties(mut p: Project, pairs: &[(&str, &str)]) -> Project {
    p.properties = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    p
}

fn group(id: &str, name: &str, position: i64) -> Group {
    let now = Utc::now();
    Group {
        id: id.to_string(),
        name: name.to_string(),
        color: "cyan".to_string(),
        icon: "briefcase".to_string(),
        position,
        created_at: now,
        updated_at: now,
    }
}

/// The fixture `views.test.ts` used: two in Work (one a favourite), an
/// ungrouped favourite, one in Personal, and one in the bin.
fn fixture() -> (Vec<Project>, Vec<Project>) {
    let live = vec![
        favourite(in_group(project("a", "Alpha"), "g-work")),
        in_group(project("b", "Bravo"), "g-work"),
        favourite(project("c", "Charlie")),
        in_group(project("d", "Delta"), "g-personal"),
    ];
    let mut gone = project("z", "Zulu");
    gone.is_deleted = true;
    (live, vec![gone])
}

fn ids(projects: Vec<&Project>) -> Vec<&str> {
    projects.into_iter().map(|p| p.id.as_str()).collect()
}

fn group_view(id: &str) -> View {
    View::Group { id: id.to_string() }
}

// ---- resolve_view ----

#[test]
fn all_is_every_live_project() {
    let (live, deleted) = fixture();
    assert_eq!(
        ids(resolve_view(&View::All, &live, &deleted, "")),
        ["a", "b", "c", "d"]
    );
}

#[test]
fn favorites_is_the_favourited_live_projects() {
    let (live, deleted) = fixture();
    assert_eq!(
        ids(resolve_view(&View::Favorites, &live, &deleted, "")),
        ["a", "c"]
    );
}

#[test]
fn a_group_is_the_projects_carrying_its_id() {
    let (live, deleted) = fixture();
    assert_eq!(
        ids(resolve_view(&group_view("g-work"), &live, &deleted, "")),
        ["a", "b"]
    );
}

#[test]
fn ungrouped_catches_projects_with_no_group() {
    let (live, deleted) = fixture();
    assert_eq!(
        ids(resolve_view(&View::Ungrouped, &live, &deleted, "")),
        ["c"]
    );
}

#[test]
fn bin_is_the_deleted_projects_which_are_not_in_the_live_list() {
    let (live, deleted) = fixture();
    assert_eq!(ids(resolve_view(&View::Bin, &live, &deleted, "")), ["z"]);
}

#[test]
fn preserves_the_order_it_was_given_because_the_caller_already_sorted() {
    let (mut live, deleted) = fixture();
    live.reverse();
    assert_eq!(
        ids(resolve_view(&View::All, &live, &deleted, "")),
        ["d", "c", "b", "a"]
    );
}

#[test]
fn applies_the_query_within_the_view_rather_than_across_everything() {
    let (live, deleted) = fixture();
    let work = group_view("g-work");
    assert_eq!(ids(resolve_view(&work, &live, &deleted, "a")), ["a", "b"]);
    assert!(resolve_view(&work, &live, &deleted, "charlie").is_empty());
}

#[test]
fn returns_an_empty_list_for_a_group_with_no_members() {
    let (live, deleted) = fixture();
    assert!(resolve_view(&group_view("g-nobody"), &live, &deleted, "").is_empty());
}

// ---- matches_query: free text ----

fn searchable() -> Project {
    let mut p = project("p", "Friction Engine");
    p.directory = r"D:\Games\fe".to_string();
    p.tags = vec!["rust".to_string(), "game".to_string()];
    p
}

#[test]
fn matches_the_name_ignoring_case() {
    assert!(matches_query(&searchable(), "FRICTION"));
}

#[test]
fn matches_the_path() {
    assert!(matches_query(&searchable(), "games"));
}

#[test]
fn matches_a_tag() {
    assert!(matches_query(&searchable(), "rust"));
}

#[test]
fn does_not_match_something_absent() {
    assert!(!matches_query(&searchable(), "unreal"));
}

#[test]
fn treats_an_empty_or_whitespace_query_as_matching_everything() {
    assert!(matches_query(&searchable(), ""));
    assert!(matches_query(&searchable(), "   "));
}

#[test]
fn matches_a_property_value_in_free_text_too() {
    let p = with_properties(project("p", "Friction Engine"), &[("client", "Acme Corp")]);
    assert!(matches_query(&p, "acme"));
}

#[test]
fn does_not_match_a_property_name_in_free_text() {
    let p = with_properties(project("p", "Friction Engine"), &[("client", "Acme Corp")]);
    assert!(!matches_query(&p, "client"));
}

// ---- matches_query: the name: value syntax ----

fn acme() -> Project {
    with_properties(project("acme", "Alpha"), &[("client", "Acme Corp")])
}

fn globex() -> Project {
    with_properties(project("globex", "Bravo"), &[("client", "Globex")])
}

fn lookalike() -> Project {
    project("none", "Acme lookalike")
}

#[test]
fn matches_on_the_named_property_only() {
    assert!(matches_query(&acme(), "client: Acme"));
    assert!(!matches_query(&globex(), "client: Acme"));
}

#[test]
fn does_not_match_a_project_lacking_the_property_even_if_the_text_appears_elsewhere() {
    // The whole point of the syntax: "client: acme" is a question about the
    // client property, not a free-text search that happens to spell acme.
    assert!(!matches_query(&lookalike(), "client: acme"));
    assert!(matches_query(&lookalike(), "acme"));
}

#[test]
fn is_case_insensitive_in_both_the_key_and_the_value() {
    assert!(matches_query(&acme(), "CLIENT: acme"));
    assert!(matches_query(&acme(), "client: ACME"));
}

#[test]
fn matches_a_stored_key_of_any_case() {
    let p = with_properties(project("p", "Alpha"), &[("Client", "Acme Corp")]);
    assert!(matches_query(&p, "client: acme"));
}

#[test]
fn tolerates_space_on_either_side_of_the_colon_or_none() {
    for q in ["client:Acme", "client : Acme", "  client:  Acme  "] {
        assert!(matches_query(&acme(), q), "{q:?} should match");
    }
}

#[test]
fn matches_every_project_that_has_the_property_when_no_value_is_given() {
    assert!(matches_query(&acme(), "client:"));
    assert!(matches_query(&globex(), "client:"));
    assert!(!matches_query(&lookalike(), "client:"));
}

#[test]
fn matches_a_substring_of_the_value_like_free_text_does() {
    assert!(matches_query(&acme(), "client: corp"));
}

#[test]
fn falls_back_to_free_text_when_nothing_precedes_the_colon() {
    // ":30" is not a property query — there is no name — so it searches text.
    let timed = project("t", "build at 12:30");
    assert!(matches_query(&timed, ":30"));
}

#[test]
fn falls_back_to_free_text_when_the_name_holds_whitespace() {
    let timed = project("t", "build at 12:30");
    assert!(matches_query(&timed, "build at 12:30"));
}

#[test]
fn treats_a_windows_drive_letter_as_free_text_not_a_property() {
    // "D:" would otherwise be read as the property "D", and every path search
    // on Windows starts this way.
    let mut on_drive = project("p", "Friction Engine");
    on_drive.directory = r"D:\Games\fe".to_string();
    assert!(matches_query(&on_drive, r"D:\Games"));
}

// ---- is_property_query ----

#[test]
fn recognises_a_name_value_query() {
    assert!(is_property_query("client: acme"));
    assert!(is_property_query("client:"));
}

#[test]
fn rejects_free_text_a_drive_letter_and_a_bare_colon() {
    assert!(!is_property_query("acme"));
    assert!(!is_property_query(r"D:\Games"));
    assert!(!is_property_query(":30"));
    assert!(!is_property_query(""));
}

#[test]
fn rejects_a_name_holding_whitespace() {
    assert!(!is_property_query("build at 12:30"));
}

#[test]
fn splits_on_the_first_colon_only() {
    // The value may itself hold a colon — a URL, a time.
    let p = with_properties(project("p", "Alpha"), &[("deadline", "friday 12:30")]);
    assert!(matches_query(&p, "deadline: 12:30"));
}

// ---- property_keys ----

#[test]
fn lists_the_distinct_keys_across_the_given_projects_sorted() {
    let one = with_properties(
        project("1", "One"),
        &[("client", "Acme"), ("engine", "Unreal")],
    );
    let two = with_properties(
        project("2", "Two"),
        &[("client", "Globex"), ("priority", "high")],
    );
    assert_eq!(
        property_keys([&one, &two]),
        ["client", "engine", "priority"]
    );
}

#[test]
fn is_empty_when_no_project_has_any() {
    assert!(property_keys([&project("p", "Plain")]).is_empty());
}

#[test]
fn folds_keys_differing_only_in_case_so_the_hint_does_not_list_both() {
    let one = with_properties(project("1", "One"), &[("Client", "Acme")]);
    let two = with_properties(project("2", "Two"), &[("client", "Globex")]);
    assert_eq!(property_keys([&one, &two]), ["client"]);
}

#[test]
fn reads_the_live_and_binned_lists_chained() {
    let live = [with_properties(project("1", "One"), &[("client", "Acme")])];
    let binned = [with_properties(project("2", "Two"), &[("engine", "Godot")])];
    assert_eq!(
        property_keys(live.iter().chain(&binned)),
        ["client", "engine"]
    );
}

// ---- view_counts ----

#[test]
fn counts_every_view_including_a_group_with_no_members() {
    let (live, deleted) = fixture();
    let groups = [
        group("g-work", "Work", 0),
        group("g-personal", "Personal", 1),
        group("g-empty", "Empty", 2),
    ];
    let expected = ViewCounts {
        all: 4,
        favorites: 2,
        ungrouped: 1,
        bin: 1,
        groups: BTreeMap::from([
            ("g-work".to_string(), 2),
            ("g-personal".to_string(), 1),
            ("g-empty".to_string(), 0),
        ]),
    };
    assert_eq!(view_counts(&live, &deleted, &groups), expected);
}

#[test]
fn does_not_count_a_project_whose_group_was_deleted_as_a_member_of_anything() {
    let (mut live, deleted) = fixture();
    live.push(in_group(project("o", "Orphan"), "g-gone"));
    let groups = [
        group("g-work", "Work", 0),
        group("g-personal", "Personal", 1),
    ];

    let counts = view_counts(&live, &deleted, &groups);
    assert_eq!(counts.all, 5);
    assert_eq!(counts.ungrouped, 1);
    assert_eq!(
        counts.groups,
        BTreeMap::from([("g-work".to_string(), 2), ("g-personal".to_string(), 1)])
    );
}

#[test]
fn an_empty_group_counts_zero_rather_than_vanishing() {
    let counts = view_counts(&[], &[], &[group("g-work", "Work", 0)]);
    assert_eq!(counts.groups.get("g-work"), Some(&0));
}

// ---- the wire shape the GUI's `View` and `ViewCounts` mirror ----

#[test]
fn a_view_serializes_as_the_frontend_union() {
    let cases = [
        (View::All, json!({ "kind": "all" })),
        (View::Favorites, json!({ "kind": "favorites" })),
        (View::Ungrouped, json!({ "kind": "ungrouped" })),
        (View::Bin, json!({ "kind": "bin" })),
        (
            group_view("g-work"),
            json!({ "kind": "group", "id": "g-work" }),
        ),
    ];
    for (view, wire) in cases {
        assert_eq!(serde_json::to_value(&view).unwrap(), wire);
        assert_eq!(serde_json::from_value::<View>(wire).unwrap(), view);
    }
}

#[test]
fn a_group_view_without_an_id_is_rejected() {
    assert!(serde_json::from_value::<View>(json!({ "kind": "group" })).is_err());
}

#[test]
fn view_counts_serialize_with_groups_as_an_object() {
    let counts = view_counts(&[], &[], &[group("g-work", "Work", 0)]);
    assert_eq!(
        serde_json::to_value(&counts).unwrap(),
        json!({ "all": 0, "favorites": 0, "ungrouped": 0, "bin": 0, "groups": { "g-work": 0 } })
    );
}
