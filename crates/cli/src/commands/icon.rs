//! Custom icons: `icon list|import|delete`, over the store the app keeps beside
//! the database. Putting one on a project is `edit --icon custom:<name>`.

use std::path::PathBuf;

use anyhow::bail;
use clap::{Args, Subcommand};
use indexer_core::domain::sorting::SortOptions;
use indexer_core::Project;

use super::Outcome;
use crate::context::Context;

#[derive(Debug, Args)]
pub struct IconArgs {
    #[command(subcommand)]
    pub action: IconAction,
}

#[derive(Debug, Subcommand)]
pub enum IconAction {
    /// List your custom icons, with how many projects use each.
    List,
    /// Add an SVG file as a custom icon, named after the file.
    Import {
        /// The .svg file. It is cleaned before it is stored, as in the app.
        file: PathBuf,
    },
    /// Delete a custom icon. Asks first.
    Delete {
        /// Its name, with or without `custom:`.
        name: String,
    },
}

pub fn run(args: IconArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    match args.action {
        IconAction::List => {
            let projects = ctx.projects.list(SortOptions::default())?;
            let icons = ctx
                .icons
                .list()?
                .into_iter()
                .map(|icon| {
                    let users = users(&projects, &icon.name);
                    (icon.name, users)
                })
                .collect();
            Ok(Outcome::CustomIcons { icons })
        }
        IconAction::Import { file } => {
            let icon = ctx.icons.import(&file)?;
            Ok(Outcome::IconImported { name: icon.name })
        }
        IconAction::Delete { name } => {
            let name = name.strip_prefix("custom:").unwrap_or(&name).to_lowercase();
            let names: Vec<String> = ctx
                .icons
                .list()?
                .into_iter()
                .map(|icon| icon.name)
                .collect();
            if !names.contains(&name) {
                if names.is_empty() {
                    bail!("no custom icon named \"{name}\": there are none yet");
                }
                bail!(
                    "no custom icon named \"{name}\". Custom icons: {}",
                    names.join(", ")
                );
            }
            let mut projects = ctx.projects.list(SortOptions::default())?;
            projects.extend(ctx.projects.list_deleted(SortOptions::default())?);
            let users = users(&projects, &name);
            let prompt = match users {
                0 => format!("delete the custom icon \"{name}\"?"),
                1 => format!("delete the custom icon \"{name}\"? 1 project uses it and will show the folder icon"),
                n => format!("delete the custom icon \"{name}\"? {n} projects use it and will show the folder icon"),
            };
            if !ctx.confirmer.confirm(&prompt)? {
                return Ok(Outcome::Cancelled);
            }
            ctx.icons.delete(&name)?;
            Ok(Outcome::IconDeleted { name, users })
        }
    }
}

/// How many of `projects` wear the custom icon `name`.
fn users(projects: &[Project], name: &str) -> usize {
    let icon = format!("custom:{name}");
    projects
        .iter()
        .filter(|p| p.icon.as_deref() == Some(icon.as_str()))
        .count()
}
