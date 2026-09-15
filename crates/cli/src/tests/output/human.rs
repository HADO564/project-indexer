use indexer_core::domain::Project;
use serde_json::json;

use crate::output::color::Color;
use crate::output::human::project_table;

/// A project from its stored JSON shape, so the test needs neither `chrono`
/// nor every detector's info struct spelled out.
fn project(
    name: &str,
    directory: &str,
    trackers: serde_json::Value,
    last_opened: Option<&str>,
) -> Project {
    serde_json::from_value(json!({
        "id": name,
        "name": name,
        "directory": directory,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "last_opened_at": last_opened,
        "open_with": null,
        "notes": null,
        "trackers": trackers,
    }))
    .unwrap()
}

fn git() -> serde_json::Value {
    json!({ "Git": {
        "repo_root": "/", "dirty": false, "detached_head": false, "repo_url": null,
        "web_url": null, "contributors": [], "curr_branch": "main", "branches": null,
        "commit_hash": null,
    }})
}

fn unreal() -> serde_json::Value {
    json!({ "Unreal": {
        "project_root": "/", "project_name": "x", "uproject_path": "/x.uproject",
        "engine_association": "5.3", "category": null, "description": null,
        "modules": [], "plugins": [], "vcs_provider": null,
    }})
}

fn two_projects() -> Vec<Project> {
    vec![
        project(
            "app",
            "/home/me/work/app",
            json!([git(), unreal()]),
            Some("2026-09-14T10:00:00Z"),
        ),
        project("app-gateway", "/home/me/infra/app-gateway", json!([]), None),
    ]
}

/// Drops ANSI colour codes, leaving the text a terminal would show.
fn strip_ansi(text: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for c in text.chars() {
        match (in_code, c) {
            (false, '\x1b') => in_code = true,
            (false, _) => out.push(c),
            (true, 'm') => in_code = false,
            (true, _) => {}
        }
    }
    out
}

#[test]
fn lines_up_every_column_under_its_header() {
    let projects = two_projects();
    let refs: Vec<&Project> = projects.iter().collect();

    let table = project_table(&refs, None);

    assert_eq!(
        table,
        "NAME         DIRECTORY          TRACKERS     LAST OPENED\n\
         app          work/app           Git, Unreal  2026-09-14\n\
         app-gateway  infra/app-gateway  -            never\n"
    );
}

#[test]
fn colour_paints_only_the_folder_and_keeps_columns_aligned() {
    let projects = two_projects();
    let refs: Vec<&Project> = projects.iter().collect();

    let coloured = project_table(&refs, Some(Color::Cyan));

    assert!(
        coloured.contains("work/\x1b[1;36mapp\x1b[0m"),
        "{coloured:?}"
    );
    assert_eq!(strip_ansi(&coloured), project_table(&refs, None));
}

#[test]
fn a_directory_at_the_root_shows_just_its_folder() {
    let projects = [project("top", "/top", json!([]), None)];
    let refs: Vec<&Project> = projects.iter().collect();

    let table = project_table(&refs, None);

    assert!(
        table.lines().nth(1).unwrap().starts_with("top   top"),
        "{table:?}"
    );
}
