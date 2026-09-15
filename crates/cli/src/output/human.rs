//! The default, human-readable rendering.

use std::io::{IsTerminal, Write};
use std::path::Path;

use indexer_core::domain::Project;

use crate::commands::Outcome;
use crate::output::color::Color;
use crate::output::Colors;

pub fn write(out: &mut impl Write, outcome: &Outcome, colors: Colors) -> anyhow::Result<()> {
    let terminal = std::io::stdout().is_terminal();
    let color = terminal && std::env::var_os("NO_COLOR").is_none();
    match outcome {
        Outcome::Projects(projects) => {
            if projects.is_empty() {
                eprintln!("indexer: no projects tracked yet");
                return Ok(());
            }
            let projects: Vec<&Project> = projects.iter().collect();
            let style = TableStyle {
                folder_color: color.then_some(colors.folder),
                header_color: color.then_some(colors.header),
                width: if terminal { terminal_width() } else { None },
            };
            write!(out, "{}", project_table(&projects, &style))?;
        }
        Outcome::Project(project) => {
            writeln!(out, "{}", project.name)?;
            writeln!(out, "  directory  {}", project.directory)?;
            writeln!(out, "  id         {}", project.id)?;
        }
        Outcome::Color { color: chosen, .. } => {
            let name = chosen.name();
            if color {
                writeln!(out, "{}", chosen.paint(&name))?;
            } else {
                writeln!(out, "{name}")?;
            }
        }
        Outcome::Done => {}
    }
    Ok(())
}

fn terminal_width() -> Option<usize> {
    terminal_size::terminal_size().map(|(terminal_size::Width(width), _)| usize::from(width))
}

/// How [`project_table`] is drawn. The default is plain — no colour and no
/// terminal width, so the table is only as wide as its cells and not centred —
/// which suits anything that isn't a terminal, like an error message or a pipe.
#[derive(Debug, Default, Clone, Copy)]
pub struct TableStyle {
    pub folder_color: Option<Color>,
    pub header_color: Option<Color>,
    /// The terminal's width in columns, when known.
    pub width: Option<usize>,
}

/// With a known terminal width, a narrower table grows to this share of it.
const MIN_WIDTH_PERCENT: usize = 70;

const HEADERS: [&str; 4] = ["NAME", "DIRECTORY", "TRACKERS", "LAST OPENED"];

/// A bordered table of projects — `list`'s view, and `show`'s when a query
/// matches several — with the columns NAME, DIRECTORY, TRACKERS and LAST
/// OPENED.
///
/// DIRECTORY is only `parent/folder`, the part `show parent/folder` takes
/// back. Given a terminal width, the table grows to at least
/// [`MIN_WIDTH_PERCENT`] of it, sharing the extra space between the columns,
/// and is centred. Padding is always worked out on the plain text, so colour
/// codes never push a column out of line.
pub fn project_table(projects: &[&Project], style: &TableStyle) -> String {
    let rows: Vec<Row> = projects.iter().map(|p| Row::of(p)).collect();

    let mut widths = HEADERS.map(text_width);
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row.cells()) {
            *width = (*width).max(text_width(&cell));
        }
    }

    let mut indent = String::new();
    if let Some(terminal) = style.width {
        stretch(&mut widths, (terminal * MIN_WIDTH_PERCENT).div_ceil(100));
        let table = table_width(&widths);
        if terminal > table {
            indent = " ".repeat((terminal - table) / 2);
        }
    }

    let header = HEADERS.map(|h| Cell {
        plain: h.to_string(),
        shown: paint(style.header_color, h),
    });

    let mut lines = vec![
        rule(&widths, ['╭', '┬', '╮']),
        cells_line(&header, &widths),
        rule(&widths, ['├', '┼', '┤']),
    ];
    for row in &rows {
        let [name, directory, trackers, last_opened] = row.cells();
        let cells = [
            Cell::plain(name),
            Cell {
                shown: format!("{}{}", row.parent, paint(style.folder_color, &row.folder)),
                plain: directory,
            },
            Cell::plain(trackers),
            Cell::plain(last_opened),
        ];
        lines.push(cells_line(&cells, &widths));
    }
    lines.push(rule(&widths, ['╰', '┴', '╯']));

    lines
        .iter()
        .map(|line| format!("{indent}{line}\n"))
        .collect()
}

