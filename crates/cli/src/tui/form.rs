use std::collections::BTreeMap;

use indexer_core::domain::normalize::normalize_tags;
use indexer_core::{Project, UpdateProject};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// One line of editable text and where the cursor is in it.
pub struct TextInput {
    value: String,
    /// Counts characters, not bytes: after `café` it is 4, though the string
    /// is 5 bytes long. Never past the end of `value`.
    cursor: usize,
}

impl TextInput {
    pub fn new(value: &str) -> Self {
        let value = value.to_string();
        let cursor_index = value.chars().count();
        Self {
            value,
            cursor: cursor_index,
        }
    }

    pub fn value(&self) -> &str {
        self.value.as_str()
    }

    /// Where the cursor is, in characters from the start.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert(&mut self, c: char) {
        let byte_pos = self.byte_index(self.cursor);
        self.value.insert(byte_pos, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let byte_pos = self.byte_index(self.cursor);
            self.value.remove(byte_pos);
        }
    }

    pub fn left(&mut self) {
        self.cursor = if self.cursor > 0 { self.cursor - 1 } else { 0 };
    }

    pub fn right(&mut self) {
        let max = self.count_chars();
        self.cursor = if self.cursor < max {
            self.cursor + 1
        } else {
            max
        };
    }

    pub fn delete(&mut self) {
        if self.cursor < self.count_chars() {
            let byte_pos = self.byte_index(self.cursor);
            self.value.remove(byte_pos);
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.count_chars();
    }

    fn count_chars(&self) -> usize {
        self.value.chars().count()
    }

    /// The byte offset where character `char_pos` starts, or the length of
    /// `value` when `char_pos` is at the end. `String::insert` and `remove`
    /// take byte offsets, and a character can be up to four bytes wide.
    fn byte_index(&self, char_pos: usize) -> usize {
        let mut byte_char = self.value.char_indices();
        let position = byte_char.nth(char_pos);
        match position {
            Some((byte_pos, _)) => byte_pos,
            None => self.value.len(),
        }
    }
}

/// One property: a name box and a value box side by side.
pub struct PropertyRow {
    name: TextInput,
    value: TextInput,
}

impl PropertyRow {
    pub fn name(&self) -> &TextInput {
        &self.name
    }

    pub fn value(&self) -> &TextInput {
        &self.value
    }

    fn is_empty(&self) -> bool {
        self.name.value().is_empty() && self.value.value().is_empty()
    }
}

pub enum Action {
    Continue,
    Save,
    Cancel,
}

/// Which box keys go to.
// `Copy`: the focus is a small value, read and replaced whole, never shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Description,
    Tags,
    Name(usize),
    Value(usize),
}

impl Focus {
    /// The row this focus is on, or `None` for the description and tags.
    fn row(self) -> Option<usize> {
        match self {
            Focus::Name(i) | Focus::Value(i) => Some(i),
            _ => None,
        }
    }
}

pub struct FormState {
    /// The project's name, for the form's title. Not editable here.
    title: String,
    description: String,
    edit_description: TextInput,
    tags: Vec<String>,
    edit_tags: TextInput,
    focus: Focus,
    wrap: bool,
    properties: BTreeMap<String, String>,
    rows: Vec<PropertyRow>,
    /// `Some(row)` while Ctrl+D waits for `y`; the next key answers it.
    pending_delete: Option<usize>,
}

impl FormState {
    pub fn new(project: &Project, wrap: bool) -> Self {
        Self {
            title: project.name.clone(),
            description: project.description.clone(),
            edit_description: TextInput::new(&project.description),
            tags: project.tags.clone(),
            edit_tags: TextInput::new(&project.tags.join(", ")),
            focus: Focus::Description,
            properties: project.properties.clone(),
            rows: project
                .properties
                .iter()
                .map(|(name, value)| PropertyRow {
                    name: TextInput::new(name),
                    value: TextInput::new(value),
                })
                .collect(),
            wrap,
            pending_delete: None,
        }
    }

    // Read-only views for `ui::form::draw`, which paints and decides nothing.

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn description_input(&self) -> &TextInput {
        &self.edit_description
    }

    pub fn tags_input(&self) -> &TextInput {
        &self.edit_tags
    }

