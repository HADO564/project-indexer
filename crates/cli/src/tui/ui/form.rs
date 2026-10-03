//! Draws the `edit` form. Painting only: what each key does, and which box
//! has focus, is all `FormState`'s; this reads it and puts it on screen.
//!
//! Styled with modifiers alone (bold, dim, reversed, crossed out), never
//! colours, so it reads the same under `NO_COLOR` and on any theme.
//!
//! ```text
//! ┌ Edit app ────────────────────────────────────┐
//! │ › Description   A tool                       │
//! │   Tags          Rust, Web                    │
//! │                                              │
//! │   Properties                                 │
//! │     Client      Acme                         │
//! │     Stage       beta                         │
//! │                                              │
//! │ Enter save · Esc cancel · Tab next · …       │
//! └──────────────────────────────────────────────┘
//! ```

use ratatui::layout::{Constraint, Layout, Margin, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::editor::FormKind;
use crate::tui::form::{Focus, FormState, TextInput};

/// Where every value starts, counted from the inner edge: the labels' column,
/// and for a property row its marker, name box and a one-column gap.
const LABEL_WIDTH: u16 = 16;
const MARKER_WIDTH: u16 = 4;

/// The keys for where focus is, most needed first: a narrow terminal drops
/// them from the end, whole, rather than cutting one off mid-word. Enter and
/// Esc say what they do here; Ctrl+D is only offered on a row.
fn hints(focus: Focus) -> Vec<&'static str> {
    let on_row = matches!(focus, Focus::Name(_) | Focus::Value(_));
    let mut hints = vec![if matches!(focus, Focus::Name(_)) {
        "Enter value"
    } else {
        "Enter save"
    }];
    if focus == Focus::Favorite {
        hints.push("Space toggle");
    }
    hints.extend([
        if on_row {
            "Esc leave properties"
        } else {
            "Esc cancel"
        },
        "Tab next",
        "Ctrl+N add property",
    ]);
    if on_row {
        hints.push("Ctrl+D delete");
    }
    // Last, so the first to go: Tab's partner is the easiest to guess.
    hints.push("Shift+Tab back");
    hints
}

