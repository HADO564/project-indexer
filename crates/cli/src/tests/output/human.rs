use indexer_core::domain::Project;
use serde_json::json;

use crate::commands::{Outcome, TrackerKind};
use crate::output::color::Color;
use crate::output::human::{project_table, write, TableStyle};
use crate::output::Colors;

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

fn table(projects: &[Project], style: TableStyle) -> String {
    let refs: Vec<&Project> = projects.iter().collect();
    project_table(&refs, &style)
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
fn a_plain_table_is_bordered_and_lines_up_every_column() {
    let rule = |left: char, middle: char, right: char| {
        let runs = [13, 19, 13, 13].map(|n| "─".repeat(n));
        format!("{left}{}{right}\n", runs.join(&middle.to_string()))
    };

    let expected = [
        rule('╭', '┬', '╮'),
        "│ NAME        │ DIRECTORY         │ TRACKERS    │ LAST OPENED │\n".to_string(),
        rule('├', '┼', '┤'),
        "│ app         │ work/app          │ Git, Unreal │ 2026-09-14  │\n".to_string(),
        "│ app-gateway │ infra/app-gateway │ -           │ never       │\n".to_string(),
        rule('╰', '┴', '╯'),
    ]
    .concat();

    assert_eq!(table(&two_projects(), TableStyle::default()), expected);
}

#[test]
fn in_a_wide_terminal_the_table_fills_seventy_percent_and_is_centred() {
    let style = TableStyle {
        width: Some(100),
        ..TableStyle::default()
    };

    let output = table(&two_projects(), style);

    // 70 columns of table, centred in 100: 15 spaces before it.
    for line in output.lines() {
        assert_eq!(line.chars().count(), 85, "{line:?}");
        let indent = line.chars().take_while(|&c| c == ' ').count();
        assert_eq!(indent, 15, "{line:?}");
    }
}

#[test]
fn a_table_wider_than_the_terminal_is_neither_stretched_nor_indented() {
    let style = TableStyle {
        width: Some(40),
        ..TableStyle::default()
    };

    assert_eq!(
        table(&two_projects(), style),
        table(&two_projects(), TableStyle::default())
    );
}

#[test]
fn colour_paints_the_header_and_folder_without_moving_any_column() {
    let style = TableStyle {
        folder_color: Some(Color::Cyan),
        header_color: Some(Color::Magenta),
        width: Some(120),
        ..Default::default()
    };

    let coloured = table(&two_projects(), style);

    assert!(coloured.contains("\x1b[1;35mNAME\x1b[0m"), "{coloured:?}");
    assert!(
        coloured.contains("work/\x1b[1;36mapp\x1b[0m"),
        "{coloured:?}"
    );
    let plain = TableStyle {
        width: Some(120),
        ..TableStyle::default()
    };
    assert_eq!(strip_ansi(&coloured), table(&two_projects(), plain));
}

#[test]
fn a_directory_at_the_root_shows_just_its_folder() {
    let output = table(
        &[project("top", "/top", json!([]), None)],
        TableStyle::default(),
    );

    let row = output.lines().nth(3).unwrap();
    assert!(row.starts_with("│ top  │ top "), "{output}");
}

/// A table of [`two_projects`] showing `trackers`' own columns.
fn tracker_table(trackers: Vec<TrackerKind>) -> String {
    table(
        &two_projects(),
        TableStyle {
            tracker: trackers,
            ..Default::default()
        },
    )
}

#[test]
fn a_kind_replaces_the_trackers_column_with_its_own() {
    let rendered = tracker_table(vec![TrackerKind::Git]);

    assert!(rendered.contains("BRANCH"), "{rendered}");
    assert!(rendered.contains("CHANGES"), "{rendered}");
    assert!(!rendered.contains("TRACKERS"), "{rendered}");
    // `two_projects`'s git tracker is on `main` with no uncommitted changes.
    assert!(rendered.contains("main"), "{rendered}");
    assert!(rendered.contains("clean"), "{rendered}");
}

#[test]
fn several_kinds_show_their_columns_in_the_order_given() {
    let rendered = tracker_table(vec![TrackerKind::Unreal, TrackerKind::Git]);
    let header = rendered.lines().nth(1).expect("a header row");

    let engine = header.find("ENGINE").expect("an ENGINE column");
    let branch = header.find("BRANCH").expect("a BRANCH column");
    assert!(engine < branch, "{header}");
}

#[test]
fn a_project_without_the_kind_shows_a_dash_in_each_of_its_columns() {
    // `app-gateway` carries no tracker at all, so every tracker column is `-`.
    let rendered = tracker_table(vec![TrackerKind::Git, TrackerKind::Unreal]);
    let row = rendered
        .lines()
        .find(|line| line.contains("app-gateway"))
        .expect("a row for app-gateway");

    let cells: Vec<&str> = row.split('│').map(str::trim).collect();
    assert_eq!(&cells[3..6], ["-", "-", "-"], "{row}");
}

/// `show`'s detail view for one project, as `write` renders it.
fn detail(project: Project, trackers: Vec<TrackerKind>) -> String {
    let outcome = Outcome::Project {
        project: Box::new(project),
        tracker: trackers,
    };
    let mut out = Vec::new();
    write(
        &mut out,
        &outcome,
        Colors {
            folder: Color::DEFAULT_FOLDER,
            header: Color::DEFAULT_HEADER,
        },
    )
    .unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn a_kind_adds_a_section_to_the_detail_view() {
    let rendered = detail(two_projects().remove(0), vec![TrackerKind::Git]);

    assert!(rendered.contains("\n  git\n"), "{rendered}");
    assert!(rendered.contains("branch"), "{rendered}");
    assert!(rendered.contains("changes"), "{rendered}");
}

#[test]
fn no_kind_leaves_the_detail_view_as_it_was() {
    let rendered = detail(two_projects().remove(0), Vec::new());

    assert!(!rendered.contains("git"), "{rendered}");
    assert_eq!(rendered.lines().count(), 3, "{rendered}");
}

#[test]
fn a_kind_the_project_lacks_prints_no_section() {
    // The second project carries no trackers at all.
    let rendered = detail(two_projects().remove(1), vec![TrackerKind::Unreal]);

    assert!(!rendered.contains("unreal"), "{rendered}");
    assert_eq!(rendered.lines().count(), 3, "{rendered}");
}
