//! The default, human-readable rendering.

use indexer_core::domain::Project;
use std::fmt::Write as _;
use std::io::{IsTerminal, Write};
use std::path::Path;

use crate::commands::Outcome;
use crate::output::color::Color;

pub fn write(out: &mut impl Write, outcome: &Outcome, folder_color: Color) -> anyhow::Result<()> {
    let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    match outcome {
        Outcome::Projects(projects) => {
            let width = projects.iter().map(|p| p.name.len()).max().unwrap_or(0);
            for project in projects {
                let path = Path::new(&project.directory);
                let parent = path.parent().unwrap_or(path);
                let folder = path
                    .file_name()
                    .map(|f| f.to_string_lossy())
                    .unwrap_or_default();
                if color {
                    writeln!(
                        out,
                        "{:<width$}  {}/{}",
                        project.name,
                        parent.display(),
                        folder_color.paint(&folder)
                    )?;
                } else {
                    writeln!(out, "{:<width$}  {}", project.name, project.directory)?;
                }
            }
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

pub fn match_list(projects: &[&Project]) -> String {
    let mut rows: Vec<[String; 4]> = Vec::new();

    for p in projects {
        let path = Path::new(&p.directory);
        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|f| f.to_string_lossy())
            .unwrap_or_default();
        let folder = path
            .file_name()
            .map(|f| f.to_string_lossy())
            .unwrap_or_default();
        let directory = format!("{parent}/{folder}");

        let mut trackers = p
            .trackers
            .iter()
            .map(|t| t.kind())
            .collect::<Vec<_>>()
            .join(", ");
        if trackers.is_empty() {
            trackers = "-".to_string();
        }

        let last_opened = match p.last_opened_at {
            Some(date) => date.format("%Y-%m-%d").to_string(),
            None => "never".to_string(),
        };

        rows.push([p.name.clone(), directory, trackers, last_opened]);
    }
    let name_width = rows
        .iter()
        .map(|r| r[0].len())
        .max()
        .unwrap_or(0)
        .max("NAME".len());
    let directory_width = rows
        .iter()
        .map(|r| r[1].len())
        .max()
        .unwrap_or(0)
        .max("DIRECTORY".len());
    let trackers_width = rows
        .iter()
        .map(|r| r[2].len())
        .max()
        .unwrap_or(0)
        .max("TRACKERS".len());
    let last_opened_width = rows
        .iter()
        .map(|r| r[3].len())
        .max()
        .unwrap_or(0)
        .max("LAST OPENED".len());
    let mut out = String::new();
    writeln!(
        out,
        "  {:<name_width$}  {:<directory_width$}  {:<trackers_width$}  {:<last_opened_width$}",
        "NAME", "DIRECTORY", "TRACKERS", "LAST OPENED"
    )
    .unwrap();
    for row in &rows {
        writeln!(
            out,
            "  {:<name_width$}  {:<directory_width$}  {:<trackers_width$}  {}",
            row[0], row[1], row[2], row[3]
        )
        .unwrap();
    }
    out
}
