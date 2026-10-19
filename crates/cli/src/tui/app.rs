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
    /// The which-key action menu is open, highlighting this item.
    Menu(usize),
    /// The `?` help overlay is open.
    Help,
    /// A destructive command is waiting for `y`/`n`: the prompt to show, and the
    /// `:` line to run if confirmed.
    Confirm {
        prompt: String,
        pending: String,
    },
}

/// Something the event loop must do that `App` cannot, having no `Context`.
/// Returned by [`App::handle`] so the loop runs the command and reloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    /// Run this `:` line, first gating it behind a `y/N` prompt if it is
    /// destructive (see `cmdline::needs_confirm`).
    Run(String),
    /// Run this `:` line now — the user has already confirmed it.
    RunConfirmed(String),
}

/// One row of the action menu: the shortcut key, its label, and the `:` command
/// it runs on the selected project. Shortcuts and the menu share this list, so
/// they can never name different commands for the same key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MenuItem {
    pub(crate) key: char,
    pub(crate) label: &'static str,
    pub(crate) command: String,
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
    /// The sidebar section `Tab` returns to from the list — Views or Groups.
    sidebar_focus: Pane,
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
        // Ungrouped lives in the Groups pane (it is the "no group" bucket), not
        // among the fixed views.
        let views = vec![View::All, View::Favorites, View::Bin];
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
            sidebar_focus: Pane::Views,
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
        self.groups_sel = self.groups_sel.min(self.group_pane_len().saturating_sub(1));
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

    /// Rows in the Groups pane: the Ungrouped bucket, then the groups.
    pub(crate) fn group_pane_len(&self) -> usize {
        1 + self.groups.len()
    }

    /// The view a Groups-pane row maps to: row 0 is Ungrouped, the rest are the
    /// groups in order.
    fn group_pane_view(&self, index: usize) -> Option<View> {
        if index == 0 {
            Some(View::Ungrouped)
        } else {
            self.groups.get(index - 1).map(|group| View::Group {
                id: group.id.clone(),
            })
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

    /// The actions that apply to the selected project — the rows of the `Space`
    /// menu, and the targets of the letter shortcuts. Each runs a `:` command
    /// on the project by id, so they go through the same path as a typed line.
    /// Empty when nothing is selected.
    pub(crate) fn menu_items(&self) -> Vec<MenuItem> {
        let Some(project) = self.selected_project() else {
            return Vec::new();
        };
        let id = &project.id;
        if matches!(self.active, View::Bin) {
            vec![
                MenuItem {
                    key: 'r',
                    label: "Restore",
                    command: format!("restore {id}"),
                },
                MenuItem {
                    key: 'x',
                    label: "Purge",
                    command: format!("purge {id}"),
                },
            ]
        } else {
            let (fav_key, fav_label, fav_cmd) = if project.favorite {
                ('f', "Unfavourite", "unfavorite")
            } else {
                ('f', "Favourite", "favorite")
            };
            vec![
                MenuItem {
                    key: 'o',
                    label: "Open",
                    command: format!("open {id}"),
                },
                MenuItem {
                    key: fav_key,
                    label: fav_label,
                    command: format!("{fav_cmd} {id}"),
                },
                MenuItem {
                    key: 'e',
                    label: "Edit",
                    command: format!("edit {id}"),
                },
                MenuItem {
                    key: 'd',
                    label: "Untrack",
                    command: format!("untrack {id}"),
                },
            ]
        }
    }

    /// The command a letter shortcut runs, if one applies to the selection.
    fn shortcut_command(&self, key: char) -> Option<String> {
        self.menu_items()
            .into_iter()
            .find(|item| item.key == key)
            .map(|item| item.command)
    }

    // --- key handling ---

    /// Applies one key press. In the browser it moves the selection or the
    /// focus, or opens the `:` line; while the `:` line is open it edits that
    /// line. Any [`Action`] returned is for the event loop to carry out, since
    /// running a command needs the `Context` this does not hold.
    pub(crate) fn handle(&mut self, key: KeyEvent) -> Option<Action> {
        match self.mode {
            Mode::Normal => self.handle_normal(key),
            Mode::Command(_) => self.handle_command(key),
            Mode::Menu(_) => self.handle_menu(key),
            Mode::Confirm { .. } => self.handle_confirm(key),
            Mode::Help => {
                // Any key dismisses the help overlay.
                self.mode = Mode::Normal;
                None
            }
        }
    }

    /// Enter the `y/N` gate for a destructive command; the event loop calls this
    /// instead of running the command straight away.
    pub(crate) fn begin_confirm(&mut self, prompt: String, pending: String) {
        self.mode = Mode::Confirm { prompt, pending };
    }

    /// The confirmation gate: only `y` (or `Y`) runs the pending command;
    /// anything else — `n`, `Esc`, a stray key — cancels, so nothing is deleted
    /// by accident.
    fn handle_confirm(&mut self, key: KeyEvent) -> Option<Action> {
        let pending = match &self.mode {
            Mode::Confirm { pending, .. } => pending.clone(),
            _ => return None,
        };
        self.mode = Mode::Normal;
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(Action::RunConfirmed(pending)),
            _ => {
                self.message = "cancelled".to_string();
                None
            }
        }
    }

    /// Browser keys: movement, focus, quit, `:` to open the command line, a
    /// letter shortcut to run a command on the selection, `Space` for the
    /// action menu, and `?` for help.
    fn handle_normal(&mut self, key: KeyEvent) -> Option<Action> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return None;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char(':') => self.mode = Mode::Command(String::new()),
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Char(' ') => {
                if !self.menu_items().is_empty() {
                    self.mode = Mode::Menu(0);
                }
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('g') => self.move_top(),
            KeyCode::Char('G') => self.move_bottom(),
            KeyCode::Char('h') | KeyCode::Left => self.focus_left(),
            KeyCode::Char('l') | KeyCode::Right => self.focus_right(),
            KeyCode::Tab => self.focus_next(),
            KeyCode::Char('[') => self.jump_section(Pane::Views),
            KeyCode::Char(']') => self.jump_section(Pane::Groups),
            // A letter shortcut runs its command on the selected project.
            KeyCode::Char(c) => {
                if let Some(command) = self.shortcut_command(c) {
                    return Some(Action::Run(command));
                }
            }
            _ => {}
        }
        None
    }

    /// The action menu: `j`/`k` move, `Enter` runs the highlighted item, a
    /// letter runs its item directly, `Esc`/`Space` close it.
    fn handle_menu(&mut self, key: KeyEvent) -> Option<Action> {
        let Mode::Menu(selected) = self.mode else {
            return None;
        };
        let items = self.menu_items();
        match key.code {
            KeyCode::Esc | KeyCode::Char(' ') => self.mode = Mode::Normal,
            KeyCode::Char('j') | KeyCode::Down => {
                self.mode = Mode::Menu((selected + 1).min(items.len().saturating_sub(1)));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.mode = Mode::Menu(selected.saturating_sub(1));
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                return items
                    .get(selected)
                    .map(|item| Action::Run(item.command.clone()));
            }
            KeyCode::Char(c) => {
                if let Some(item) = items.into_iter().find(|item| item.key == c) {
                    self.mode = Mode::Normal;
                    return Some(Action::Run(item.command));
                }
            }
            _ => {}
        }
        None
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
            Pane::Groups => self.groups_sel = step_down(self.groups_sel, self.group_pane_len()),
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
            Pane::Groups => self.groups_sel = self.group_pane_len().saturating_sub(1),
            Pane::List => {
                self.list_sel = self.visible().len().saturating_sub(1);
                return;
            }
        }
        self.sync_active();
    }

    /// `h` / `Left`: focus the pane to the left (List → Groups → Views). The
    /// Groups pane always holds at least Ungrouped, so it is never skipped.
    fn focus_left(&mut self) {
        let next = match self.focus {
            Pane::List => Pane::Groups,
            Pane::Groups | Pane::Views => Pane::Views,
        };
        self.set_focus(next);
    }

    /// `l` / `Right`: focus the pane to the right (Views → Groups → List).
    fn focus_right(&mut self) {
        let next = match self.focus {
            Pane::Views => Pane::Groups,
            Pane::Groups | Pane::List => Pane::List,
        };
        self.set_focus(next);
    }

    /// `Tab`: toggle between the sidebar and the list, so you reach the projects
    /// from the views in one key without passing through Groups. From the list
    /// it returns to whichever sidebar section you were last on.
    fn focus_next(&mut self) {
        let next = match self.focus {
            Pane::Views | Pane::Groups => Pane::List,
            Pane::List => self.sidebar_focus,
        };
        self.set_focus(next);
    }

    /// `[` / `]`: jump straight to a sidebar section.
    fn jump_section(&mut self, pane: Pane) {
        self.set_focus(pane);
    }

    /// Change focus, remember the sidebar section for `Tab`, and point the list
    /// at the newly focused entry.
    fn set_focus(&mut self, pane: Pane) {
        self.focus = pane;
        if matches!(pane, Pane::Views | Pane::Groups) {
            self.sidebar_focus = pane;
        }
        self.sync_active();
    }

    /// Point the list at whatever sidebar entry the cursor is on, and send the
    /// list cursor to the top. A no-op while the list itself is focused, so
    /// moving into and back out of the list keeps the project you were on.
    fn sync_active(&mut self) {
        let next = match self.focus {
            Pane::Views => self.views.get(self.views_sel).cloned(),
            Pane::Groups => self.group_pane_view(self.groups_sel),
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
