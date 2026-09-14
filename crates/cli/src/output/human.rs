//! The default, human-readable rendering.

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
