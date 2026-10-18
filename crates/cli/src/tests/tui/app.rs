//! Navigation rules for the read-only TUI browser, driven as pure state — no
//! terminal, no database — the way the form's state is tested.

use indexer_core::domain::views::View;
use indexer_core::domain::{Group, Project};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

use crate::tui::app::{view_label, Action, App, Mode, Pane};

fn project(name: &str, group_id: Option<&str>, favorite: bool) -> Project {
    serde_json::from_value(json!({
        "id": format!("{name}-id"),
        "name": name,
        "directory": format!("/tmp/{name}"),
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "last_opened_at": null,
        "trackers": [],
        "favorite": favorite,
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

/// Two groups (Work, Clients), four live projects spread across them and
/// Ungrouped, one favourite, and one in the bin.
fn fixture() -> App {
    let projects = vec![
        project("alpha", None, false),              // ungrouped
        project("bravo", Some("g-work"), true),     // Work, favourite
        project("charlie", Some("g-work"), false),  // Work
        project("delta", Some("g-clients"), false), // Clients
    ];
    let deleted = vec![project("echo", None, false)];
    let groups = vec![group("g-work", "Work"), group("g-clients", "Clients")];
    App::new(projects, deleted, groups)
}

fn press(app: &mut App, code: KeyCode) {
    app.handle(KeyEvent::new(code, KeyModifiers::NONE));
}

fn type_str(app: &mut App, text: &str) {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch));
    }
}

fn names(app: &App) -> Vec<String> {
    app.visible().iter().map(|p| p.name.clone()).collect()
}

#[test]
fn starts_on_all_focused_on_the_views() {
    let app = fixture();
    assert_eq!(app.focus(), Pane::Views);
    assert_eq!(app.active(), &View::All);
    assert_eq!(names(&app), ["alpha", "bravo", "charlie", "delta"]);
    assert_eq!(
        app.selected_project().map(|p| p.name.as_str()),
        Some("alpha")
    );
}

#[test]
fn moving_down_the_views_switches_the_active_view() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('j')); // Favourites
    assert_eq!(app.active(), &View::Favorites);
    assert_eq!(names(&app), ["bravo"]);
    press(&mut app, KeyCode::Char('j')); // Ungrouped
    assert_eq!(app.active(), &View::Ungrouped);
    assert_eq!(names(&app), ["alpha"]);
    press(&mut app, KeyCode::Char('j')); // Bin — drawn from the deleted list
    assert_eq!(app.active(), &View::Bin);
    assert_eq!(names(&app), ["echo"]);
}

#[test]
fn the_views_cursor_clamps_at_both_ends() {
    let mut app = fixture();
    for _ in 0..10 {
        press(&mut app, KeyCode::Up);
    }
    assert_eq!(app.active(), &View::All);
    for _ in 0..10 {
        press(&mut app, KeyCode::Down);
    }
    assert_eq!(app.active(), &View::Bin);
}

#[test]
fn entering_the_groups_pane_shows_the_first_group_at_once() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']')); // jump to Groups
    assert_eq!(app.focus(), Pane::Groups);
    // The first group is active immediately, not only after a move.
    assert_eq!(
        app.active(),
        &View::Group {
            id: "g-work".into()
        }
    );
    assert_eq!(names(&app), ["bravo", "charlie"]);
    press(&mut app, KeyCode::Char('j')); // Clients
    assert_eq!(
        app.active(),
        &View::Group {
            id: "g-clients".into()
        }
    );
    assert_eq!(names(&app), ["delta"]);
}

#[test]
fn g_and_capital_g_jump_to_the_ends_of_a_pane() {
    // Views pane: G to the last view (Bin), g back to the first (All).
    let mut app = fixture();
    press(&mut app, KeyCode::Char('G'));
    assert_eq!(app.active(), &View::Bin);
    press(&mut app, KeyCode::Char('g'));
    assert_eq!(app.active(), &View::All);

    // List pane: no groups, so Tab lands straight on the list, still on All.
    let mut app = App::new(
        vec![
            project("a", None, false),
            project("b", None, false),
            project("c", None, false),
        ],
        vec![],
        vec![],
    );
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus(), Pane::List);
    press(&mut app, KeyCode::Char('G'));
    assert_eq!(app.selected_project().map(|p| p.name.as_str()), Some("c"));
    press(&mut app, KeyCode::Char('g'));
    assert_eq!(app.selected_project().map(|p| p.name.as_str()), Some("a"));
}

