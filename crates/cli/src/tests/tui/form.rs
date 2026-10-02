use std::collections::BTreeMap;

use indexer_core::Project;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use serde_json::json;

use crate::editor::FormKind;
use crate::tui::form::{Action, FormState, TextInput};

// The cursor is private, so these tests find it the way a person would: by
// typing a marker and looking where it landed.

/// The value after typing `|` at the cursor — shows where the cursor is.
fn with_marker(mut input: TextInput) -> String {
    input.insert('|');
    input.value().to_string()
}

#[test]
fn a_new_input_holds_its_text_with_the_cursor_at_the_end() {
    let input = TextInput::new("rust");
    assert_eq!(input.value(), "rust");
    assert_eq!(with_marker(input), "rust|");
}

#[test]
fn an_empty_input_accepts_typing() {
    let mut input = TextInput::new("");
    input.insert('a');
    input.insert('b');
    assert_eq!(input.value(), "ab");
}

#[test]
fn typing_inserts_at_the_cursor_not_the_end() {
    let mut input = TextInput::new("ac");
    input.left();
    input.insert('b');
    assert_eq!(with_marker(input), "ab|c");
}

#[test]
fn home_and_end_jump_to_the_edges() {
    let mut input = TextInput::new("web");
    input.home();
    input.insert('>');
    input.end();
    input.insert('<');
    assert_eq!(input.value(), ">web<");
}

#[test]
fn left_stops_at_the_start() {
    let mut input = TextInput::new("ab");
    for _ in 0..5 {
        input.left();
    }
    assert_eq!(with_marker(input), "|ab");
}

#[test]
fn right_stops_at_the_end() {
    let mut input = TextInput::new("ab");
    input.home();
    for _ in 0..5 {
        input.right();
    }
    assert_eq!(with_marker(input), "ab|");
}

#[test]
fn backspace_removes_the_character_before_the_cursor() {
    let mut input = TextInput::new("abc");
    input.left();
    input.backspace();
    assert_eq!(with_marker(input), "a|c");
}

#[test]
fn backspace_at_the_start_does_nothing() {
    let mut input = TextInput::new("abc");
    input.home();
    input.backspace();
    assert_eq!(with_marker(input), "|abc");
}

#[test]
fn delete_removes_the_character_at_the_cursor_and_stays() {
    let mut input = TextInput::new("abc");
    input.home();
    input.delete();
    assert_eq!(with_marker(input), "|bc");
}

#[test]
fn delete_at_the_end_does_nothing() {
    let mut input = TextInput::new("abc");
    input.delete();
    assert_eq!(with_marker(input), "abc|");
}

// Non-ASCII: `é` is 2 bytes, `€` 3, `🦀` 4. Indexing by character count
// instead of bytes would panic in every one of these.

#[test]
fn the_cursor_counts_characters_not_bytes() {
    let input = TextInput::new("café");
    assert_eq!(with_marker(input), "café|");
}

#[test]
fn typing_after_a_wide_character_lands_after_it() {
    let mut input = TextInput::new("€5");
    input.left();
    input.insert('x');
    assert_eq!(input.value(), "€x5");
}

#[test]
fn backspace_removes_a_whole_wide_character() {
    let mut input = TextInput::new("café");
    input.backspace();
    assert_eq!(with_marker(input), "caf|");
}

#[test]
fn delete_removes_a_whole_emoji() {
    let mut input = TextInput::new("a🦀b");
    input.home();
    input.right();
    input.delete();
    assert_eq!(with_marker(input), "a|b");
}

#[test]
fn moving_across_mixed_widths_keeps_the_cursor_on_character_boundaries() {
    let mut input = TextInput::new("é€🦀");
    input.home();
    input.right();
    input.right();
    input.insert('|');
    assert_eq!(input.value(), "é€|🦀");
}

// `FormState::handle` — the actions. What typing does to the fields is tested
// through `changes()`.

fn project(description: &str, tags: &[&str]) -> Project {
    serde_json::from_value(json!({
        "id": "app-0000-4000-8000-000000000000",
        "name": "app",
        "directory": "/home/me/work/app",
        "description": description,
        "tags": tags,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
    }))
    .unwrap()
}

