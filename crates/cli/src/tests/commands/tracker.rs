//! `--tracker <kind>`: the flag clap parses, and the filtering it drives.

use clap::Parser;
use indexer_core::domain::Project;
use serde_json::{json, Value};

use crate::commands::{unique_kinds, with_tracker, Command, TrackerKind};
use crate::{Cli, Invocation};

/// A project from its stored JSON shape, as in the output tests.
fn project(name: &str, trackers: Value) -> Project {
    serde_json::from_value(json!({
        "id": name,
        "name": name,
        "directory": format!("/home/me/work/{name}"),
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "last_opened_at": null,
        "open_with": null,
        "notes": null,
        "trackers": trackers,
    }))
    .unwrap()
}

fn git() -> Value {
    json!({ "Git": {
        "repo_root": "/", "dirty": false, "detached_head": false, "repo_url": null,
        "web_url": null, "contributors": [], "curr_branch": "main", "branches": null,
        "commit_hash": null,
    }})
}

fn unreal() -> Value {
    json!({ "Unreal": {
        "project_root": "/", "project_name": "x", "uproject_path": "/x.uproject",
        "engine_association": "5.3", "category": null, "description": null,
        "modules": [], "plugins": [], "vcs_provider": null,
    }})
}

fn corpus() -> Vec<Project> {
    vec![
        project("git-only", json!([git()])),
        project("unreal-only", json!([unreal()])),
        project("both", json!([git(), unreal()])),
        project("untracked", json!([])),
    ]
}

fn names(projects: Vec<Project>) -> Vec<String> {
    projects.into_iter().map(|p| p.name).collect()
}

#[test]
fn a_kind_keeps_every_project_carrying_it() {
    let kept = with_tracker(corpus(), &[TrackerKind::Git]);

    assert_eq!(names(kept), ["git-only", "both"]);
}

#[test]
fn a_project_with_several_trackers_matches_each_of_them() {
    let kept = with_tracker(corpus(), &[TrackerKind::Unreal]);

    assert_eq!(names(kept), ["unreal-only", "both"]);
}

#[test]
fn several_kinds_keep_a_project_carrying_any_of_them() {
    let kept = with_tracker(corpus(), &[TrackerKind::Git, TrackerKind::Unreal]);

    assert_eq!(names(kept), ["git-only", "unreal-only", "both"]);
}

#[test]
fn no_flag_keeps_the_corpus_untouched() {
    let kept = with_tracker(corpus(), &[]);

    assert_eq!(
        names(kept),
        ["git-only", "unreal-only", "both", "untracked"]
    );
}

#[test]
fn repeated_kinds_are_listed_once_in_the_order_given() {
    let kinds = [TrackerKind::Unreal, TrackerKind::Git, TrackerKind::Unreal];

    assert_eq!(
        unique_kinds(&kinds),
        [TrackerKind::Unreal, TrackerKind::Git]
    );
}

/// The parsed `--tracker` kinds of a `list` invocation, or a panic naming
/// what came back instead.
fn parsed_list_tracker(argv: &[&str]) -> Vec<TrackerKind> {
    let cli = Cli::try_parse_from(argv).expect("arguments should parse");
    match cli.invocation {
        Some(Invocation::Command(Command::List(args))) => args.tracker,
        other => panic!("expected `list`, got {other:?}"),
    }
}

#[test]
fn the_flag_parses_long_and_short_on_list() {
    assert_eq!(
        parsed_list_tracker(&["indexer", "list", "--tracker", "git"]),
        [TrackerKind::Git]
    );
    assert_eq!(
        parsed_list_tracker(&["indexer", "list", "-t", "unreal"]),
        [TrackerKind::Unreal]
    );
    assert!(parsed_list_tracker(&["indexer", "list"]).is_empty());
}

#[test]
fn several_kinds_come_from_a_comma_list_or_a_repeated_flag() {
    let expected = [TrackerKind::Git, TrackerKind::Unreal];

    assert_eq!(
        parsed_list_tracker(&["indexer", "list", "-t", "git,unreal"]),
        expected
    );
    assert_eq!(
        parsed_list_tracker(&["indexer", "list", "-t", "git", "-t", "unreal"]),
        expected
    );
}

#[test]
fn a_kind_no_detector_reports_is_rejected() {
    let parsed = Cli::try_parse_from(["indexer", "list", "--tracker", "godot"]);

    assert!(parsed.is_err(), "an unknown kind should not parse");
}
