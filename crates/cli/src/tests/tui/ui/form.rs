use indexer_core::Project;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Position;
use ratatui::style::Modifier;
use ratatui::Terminal;
use serde_json::json;

use crate::editor::FormKind;
use crate::tui::form::FormState;
use crate::tui::ui::form::draw;

fn project(properties: serde_json::Value) -> Project {
    serde_json::from_value(json!({
        "id": "app-0000-4000-8000-000000000000",
        "name": "app",
        "directory": "/home/me/work/app",
        "description": "A tool",
        "tags": ["Rust", "Web"],
        "properties": properties,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
    }))
    .unwrap()
}

fn form() -> FormState {
    FormState::new(
        &project(json!({"Client": "Acme", "Stage": "beta"})),
        true,
        FormKind::Compact,
    )
}

fn press(form: &mut FormState, code: KeyCode) {
    form.handle(KeyEvent::new(code, KeyModifiers::NONE));
}

fn ctrl(form: &mut FormState, c: char) {
    form.handle(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}

/// Draws the form on a `width` × `height` test terminal.
fn render(form: &FormState, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(frame, form)).unwrap();
    terminal
}

/// The screen as text, one string per line, trailing spaces trimmed.
fn screen(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            let line: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            line.trim_end().to_string()
        })
        .collect()
}

fn cursor(terminal: &mut Terminal<TestBackend>) -> Position {
    terminal.get_cursor_position().unwrap()
}

fn modifier_at(terminal: &Terminal<TestBackend>, x: u16, y: u16) -> Modifier {
    terminal.backend().buffer()[(x, y)].modifier
}

#[test]
fn the_form_lays_out_its_fields() {
    let terminal = render(&form(), 60, 11);
    assert_eq!(
        screen(&terminal),
        [
            "┌ Edit app ────────────────────────────────────────────────┐",
            "│ › Description   A tool                                   │",
            "│   Tags          Rust, Web                                │",
            "│                                                          │",
            "│   Properties                                             │",
            "│     Client      Acme                                     │",
            "│     Stage       beta                                     │",
            "│                                                          │",
            "│                                                          │",
            "│ Enter save · Esc cancel · Tab next · Ctrl+N add property │",
            "└──────────────────────────────────────────────────────────┘",
        ]
    );
}

#[test]
fn the_focused_box_is_reversed_and_holds_the_cursor() {
    let mut terminal = render(&form(), 60, 11);
    // "A tool" starts at column 18 (border, padding, 16-column labels).
    assert!(modifier_at(&terminal, 18, 1).contains(Modifier::REVERSED));
    assert!(!modifier_at(&terminal, 18, 2).contains(Modifier::REVERSED));
    assert_eq!(
        cursor(&mut terminal),
        Position::new(24, 1),
        "after `A tool`"
    );
}

#[test]
fn focus_on_a_row_marks_it_and_reverses_that_box() {
    let mut form = form();
    for _ in 0..3 {
        press(&mut form, KeyCode::Tab); // tags, Client's name, Client's value
    }
    let mut terminal = render(&form, 60, 11);
    assert_eq!(
        screen(&terminal)[5],
        "│   › Client      Acme                                     │"
    );
    assert!(modifier_at(&terminal, 18, 5).contains(Modifier::REVERSED));
    assert!(!modifier_at(&terminal, 6, 5).contains(Modifier::REVERSED));
    assert_eq!(cursor(&mut terminal), Position::new(22, 5));
}

#[test]
fn no_properties_says_how_to_add_one() {
    let form = FormState::new(&project(json!({})), true, FormKind::Compact);
    let terminal = render(&form, 60, 11);
    assert_eq!(
        screen(&terminal)[4],
        "│   Properties  none · Ctrl+N adds one                     │"
    );
}

#[test]
fn ctrl_d_shows_its_question_in_place_of_the_hint() {
    let mut form = form();
    press(&mut form, KeyCode::Tab);
    press(&mut form, KeyCode::Tab); // Client's name
    ctrl(&mut form, 'd');
    let terminal = render(&form, 60, 11);
    assert_eq!(
        screen(&terminal)[9],
        "│ Delete \"Client\"? y/n                                     │"
    );
    assert!(modifier_at(&terminal, 6, 5).contains(Modifier::CROSSED_OUT));
}

#[test]
fn a_long_value_scrolls_to_keep_the_cursor_in_view() {
    let mut form = form();
    for c in " that is very long indeed".chars() {
        press(&mut form, KeyCode::Char(c));
    }
    // 30 wide: the description box is 30 - 2 borders - 2 padding - 16 = 10
    // columns, nine of text and the cursor's own after them.
    let mut terminal = render(&form, 30, 11);
    assert_eq!(screen(&terminal)[1], "│ › Description   ng indeed  │");
    assert_eq!(cursor(&mut terminal), Position::new(27, 1));
}

#[test]
fn the_cursor_counts_columns_for_wide_characters() {
    let mut form = FormState::new(&project(json!({})), true, FormKind::Compact);
    press(&mut form, KeyCode::End);
    for _ in 0..10 {
        press(&mut form, KeyCode::Backspace);
    }
    for c in "日本".chars() {
        press(&mut form, KeyCode::Char(c));
    }
    let mut terminal = render(&form, 60, 11);
    // Two characters, four columns.
    assert_eq!(cursor(&mut terminal), Position::new(18 + 4, 1));
}

#[test]
fn rows_scroll_to_keep_the_focused_row_on_screen() {
    let mut form = FormState::new(
        &project(json!({"A": "1", "B": "2", "C": "3", "D": "4", "E": "5"})),
        true,
        FormKind::Compact,
    );
    // The last value: Shift+Tab from the description wraps there.
    press(&mut form, KeyCode::BackTab);
    // 10 high: borders 2, fields and spacing 6, leaving 2 lines for rows.
    let mut terminal = render(&form, 60, 10);
    let lines = screen(&terminal);
    assert_eq!(
        lines[5],
        "│     D           4                                        │"
    );
    assert_eq!(
        lines[6],
        "│   › E           5                                        │"
    );
    assert_eq!(cursor(&mut terminal).y, 6);
}

#[test]
fn the_hints_follow_the_focus() {
    // The hint line's text, without the borders and padding.
    let line = |form: &FormState| {
        screen(&render(form, 100, 11))[9]
            .trim_matches(|c| c == '│' || c == ' ')
            .to_string()
    };

    let mut form = form();
    assert_eq!(
        line(&form),
        "Enter save · Esc cancel · Tab next · Ctrl+N add property · Shift+Tab back",
        "on the description: no Ctrl+D, nothing to delete"
    );

    press(&mut form, KeyCode::Tab);
    press(&mut form, KeyCode::Tab); // Client's name
    assert_eq!(
        line(&form),
        "Enter value · Esc leave properties · Tab next · Ctrl+N add property · Ctrl+D delete"
    );

    press(&mut form, KeyCode::Enter); // Client's value
    assert!(line(&form).starts_with("Enter save · Esc leave properties"));
}

#[test]
fn the_full_form_says_so_in_its_title() {
    let form = FormState::new(&project(json!({})), true, FormKind::Full);
    assert!(screen(&render(&form, 60, 11))[0].starts_with("┌ Edit app · all fields ─"));
}
