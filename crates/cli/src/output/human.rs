//! The default, human-readable rendering.

use std::fmt::Write as _;
use std::io::{IsTerminal, Write};
use std::path::Path;

use indexer_core::domain::Project;

use crate::commands::Outcome;
use crate::output::color::Color;

pub fn write(out: &mut impl Write, outcome: &Outcome, folder_color: Color) -> anyhow::Result<()> {
    let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    match outcome {
        Outcome::Projects(projects) => {
            if projects.is_empty() {
                eprintln!("indexer: no projects tracked yet");
                return Ok(());
            }
            let projects: Vec<&Project> = projects.iter().collect();
            write!(
                out,
                "{}",
                project_table(&projects, color.then_some(folder_color))
            )?;
        }
        Outcome::Project(project) => {
            writeln!(out, "{}", project.name)?;
            writeln!(out, "  directory  {}", project.directory)?;
            writeln!(out, "  id         {}", project.id)?;
        }
        Outcome::FolderColor(chosen) => {
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

/// A table of projects — `list`'s view, and `show`'s when a query matches
/// several — with the columns NAME, DIRECTORY, TRACKERS and LAST OPENED.
///
/// DIRECTORY is only `parent/folder`, the part `show parent/folder` takes
/// back. With `folder_color`, the folder is painted; the padding is worked
/// out on the plain text, so the colour codes never push a column out of line.
pub fn project_table(projects: &[&Project], folder_color: Option<Color>) -> String {
    let rows: Vec<Row> = projects.iter().map(|p| Row::of(p)).collect();

    let name_width = column_width("NAME", rows.iter().map(|r| r.name.as_str()));
    let directory_width = column_width("DIRECTORY", rows.iter().map(|r| r.directory()));
    let trackers_width = column_width("TRACKERS", rows.iter().map(|r| r.trackers.as_str()));

    let mut out = String::new();
    writeln!(
        out,
        "{:<name_width$}  {:<directory_width$}  {:<trackers_width$}  LAST OPENED",
        "NAME", "DIRECTORY", "TRACKERS"
    )
    .unwrap();
    for row in &rows {
        let directory = match folder_color {
            Some(color) => format!("{}{}", row.parent, color.paint(&row.folder)),
            None => row.directory(),
        };
        let padding = " ".repeat(directory_width - row.directory().chars().count());
        writeln!(
            out,
            "{:<name_width$}  {directory}{padding}  {:<trackers_width$}  {}",
            row.name, row.trackers, row.last_opened
        )
        .unwrap();
    }
    out
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

    fn directory(&self) -> String {
        format!("{}{}", self.parent, self.folder)
    }
}

/// The widest cell in a column, header included, counted in characters so a
/// non-ASCII name lines up the same as the padding `format!` adds.
fn column_width<S: AsRef<str>>(header: &str, cells: impl Iterator<Item = S>) -> usize {
    cells
        .map(|cell| cell.as_ref().chars().count())
        .max()
        .unwrap_or(0)
        .max(header.chars().count())
}
