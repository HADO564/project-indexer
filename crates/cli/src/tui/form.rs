use crate::editor::FormKind;
use std::collections::BTreeMap;

use indexer_core::domain::normalize::normalize_tags;
use indexer_core::{Group, Project, UpdateProject};
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
    /// The project's name box, in the full form only. Not `Name`: that is a
    /// property row's name.
    ProjectName,
    /// The project's folder, in the full form only.
    Directory,
    Description,
    Tags,
    /// The favourite checkbox, in the full form only.
    Favorite,
    /// The notes box, in the full form only.
    Notes,
    /// The open-with box, in the full form only.
    OpenWith,
    /// The group choice, in the full form only: picked with ←/→, not typed.
    Group,
    /// The project's own colour, in the full form only.
    Color,
    /// The project's own icon, in the full form only.
    Icon,
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
    /// The project as loaded: the form's title, and what `changes()` compares
    /// every box against. Never edited — the boxes below are.
    original: Project,

    // The boxes, as the user has them now, in the full form's order.
    /// The project's name, in the full form.
    name: TextInput,
    /// The project's folder as typed, in the full form. Sent as typed;
    /// `TerminalEditor` resolves it before saving.
    directory: TextInput,
    description: TextInput,
    /// The tags as one comma-separated line, as the app's tag box has them.
    tags: TextInput,
    /// The full form's checkbox.
    favorite: bool,
    /// One line, as the app's notes box; empty means no notes.
    notes: TextInput,
    /// The app to open the project with; empty means the system's default.
    open_with: TextInput,
    /// The chosen group's id, or `None` for ungrouped. Kept as the id rather
    /// than a place in `groups`, so a group deleted elsewhere while the form
    /// is open is still "no change" until the user picks another.
    group: Option<String>,
    /// A palette name or `#rrggbb`, as typed; empty means none. Sent as typed,
    /// for `TerminalEditor` to check, as the directory is.
    color: TextInput,
    /// A bundled icon's name or `custom:<name>`, as typed; empty means none.
    icon: TextInput,
    rows: Vec<PropertyRow>,

    // The form's own state.
    focus: Focus,
    wrap: bool,
    /// Compact, or every field with `edit --full`.
    kind: FormKind,
    /// What ←/→ cycle through on the group line, after "Ungrouped", in the
    /// sidebar's order. Empty in the compact form, which has no group line.
    groups: Vec<Group>,
    /// `Some(row)` while Ctrl+D waits for `y`; the next key answers it.
    pending_delete: Option<usize>,
    /// Why the last save could not go ahead, shown until the next key.
    error: Option<String>,
}

impl FormState {
    pub fn new(project: &Project, wrap: bool, kind: FormKind) -> Self {
        Self {
            original: project.clone(),
            name: TextInput::new(&project.name),
            directory: TextInput::new(&project.directory),
            description: TextInput::new(&project.description),
            tags: TextInput::new(&project.tags.join(", ")),
            favorite: project.favorite,
            rows: project
                .properties
                .iter()
                .map(|(name, value)| PropertyRow {
                    name: TextInput::new(name),
                    value: TextInput::new(value),
                })
                .collect(),
            // The first box in `order`: the name, in the full form.
            focus: match kind {
                FormKind::Compact => Focus::Description,
                FormKind::Full => Focus::ProjectName,
            },
            wrap,
            kind,
            notes: TextInput::new(project.notes.as_deref().unwrap_or_default()),
            open_with: TextInput::new(project.open_with.as_deref().unwrap_or_default()),
            group: project.group_id.clone(),
            color: TextInput::new(project.color.as_deref().unwrap_or_default()),
            icon: TextInput::new(project.icon.as_deref().unwrap_or_default()),
            groups: Vec::new(),
            pending_delete: None,
            error: None,
        }
    }

    /// The groups the group line offers. The form cannot read the database,
    /// so `TerminalEditor` hands them in.
    pub fn with_groups(mut self, groups: Vec<Group>) -> Self {
        self.groups = groups;
        self
    }

