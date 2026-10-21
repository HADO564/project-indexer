//! Navigation rules for the read-only TUI browser, driven as pure state — no
//! terminal, no database — the way the form's state is tested.

use std::path::PathBuf;

use indexer_core::domain::views::View;
use indexer_core::domain::{Group, Project};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

use crate::tui::app::{view_label, Action, App, Mode, Naming, Pane};

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
    let (projects, groups) = fixture_data(|_, _| {});
    App::new(projects, deleted(), groups)
}

/// The fixture's live projects and groups, with `edit` applied — so a test can
/// hand `set_data` the data "another process" changed.
fn fixture_data(
    edit: impl FnOnce(&mut Vec<Project>, &mut Vec<Group>),
) -> (Vec<Project>, Vec<Group>) {
    let mut projects = vec![
        project("alpha", None, false),              // ungrouped
        project("bravo", Some("g-work"), true),     // Work, favourite
        project("charlie", Some("g-work"), false),  // Work
        project("delta", Some("g-clients"), false), // Clients
    ];
    let mut groups = vec![group("g-work", "Work"), group("g-clients", "Clients")];
    edit(&mut projects, &mut groups);
    (projects, groups)
}

fn deleted() -> Vec<Project> {
    vec![project("echo", None, false)]
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
fn e_and_capital_e_open_the_compact_and_the_full_form() {
    let mut app = fixture();
    let action = app.handle(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    assert_eq!(action, Some(Action::Run("edit alpha-id".into())));
    let action = app.handle(KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT));
    assert_eq!(action, Some(Action::Run("edit alpha-id --full".into())));
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
fn tab_in_the_scan_dialog_opens_the_browser() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('s'));
    let action = app.handle(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(action, Some(Action::OpenBrowser));
}

#[test]
fn the_folder_browser_navigates_and_scans() {
    // Paths are built with the same APIs the code uses, so this holds on
    // Windows (`\`) as well as Unix (`/`).
    let base = PathBuf::from("/home/me/code");
    let mut app = fixture();

    app.set_browse(base.clone(), vec!["api".into(), "web".into()]);
    assert!(matches!(app.mode(), Mode::Browse { .. }));

    // j selects "web"; Enter descends into it.
    press(&mut app, KeyCode::Char('j'));
    assert_eq!(
        app.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(Action::BrowseTo(base.join("web")))
    );

    // h goes to the parent.
    app.set_browse(base.clone(), vec!["api".into()]);
    assert_eq!(
        app.handle(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Some(Action::BrowseTo(base.parent().unwrap().to_path_buf()))
    );

    // s scans the open folder and closes the browser.
    app.set_browse(base.clone(), vec!["api".into()]);
    assert_eq!(
        app.handle(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
        Some(Action::Run(format!("scan \"{}\" --import", base.display())))
    );
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn esc_closes_the_folder_browser() {
    let mut app = fixture();
    app.set_browse(PathBuf::from("/tmp"), vec![]);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn slash_opens_the_search_line_and_filters_live() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('/'));
    assert_eq!(app.mode(), &Mode::Search(String::new()));
    // Focus moves to the list, and an empty query shows everything in view.
    assert_eq!(app.focus(), Pane::List);
    assert_eq!(names(&app), ["alpha", "bravo", "charlie", "delta"]);

    type_str(&mut app, "brav");
    assert_eq!(app.mode(), &Mode::Search("brav".into()));
    assert_eq!(names(&app), ["bravo"]);
    assert_eq!(
        app.selected_project().map(|p| p.name.as_str()),
        Some("bravo")
    );
}

#[test]
fn enter_keeps_the_filter_and_backspace_widens_it() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('/'));
    type_str(&mut app, "char");
    press(&mut app, KeyCode::Backspace); // "cha"
    assert_eq!(names(&app), ["charlie"]);
    // Search is in-memory filtering, so Enter runs no command — it just keeps
    // the filter and returns to the browser.
    let action = app.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(action, None);
    assert_eq!(app.mode(), &Mode::Normal);
    assert_eq!(app.search_query(), "cha");
    assert_eq!(names(&app), ["charlie"]);
}

#[test]
fn esc_clears_a_committed_filter_before_it_quits() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('/'));
    type_str(&mut app, "brav");
    press(&mut app, KeyCode::Enter); // commit
    assert_eq!(names(&app), ["bravo"]);

    press(&mut app, KeyCode::Esc); // clears the filter, does not quit
    assert!(!app.quit());
    assert_eq!(app.search_query(), "");
    assert_eq!(names(&app), ["alpha", "bravo", "charlie", "delta"]);

    press(&mut app, KeyCode::Esc); // nothing filtered now, so Esc quits
    assert!(app.quit());
}

