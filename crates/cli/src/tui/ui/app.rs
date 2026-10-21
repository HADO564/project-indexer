//! Drawing the read-only panes: the sidebar (fixed views, then groups), the
//! project list, and the selected project's detail, with a status line and a
//! hint line beneath.
//!
//! Icons and mark colours come from the shared `appearance` module, exactly as
//! `dexily list` and `show` draw them — the project's own colour, else its
//! group's, with the glyph from `config icons`. `NO_COLOR` drops the colour but
//! keeps the glyphs, and the cursor and focus are drawn with modifiers
//! (reversed, bold, dim) so they still read when colour is off.
//!
//! On a terminal that can show images, a project's custom icon is drawn as one
//! at the top of the detail pane (`tui/images.rs`); everywhere else, and on any
//! other terminal, it is the folder glyph, as in `list`.

use ratatui::layout::{Constraint, Flex, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use ratatui_image::Image;

use indexer_core::domain::views::View;
use indexer_core::domain::{Group, Project};

use crate::appearance::{self, IconStyle};
use crate::tui::app::{view_label, App, Mode, Naming, Pane};
use crate::tui::images::{IconImages, ICON_COLS, ICON_ROWS};

/// Draws the browser. `images` is `None` on a terminal that can't show images,
/// and every icon is then a glyph.
pub fn draw(frame: &mut Frame, app: &App, images: Option<&mut IconImages>) {
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
    draw_detail(frame, detail, app, images);
    draw_status(frame, status, app);
    frame.render_widget(Paragraph::new(hint_line(app)), hint);

    // Overlays sit on top of the panes, over the body area.
    match app.mode() {
        Mode::Menu(selected) => draw_menu(frame, body, app, *selected),
        Mode::Help => draw_help(frame, body),
        Mode::Scan(buffer) => draw_input(
            frame,
            body,
            "Scan a folder",
            buffer,
            ". = current folder · Tab = browse · Enter scan · Esc cancel",
        ),
        Mode::Name { naming, buffer } => {
            let title = match naming {
                Naming::NewGroup => "New group".to_string(),
                Naming::RenameGroup { name } => format!("Rename \"{name}\""),
            };
            draw_input(frame, body, &title, buffer, "Enter save · Esc cancel");
        }
        Mode::Browse { path, entries, sel } => draw_browse(frame, body, path, entries, *sel),
        _ => {}
    }
}

/// The folder browser: a centred box titled with the open directory, listing
/// its subfolders with the highlighted one selectable.
fn draw_browse(
    frame: &mut Frame,
    area: Rect,
    path: &std::path::Path,
    entries: &[String],
    sel: usize,
) {
    let popup = centered(60, 16, area);
    frame.render_widget(Clear, popup);
    let title = format!("Scan — {}", path.display());
    let items: Vec<ListItem> = if entries.is_empty() {
        vec![ListItem::new(Span::styled(
            "(no subfolders)",
            Style::new().add_modifier(Modifier::DIM),
        ))]
    } else {
        entries
            .iter()
            .map(|name| ListItem::new(format!("{name}/")))
            .collect()
    };
    let list = List::new(items)
        .block(Block::bordered().title(title))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ListState::default();
    if !entries.is_empty() {
        state.select(Some(sel));
    }
    frame.render_stateful_widget(list, popup, &mut state);
}

/// A one-line dialog — the scan folder, a group's name: a centred box titled
/// with what it asks for, the text typed so far with a cursor, and its keys.
fn draw_input(frame: &mut Frame, area: Rect, title: &str, buffer: &str, keys: &str) {
    let popup = centered(54, 4, area);
    frame.render_widget(Clear, popup);
    let block = Block::bordered().title(title.to_string());
    let inner = block.inner(popup);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("> {buffer}")),
            Line::from(Span::styled(
                keys.to_string(),
                Style::new().add_modifier(Modifier::DIM),
            )),
        ])
        .block(block),
        popup,
    );
    let x =
        (inner.x + 2 + buffer.chars().count() as u16).min(inner.x + inner.width.saturating_sub(1));
    frame.set_cursor_position(Position::new(x, inner.y));
}