/// One project's cells, as plain text.
struct Row {
    name: String,
    /// The parent folder with its trailing `/` (`"work/"`), or empty at the
    /// root of the filesystem.
    parent: String,
    folder: String,
    /// Every tracker's kind, joined with `, `; `-` when there are none.
    trackers: String,
    /// `YYYY-MM-DD`, or `never`.
    last_opened: String,
}

impl Row {
    fn of(project: &Project) -> Self {
        let path = Path::new(&project.directory);
        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|f| format!("{}/", f.to_string_lossy()))
            .unwrap_or_default();
        let folder = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| project.directory.clone());

        let mut trackers = project
            .trackers
            .iter()
            .map(|t| t.kind())
            .collect::<Vec<_>>()
            .join(", ");
        if trackers.is_empty() {
            trackers = "-".to_string();
        }

        let last_opened = match project.last_opened_at {
            Some(date) => date.format("%Y-%m-%d").to_string(),
            None => "never".to_string(),
        };

        Row {
            name: project.name.clone(),
            parent,
            folder,
            trackers,
            last_opened,
        }
    }

    /// The four cells in column order.
    fn cells(&self) -> [String; 4] {
        [
            self.name.clone(),
            format!("{}{}", self.parent, self.folder),
            self.trackers.clone(),
            self.last_opened.clone(),
        ]
    }
}

/// A cell's text as measured (`plain`) and as printed (`shown`, which may
/// carry colour codes).
struct Cell {
    plain: String,
    shown: String,
}

impl Cell {
    fn plain(text: String) -> Self {
        Cell {
            shown: text.clone(),
            plain: text,
        }
    }
}

/// `│ a │ b │ … │`, each cell padded to its column's width.
fn cells_line(cells: &[Cell; 4], widths: &[usize; 4]) -> String {
    let mut line = String::from("│");
    for (cell, width) in cells.iter().zip(widths) {
        let padding = " ".repeat(width - text_width(&cell.plain));
        line.push_str(&format!(" {}{padding} │", cell.shown));
    }
    line
}

/// A horizontal border: `left`, a run of `─` over each column (and its two
/// spaces of padding), `middle` between columns, then `right`.
fn rule(widths: &[usize; 4], [left, middle, right]: [char; 3]) -> String {
    let runs: Vec<String> = widths.iter().map(|w| "─".repeat(w + 2)).collect();
    format!("{left}{}{right}", runs.join(&middle.to_string()))
}

/// The table's printed width: every column, a space either side of each, and
/// one border character before, between and after them.
fn table_width(widths: &[usize]) -> usize {
    widths.iter().sum::<usize>() + 3 * widths.len() + 1
}

/// Widens the columns until the table is `target` wide, sharing the extra
/// evenly and giving any remainder to the leftmost columns. A table already
/// that wide is left alone.
fn stretch(widths: &mut [usize; 4], target: usize) {
    let current = table_width(widths);
    if current >= target {
        return;
    }
    let extra = target - current;
    let columns = widths.len();
    for (i, width) in widths.iter_mut().enumerate() {
        *width += extra / columns + usize::from(i < extra % columns);
    }
}

fn paint(color: Option<Color>, text: &str) -> String {
    match color {
        Some(color) => color.paint(text),
        None => text.to_string(),
    }
}

/// Counted in characters rather than bytes, so a non-ASCII name lines up.
fn text_width(text: &str) -> usize {
    text.chars().count()
}
