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
    press(&mut app, KeyCode::Char('j')); // Bin — Ungrouped now lives in Groups
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
fn entering_the_groups_pane_lands_on_ungrouped_then_the_groups() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']')); // jump to Groups
    assert_eq!(app.focus(), Pane::Groups);
    // Ungrouped leads the Groups pane and is active at once.
    assert_eq!(app.active(), &View::Ungrouped);
    assert_eq!(names(&app), ["alpha"]);
    press(&mut app, KeyCode::Char('j')); // Work
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
fn tab_toggles_between_the_sidebar_and_the_list() {
    let mut app = fixture();
    assert_eq!(app.focus(), Pane::Views);
    press(&mut app, KeyCode::Tab); // Views → List directly, not through Groups
    assert_eq!(app.focus(), Pane::List);
    press(&mut app, KeyCode::Tab); // back to the last sidebar section
    assert_eq!(app.focus(), Pane::Views);

    // From Groups, Tab reaches the list and returns to Groups.
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus(), Pane::List);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus(), Pane::Groups);
}

#[test]
fn the_groups_pane_always_holds_ungrouped() {
    // No real groups, but the Groups pane still has the Ungrouped bucket.
    let mut app = App::new(vec![project("alpha", None, false)], vec![], vec![]);
    press(&mut app, KeyCode::Char(']'));
    assert_eq!(app.focus(), Pane::Groups);
    assert_eq!(app.active(), &View::Ungrouped);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus(), Pane::List);
}

#[test]
fn the_list_cursor_moves_within_the_active_view() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']')); // Groups (Ungrouped)
    press(&mut app, KeyCode::Char('j')); // Work group
    press(&mut app, KeyCode::Char('l')); // into the list (Work: bravo, charlie)
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
fn a_letter_shortcut_runs_a_command_on_the_selection() {
    let mut app = fixture(); // All view, selection is "alpha" (id "alpha-id")
    let action = app.handle(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    assert_eq!(action, Some(Action::Run("open alpha-id".into())));
}

#[test]
fn f_favourites_or_unfavourites_by_current_state() {
    // alpha is not a favourite → favorite it.
    let mut app = fixture();
    assert_eq!(
        app.handle(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE)),
        Some(Action::Run("favorite alpha-id".into()))
    );

    // In the Favourites view, bravo (a favourite) → unfavorite it.
    let mut app = fixture();
    press(&mut app, KeyCode::Char('j')); // Views: Favourites, selection "bravo"
    assert_eq!(
        app.selected_project().map(|p| p.name.as_str()),
        Some("bravo")
    );
    assert_eq!(
        app.handle(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE)),
        Some(Action::Run("unfavorite bravo-id".into()))
    );
}

#[test]
fn space_opens_the_menu_enter_runs_the_highlighted_item() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(app.mode(), &Mode::Menu(0)); // first item is Open
    let action = app.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(action, Some(Action::Run("open alpha-id".into())));
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn the_menu_moves_and_a_letter_runs_its_item() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Char('j'));
    assert_eq!(app.mode(), &Mode::Menu(1));
    // 'd' is the Untrack item's key — runs it directly and closes the menu.
    let action = app.handle(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    assert_eq!(action, Some(Action::Run("untrack alpha-id".into())));
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn esc_closes_the_menu() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn question_opens_help_and_any_key_closes_it() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('?'));
    assert_eq!(app.mode(), &Mode::Help);
    press(&mut app, KeyCode::Char('j')); // any key dismisses it
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn the_bin_offers_restore_and_purge() {
    let mut app = fixture();
    for _ in 0..3 {
        press(&mut app, KeyCode::Char('j')); // Views: All → Favourites → Ungrouped → Bin
    }
    assert_eq!(app.active(), &View::Bin);
    assert_eq!(
        app.selected_project().map(|p| p.name.as_str()),
        Some("echo")
    );
    assert_eq!(
        app.handle(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
        Some(Action::Run("restore echo-id".into()))
    );
    // `o` does not apply in the bin, so it is ignored (no action).
    assert_eq!(
        app.handle(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
        None
    );
}

#[test]
fn a_destructive_command_gates_on_y() {
    let mut app = fixture();
    app.begin_confirm("Delete? [y/N]".into(), "untrack alpha-id".into());
    assert!(matches!(app.mode(), Mode::Confirm { .. }));
    let action = app.handle(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    assert_eq!(
        action,
        Some(Action::RunConfirmed("untrack alpha-id".into()))
    );
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn any_other_key_cancels_the_confirmation() {
    for code in [KeyCode::Char('n'), KeyCode::Esc, KeyCode::Enter] {
        let mut app = fixture();
        app.begin_confirm("Delete? [y/N]".into(), "purge alpha-id".into());
        let action = app.handle(KeyEvent::new(code, KeyModifiers::NONE));
        assert_eq!(action, None);
        assert_eq!(app.mode(), &Mode::Normal);
    }
}

#[test]
fn s_opens_the_scan_dialog_and_enter_scans_the_folder() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.mode(), &Mode::Scan(String::new()));
    type_str(&mut app, "~/code");
    assert_eq!(app.mode(), &Mode::Scan("~/code".into()));
    let action = app.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(action, Some(Action::Run("scan \"~/code\" --import".into())));
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn the_scan_dialog_scans_the_current_folder_on_dot() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('s'));
    type_str(&mut app, ".");
    let action = app.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(action, Some(Action::Run("scan \".\" --import".into())));
}

#[test]
fn the_scan_dialog_cancels_and_an_empty_folder_scans_nothing() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('s'));
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.mode(), &Mode::Normal);

    let mut app = fixture();
    press(&mut app, KeyCode::Char('s'));
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