    // Read-only views for `ui::form::draw`, which paints and decides nothing.

    pub fn title(&self) -> &str {
        &self.original.name
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// The compact form, or the full one `edit --full` asked for.
    pub fn kind(&self) -> FormKind {
        self.kind
    }

    /// Whether the favourite checkbox is ticked, as the form shows it now.
    pub fn favorite_checked(&self) -> bool {
        self.favorite
    }

    pub fn name_input(&self) -> &TextInput {
        &self.name
    }

    pub fn directory_input(&self) -> &TextInput {
        &self.directory
    }

    /// Why the last save could not go ahead, while it is shown.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Keeps the form open with `message` in place of the hints, when what
    /// was typed cannot be saved — a directory that does not resolve. The
    /// next key clears it; nothing typed is lost.
    pub fn show_error(&mut self, message: String) {
        self.error = Some(message);
    }

    pub fn notes_input(&self) -> &TextInput {
        &self.notes
    }

    pub fn open_with_input(&self) -> &TextInput {
        &self.open_with
    }

    /// The group line's text: the chosen group's name, "Ungrouped", or, for
    /// a group deleted while the form was open, a note saying so.
    pub fn group_label(&self) -> &str {
        match &self.group {
            None => "Ungrouped",
            Some(id) => self
                .groups
                .iter()
                .find(|g| &g.id == id)
                .map_or("(deleted group)", |g| g.name.as_str()),
        }
    }

    pub fn color_input(&self) -> &TextInput {
        &self.color
    }

    pub fn icon_input(&self) -> &TextInput {
        &self.icon
    }

    /// Whether there is any group to pick; with none, ←/→ have nothing to do.
    pub fn has_groups(&self) -> bool {
        !self.groups.is_empty()
    }

    pub fn description_input(&self) -> &TextInput {
        &self.description
    }

    pub fn tags_input(&self) -> &TextInput {
        &self.tags
    }

    pub fn rows(&self) -> &[PropertyRow] {
        &self.rows
    }

    /// The row Ctrl+D is asking about, while it waits for `y`.
    pub fn pending_delete(&self) -> Option<usize> {
        self.pending_delete
    }

    /// Every box Tab visits, in order. The one place the order lives: a field
    /// the full form adds is a line here, and `next_focus` and
    /// `previous_focus` follow.
    fn order(&self) -> Vec<Focus> {
        let full = self.kind == FormKind::Full;
        let mut order = Vec::new();
        if full {
            order.extend([Focus::ProjectName, Focus::Directory]);
        }
        order.extend([Focus::Description, Focus::Tags]);
        if full {
            order.extend([
                Focus::Favorite,
                Focus::Notes,
                Focus::OpenWith,
                Focus::Group,
                Focus::Color,
                Focus::Icon,
            ]);
        }
        for row in 0..self.rows.len() {
            order.push(Focus::Name(row));
            order.push(Focus::Value(row));
        }
        order
    }

    /// Where the focus sits in `order`. Focus only ever points at a box that
    /// exists — leaving or deleting a row moves it first — so it is always
    /// found.
    fn position(&self, order: &[Focus]) -> usize {
        order
            .iter()
            .position(|&focus| focus == self.focus)
            .expect("focus is always on a box in the order")
    }

    /// Tab / ↓: the next box in `order`. On the last, wraps to the first or
    /// stays, per `wrap`.
    fn next_focus(&mut self) {
        let from = self.focus;
        let order = self.order();
        let at = self.position(&order);
        self.focus = match order.get(at + 1) {
            Some(&next) => next,
            None if self.wrap => order[0],
            None => self.focus,
        };
        self.leave_row(from);
    }

    /// Shift+Tab / ↑: the previous box in `order`. On the first, wraps to the
    /// last or stays, per `wrap`.
    fn previous_focus(&mut self) {
        let from = self.focus;
        let order = self.order();
        let at = self.position(&order);
        self.focus = if at > 0 {
            order[at - 1]
        } else if self.wrap {
            order[order.len() - 1]
        } else {
            self.focus
        };
        self.leave_row(from);
    }

    /// ←/→ on the group line: the next or previous choice in "Ungrouped",
    /// then each group. At the ends it wraps or stays, per `wrap`, as Tab
    /// does. A deleted group is in neither place, so either key starts over
    /// from "Ungrouped".
    fn cycle_group(&mut self, forward: bool) {
        let choices: Vec<Option<&String>> = std::iter::once(None)
            .chain(self.groups.iter().map(|g| Some(&g.id)))
            .collect();
        let last = choices.len() - 1;
        let next = match choices.iter().position(|c| *c == self.group.as_ref()) {
            None => 0,
            Some(at) if forward && at < last => at + 1,
            Some(at) if !forward && at > 0 => at - 1,
            Some(_) if self.wrap => {
                if forward {
                    0
                } else {
                    last
                }
            }
            Some(at) => at,
        };
        self.group = choices[next].cloned();
    }

    /// The text box keys type into, or `None` on the favourite checkbox and
    /// the group line, which are not ones.
    fn focused_input(&mut self) -> Option<&mut TextInput> {
        match self.focus {
            Focus::ProjectName => Some(&mut self.name),
            Focus::Directory => Some(&mut self.directory),
            Focus::Notes => Some(&mut self.notes),
            Focus::Description => Some(&mut self.description),
            Focus::Tags => Some(&mut self.tags),
            Focus::Favorite | Focus::Group => None,
            Focus::Name(i) => Some(&mut self.rows[i].name),
            Focus::Value(i) => Some(&mut self.rows[i].value),
            Focus::OpenWith => Some(&mut self.open_with),
            Focus::Color => Some(&mut self.color),
            Focus::Icon => Some(&mut self.icon),
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
        // An error from the last save stays up until the next key, which
        // then does what it always does — the user carries on editing.
        self.error = None;
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
            // Enter saves, except in a name box, where it moves on to that
            // row's value: after typing a name, a value comes next, not a save.
            KeyCode::Enter => match self.focus {
                Focus::Name(i) => {
                    self.focus = Focus::Value(i);
                    Action::Continue
                }
                _ => Action::Save,
            },
            // Esc on a property row leaves the properties for the tags (an
            // untouched row goes, as when leaving any empty row); only Esc
            // outside them cancels, so a stray Esc mid-row loses nothing.
            KeyCode::Esc => {
                if self.focus.row().is_some() {
                    let from = self.focus;
                    self.focus = Focus::Tags;
                    self.leave_row(from);
                    Action::Continue
                } else {
                    Action::Cancel
                }
            }
            KeyCode::Tab | KeyCode::Down => {
                self.next_focus();
                Action::Continue
            }
            // Shift+Tab: terminals report it as its own key, not Tab + Shift.
            KeyCode::BackTab | KeyCode::Up => {
                self.previous_focus();
                Action::Continue
            }
            // On the group line ←/→ choose; everywhere else they move the
            // cursor.
            KeyCode::Left if self.focus == Focus::Group => {
                self.cycle_group(false);
                Action::Continue
            }
            KeyCode::Right if self.focus == Focus::Group => {
                self.cycle_group(true);
                Action::Continue
            }
            KeyCode::Left => {
                if let Some(input) = self.focused_input() {
                    input.left();
                }
                Action::Continue
            }
            KeyCode::Right => {
                if let Some(input) = self.focused_input() {
                    input.right();
                }
                Action::Continue
            }
            KeyCode::Home => {
                if let Some(input) = self.focused_input() {
                    input.home();
                }
                Action::Continue
            }
            KeyCode::End => {
                if let Some(input) = self.focused_input() {
                    input.end();
                }
                Action::Continue
            }
            KeyCode::Backspace => {
                if let Some(input) = self.focused_input() {
                    input.backspace();
                }
                Action::Continue
            }
            KeyCode::Delete => {
                if let Some(input) = self.focused_input() {
                    input.delete();
                }
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
            // Space ticks or unticks the full form's favourite checkbox. Above
            // typing, so it is not swallowed as text; anywhere else Space types.
            KeyCode::Char(' ') if self.focus == Focus::Favorite => {
                self.favorite = !self.favorite;
                Action::Continue
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // No text box under the checkbox: other characters do nothing.
                if let Some(input) = self.focused_input() {
                    input.insert(c);
                }
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    /// Only the fields that differ from the project as loaded; everything
    /// else `None`, so saving an untouched form writes nothing.
    pub fn changes(&self) -> UpdateProject {
        // An emptied name is sent too, and core refuses it — as it refuses a
        // nameless project anywhere.
        let name = if self.name.value() != self.original.name {
            Some(self.name.value().to_string())
        } else {
            None
        };

        // Sent as typed — relative, `~/…` or empty — for `TerminalEditor` to
        // resolve; only a box left as loaded is no change.
        let directory = if self.directory.value() != self.original.directory {
            Some(self.directory.value().to_string())
        } else {
            None
        };

        // Core stores "no description" as `""`, so an emptied field is
        // `Some("")`, a real change, not `None`.
        let description = if self.description.value() != self.original.description {
            Some(self.description.value().to_string())
        } else {
            None
        };

        // Compared as core stores them: `rust` retyped over `Rust` is no
        // change. The list is sent as typed; core normalizes on save.
        let typed_tags = parse_tags(self.tags.value());
        let tags =
            if normalize_tags(typed_tags.clone()) != normalize_tags(self.original.tags.clone()) {
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
        let properties =
            if Self::lowercase_names(&typed) != Self::lowercase_names(&self.original.properties) {
                Some(typed)
            } else {
                None
            };

        // Only when the box was left differently from how the project had it:
        // ticking and unticking again sends nothing.
        let favorite = if self.favorite != self.original.favorite {
            Some(self.favorite)
        } else {
            None
        };

        // A box in a box, as `--notes` builds it: "no notes" and an empty box
        // are the same, and emptying the box clears them (`Some(None)`).
        let notes = if self.notes.value() != self.original.notes.as_deref().unwrap_or_default() {
            let text = self.notes.value();
            Some(if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            })
        } else {
            None
        };

        // The same, trimmed as `--open-with` is: spaces around an app's name
        // are never part of it, so adding one is no change.
        let typed = self.open_with.value().trim();
        let open_with = if typed != self.original.open_with.as_deref().unwrap_or_default() {
            Some(if typed.is_empty() {
                None
            } else {
                Some(typed.to_string())
            })
        } else {
            None
        };

        // The id as chosen against the id as loaded: cycling all the way
        // round back to the start is no change.
        let group_id = if self.group != self.original.group_id {
            Some(self.group.clone())
        } else {
            None
        };

        // Trimmed, and compared ignoring case, as both are saved lowercased: `Cyan` over a stored `cyan` is no change. Anything else
        // goes as typed, for `TerminalEditor` to check against the palette
        // and the icons before saving.
        let color = cleared_or_typed(
            self.color.value(),
            self.original.color.as_deref(),
            |typed, stored| typed.eq_ignore_ascii_case(stored),
        );
        // Icons too: bundled and custom names alike are stored lowercase.
        let icon = cleared_or_typed(
            self.icon.value(),
            self.original.icon.as_deref(),
            |typed, stored| typed.eq_ignore_ascii_case(stored),
        );

        // Every field by name, with no `..Default::default()`: a field core
        // adds to `UpdateProject` will not compile here until the form
        // decides what to do with it.
        UpdateProject {
            name,
            directory,
            description,
            tags,
            favorite,
            notes,
            open_with,
            group_id,
            color,
            icon,
            properties,
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

/// A trimmed box as a box in a box: `None` when it still says what was
/// stored (`same` decides), `Some(None)` when emptied, else the text.
fn cleared_or_typed(
    typed: &str,
    stored: Option<&str>,
    same: fn(&str, &str) -> bool,
) -> Option<Option<String>> {
    let typed = typed.trim();
    if same(typed, stored.unwrap_or_default()) {
        None
    } else if typed.is_empty() {
        Some(None)
    } else {
        Some(Some(typed.to_string()))
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