#[test]
fn esc_while_typing_restores_the_committed_filter() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('/'));
    type_str(&mut app, "brav");
    press(&mut app, KeyCode::Enter); // committed "brav"

    press(&mut app, KeyCode::Char('/')); // a fresh line, buffer empty
    type_str(&mut app, "zzz"); // matches nothing, live
    assert_eq!(names(&app), Vec::<String>::new());
    press(&mut app, KeyCode::Esc); // cancels the edit, keeps the old filter
    assert_eq!(app.mode(), &Mode::Normal);
    assert_eq!(app.search_query(), "brav");
    assert_eq!(names(&app), ["bravo"]);
}

#[test]
fn search_filters_within_the_active_view() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']')); // Groups → Ungrouped
    press(&mut app, KeyCode::Char('j')); // Work (bravo, charlie)
    assert_eq!(names(&app), ["bravo", "charlie"]);
    press(&mut app, KeyCode::Char('/'));
    type_str(&mut app, "char");
    // Only Work's projects are searched — alpha (ungrouped) never appears.
    assert_eq!(names(&app), ["charlie"]);
}

// --- the Groups pane: keys act on groups ---

fn key(app: &mut App, code: KeyCode) -> Option<Action> {
    app.handle(KeyEvent::new(code, KeyModifiers::NONE))
}

fn menu_labels(app: &App) -> Vec<&'static str> {
    app.menu_items().iter().map(|item| item.label).collect()
}

#[test]
fn the_groups_pane_offers_group_actions_not_project_ones() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']')); // Ungrouped: not a group
    assert_eq!(menu_labels(&app), ["New group"]);
    press(&mut app, KeyCode::Char('j')); // Work
    assert_eq!(menu_labels(&app), ["New group", "Rename", "Delete"]);
    // A project's keys don't reach through to the list from here.
    assert_eq!(key(&mut app, KeyCode::Char('o')), None);
    assert_eq!(key(&mut app, KeyCode::Char('f')), None);
    // Back in the list, the project's actions return.
    press(&mut app, KeyCode::Tab);
    assert_eq!(
        menu_labels(&app),
        ["Open", "Unfavourite", "Edit", "Edit all fields", "Untrack"]
    );
}

#[test]
fn n_asks_for_a_name_and_creates_the_group_with_it_quoted() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']'));
    assert_eq!(key(&mut app, KeyCode::Char('n')), None);
    assert_eq!(
        app.mode(),
        &Mode::Name {
            naming: Naming::NewGroup,
            buffer: String::new()
        }
    );
    // Keys type into the name rather than acting.
    type_str(&mut app, "Client jobs");
    assert_eq!(
        key(&mut app, KeyCode::Enter),
        Some(Action::Run("group create \"Client jobs\"".into()))
    );
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn e_renames_the_group_starting_from_its_name() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char('j')); // Work
    press(&mut app, KeyCode::Char('e'));
    assert_eq!(
        app.mode(),
        &Mode::Name {
            naming: Naming::RenameGroup {
                name: "Work".into()
            },
            buffer: "Work".into()
        }
    );
    type_str(&mut app, " stuff");
    assert_eq!(
        key(&mut app, KeyCode::Enter),
        Some(Action::Run(
            "group edit \"Work\" --name \"Work stuff\"".into()
        ))
    );
}

