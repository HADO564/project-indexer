//! Drawing the read-only panes: the sidebar (fixed views, then groups), the
//! project list, and the selected project's detail, with a status line and a
//! hint line beneath. Styled with modifiers alone — reversed for the cursor,
//! bold for the focused pane's border, dim for labels — so it reads the same
//! under `NO_COLOR`, the way the edit form does.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use indexer_core::domain::views::View;
use indexer_core::domain::{Group, Project};

use crate::tui::app::{view_label, App, Pane};

pub fn draw(frame: &mut Frame, app: &App) {
    let [body, status, hint] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let [sidebar, list, detail] = Layout::horizontal([
        Constraint::Length(24),
        Constraint::Fill(1),
        Constraint::Length(36),
    ])
    .areas(body);

    draw_sidebar(frame, sidebar, app);
    draw_list(frame, list, app);
    draw_detail(frame, detail, app);
    draw_status(frame, status, app);
    frame.render_widget(Paragraph::new(hint_line()), hint);
}

fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    // The four fixed views sit above the groups: a fixed block tall enough for
    // them plus its border, and the groups take whatever is left and scroll.
    let views_height = app.views().len() as u16 + 2;
    let [views_area, groups_area] =
        Layout::vertical([Constraint::Length(views_height), Constraint::Fill(1)]).areas(area);

    let view_items: Vec<ListItem> = app
        .views()
        .iter()
        .map(|view| {
            ListItem::new(entry_line(
                &view_label(view, app.groups()),
                app.count_for(view),
            ))
        })
        .collect();
    render_list(
        frame,
        views_area,
        "VIEWS",
        view_items,
        app.views_sel(),
        app.focus() == Pane::Views,
    );

    let group_items: Vec<ListItem> = app
        .groups()
        .iter()
        .map(|group| {
            let count = app.count_for(&View::Group {
                id: group.id.clone(),
            });
            ListItem::new(entry_line(&group.name, count))
        })
        .collect();
    render_list(
        frame,
        groups_area,
        "GROUPS",
        group_items,
        app.groups_sel(),
        app.focus() == Pane::Groups,
    );
}

fn draw_list(frame: &mut Frame, area: Rect, app: &App) {
    let visible = app.visible();
    let items: Vec<ListItem> = visible
        .iter()
        .map(|project| ListItem::new(project_line(project)))
        .collect();
    let title = format!("PROJECTS — {}", view_label(app.active(), app.groups()));
    render_list(
        frame,
        area,
        &title,
        items,
        app.list_sel(),
        app.focus() == Pane::List,
    );
}

fn draw_detail(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::bordered().title("DETAIL");
    let lines = match app.selected_project() {
        None => vec![Line::from(Span::styled(
            "no project selected",
            Style::new().add_modifier(Modifier::DIM),
        ))],
        Some(project) => detail_lines(project, app.groups()),
    };
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let count = app.visible().len();
    let noun = if count == 1 { "project" } else { "projects" };
    let text = format!(
        " {count} {noun} · {}",
        view_label(app.active(), app.groups())
    );
    frame.render_widget(Paragraph::new(text), area);
}

/// A bordered list with its selected row reversed when the pane is focused, and
/// only dimmed when it is not — so the cursor is always locatable, but the
/// focused pane is the obvious one.
fn render_list(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: Vec<ListItem>,
    selected: usize,
    focused: bool,
) {
    let mut block = Block::bordered().title(title.to_string());
    if focused {
        block = block.border_style(Style::new().add_modifier(Modifier::BOLD));
    }
    let highlight = if focused {
        Style::new().add_modifier(Modifier::REVERSED)
    } else {
        Style::new().add_modifier(Modifier::DIM)
    };
    let list = List::new(items)
        .block(block)
        .highlight_style(highlight)
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn entry_line(label: &str, count: usize) -> Line<'static> {
    Line::from(vec![
        Span::raw(label.to_string()),
        Span::styled(
            format!("  ({count})"),
            Style::new().add_modifier(Modifier::DIM),
        ),
    ])
}

fn project_line(project: &Project) -> Line<'static> {
    let star = if project.favorite { "★ " } else { "  " };
    let trackers = tracker_labels(project);
    Line::from(vec![
        Span::styled(star.to_string(), Style::new().add_modifier(Modifier::DIM)),
        Span::raw(project.name.clone()),
        Span::styled(
            if trackers.is_empty() {
                String::new()
            } else {
                format!("   {trackers}")
            },
            Style::new().add_modifier(Modifier::DIM),
        ),
    ])
}

fn detail_lines(project: &Project, groups: &[Group]) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::styled(
        project.name.clone(),
        Style::new().add_modifier(Modifier::BOLD),
    ))];
    lines.push(row("directory", &project.directory));
    lines.push(row("id", short_id(&project.id)));
    if let Some(id) = &project.group_id {
        if let Some(group) = groups.iter().find(|g| &g.id == id) {
            lines.push(row("group", &group.name));
        }
    }
    if !project.tags.is_empty() {
        lines.push(row("tags", &project.tags.join(", ")));
    }
    let trackers = tracker_labels(project);
    if !trackers.is_empty() {
        lines.push(row("trackers", &trackers));
    }
    if project.favorite {
        lines.push(row("favourite", "yes"));
    }
    let last_opened = project
        .last_opened_at
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "never".to_string());
    lines.push(row("last opened", &last_opened));
    lines
}

fn row(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<12}"),
            Style::new().add_modifier(Modifier::DIM),
        ),
        Span::raw(value.to_string()),
    ])
}

fn tracker_labels(project: &Project) -> String {
    project
        .trackers
        .iter()
        .map(|tracker| tracker.kind())
        .collect::<Vec<_>>()
        .join(", ")
}

fn short_id(id: &str) -> &str {
    &id[..8.min(id.len())]
}

fn hint_line() -> Line<'static> {
    Line::from(Span::styled(
        " j/k move · Tab/h/l pane · [ ] section · q quit",
        Style::new().add_modifier(Modifier::DIM),
    ))
}