pub fn draw(frame: &mut Frame, state: &FormState) {
    // The full form says so, so `edit --full` and a bare `edit` cannot be
    // mistaken for each other.
    let title = match state.kind() {
        FormKind::Compact => format!(" Edit {} ", state.title()),
        FormKind::Full => format!(" Edit {} · all fields ", state.title()),
    };
    let block = Block::bordered().title(title);
    let inner = block.inner(frame.area()).inner(Margin::new(1, 0));
    frame.render_widget(block, frame.area());

    // The full form's extra lines take no height in the compact one.
    let full = state.kind() == FormKind::Full;
    let extra = Constraint::Length(u16::from(full));
    let [name, directory, description, tags, favorite, notes, _, header, rows, _, hint] =
        Layout::vertical([
            extra,
            extra,
            Constraint::Length(1),
            Constraint::Length(1),
            extra,
            extra,
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);

    let focus = state.focus();
    if full {
        field(
            frame,
            name,
            "Name",
            state.name_input(),
            focus == Focus::ProjectName,
        );
        field(
            frame,
            directory,
            "Directory",
            state.directory_input(),
            focus == Focus::Directory,
        );
    }
    field(
        frame,
        description,
        "Description",
        state.description_input(),
        focus == Focus::Description,
    );
    field(
        frame,
        tags,
        "Tags",
        state.tags_input(),
        focus == Focus::Tags,
    );
    if full {
        checkbox(
            frame,
            favorite,
            "Favourite",
            state.favorite_checked(),
            focus == Focus::Favorite,
        );
        field(
            frame,
            notes,
            "Notes",
            state.notes_input(),
            focus == Focus::Notes,
        );
    }
    properties(frame, header, rows, state);
    hint_line(frame, hint, state);
}

/// A labelled one-line input: the description or the tags.
fn field(frame: &mut Frame, area: Rect, label: &str, input: &TextInput, focused: bool) {
    let [label_area, input_area] =
        Layout::horizontal([Constraint::Length(LABEL_WIDTH), Constraint::Fill(1)]).areas(area);
    frame.render_widget(Paragraph::new(label_line(label, focused)), label_area);
    text_box(frame, input_area, input, focused, Style::new());
}

/// A field's label, marked `›` and bold when its field has focus.
fn label_line(label: &str, focused: bool) -> Line<'static> {
    if focused {
        Line::from(Span::styled(
            format!("› {label}"),
            Style::new().add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(format!("  {label}"))
    }
}

/// A labelled tick box: `[x]` or `[ ]`, reversed when focused, as a text box
/// is. It holds no cursor — there is nothing to type into.
fn checkbox(frame: &mut Frame, area: Rect, label: &str, checked: bool, focused: bool) {
    let [label_area, box_area] =
        Layout::horizontal([Constraint::Length(LABEL_WIDTH), Constraint::Length(3)]).areas(area);
    frame.render_widget(Paragraph::new(label_line(label, focused)), label_area);
    let style = if focused {
        Style::new().add_modifier(Modifier::REVERSED)
    } else {
        Style::new()
    };
    let mark = if checked { "[x]" } else { "[ ]" };
    frame.render_widget(Paragraph::new(mark).style(style), box_area);
}

fn properties(frame: &mut Frame, header: Rect, area: Rect, state: &FormState) {
    let rows = state.rows();
    let mut title = vec![Span::raw("  Properties")];
    if rows.is_empty() {
        title.push(Span::styled(
            "  none · Ctrl+N adds one",
            Style::new().add_modifier(Modifier::DIM),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(title)), header);

    let focused_row = match state.focus() {
        Focus::Name(i) | Focus::Value(i) => Some(i),
        _ => None,
    };
    // More rows than lines: scroll just far enough to keep the focused row
    // on screen.
    let height = area.height as usize;
    let first = match focused_row {
        Some(i) if height > 0 && i >= height => i + 1 - height,
        _ => 0,
    };

    for (line, (i, row)) in rows.iter().enumerate().skip(first).enumerate() {
        if line >= height {
            break;
        }
        let line_area = Rect {
            y: area.y + line as u16,
            height: 1,
            ..area
        };
        let [marker, name, _, value] = Layout::horizontal([
            Constraint::Length(MARKER_WIDTH),
            Constraint::Length(LABEL_WIDTH - MARKER_WIDTH - 1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(line_area);

        if focused_row == Some(i) {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "  ›",
                    Style::new().add_modifier(Modifier::BOLD),
                )),
                marker,
            );
        }
        // The row Ctrl+D is asking about is struck through until answered.
        let pending = if state.pending_delete() == Some(i) {
            Style::new().add_modifier(Modifier::CROSSED_OUT)
        } else {
            Style::new()
        };
        text_box(
            frame,
            name,
            row.name(),
            state.focus() == Focus::Name(i),
            pending,
        );
        text_box(
            frame,
            value,
            row.value(),
            state.focus() == Focus::Value(i),
            pending,
        );
    }
}

/// One input's text. The focused box is drawn reversed across its whole
/// width, so it shows even when empty, and holds the terminal's cursor.
fn text_box(frame: &mut Frame, area: Rect, input: &TextInput, focused: bool, style: Style) {
    let (visible, cursor_x) = scrolled(input, area.width);
    let style = if focused {
        style.add_modifier(Modifier::REVERSED)
    } else {
        style
    };
    frame.render_widget(Paragraph::new(visible).style(style), area);
    if focused {
        frame.set_cursor_position(Position::new(area.x + cursor_x, area.y));
    }
}

/// The part of the input that fits `width` columns with the cursor in view,
/// and the cursor's column within it. Measured in display columns, not
/// characters: an emoji or a CJK character is one character and two columns.
fn scrolled(input: &TextInput, width: u16) -> (String, u16) {
    let chars: Vec<char> = input.value().chars().collect();
    let cursor = input.cursor();
    let columns = |chars: &[char]| Span::raw(chars.iter().collect::<String>()).width();
    // Text before the cursor plus the cursor's own column must fit.
    let mut start = 0;
    while start < cursor && columns(&chars[start..cursor]) >= width as usize {
        start += 1;
    }
    let visible = chars[start..].iter().collect();
    (visible, columns(&chars[start..cursor]) as u16)
}

/// The keys; or in their place, why a save could not go ahead, or while
/// Ctrl+D waits, its question.
fn hint_line(frame: &mut Frame, area: Rect, state: &FormState) {
    if let Some(error) = state.error() {
        let line = Line::from(Span::styled(
            format!("Not saved: {error}"),
            Style::new().add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(line), area);
        return;
    }
    let line = match state.pending_delete() {
        Some(i) => {
            let name = state.rows()[i].name().value();
            let question = if name.is_empty() {
                "Delete this property? y/n".to_string()
            } else {
                format!("Delete \"{name}\"? y/n")
            };
            Line::from(Span::styled(
                question,
                Style::new().add_modifier(Modifier::BOLD),
            ))
        }
        None => Line::from(Span::styled(
            fitting_hints(&hints(state.focus()), area.width as usize),
            Style::new().add_modifier(Modifier::DIM),
        )),
    };
    frame.render_widget(Paragraph::new(line), area);
}

/// As many of `hints` as fit in `width` columns, joined with ` · `.
fn fitting_hints(hints: &[&str], width: usize) -> String {
    let mut line = String::new();
    for hint in hints {
        let next = if line.is_empty() {
            hint.to_string()
        } else {
            format!("{line} · {hint}")
        };
        if Span::raw(next.as_str()).width() > width {
            break;
        }
        line = next;
    }
    line
}