/// The which-key action menu: a small centred popup of the selection's actions.
fn draw_menu(frame: &mut Frame, area: Rect, app: &App, selected: usize) {
    let items = app.menu_items();
    let rows: Vec<ListItem> = items
        .iter()
        .map(|item| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {}  ", item.key),
                    Style::new().add_modifier(Modifier::BOLD),
                ),
                Span::raw(item.label.to_string()),
            ]))
        })
        .collect();
    let popup = centered(28, rows.len() as u16 + 2, area);
    frame.render_widget(Clear, popup);
    let mut state = ListState::default();
    state.select(Some(selected));
    let list = List::new(rows)
        .block(Block::bordered().title("Actions"))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, popup, &mut state);
}

/// The `?` help overlay: every key and what it does, a centred popup.
fn draw_help(frame: &mut Frame, area: Rect) {
    let rows = [
        ("j / k, ↑ / ↓", "move the selection"),
        ("g / G", "top / bottom"),
        ("Tab, h / l", "switch pane"),
        ("[ / ]", "sidebar section"),
        ("/", "search / filter the list"),
        ("o / f", "open / favourite"),
        ("e / E", "edit / edit all fields"),
        ("d", "untrack"),
        ("r / x", "restore / purge (bin)"),
        ("n / e / d", "in Groups: new / rename / delete"),
        ("n", "add a project here"),
        ("s", "scan a folder"),
        ("Space", "action menu"),
        (":", "command line"),
        ("?", "this help"),
        ("q, Ctrl+C", "quit"),
    ];
    let lines: Vec<Line> = rows
        .iter()
        .map(|(keys, action)| {
            Line::from(vec![
                Span::styled(
                    format!(" {keys:<14}"),
                    Style::new().add_modifier(Modifier::BOLD),
                ),
                Span::raw((*action).to_string()),
            ])
        })
        .collect();
    let popup = centered(50, lines.len() as u16 + 2, area);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("Keys")),
        popup,
    );
}

/// A fixed-size rectangle centred in `area` (clamped to it if larger).
fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let [row] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    let [cell] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(row);
    cell
}

fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    // The fixed views sit above the groups: a fixed block tall enough for them
    // plus its border, and the groups take whatever is left and scroll.
    let views_height = app.views().len() as u16 + 2;
    let [views_area, groups_area] =
        Layout::vertical([Constraint::Length(views_height), Constraint::Fill(1)]).areas(area);

    // The fixed views are not projects or groups, so they carry no icon or
    // colour — only their name and count.
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

    // Ungrouped — the "no group" bucket — leads the Groups pane as a plain
    // entry (it is not a real group, so no icon or colour), then the groups.
    let mut group_items = vec![ListItem::new(entry_line(
        "Ungrouped",
        app.count_for(&View::Ungrouped),
    ))];
    group_items.extend(app.groups().iter().map(|group| {
        let count = app.count_for(&View::Group {
            id: group.id.clone(),
        });
        ListItem::new(group_line(group, count, app.icons(), app.color_enabled()))
    }));
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
    let focused = app.focus() == Pane::List;
    let title = format!("PROJECTS — {}", view_label(app.active(), app.groups()));

    // An empty view says so rather than leaving a blank pane with nothing to
    // tell a scan that found nothing from a bin that is simply empty.
    if visible.is_empty() {
        let mut block = Block::bordered().title(title);
        if focused {
            block = block.border_style(Style::new().add_modifier(Modifier::BOLD));
        }
        // A filter that matches nothing reads differently from a view that is
        // simply empty.
        let note_text = if app.search_query().is_empty() {
            "  no projects in this view".to_string()
        } else {
            format!("  no projects match \"{}\"", app.search_query())
        };
        let note = Paragraph::new(Line::from(Span::styled(
            note_text,
            Style::new().add_modifier(Modifier::DIM),
        )))
        .block(block);
        frame.render_widget(note, area);
        return;
    }

    let items: Vec<ListItem> = visible
        .iter()
        .map(|project| {
            let mark = appearance::mark_rgb(project.color.as_deref(), app.group_color_of(project));
            let style = mark_style(mark, app.color_enabled());
            ListItem::new(project_line(
                project,
                app.icons(),
                style,
                app.color_enabled(),
            ))
        })
        .collect();
    render_list(frame, area, &title, items, app.list_sel(), focused);
}

