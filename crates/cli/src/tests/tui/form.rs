use indexer_core::Project;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use serde_json::json;

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
    FormState::new(&project("A tool", &["Rust", "Web"]), true)
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
    FormState::new(&project("A tool", &["Rust"]), wrap)
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