#[test]
fn tab_cycles_views_groups_list() {
    let mut app = fixture();
    assert_eq!(app.focus(), Pane::Views);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus(), Pane::Groups);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus(), Pane::List);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus(), Pane::Views);
}

#[test]
fn tab_skips_an_empty_groups_pane() {
    let mut app = App::new(vec![project("alpha", None, false)], vec![], vec![]);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus(), Pane::List);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus(), Pane::Views);
    // `]` has nothing to land on, so it is a no-op.
    press(&mut app, KeyCode::Char(']'));
    assert_eq!(app.focus(), Pane::Views);
}

#[test]
fn the_list_cursor_moves_within_the_active_view() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('l')); // focus Groups
    press(&mut app, KeyCode::Char('l')); // focus List (Work: bravo, charlie)
    assert_eq!(app.focus(), Pane::List);
    assert_eq!(
        app.selected_project().map(|p| p.name.as_str()),
        Some("bravo")
    );
    press(&mut app, KeyCode::Char('j'));
    assert_eq!(
        app.selected_project().map(|p| p.name.as_str()),
        Some("charlie")
    );
    press(&mut app, KeyCode::Char('j')); // clamps at the end
    assert_eq!(
        app.selected_project().map(|p| p.name.as_str()),
        Some("charlie")
    );
}

#[test]
fn leaving_and_returning_to_the_list_keeps_the_view_but_resets_the_cursor() {
    let mut app = fixture();
    // Into the list on All, move to the third project.
    press(&mut app, KeyCode::Char('l')); // Groups
    press(&mut app, KeyCode::Char('h')); // back to Views
    assert_eq!(app.focus(), Pane::Views);
    assert_eq!(app.active(), &View::All);
}

#[test]
fn q_esc_and_ctrl_c_quit() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('q'));
    assert!(app.quit());

    let mut app = fixture();
    press(&mut app, KeyCode::Esc);
    assert!(app.quit());

    let mut app = fixture();
    app.handle(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(app.quit());
}

#[test]
fn colon_opens_the_command_line_and_typing_edits_it() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(':'));
    assert_eq!(app.mode(), &Mode::Command(String::new()));
    type_str(&mut app, "open app");
    assert_eq!(app.mode(), &Mode::Command("open app".into()));
    press(&mut app, KeyCode::Backspace);
    assert_eq!(app.mode(), &Mode::Command("open ap".into()));
}

#[test]
fn nav_keys_type_into_the_command_line_rather_than_acting() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(':'));
    // j, k, q would move or quit in the browser; here they are just text.
    type_str(&mut app, "jkq");
    assert_eq!(app.mode(), &Mode::Command("jkq".into()));
    assert!(!app.quit());
}

#[test]
fn esc_cancels_the_command_line_without_quitting() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(':'));
    type_str(&mut app, "open app");
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.mode(), &Mode::Normal);
    assert!(!app.quit());
}

#[test]
fn enter_runs_the_typed_command_and_returns_to_normal() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(':'));
    type_str(&mut app, "favorite app");
    let action = app.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(action, Some(Action::Run("favorite app".into())));
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn an_empty_command_line_runs_nothing() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(':'));
    let action = app.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(action, None);
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn view_label_names_every_view() {
    let groups = vec![group("g-work", "Work")];
    assert_eq!(view_label(&View::All, &groups), "All");
    assert_eq!(view_label(&View::Favorites, &groups), "Favourites");
    assert_eq!(view_label(&View::Ungrouped, &groups), "Ungrouped");
    assert_eq!(view_label(&View::Bin, &groups), "Bin");
    assert_eq!(
        view_label(
            &View::Group {
                id: "g-work".into()
            },
            &groups
        ),
        "Work"
    );
    assert_eq!(
        view_label(
            &View::Group {
                id: "missing".into()
            },
            &groups
        ),
        "Unknown group"
    );
}