    pub fn rows(&self) -> &[PropertyRow] {
        &self.rows
    }

    /// The row Ctrl+D is asking about, while it waits for `y`.
    pub fn pending_delete(&self) -> Option<usize> {
        self.pending_delete
    }

    /// Tab / ↓. On the last box (the last row's value, or the tags when there
    /// are no rows), wraps to the first or stays, per `wrap`.
    fn next_focus(&mut self) {
        let from = self.focus;
        self.focus = match self.focus {
            Focus::Description => Focus::Tags,
            Focus::Tags if !self.rows.is_empty() => Focus::Name(0),
            // No rows below: the tags are the last field.
            Focus::Tags if self.wrap => Focus::Description,
            Focus::Tags => Focus::Tags,
            Focus::Name(i) => Focus::Value(i),
            Focus::Value(i) if i < self.rows.len() - 1 => Focus::Name(i + 1),
            Focus::Value(i) if i == self.rows.len() - 1 && self.wrap => Focus::Description,
            Focus::Value(i) => Focus::Value(i),
        };
        self.leave_row(from);
    }

    /// Shift+Tab / ↑. On the first field, wraps to the last box (the last
    /// row's value, or the tags when there are no rows) or stays, per `wrap`.
    fn previous_focus(&mut self) {
        let from = self.focus;
        self.focus = match self.focus {
            Focus::Tags => Focus::Description,
            Focus::Description if !self.rows.is_empty() && self.wrap => {
                Focus::Value(self.rows.len() - 1)
            }
            Focus::Description if self.wrap => Focus::Tags,
            Focus::Description => Focus::Description,
            // Before the general arm: row 0 has no row above it, and `0 - 1`
            // would panic.
            Focus::Name(0) => Focus::Tags,
            Focus::Name(i) => Focus::Value(i - 1),
            Focus::Value(i) => Focus::Name(i),
        };
        self.leave_row(from);
    }

    fn focused_input(&mut self) -> &mut TextInput {
        match self.focus {
            Focus::Description => &mut self.edit_description,
            Focus::Tags => &mut self.edit_tags,
            Focus::Name(i) => &mut self.rows[i].name,
            Focus::Value(i) => &mut self.rows[i].value,
        }
    }