fn draw_detail(frame: &mut Frame, area: Rect, app: &App, images: Option<&mut IconImages>) {
    let block = Block::bordered().title("DETAIL");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(project) = app.selected_project() else {
        let none = Span::styled(
            "no project selected",
            Style::new().add_modifier(Modifier::DIM),
        );
        frame.render_widget(Paragraph::new(Line::from(none)), inner);
        return;
    };

    // A custom icon as an image, above the details — when the terminal can
    // show one, the icon renders, and the pane has room for it and a few rows
    // beneath. Otherwise the title carries the glyph, as it always has.
    let roomy = inner.width >= ICON_COLS && inner.height >= ICON_ROWS + 4;
    let image = images
        .filter(|_| roomy)
        .and_then(|images| images.get(project.icon.as_deref()?));
    let text = match image {
        Some(protocol) => {
            let [icon, _gap, text] = Layout::vertical([
                Constraint::Length(ICON_ROWS),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .areas(inner);
            let icon = Rect {
                width: ICON_COLS,
                ..icon
            };
            frame.render_widget(Image::new(protocol), icon);
            text
        }
        None => inner,
    };

    let mark = appearance::mark_rgb(project.color.as_deref(), app.group_color_of(project));
    let lines = detail_lines(
        project,
        app.groups(),
        app.icons(),
        mark_style(mark, app.color_enabled()),
        text == inner,
    );
    // Wrapped, not clipped: the pane is narrow, and a description or a line
    // of notes is often wider than it.
    let detail = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(detail, text);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    match app.mode() {
        // Typing a command: the line is the input, with a cursor at its end.
        Mode::Command(buffer) => {
            frame.render_widget(Paragraph::new(format!(":{buffer}")), area);
            let x = area.x + 1 + buffer.chars().count() as u16;
            let x = x.min(area.x + area.width.saturating_sub(1));
            frame.set_cursor_position(Position::new(x, area.y));
        }
        // Typing a `/` search: the query, a cursor at its end, same as a command.
        Mode::Search(buffer) => {
            frame.render_widget(Paragraph::new(format!("/{buffer}")), area);
            let x = area.x + 1 + buffer.chars().count() as u16;
            let x = x.min(area.x + area.width.saturating_sub(1));
            frame.set_cursor_position(Position::new(x, area.y));
        }
        // A destructive command is waiting for y/N: show the prompt, bold.
        Mode::Confirm { prompt, .. } => {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!(" {prompt}"),
                    Style::new().add_modifier(Modifier::BOLD),
                ))),
                area,
            );
        }
        // Browsing (or an overlay is up): the last command's result, else a
        // count summary.
        _ => {
            let text = if app.message().is_empty() {
                let count = app.visible().len();
                let noun = if count == 1 { "project" } else { "projects" };
                let view = view_label(app.active(), app.groups());
                // A committed filter is named after the view, so it is clear why
                // the count is smaller than the sidebar's.
                if app.search_query().is_empty() {
                    format!(" {count} {noun} · {view}")
                } else {
                    format!(" {count} {noun} · {view} · /{}", app.search_query())
                }
            } else {
                format!(" {}", app.message())
            };
            frame.render_widget(Paragraph::new(text), area);
        }
    }
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

/// A plain sidebar entry — a fixed view — with its count dimmed.
fn entry_line(label: &str, count: usize) -> Line<'static> {
    Line::from(vec![
        Span::raw(label.to_string()),
        Span::styled(
            format!("  ({count})"),
            Style::new().add_modifier(Modifier::DIM),
        ),
    ])
}

/// A group row: its icon (if icons are on), its name in the group's colour, and
/// its count.
fn group_line(group: &Group, count: usize, icons: IconStyle, color_enabled: bool) -> Line<'static> {
    let mut spans = Vec::new();
    if let Some(glyph) = appearance::glyph(&group.icon, icons) {
        spans.push(Span::raw(format!("{glyph} ")));
    }
    spans.push(Span::styled(
        group.name.clone(),
        mark_style(appearance::rgb(&group.color), color_enabled),
    ));
    spans.push(Span::styled(
        format!("  ({count})"),
        Style::new().add_modifier(Modifier::DIM),
    ));
    Line::from(spans)
}

