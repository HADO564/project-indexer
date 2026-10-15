//! The browser's drawing, rendered to a `TestBackend` so the panes are
//! exercised without a terminal — this catches a layout that panics or a label
//! that goes missing, which the pure-state tests in `tests/tui/app.rs` cannot.

use indexer_core::domain::{Group, Project};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use serde_json::json;

use crate::tui::app::App;
use crate::tui::ui::app::draw;

fn project(name: &str, group_id: Option<&str>) -> Project {
    serde_json::from_value(json!({
        "id": format!("{name}-0000-4000-8000-000000000000"),
        "name": name,
        "directory": format!("/home/me/{name}"),
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "last_opened_at": null,
        "trackers": [],
        "group_id": group_id,
    }))
    .unwrap()
}

fn group(id: &str, name: &str) -> Group {
    serde_json::from_value(json!({
        "id": id,
        "name": name,
        "color": "cyan",
        "icon": "briefcase",
        "position": 0,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
    }))
    .unwrap()
}

fn screen(width: u16, height: u16) -> Vec<String> {
    let app = App::new(
        vec![project("alpha", None), project("bravo", Some("g-work"))],
        vec![],
        vec![group("g-work", "Work")],
    );
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(frame, &app)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn renders_the_three_panes_the_status_and_the_hint() {
    let text = screen(90, 20).join("\n");
    for needle in [
        "VIEWS",    // sidebar: the fixed views block
        "All",      //   its first entry
        "GROUPS",   // sidebar: the groups block
        "Work",     //   the one group
        "PROJECTS", // the list pane title
        "alpha",    //   a project in the All view
        "DETAIL",   // the detail pane
        "projects", // the status line ("2 projects · All")
        "quit",     // the hint line
    ] {
        assert!(text.contains(needle), "expected {needle:?} in:\n{text}");
    }
}

#[test]
fn does_not_panic_in_a_small_terminal() {
    // A cramped terminal must still render — ratatui clips, it does not crash.
    let _ = screen(20, 6);
}
