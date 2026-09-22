use indexer_core::domain::Project;
use serde_json::{json, Value};

use crate::commands::{Failure, Outcome};
use crate::output::json::{write, write_error};

/// A project from its stored JSON shape, as in `human.rs`.
fn project(name: &str, trackers: Value) -> Project {
    serde_json::from_value(json!({
        "id": format!("{name}-id"),
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
        "repo_root": "/", "dirty": true, "detached_head": false, "repo_url": null,
        "web_url": null, "contributors": [], "curr_branch": "main", "branches": null,
        "commit_hash": null,
    }})
}

fn render(outcome: &Outcome) -> Value {
    let mut out = Vec::new();
    write(&mut out, outcome).unwrap();
    serde_json::from_slice(&out).unwrap()
}

fn render_error(error: anyhow::Error) -> Value {
    let mut out = Vec::new();
    write_error(&mut out, &error).unwrap();
    serde_json::from_slice(&out).unwrap()
}

#[test]
fn results_are_wrapped_in_the_versioned_envelope() {
    let doc = render(&Outcome::Projects {
        projects: vec![project("app", json!([]))],
        query: None,
        tracker: Vec::new(),
    });
    assert_eq!(doc["schema"], 1);
    assert_eq!(doc["data"][0]["name"], "app");
    assert_eq!(doc["data"][0]["directory"], "/home/me/work/app");
}

#[test]
fn a_query_matching_nothing_is_an_empty_array_not_an_error() {
    let doc = render(&Outcome::Projects {
        projects: Vec::new(),
        query: Some("nothing".into()),
        tracker: Vec::new(),
    });
    assert_eq!(doc["data"], json!([]));
    assert!(doc.get("error").is_none());
}

#[test]
fn a_tracker_carries_its_kind_under_one_key_beside_its_fields() {
    let doc = render(&Outcome::Project {
        project: Box::new(project("app", json!([git()]))),
        tracker: Vec::new(),
        group: None,
    });
    let tracker = &doc["data"]["trackers"][0];
    assert_eq!(tracker["kind"], "git");
    assert_eq!(tracker["curr_branch"], "main");
    assert_eq!(tracker["dirty"], true);
    assert!(
        tracker.get("Git").is_none(),
        "serde's variant key leaked: {tracker}"
    );
}

#[test]
fn a_query_matching_nothing_is_a_not_found_error() {
    let doc = render_error(
        Failure::NotFound {
            query: "nope".into(),
        }
        .into(),
    );
    assert_eq!(doc["schema"], 1);
    assert_eq!(doc["error"]["kind"], "not_found");
    assert_eq!(doc["error"]["query"], "nope");
    assert!(doc["error"].get("matches").is_none());
    assert!(doc.get("data").is_none());
}

#[test]
fn several_matches_are_listed_as_projects_with_a_one_line_message() {
    let doc = render_error(
        Failure::Ambiguous {
            query: "app".into(),
            matches: vec![project("app", json!([])), project("app-gateway", json!([]))],
            tracker: Vec::new(),
        }
        .into(),
    );
    assert_eq!(doc["error"]["kind"], "ambiguous");
    assert_eq!(
        doc["error"]["message"],
        "multiple matches found, which one did you mean?"
    );
    let names: Vec<&str> = doc["error"]["matches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["app", "app-gateway"]);
}

#[test]
fn any_other_failure_is_a_plain_error_with_its_cause_chain() {
    let error = anyhow::anyhow!("database is locked").context("could not open projects.db");
    let doc = render_error(error);
    assert_eq!(doc["error"]["kind"], "error");
    assert_eq!(
        doc["error"]["message"],
        "could not open projects.db: database is locked"
    );
}

#[test]
fn add_reports_the_project_and_whether_it_was_already_tracked() {
    // The one fact `ensure_project` will not tell a caller, so a script can
    // tell a new registration from a no-op without diffing the database.
    let doc = render(&Outcome::Added {
        project: Box::new(project("app", json!([]))),
        already_tracked: false,
    });
    assert_eq!(doc["schema"], 1);
    assert_eq!(doc["data"]["project"]["name"], "app");
    assert_eq!(doc["data"]["already_tracked"], false);

    let doc = render(&Outcome::Added {
        project: Box::new(project("app", json!([]))),
        already_tracked: true,
    });
    assert_eq!(doc["data"]["already_tracked"], true);
}

#[test]
fn untrack_and_open_report_the_project_itself() {
    let doc = render(&Outcome::Untracked {
        project: Box::new(project("app", json!([]))),
    });
    // The row is gone from the database by now, so this document is the only
    // record of what was removed.
    assert_eq!(doc["data"]["name"], "app");
    assert_eq!(doc["data"]["id"], "app-id");

    let doc = render(&Outcome::Opened {
        project: Box::new(project("app", git_tracker_project())),
    });
    assert_eq!(doc["data"]["name"], "app");
    assert_eq!(doc["data"]["trackers"][0]["kind"], "git");
}

#[test]
fn declining_a_confirmation_is_data_not_an_error() {
    // Nothing went wrong, so it is a successful run with `data` — a reader
    // must not have to treat "you said no" as a failure.
    let doc = render(&Outcome::Cancelled);
    assert_eq!(doc["schema"], 1);
    assert_eq!(doc["data"]["cancelled"], true);
    assert!(doc.get("error").is_none());
}

/// A project carrying one git tracker, for the shape assertions above.
fn git_tracker_project() -> Value {
    json!([git()])
}
