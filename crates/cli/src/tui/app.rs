//! TUI state: the sidebar panes (the fixed views and the groups), the project
//! list, the selection and the focus.
//!
//! Pure state, the way `FormState` is pure: keys go in through [`App::handle`]
//! and the fields change, with no terminal and no database in sight, so every
//! navigation rule is a unit test. The event loop in [`super::run`] owns the
//! I/O — reading the database once at start, drawing each frame — and this owns
//! what is on screen and where the cursor is.
//!
//! The `:` command line runs the shell's commands against the same core (the
//! parsing and running live in [`super::cmdline`]); search and live refresh are
//! later phases (see `docs/superpowers/specs/2026-10-15-tui-design.md`).

use indexer_core::domain::sorting::SortOptions;
use indexer_core::domain::views::{resolve_view, view_counts, View, ViewCounts};
use indexer_core::domain::{Group, Project};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::appearance::IconStyle;
use crate::context::Context;
use crate::settings;

/// Which pane the keys drive. The sidebar is two panes — the fixed views and
/// the groups — so `[` / `]` can jump between them; the list is the third.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pane {
    Views,
    Groups,
    List,
}

/// What the status line is doing: the normal browser, or a `:` command being
/// typed — the string is what has been typed so far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Mode {
    Normal,
    Command(String),
}

/// Something the event loop must do that `App` cannot, having no `Context`.
/// Returned by [`App::handle`] so the loop runs the command and reloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    /// Run this `:` line and show its result.
    Run(String),
}

/// The whole TUI state for a frame.
pub(crate) struct App {
    /// Live projects, sorted as `dexily list` sorts them.
    projects: Vec<Project>,
    /// Binned projects, for the Bin view.
    deleted: Vec<Project>,
    groups: Vec<Group>,
    counts: ViewCounts,
    /// The fixed views, in sidebar order.
    views: Vec<View>,
    views_sel: usize,
    groups_sel: usize,
    /// What the list shows — set from whichever sidebar entry the cursor last
    /// moved onto, so selecting a view or group changes the list at once.
    active: View,
    list_sel: usize,
    focus: Pane,
    /// How icons are drawn, from `config icons`; `Off` by default.
    icons: IconStyle,
    /// Whether colour is drawn — false under `NO_COLOR`.
    color: bool,
    /// The status line's state: browsing, or typing a `:` command.
    mode: Mode,
    /// The last command's result, shown on the status line until the next one.
    message: String,
    quit: bool,
}

impl App {
    /// Reads the database once and builds the initial state.
    pub(crate) fn load(ctx: &Context) -> anyhow::Result<Self> {
        let opts = SortOptions::default();
        let projects = ctx.projects.list(opts)?;
        let deleted = ctx.projects.list_deleted(opts)?;
        let groups = ctx.groups.list()?;
        let mut app = Self::new(projects, deleted, groups);
        // A broken settings file must not stop the browser opening — icons off
        // is the safe default, as `resolve_look` falls back for the colours.
        app.icons = settings::load()
            .ok()
            .and_then(|saved| saved.icons)
            .unwrap_or_default();
        app.color = std::env::var_os("NO_COLOR").is_none();
        Ok(app)
    }

    /// The part of [`load`](Self::load) that has no I/O, so tests can drive the
    /// navigation without a database.
    pub(crate) fn new(projects: Vec<Project>, deleted: Vec<Project>, groups: Vec<Group>) -> Self {
        let counts = view_counts(&projects, &deleted, &groups);
        let views = vec![View::All, View::Favorites, View::Ungrouped, View::Bin];
        Self {
            active: views[0].clone(),
            projects,
            deleted,
            groups,
            counts,
            views,
            views_sel: 0,
            groups_sel: 0,
            list_sel: 0,
            focus: Pane::Views,
            icons: IconStyle::Off,
            color: true,
            mode: Mode::Normal,
            message: String::new(),
            quit: false,
        }
    }

    /// Re-reads the database after a command may have changed it, keeping the
    /// active view and clamping the selections. A view whose group has since
    /// been deleted falls back to All, so the list is never left pointing at
    /// nothing.
    pub(crate) fn reload(&mut self, ctx: &Context) -> anyhow::Result<()> {
        let opts = SortOptions::default();
        self.projects = ctx.projects.list(opts)?;
        self.deleted = ctx.projects.list_deleted(opts)?;
        self.groups = ctx.groups.list()?;
        self.counts = view_counts(&self.projects, &self.deleted, &self.groups);
        if let View::Group { id } = &self.active {
            if !self.groups.iter().any(|group| &group.id == id) {
                self.active = View::All;
                self.focus = Pane::Views;
                self.views_sel = 0;
            }
        }
        self.groups_sel = self.groups_sel.min(self.groups.len().saturating_sub(1));
        self.list_sel = self.list_sel.min(self.visible().len().saturating_sub(1));
        Ok(())
    }