#[test]
fn d_deletes_the_group_by_name() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('j')); // Clients
                                         // The loop gates this behind y/N, as it does every delete.
    assert_eq!(
        key(&mut app, KeyCode::Char('d')),
        Some(Action::Run("group delete \"Clients\"".into()))
    );
}

#[test]
fn the_name_dialog_runs_nothing_when_empty_unchanged_or_cancelled() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char('n'));
    type_str(&mut app, "   ");
    assert_eq!(key(&mut app, KeyCode::Enter), None, "only spaces");

    press(&mut app, KeyCode::Char('j')); // Work
    press(&mut app, KeyCode::Char('e'));
    assert_eq!(key(&mut app, KeyCode::Enter), None, "the same name");

    press(&mut app, KeyCode::Char('e'));
    type_str(&mut app, "x");
    assert_eq!(key(&mut app, KeyCode::Esc), None);
    assert_eq!(app.mode(), &Mode::Normal, "Esc cancels");
}

#[test]
fn the_groups_menu_opens_the_name_dialog_too() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(app.mode(), &Mode::Menu(0));
    assert_eq!(key(&mut app, KeyCode::Enter), None);
    assert!(matches!(
        app.mode(),
        Mode::Name {
            naming: Naming::NewGroup,
            ..
        }
    ));
}

#[test]
fn a_refresh_closes_the_groups_menu_if_its_group_went_away() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char('j')); // Work
    press(&mut app, KeyCode::Char(' '));
    refresh(&mut app, |_, groups| groups.push(group("g-home", "Home")));
    assert_eq!(app.mode(), &Mode::Menu(0), "Work is still there");
    refresh(&mut app, |_, groups| groups.retain(|g| g.id != "g-work"));
    assert_eq!(app.mode(), &Mode::Normal);
}

// --- live refresh: fresh data swapped in under the cursor ---

/// Swap in the fixture's data with `edit` applied, as a reload would after
/// another process wrote.
fn refresh(app: &mut App, edit: impl FnOnce(&mut Vec<Project>, &mut Vec<Group>)) {
    let (projects, groups) = fixture_data(edit);
    app.set_data(projects, deleted(), groups);
}

fn selected(app: &App) -> Option<String> {
    app.selected_project().map(|p| p.name.clone())
}

#[test]
fn a_refresh_keeps_the_cursor_on_the_same_project() {
    let mut app = fixture();
    press(&mut app, KeyCode::Tab); // into the list
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('j')); // charlie, row 2

    // Another process adds a project that sorts above it.
    refresh(&mut app, |projects, _| {
        projects.insert(0, project("aaron", None, false))
    });
    assert_eq!(names(&app), ["aaron", "alpha", "bravo", "charlie", "delta"]);
    assert_eq!(selected(&app).as_deref(), Some("charlie"));
    assert_eq!(app.list_sel(), 3);
}

#[test]
fn a_refresh_that_removes_the_selected_project_keeps_the_row() {
    let mut app = fixture();
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('j')); // charlie
    refresh(&mut app, |projects, _| {
        projects.retain(|p| p.name != "charlie")
    });
    // The row it held is now delta's.
    assert_eq!(selected(&app).as_deref(), Some("delta"));
    // And the last row clamps rather than pointing past the end.
    refresh(&mut app, |projects, _| projects.truncate(2));
    assert_eq!(selected(&app).as_deref(), Some("bravo"));
}

#[test]
fn a_refresh_keeps_the_groups_cursor_on_the_same_group() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']')); // Ungrouped
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('j')); // Clients, row 2
    let clients = View::Group {
        id: "g-clients".into(),
    };
    assert_eq!(app.active(), &clients);
    // A group created elsewhere lands above it.
    refresh(&mut app, |_, groups| {
        groups.insert(0, group("g-home", "Home"))
    });
    assert_eq!(app.groups_sel(), 3);
    assert_eq!(app.active(), &clients);
    assert_eq!(names(&app), ["delta"]);
}