fn form() -> FormState {
    FormState::new(
        &project("A tool", &["Rust", "Web"]),
        true,
        FormKind::Compact,
    )
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

#[test]
fn enter_saves() {
    assert!(matches!(form().handle(press(KeyCode::Enter)), Action::Save));
}

#[test]
fn esc_cancels() {
    assert!(matches!(form().handle(press(KeyCode::Esc)), Action::Cancel));
}

#[test]
fn ctrl_c_cancels() {
    assert!(matches!(form().handle(ctrl('c')), Action::Cancel));
}

#[test]
fn editing_keys_continue() {
    let mut form = form();
    for code in [
        KeyCode::Char('x'),
        KeyCode::Char('C'),
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::F(1),
    ] {
        assert!(
            matches!(form.handle(press(code)), Action::Continue),
            "{code:?}"
        );
    }
}

#[test]
fn a_ctrl_letter_other_than_c_continues() {
    assert!(matches!(form().handle(ctrl('n')), Action::Continue));
}

#[test]
fn a_key_release_does_nothing() {
    let release =
        KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release);
    assert!(matches!(form().handle(release), Action::Continue));
}

fn type_text(form: &mut FormState, text: &str) {
    for c in text.chars() {
        form.handle(press(KeyCode::Char(c)));
    }
}

/// Empties the focused field.
fn clear(form: &mut FormState) {
    form.handle(press(KeyCode::End));
    for _ in 0..100 {
        form.handle(press(KeyCode::Backspace));
    }
}

// `FormState::changes` — and through it, what keys do to the fields.

#[test]
fn an_untouched_form_changes_nothing() {
    let changes = form().changes();
    assert_eq!(changes.description, None);
    assert_eq!(changes.tags, None);
}

#[test]
fn typing_edits_the_description_first() {
    let mut form = form();
    type_text(&mut form, "!");
    let changes = form.changes();
    assert_eq!(changes.description.as_deref(), Some("A tool!"));
    assert_eq!(changes.tags, None, "only the edited field is sent");
}

#[test]
fn an_emptied_description_is_sent_as_empty_not_left_alone() {
    let mut form = form();
    clear(&mut form);
    assert_eq!(form.changes().description.as_deref(), Some(""));
}

#[test]
fn retyping_the_description_unchanged_is_no_change() {
    let mut form = form();
    form.handle(press(KeyCode::Backspace));
    type_text(&mut form, "l");
    assert_eq!(form.changes().description, None);
}

#[test]
fn tab_moves_to_the_tags() {
    let mut form = form();
    form.handle(press(KeyCode::Tab));
    type_text(&mut form, ", Cli");
    let changes = form.changes();
    assert_eq!(changes.description, None);
    assert_eq!(
        changes.tags,
        Some(vec!["Rust".into(), "Web".into(), "Cli".into()])
    );
}

#[test]
fn down_moves_to_the_tags_and_up_back() {
    let mut form = form();
    form.handle(press(KeyCode::Down));
    form.handle(press(KeyCode::Up));
    type_text(&mut form, "!");
    assert_eq!(form.changes().description.as_deref(), Some("A tool!"));
}

#[test]
fn both_fields_edited_are_both_sent() {
    let mut form = form();
    type_text(&mut form, "!");
    form.handle(press(KeyCode::Tab));
    clear(&mut form);
    let changes = form.changes();
    assert_eq!(changes.description.as_deref(), Some("A tool!"));
    assert_eq!(changes.tags, Some(vec![]));
}

#[test]
fn a_tag_retyped_in_another_case_is_no_change() {
    let mut form = form();
    form.handle(press(KeyCode::Tab));
    clear(&mut form);
    type_text(&mut form, "rust,web");
    assert_eq!(form.changes().tags, None);
}

#[test]
fn tags_are_trimmed_and_empty_pieces_dropped() {
    let mut form = form();
    form.handle(press(KeyCode::Tab));
    clear(&mut form);
    type_text(&mut form, " Go ,, Web ,");
    assert_eq!(form.changes().tags, Some(vec!["Go".into(), "Web".into()]));
}

