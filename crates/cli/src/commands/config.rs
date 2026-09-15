use clap::{Args, Subcommand};

use super::{ColorSetting, Outcome};
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
    /// Show or set the default colour of table headers.
    HeaderColor {
        /// The new default. Omit it to show the current one.
        #[arg(value_enum)]
        color: Option<Color>,

        /// Go back to the built-in default.
        #[arg(long, conflicts_with = "color")]
        reset: bool,
    },
}

pub fn run(args: ConfigArgs, _ctx: &Context) -> anyhow::Result<Outcome> {
    let (setting, color, reset) = match args.setting {
        Setting::FolderColor { color, reset } => (ColorSetting::Folder, color, reset),
        Setting::HeaderColor { color, reset } => (ColorSetting::Header, color, reset),
    };

    // `--reset` is how a user recovers from a broken settings file, so it must
    // not fail on one. Anything else reports the problem.
    let mut settings = if reset {
        settings::load().unwrap_or_default()
    } else {
        settings::load()?
    };

    let slot = match setting {
        ColorSetting::Folder => &mut settings.folder_color,
        ColorSetting::Header => &mut settings.header_color,
    };
    let changed = if reset {
        *slot = None;
        true
    } else if let Some(color) = color {
        *slot = Some(color);
        true
    } else {
        false
    };
    let current = slot.unwrap_or(setting.default_color());

    if changed {
        settings::save(&settings)?;
    }
    Ok(Outcome::Color {
        setting,
        color: current,
    })
}