#[test]
fn a_refresh_after_the_active_group_is_deleted_falls_back_to_all() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char('j')); // Work
    press(&mut app, KeyCode::Tab); // into Work's list
    refresh(&mut app, |p, g| delete_work(p, g));
    assert_eq!(app.active(), &View::All);
    assert_eq!(app.focus(), Pane::Views);
    assert_eq!(names(&app), ["alpha", "bravo", "charlie", "delta"]);
}

#[test]
fn deleting_the_group_under_the_groups_cursor_stays_in_the_groups_pane() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char('j')); // Work
    refresh(&mut app, |p, g| delete_work(p, g));
    // Clients moved up into Work's row, and the list follows it.
    assert_eq!(app.focus(), Pane::Groups);
    assert_eq!(app.groups_sel(), 1);
    assert_eq!(
        app.active(),
        &View::Group {
            id: "g-clients".into()
        }
    );
    assert_eq!(names(&app), ["delta"]);

    // Deleting the last group leaves the cursor on Ungrouped.
    refresh(&mut app, |projects, groups| {
        delete_work(projects, groups);
        groups.clear();
        for p in projects.iter_mut() {
            p.group_id = None;
        }
    });
    assert_eq!(app.focus(), Pane::Groups);
    assert_eq!(app.active(), &View::Ungrouped);
}

/// Work deleted, its projects ungrouped — what `group delete Work` leaves.
fn delete_work(projects: &mut [Project], groups: &mut Vec<Group>) {
    groups.retain(|g| g.id != "g-work");
    for p in projects.iter_mut() {
        if p.group_id.as_deref() == Some("g-work") {
            p.group_id = None;
        }
    }
}

#[test]
fn a_refresh_keeps_the_search_filter_and_its_match() {
    let mut app = fixture();
    press(&mut app, KeyCode::Char('/'));
    type_str(&mut app, "a"); // alpha, bravo, charlie, delta all have an `a`
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('j')); // bravo
    refresh(&mut app, |projects, _| {
        projects.insert(0, project("zulu", None, false)); // no `a`: filtered out
        projects.insert(0, project("anna", None, false)); // matches
    });
    assert_eq!(app.search_query(), "a");
    assert_eq!(names(&app), ["anna", "alpha", "bravo", "charlie", "delta"]);
    assert_eq!(selected(&app).as_deref(), Some("bravo"));
}

#[test]
fn a_refresh_closes_the_menu_only_if_its_project_went_away() {
    let mut app = fixture();
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('j')); // bravo
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(app.mode(), &Mode::Menu(0));
    // A change elsewhere that leaves bravo selected keeps the menu up.
    refresh(&mut app, |projects, _| {
        projects.insert(0, project("aaron", None, false))
    });
    assert_eq!(app.mode(), &Mode::Menu(0));
    // bravo untracked elsewhere: the menu would now act on someone else.
    refresh(&mut app, |projects, _| {
        projects.retain(|p| p.name != "bravo")
    });
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

#[test]
fn n_asks_the_loop_to_offer_the_current_folder() {
    // `n` outside the Groups pane is "add a project": the App has no idea what
    // the current folder is, so it hands that to the loop and stays put.
    let mut app = fixture();
    assert_eq!(key(&mut app, KeyCode::Char('n')), Some(Action::AddHere));
    assert_eq!(app.mode(), &Mode::Normal);

    press(&mut app, KeyCode::Tab); // the project list
    assert_eq!(key(&mut app, KeyCode::Char('n')), Some(Action::AddHere));
}

#[test]
fn begin_command_opens_the_line_holding_the_text_to_edit_or_run() {
    let mut app = fixture();
    app.begin_command("add \"/home/me/app\"".to_string());
    assert_eq!(
        app.mode(),
        &Mode::Command("add \"/home/me/app\"".to_string())
    );

    // Ordinary `:` line editing from there: change the folder, then run it.
    for _ in 0.."app\"".len() {
        press(&mut app, KeyCode::Backspace);
    }
    type_str(&mut app, "web\"");
    assert_eq!(
        key(&mut app, KeyCode::Enter),
        Some(Action::Run("add \"/home/me/web\"".to_string()))
    );
    assert_eq!(app.mode(), &Mode::Normal);
}