    // --- read by the drawing code and the tests ---

    pub(crate) fn focus(&self) -> Pane {
        self.focus
    }
    pub(crate) fn views(&self) -> &[View] {
        &self.views
    }
    pub(crate) fn groups(&self) -> &[Group] {
        &self.groups
    }
    pub(crate) fn views_sel(&self) -> usize {
        self.views_sel
    }
    pub(crate) fn groups_sel(&self) -> usize {
        self.groups_sel
    }
    pub(crate) fn list_sel(&self) -> usize {
        self.list_sel
    }
    pub(crate) fn active(&self) -> &View {
        &self.active
    }
    pub(crate) fn quit(&self) -> bool {
        self.quit
    }
    pub(crate) fn icons(&self) -> IconStyle {
        self.icons
    }
    pub(crate) fn color_enabled(&self) -> bool {
        self.color
    }
    pub(crate) fn mode(&self) -> &Mode {
        &self.mode
    }
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    /// Set by the event loop to the result of the command it just ran.
    pub(crate) fn set_message(&mut self, message: String) {
        self.message = message;
    }

    /// The colour of a project's group, if it has one — the fallback for a
    /// project's own mark colour, as `dexily list` resolves it.
    pub(crate) fn group_color_of(&self, project: &Project) -> Option<&str> {
        let id = project.group_id.as_deref()?;
        self.groups
            .iter()
            .find(|group| group.id == id)
            .map(|group| group.color.as_str())
    }

    /// How many projects a sidebar entry holds, from the precomputed counts.
    pub(crate) fn count_for(&self, view: &View) -> usize {
        match view {
            View::All => self.counts.all,
            View::Favorites => self.counts.favorites,
            View::Ungrouped => self.counts.ungrouped,
            View::Bin => self.counts.bin,
            View::Group { id } => self.counts.groups.get(id).copied().unwrap_or(0),
        }
    }

    /// The projects the active view shows. Phase 1 applies no search, so the
    /// query is empty — which `resolve_view` treats as "everything in view".
    pub(crate) fn visible(&self) -> Vec<&Project> {
        resolve_view(&self.active, &self.projects, &self.deleted, "")
    }

    /// The project under the list cursor, if the view is not empty.
    pub(crate) fn selected_project(&self) -> Option<&Project> {
        self.visible().get(self.list_sel).copied()
    }

    // --- key handling ---

    /// Applies one key press. In the browser it moves the selection or the
    /// focus, or opens the `:` line; while the `:` line is open it edits that
    /// line. Any [`Action`] returned is for the event loop to carry out, since
    /// running a command needs the `Context` this does not hold.
    pub(crate) fn handle(&mut self, key: KeyEvent) -> Option<Action> {
        match self.mode {
            Mode::Normal => {
                self.handle_normal(key);
                None
            }
            Mode::Command(_) => self.handle_command(key),
        }
    }

