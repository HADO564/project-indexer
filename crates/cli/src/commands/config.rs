use clap::{Args, Subcommand};

use super::Outcome;
use crate::context::Context;
use crate::output::color::Color;
use crate::settings;

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub setting: Setting,
}

#[derive(Debug, Subcommand)]
pub enum Setting {
    /// Show or set the default folder colour.
    FolderColor {
        /// The new default. Omit it to show the current one.
        #[arg(value_enum)]
        color: Option<Color>,

        /// Go back to the built-in default.
        #[arg(long, conflicts_with = "color")]
        reset: bool,
    },
}

pub fn run(args: ConfigArgs, _ctx: &Context) -> anyhow::Result<Outcome> {
    match args.setting {
        Setting::FolderColor { color, reset } => {
            // `--reset` is how a user recovers from a broken settings file, so
            // it must not fail on one. Anything else reports the problem.
            let mut settings = if reset {
                settings::load().unwrap_or_default()
            } else {
                settings::load()?
            };
            if reset {
                settings.folder_color = None;
                settings::save(&settings)?;
            } else if let Some(color) = color {
                settings.folder_color = Some(color);
                settings::save(&settings)?;
            }
            Ok(Outcome::FolderColor(
                settings.folder_color.unwrap_or(Color::DEFAULT),
            ))
        }
    }
}