#[test]
fn typing_non_ascii_through_the_form() {
    let mut form = form();
    clear(&mut form);
    type_text(&mut form, "café 🦀");
    form.handle(press(KeyCode::Left));
    form.handle(press(KeyCode::Backspace));
    assert_eq!(form.changes().description.as_deref(), Some("café🦀"));
}

#[test]
fn a_ctrl_letter_is_not_typed() {
    let mut form = form();
    form.handle(ctrl('n'));
    assert_eq!(form.changes().description, None);
}

#[test]
fn shift_still_types_capitals() {
    let mut form = form();
    form.handle(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT));
    assert_eq!(form.changes().description.as_deref(), Some("A toolX"));
}

#[test]
fn a_key_release_types_nothing() {
    let mut form = form();
    form.handle(KeyEvent::new_with_kind(
        KeyCode::Char('x'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ));
    assert_eq!(form.changes().description, None);
}

// Focus at the ends: wraps when `wrap` is on, stays when it is off. The
// description is first, the tags last.

fn form_with_wrap(wrap: bool) -> FormState {
    FormState::new(&project("A tool", &["Rust"]), wrap, FormKind::Compact)
}

#[test]
fn with_wrap_tab_on_the_last_field_goes_to_the_first() {
    let mut form = form_with_wrap(true);
    form.handle(press(KeyCode::Tab));
    form.handle(press(KeyCode::Tab));
    type_text(&mut form, "!");
    assert_eq!(form.changes().description.as_deref(), Some("A tool!"));
}

#[test]
fn with_wrap_shift_tab_on_the_first_field_goes_to_the_last() {
    let mut form = form_with_wrap(true);
    form.handle(press(KeyCode::BackTab));
    type_text(&mut form, ", Go");
    assert_eq!(form.changes().tags, Some(vec!["Rust".into(), "Go".into()]));
}

#[test]
fn without_wrap_tab_on_the_last_field_stays() {
    let mut form = form_with_wrap(false);
    form.handle(press(KeyCode::Tab));
    form.handle(press(KeyCode::Tab));
    type_text(&mut form, ", Go");
    let changes = form.changes();
    assert_eq!(changes.description, None);
    assert_eq!(changes.tags, Some(vec!["Rust".into(), "Go".into()]));
}

#[test]
fn without_wrap_shift_tab_on_the_first_field_stays() {
    let mut form = form_with_wrap(false);
    form.handle(press(KeyCode::BackTab));
    type_text(&mut form, "!");
    let changes = form.changes();
    assert_eq!(changes.description.as_deref(), Some("A tool!"));
    assert_eq!(changes.tags, None);
}

// Property rows. The form below has two: `Client: Acme` (row 0) and
// `Stage: beta` (row 1) — sorted by name, as the map stores them.

fn props(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn form_with_rows(wrap: bool) -> FormState {
    let mut project = project("A tool", &["Rust"]);
    project.properties = props(&[("Stage", "beta"), ("Client", "Acme")]);
    FormState::new(&project, wrap, FormKind::Compact)
}

fn tab(form: &mut FormState, times: usize) {
    for _ in 0..times {
        form.handle(press(KeyCode::Tab));
    }
}

fn shift_tab(form: &mut FormState, times: usize) {
    for _ in 0..times {
        form.handle(press(KeyCode::BackTab));
    }
}

#[test]
fn an_untouched_form_with_properties_changes_nothing() {
    assert_eq!(form_with_rows(true).changes().properties, None);
}

#[test]
fn tab_walks_name_then_value_row_by_row() {
    let mut form = form_with_rows(true);
    tab(&mut form, 2); // description → tags → Client's name
    type_text(&mut form, "1");
    tab(&mut form, 1); // → Client's value
    type_text(&mut form, "1");
    tab(&mut form, 1); // → Stage's name
    type_text(&mut form, "2");
    assert_eq!(
        form.changes().properties,
        Some(props(&[("Client1", "Acme1"), ("Stage2", "beta")]))
    );
}

#[test]
fn with_wrap_tab_on_the_last_value_goes_to_the_description() {
    let mut form = form_with_rows(true);
    tab(&mut form, 6); // … Stage's value (5 tabs), then wrap
    type_text(&mut form, "!");
    let changes = form.changes();
    assert_eq!(changes.description.as_deref(), Some("A tool!"));
    assert_eq!(changes.properties, None);
}

#[test]
fn without_wrap_tab_on_the_last_value_stays() {
    let mut form = form_with_rows(false);
    tab(&mut form, 9);
    type_text(&mut form, "!");
    assert_eq!(
        form.changes().properties,
        Some(props(&[("Client", "Acme"), ("Stage", "beta!")]))
    );
}

#[test]
fn with_wrap_shift_tab_on_the_description_goes_to_the_last_value() {
    let mut form = form_with_rows(true);
    shift_tab(&mut form, 1);
    type_text(&mut form, "!");
    assert_eq!(
        form.changes().properties,
        Some(props(&[("Client", "Acme"), ("Stage", "beta!")]))
    );
}

#[test]
fn shift_tab_walks_back_through_the_rows_to_the_tags() {
    let mut form = form_with_rows(true);
    shift_tab(&mut form, 5); // Stage value, Stage name, Client value, Client name, tags
    type_text(&mut form, ", Go");
    let changes = form.changes();
    assert_eq!(changes.tags, Some(vec!["Rust".into(), "Go".into()]));
    assert_eq!(changes.properties, None);
}

#[test]
fn ctrl_n_adds_a_row_and_focuses_its_name() {
    let mut form = form_with_rows(true);
    form.handle(ctrl('n'));
    type_text(&mut form, "Owner");
    tab(&mut form, 1);
    type_text(&mut form, "me");
    assert_eq!(
        form.changes().properties,
        Some(props(&[
            ("Client", "Acme"),
            ("Owner", "me"),
            ("Stage", "beta")
        ]))
    );
}

#[test]
fn ctrl_n_works_with_no_rows_at_all() {
    let mut form = form();
    form.handle(ctrl('n'));
    type_text(&mut form, "Owner");
    assert_eq!(form.changes().properties, Some(props(&[("Owner", "")])));
}

#[test]
fn ctrl_n_on_an_empty_row_adds_no_second_one() {
    let mut form = form_with_rows(true);
    form.handle(ctrl('n'));
    form.handle(ctrl('n'));
    // Had a second row been added, focus would be on it and this would
    // land there; instead it is the first new row's name.
    type_text(&mut form, "Owner");
    shift_tab(&mut form, 1); // back to Stage's value: the new row was the last
    type_text(&mut form, "!");
    assert_eq!(
        form.changes().properties,
        Some(props(&[
            ("Client", "Acme"),
            ("Owner", ""),
            ("Stage", "beta!")
        ]))
    );
}

#[test]
fn leaving_an_unfilled_new_row_removes_it() {
    let mut form = form_with_rows(true);
    form.handle(ctrl('n'));
    shift_tab(&mut form, 1); // to Stage's value; the blank row goes
    tab(&mut form, 1); // with the row gone, Stage's value is last: wraps
    type_text(&mut form, "!");
    let changes = form.changes();
    assert_eq!(changes.description.as_deref(), Some("A tool!"));
    assert_eq!(changes.properties, None);
}

#[test]
fn a_row_with_a_value_but_no_name_stays_and_is_sent() {
    let mut form = form_with_rows(true);
    form.handle(ctrl('n'));
    tab(&mut form, 1);
    type_text(&mut form, "orphan");
    tab(&mut form, 1); // leaves the row; it has a value, so it stays
    assert_eq!(
        form.changes().properties,
        Some(props(&[
            ("", "orphan"),
            ("Client", "Acme"),
            ("Stage", "beta")
        ])),
        "core refuses the empty name on save"
    );
}

#[test]
fn clearing_a_loaded_row_and_leaving_it_deletes_the_property() {
    let mut form = form_with_rows(true);
    tab(&mut form, 2); // Client's name
    clear(&mut form);
    tab(&mut form, 1); // Client's value
    clear(&mut form);
    tab(&mut form, 1); // leaves the empty row: removed, focus renumbered
    type_text(&mut form, "X"); // Stage's name, now row 0
    assert_eq!(
        form.changes().properties,
        Some(props(&[("StageX", "beta")]))
    );
}

#[test]
fn a_blank_row_still_focused_on_save_is_dropped() {
    let mut form = form_with_rows(true);
    form.handle(ctrl('n'));
    assert_eq!(form.changes().properties, None);
}

#[test]
fn ctrl_d_asks_and_y_deletes() {
    let mut form = form_with_rows(true);
    tab(&mut form, 2); // Client's name
    form.handle(ctrl('d'));
    assert_eq!(form.changes().properties, None, "nothing deleted yet");
    form.handle(press(KeyCode::Char('y')));
    assert_eq!(form.changes().properties, Some(props(&[("Stage", "beta")])));
}

#[test]
fn ctrl_d_then_any_other_key_keeps_the_row_and_is_not_typed() {
    for answer in [
        KeyCode::Char('n'),
        KeyCode::Esc,
        KeyCode::Enter,
        KeyCode::Char('x'),
    ] {
        let mut form = form_with_rows(true);
        tab(&mut form, 2);
        form.handle(ctrl('d'));
        assert!(
            matches!(form.handle(press(answer)), Action::Continue),
            "{answer:?} answers the question, it does not save or cancel"
        );
        assert_eq!(form.changes().properties, None, "{answer:?}");
    }
}

#[test]
fn ctrl_c_during_the_question_still_cancels_the_form() {
    let mut form = form_with_rows(true);
    tab(&mut form, 2);
    form.handle(ctrl('d'));
    assert!(matches!(form.handle(ctrl('c')), Action::Cancel));
}

#[test]
fn deleting_moves_focus_to_the_row_below() {
    let mut form = form_with_rows(true);
    tab(&mut form, 2); // Client's name
    form.handle(ctrl('d'));
    form.handle(press(KeyCode::Char('y')));
    type_text(&mut form, "X"); // Stage's name took Client's place
    assert_eq!(
        form.changes().properties,
        Some(props(&[("StageX", "beta")]))
    );
}

#[test]
fn deleting_the_last_row_moves_focus_to_the_one_above() {
    let mut form = form_with_rows(true);
    tab(&mut form, 4); // Stage's name
    form.handle(ctrl('d'));
    form.handle(press(KeyCode::Char('y')));
    type_text(&mut form, "X");
    assert_eq!(
        form.changes().properties,
        Some(props(&[("ClientX", "Acme")]))
    );
}

#[test]
fn deleting_the_only_row_moves_focus_to_the_tags() {
    let mut form = form();
    form.handle(ctrl('n'));
    type_text(&mut form, "Owner");
    form.handle(ctrl('d'));
    form.handle(press(KeyCode::Char('y')));
    type_text(&mut form, ", Go");
    let changes = form.changes();
    assert_eq!(changes.properties, None);
    assert_eq!(
        changes.tags,
        Some(vec!["Rust".into(), "Web".into(), "Go".into()])
    );
}

#[test]
fn ctrl_d_on_a_blank_row_deletes_without_asking() {
    let mut form = form_with_rows(true);
    form.handle(ctrl('n'));
    form.handle(ctrl('d'));
    type_text(&mut form, "X"); // not swallowed as an answer: typed into Stage's name
    assert_eq!(
        form.changes().properties,
        Some(props(&[("Client", "Acme"), ("StageX", "beta")]))
    );
}

#[test]
fn ctrl_d_off_a_row_does_nothing() {
    let mut form = form_with_rows(true);
    form.handle(ctrl('d'));
    type_text(&mut form, "y");
    let changes = form.changes();
    assert_eq!(changes.description.as_deref(), Some("A tooly"));
    assert_eq!(changes.properties, None);
}

#[test]
fn a_name_retyped_in_another_case_is_no_change() {
    let mut form = form_with_rows(true);
    tab(&mut form, 2);
    clear(&mut form);
    type_text(&mut form, "client");
    assert_eq!(form.changes().properties, None);
}

#[test]
fn a_value_retyped_in_another_case_is_a_change() {
    let mut form = form_with_rows(true);
    tab(&mut form, 3); // Client's value
    clear(&mut form);
    type_text(&mut form, "ACME");
    assert_eq!(
        form.changes().properties,
        Some(props(&[("Client", "ACME"), ("Stage", "beta")]))
    );
}

#[test]
fn of_two_names_equal_ignoring_case_the_lower_row_wins() {
    let mut form = form_with_rows(true);
    form.handle(ctrl('n'));
    type_text(&mut form, "client");
    tab(&mut form, 1);
    type_text(&mut form, "Globex");
    assert_eq!(
        form.changes().properties,
        Some(props(&[("Stage", "beta"), ("client", "Globex")]))
    );
}

// Enter in a name box moves to the value; anywhere else it saves.

#[test]
fn enter_in_a_name_box_moves_to_its_value_instead_of_saving() {
    let mut form = form_with_rows(true);
    tab(&mut form, 2); // Client's name
    assert!(matches!(
        form.handle(press(KeyCode::Enter)),
        Action::Continue
    ));
    type_text(&mut form, "!");
    assert_eq!(
        form.changes().properties,
        Some(props(&[("Client", "Acme!"), ("Stage", "beta")]))
    );
}

#[test]
fn ctrl_n_name_enter_value_enter_adds_the_property_and_saves() {
    let mut form = form_with_rows(true);
    form.handle(ctrl('n'));
    type_text(&mut form, "Owner");
    assert!(matches!(
        form.handle(press(KeyCode::Enter)),
        Action::Continue
    ));
    type_text(&mut form, "me");
    assert!(matches!(form.handle(press(KeyCode::Enter)), Action::Save));
    assert_eq!(
        form.changes().properties,
        Some(props(&[
            ("Client", "Acme"),
            ("Owner", "me"),
            ("Stage", "beta")
        ]))
    );
}

#[test]
fn enter_in_a_value_box_saves() {
    let mut form = form_with_rows(true);
    tab(&mut form, 3); // Client's value
    assert!(matches!(form.handle(press(KeyCode::Enter)), Action::Save));
}

#[test]
fn enter_in_the_tags_saves() {
    let mut form = form_with_rows(true);
    tab(&mut form, 1);
    assert!(matches!(form.handle(press(KeyCode::Enter)), Action::Save));
}

// Esc on a row leaves the properties; elsewhere it cancels.

#[test]
fn esc_on_a_row_moves_to_the_tags_instead_of_cancelling() {
    let mut form = form_with_rows(true);
    tab(&mut form, 3); // Client's value
    type_text(&mut form, "!");
    assert!(matches!(form.handle(press(KeyCode::Esc)), Action::Continue));
    type_text(&mut form, ", Go"); // now in the tags
    let changes = form.changes();
    assert_eq!(changes.tags, Some(vec!["Rust".into(), "Go".into()]));
    assert_eq!(
        changes.properties,
        Some(props(&[("Client", "Acme!"), ("Stage", "beta")])),
        "the row's edit is kept"
    );
}

#[test]
fn a_second_esc_outside_the_properties_cancels() {
    let mut form = form_with_rows(true);
    tab(&mut form, 2);
    form.handle(press(KeyCode::Esc));
    assert!(matches!(form.handle(press(KeyCode::Esc)), Action::Cancel));
}

#[test]
fn esc_out_of_an_untouched_new_row_removes_it() {
    let mut form = form_with_rows(true);
    form.handle(ctrl('n'));
    form.handle(press(KeyCode::Esc));
    assert_eq!(form.changes().properties, None);
    // The tags have focus, and the blank row is gone: Tab reaches Client's
    // name, and Shift+Tab from the description wraps to Stage's value.
    shift_tab(&mut form, 2);
    type_text(&mut form, "!");
    assert_eq!(
        form.changes().properties,
        Some(props(&[("Client", "Acme"), ("Stage", "beta!")]))
    );
}

#[test]
fn esc_during_the_delete_question_only_answers_no() {
    let mut form = form_with_rows(true);
    tab(&mut form, 2);
    form.handle(ctrl('d'));
    form.handle(press(KeyCode::Esc)); // "no": still on Client's name
    type_text(&mut form, "X");
    assert_eq!(
        form.changes().properties,
        Some(props(&[("ClientX", "Acme"), ("Stage", "beta")]))
    );
}