/// A project row: its icon, a gold star when favourited, and its name in its
/// mark colour, with its trackers dimmed after.
fn project_line(
    project: &Project,
    icons: IconStyle,
    name_style: Style,
    color_enabled: bool,
) -> Line<'static> {
    let mut spans = Vec::new();
    if let Some(glyph) = appearance::glyph(project.icon.as_deref().unwrap_or("folder"), icons) {
        spans.push(Span::styled(format!("{glyph} "), name_style));
    }
    // A two-column slot either way, so names line up whether or not a project
    // is favourited.
    if project.favorite {
        spans.push(Span::styled(
            "★ ",
            mark_style(appearance::rgb("gold"), color_enabled),
        ));
    } else {
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled(project.name.clone(), name_style));
    let trackers = tracker_labels(project);
    if !trackers.is_empty() {
        spans.push(Span::styled(
            format!("   {trackers}"),
            Style::new().add_modifier(Modifier::DIM),
        ));
    }
    Line::from(spans)
}

/// The detail pane's text. `glyph` is whether the title leads with the icon's
/// glyph — not when the icon is drawn as an image above it.
fn detail_lines(
    project: &Project,
    groups: &[Group],
    icons: IconStyle,
    name_style: Style,
    glyph: bool,
) -> Vec<Line<'static>> {
    let bold = name_style.add_modifier(Modifier::BOLD);
    let mut title = Vec::new();
    let icon = project.icon.as_deref().unwrap_or("folder");
    if let Some(glyph) = appearance::glyph(icon, icons).filter(|_| glyph) {
        title.push(Span::styled(format!("{glyph} "), bold));
    }
    title.push(Span::styled(project.name.clone(), bold));
    let mut lines = vec![Line::from(title)];

    lines.push(row("directory", &project.directory));
    lines.push(row("id", short_id(&project.id)));
    if !project.description.is_empty() {
        lines.push(row("description", &project.description));
    }
    if let Some(id) = &project.group_id {
        if let Some(group) = groups.iter().find(|g| &g.id == id) {
            lines.push(row("group", &group.name));
        }
    }
    if !project.tags.is_empty() {
        lines.push(row("tags", &project.tags.join(", ")));
    }
    let properties = project
        .properties
        .iter()
        .map(|(key, value)| format!("{key} = {value}"));
    lines.extend(rows("properties", properties));
    if let Some(notes) = &project.notes {
        lines.extend(rows("notes", notes.lines().map(str::to_string)));
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

/// A row per value, labelled on the first and blank on the rest, so the values
/// line up under one another — as `show` prints properties. No values, no rows:
/// a field the project does not have takes no space.
fn rows(label: &str, values: impl Iterator<Item = String>) -> Vec<Line<'static>> {
    values
        .enumerate()
        .map(|(i, value)| row(if i == 0 { label } else { "" }, &value))
        .collect()
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

/// A foreground style from a mark colour, or plain when there is no colour or
/// `NO_COLOR` is set.
fn mark_style(rgb: Option<(u8, u8, u8)>, color_enabled: bool) -> Style {
    match rgb {
        Some((r, g, b)) if color_enabled => Style::new().fg(Color::Rgb(r, g, b)),
        _ => Style::new(),
    }
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

fn hint_line(app: &App) -> Line<'static> {
    let keys = match app.mode() {
        Mode::Command(_) => " Enter run · Esc cancel",
        Mode::Search(_) => " type to filter · Enter keep · Esc clear",
        Mode::Scan(_) => " type a folder · Tab browse · Enter scan · Esc cancel",
        Mode::Browse { .. } => " j/k move · Enter open · h up · s scan here · Esc cancel",
        Mode::Menu(_) => " j/k move · Enter run · Esc close",
        Mode::Confirm { .. } => " y confirm · any other key cancels",
        Mode::Name { .. } => " type a name · Enter save · Esc cancel",
        Mode::Help => " any key closes",
        // In the Groups pane the letters act on groups, so say which ones.
        Mode::Normal if app.focus() == Pane::Groups => {
            if app.selected_group().is_some() {
                " j/k move · n new group · e rename · d delete · Space menu · : cmd · ? help · q quit"
            } else {
                " j/k move · n new group · Space menu · : cmd · ? help · q quit"
            }
        }
        Mode::Normal => {
            " j/k move · / search · o/f/e/E/d act · s scan · Space menu · : cmd · ? help · q quit"
        }
    };
    Line::from(Span::styled(keys, Style::new().add_modifier(Modifier::DIM)))
}