    /// What one key does to the form, and whether the caller should keep
    /// going, save or cancel.
    pub fn handle(&mut self, key: KeyEvent) -> Action {
        // Windows reports releases too; acting on them would type every key
        // twice.
        if key.kind != KeyEventKind::Press {
            return Action::Continue;
        } else if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            // In raw mode Ctrl+C is a key, not a signal, so the form must
            // handle it or it cannot be quit that way.
            return Action::Cancel;
        }
        // Ctrl+D asked "delete this row?": this key is the answer, whatever
        // it is. `take` closes the question either way; only `y` deletes,
        // and the key does nothing else.
        if let Some(r) = self.pending_delete.take() {
            if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                self.remove_row(r);
            }
            return Action::Continue;
        }
        match key.code {
            KeyCode::Enter => Action::Save,
            KeyCode::Esc => Action::Cancel,
            KeyCode::Tab | KeyCode::Down => {
                self.next_focus();
                Action::Continue
            }
            // Shift+Tab: terminals report it as its own key, not Tab + Shift.
            KeyCode::BackTab | KeyCode::Up => {
                self.previous_focus();
                Action::Continue
            }
            KeyCode::Left => {
                self.focused_input().left();
                Action::Continue
            }
            KeyCode::Right => {
                self.focused_input().right();
                Action::Continue
            }
            KeyCode::Home => {
                self.focused_input().home();
                Action::Continue
            }
            KeyCode::End => {
                self.focused_input().end();
                Action::Continue
            }
            KeyCode::Backspace => {
                self.focused_input().backspace();
                Action::Continue
            }
            KeyCode::Delete => {
                self.focused_input().delete();
                Action::Continue
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.add_row();
                Action::Continue
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.delete_row();
                Action::Continue
            }
            // Typing. A Ctrl-held letter is a shortcut, never text: Ctrl+N
            // must not also type an `n`.
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.focused_input().insert(c);
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    /// Only the fields that differ from the project as loaded; everything
    /// else `None`, so saving an untouched form writes nothing.
    pub fn changes(&self) -> UpdateProject {
        // Core stores "no description" as `""`, so an emptied field is
        // `Some("")`, a real change, not `None`.
        let description = if self.edit_description.value() != self.description {
            Some(self.edit_description.value().to_string())
        } else {
            None
        };

        // Compared as core stores them: `rust` retyped over `Rust` is no
        // change. The list is sent as typed; core normalizes on save.
        let typed_tags = parse_tags(self.edit_tags.value());
        let tags = if normalize_tags(typed_tags.clone()) != normalize_tags(self.tags.clone()) {
            Some(typed_tags)
        } else {
            None
        };

        // Names match ignoring case, as `edit::edited_properties` and the
        // search bar do: of two equal names the lower row wins. A blank row
        // is dropped; a value without a name is kept, and core refuses it.
        let mut typed = BTreeMap::new();
        for row in &self.rows {
            if row.is_empty() {
                continue;
            }
            let wanted = row.name.value().trim().to_lowercase();
            typed.retain(|existing: &String, _| existing.trim().to_lowercase() != wanted);
            typed.insert(row.name.value().to_string(), row.value.value().to_string());
        }
        let properties = if Self::lowercase_names(&typed) != Self::lowercase_names(&self.properties)
        {
            Some(typed)
        } else {
            None
        };

        UpdateProject {
            description,
            tags,
            properties,
            ..Default::default()
        }
    }

    /// The map with each name trimmed and lowercased, for comparing; values
    /// are compared exactly, as core keeps them.
    fn lowercase_names(map: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        map.iter()
            .map(|(k, v)| (k.trim().to_lowercase(), v.clone()))
            .collect()
    }

    /// Ctrl+N: a new empty row at the end, focused on its name. Does nothing
    /// while the focused row is itself empty, so blank rows cannot stack up.
    fn add_row(&mut self) {
        if let Some(r) = self.focus.row() {
            if self.rows[r].is_empty() {
                return;
            }
        }
        let from = self.focus;
        self.rows.push(PropertyRow {
            name: TextInput::new(""),
            value: TextInput::new(""),
        });
        self.focus = Focus::Name(self.rows.len() - 1);
        // Every focus move goes through `leave_row`. It removes nothing here:
        // an empty row stopped us above.
        self.leave_row(from);
    }

    /// Ctrl+D: deletes the focused row and focuses the row that took its
    /// place, else the one above, else the tags. Off a row it does nothing.
    /// Removes row `r` and focuses the row that took its place, else the one
    /// above, else the tags.
    fn remove_row(&mut self, r: usize) {
        self.rows.remove(r);
        self.focus = if r < self.rows.len() {
            Focus::Name(r)
        } else if r > 0 {
            Focus::Name(r - 1)
        } else {
            Focus::Tags
        };
    }

    /// Ctrl+D on a row: asks first (`pending_delete`), unless both boxes are
    /// empty and nothing would be lost. Off a row it does nothing.
    fn delete_row(&mut self) {
        let Some(r) = self.focus.row() else {
            return;
        };
        if self.rows[r].is_empty() {
            self.remove_row(r);
        } else {
            self.pending_delete = Some(r);
        }
    }

    /// Called after every focus move. When focus has left a row whose name
    /// and value are both empty, removes it and renumbers the focus if it was
    /// below — a row needs a name, and a blank one should not linger.
    fn leave_row(&mut self, from: Focus) {
        let Some(r) = from.row() else {
            return;
        };
        if self.focus.row() == Some(r) {
            return;
        }

        if !self.rows[r].is_empty() {
            return;
        }
        self.rows.remove(r);
        self.focus = match self.focus {
            Focus::Name(i) if i > r => Focus::Name(i - 1),
            Focus::Value(i) if i > r => Focus::Value(i - 1),
            other => other,
        };
    }
}

/// The tags field's text as a list: split on commas, as the GUI's tag box
/// does, trimmed, with empty pieces (`a,,b`, a trailing comma) dropped.
fn parse_tags(text: &str) -> Vec<String> {
    text.split(',')
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_string())
        .collect()
}
