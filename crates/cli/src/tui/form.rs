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

pub enum Action {
    Continue,
    Save,
    Cancel,
}

enum Focus {
    Description,
    Tags,
}

pub struct FormState {
    description: String,
    edit_description: TextInput,
    tags: Vec<String>,
    edit_tags: TextInput,
    focus: Focus,
    wrap: bool,
}

impl FormState {
    pub fn new(project: &Project, wrap: bool) -> Self {
        Self {
            description: project.description.clone(),
            edit_description: TextInput::new(&project.description),
            tags: project.tags.clone(),
            edit_tags: TextInput::new(&project.tags.join(", ")),
            focus: Focus::Description,
            wrap,
        }
    }

    /// Tab / ↓. On the last field, wraps to the first or stays, per `wrap`.
    fn next_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Description => Focus::Tags,
            Focus::Tags if self.wrap => Focus::Description,
            Focus::Tags => Focus::Tags,
        };
    }

    /// Shift+Tab / ↑. On the first field, wraps to the last or stays, per
    /// `wrap`.
    fn previous_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Tags => Focus::Description,
            Focus::Description if self.wrap => Focus::Tags,
            Focus::Description => Focus::Description,
        };
    }

    fn focused_input(&mut self) -> &mut TextInput {
        match self.focus {
            Focus::Description => &mut self.edit_description,
            Focus::Tags => &mut self.edit_tags,
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

        UpdateProject {
            description,
            tags,
            ..Default::default()
        }
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