    /// Browser keys: movement, focus, quit, and `:` to open the command line.
    fn handle_normal(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char(':') => self.mode = Mode::Command(String::new()),
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('g') => self.move_top(),
            KeyCode::Char('G') => self.move_bottom(),
            KeyCode::Char('h') | KeyCode::Left => self.focus_left(),
            KeyCode::Char('l') | KeyCode::Right => self.focus_right(),
            KeyCode::Tab => self.focus_next(),
            KeyCode::Char('[') => self.jump_section(Pane::Views),
            KeyCode::Char(']') => self.jump_section(Pane::Groups),
            _ => {}
        }
    }

    /// Editing the `:` line: type into it, `Backspace` erases, `Esc` (or
    /// `Ctrl+C`) cancels back to the browser, `Enter` runs what was typed.
    fn handle_command(&mut self, key: KeyEvent) -> Option<Action> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if key.code == KeyCode::Esc || (ctrl && key.code == KeyCode::Char('c')) {
            self.mode = Mode::Normal;
            return None;
        }
        let Mode::Command(buffer) = &mut self.mode else {
            return None;
        };
        match key.code {
            KeyCode::Enter => {
                let line = std::mem::take(buffer);
                self.mode = Mode::Normal;
                (!line.trim().is_empty()).then_some(Action::Run(line))
            }
            KeyCode::Backspace => {
                buffer.pop();
                None
            }
            KeyCode::Char(c) if !ctrl => {
                buffer.push(c);
                None
            }
            _ => None,
        }
    }

    fn move_down(&mut self) {
        match self.focus {
            Pane::Views => self.views_sel = step_down(self.views_sel, self.views.len()),
            Pane::Groups => self.groups_sel = step_down(self.groups_sel, self.groups.len()),
            Pane::List => {
                self.list_sel = step_down(self.list_sel, self.visible().len());
                return;
            }
        }
        self.sync_active();
    }

    fn move_up(&mut self) {
        match self.focus {
            Pane::Views => self.views_sel = self.views_sel.saturating_sub(1),
            Pane::Groups => self.groups_sel = self.groups_sel.saturating_sub(1),
            Pane::List => {
                self.list_sel = self.list_sel.saturating_sub(1);
                return;
            }
        }
        self.sync_active();
    }

    /// `g`: jump to the top of the focused pane.
    fn move_top(&mut self) {
        match self.focus {
            Pane::Views => self.views_sel = 0,
            Pane::Groups => self.groups_sel = 0,
            Pane::List => {
                self.list_sel = 0;
                return;
            }
        }
        self.sync_active();
    }

    /// `G`: jump to the bottom of the focused pane.
    fn move_bottom(&mut self) {
        match self.focus {
            Pane::Views => self.views_sel = self.views.len().saturating_sub(1),
            Pane::Groups => self.groups_sel = self.groups.len().saturating_sub(1),
            Pane::List => {
                self.list_sel = self.visible().len().saturating_sub(1);
                return;
            }
        }
        self.sync_active();
    }

    /// `h` / `Left`: focus the pane to the left, skipping Groups when empty.
    fn focus_left(&mut self) {
        self.focus = match self.focus {
            Pane::List if self.groups.is_empty() => Pane::Views,
            Pane::List => Pane::Groups,
            Pane::Groups | Pane::Views => Pane::Views,
        };
        self.sync_active();
    }

    /// `l` / `Right`: focus the pane to the right, skipping Groups when empty.
    fn focus_right(&mut self) {
        self.focus = match self.focus {
            Pane::Views if self.groups.is_empty() => Pane::List,
            Pane::Views => Pane::Groups,
            Pane::Groups | Pane::List => Pane::List,
        };
        self.sync_active();
    }

    /// `Tab`: cycle Views → Groups → List → Views, skipping an empty Groups.
    fn focus_next(&mut self) {
        self.focus = match self.focus {
            Pane::Views if self.groups.is_empty() => Pane::List,
            Pane::Views => Pane::Groups,
            Pane::Groups => Pane::List,
            Pane::List => Pane::Views,
        };
        self.sync_active();
    }

    /// `[` / `]`: jump straight to a sidebar section. Groups is a no-op when it
    /// is empty — there is nothing to land on.
    fn jump_section(&mut self, pane: Pane) {
        if pane == Pane::Groups && self.groups.is_empty() {
            return;
        }
        self.focus = pane;
        self.sync_active();
    }

    /// Point the list at whatever sidebar entry the cursor is on, and send the
    /// list cursor to the top. A no-op while the list itself is focused, so
    /// moving into and back out of the list keeps the project you were on.
    fn sync_active(&mut self) {
        let next = match self.focus {
            Pane::Views => self.views.get(self.views_sel).cloned(),
            Pane::Groups => self.groups.get(self.groups_sel).map(|group| View::Group {
                id: group.id.clone(),
            }),
            Pane::List => None,
        };
        if let Some(view) = next {
            self.active = view;
            self.list_sel = 0;
        }
    }
}

/// The plain-language name of a view, for the sidebar and the status line.
pub(crate) fn view_label(view: &View, groups: &[Group]) -> String {
    match view {
        View::All => "All".to_string(),
        View::Favorites => "Favourites".to_string(),
        View::Ungrouped => "Ungrouped".to_string(),
        View::Bin => "Bin".to_string(),
        View::Group { id } => groups
            .iter()
            .find(|g| &g.id == id)
            .map(|g| g.name.clone())
            .unwrap_or_else(|| "Unknown group".to_string()),
    }
}

/// Move a cursor down one, stopping at the last item; `0` for an empty list.
fn step_down(index: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (index + 1).min(len - 1)
    }
}
