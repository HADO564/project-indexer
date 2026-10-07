use clap::{Args, Subcommand};

use super::{ColorSetting, Outcome};
use crate::appearance::IconStyle;
use crate::context::Context;
use crate::output::color::Color;
use crate::settings;

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub setting: Setting,
}

/// `on` or `off`, as `config form-wrap` takes it.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Switch {
    On,
    Off,
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
    /// Show or set whether Tab and Shift+Tab wrap at the edit form's ends.
    FormWrap {
        /// `on` or `off`. Omit it to show the current one.
        #[arg(value_enum)]
        value: Option<Switch>,

        /// Go back to the built-in default (on).
        #[arg(long, conflicts_with = "value")]
        reset: bool,
    },
    /// Show or set how icons are drawn: as Nerd Font glyphs (needs a Nerd
    /// Font in the terminal), as emoji, or not at all.
    Icons {
        /// `nerd`, `emoji` or `off`. Omit it to show the current one.
        #[arg(value_enum)]
        style: Option<IconStyle>,

        /// Go back to the built-in default (off).
        #[arg(long, conflicts_with = "style")]
        reset: bool,
    },
}

pub fn run(args: ConfigArgs, _ctx: &Context) -> anyhow::Result<Outcome> {
    match args.setting {
        Setting::FolderColor { color, reset } => color_setting(ColorSetting::Folder, color, reset),
        Setting::HeaderColor { color, reset } => color_setting(ColorSetting::Header, color, reset),
        Setting::FormWrap { value, reset } => form_wrap(value, reset),
        Setting::Icons { style, reset } => icons(style, reset),
    }
}

/// `--reset` is how a user recovers from a broken settings file, so it must
/// not fail on one. Anything else reports the problem.
fn load_settings(reset: bool) -> anyhow::Result<settings::Settings> {
    if reset {
        Ok(settings::load().unwrap_or_default())
    } else {
        settings::load()
    }
}

fn color_setting(
    setting: ColorSetting,
    color: Option<Color>,
    reset: bool,
) -> anyhow::Result<Outcome> {
    let mut settings = load_settings(reset)?;

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

fn form_wrap(value: Option<Switch>, reset: bool) -> anyhow::Result<Outcome> {
    let mut settings = load_settings(reset)?;
    let changed = if reset {
        settings.form_wrap = None;
        true
    } else if let Some(value) = value {
        settings.form_wrap = Some(matches!(value, Switch::On));
        true
    } else {
        false
    };
    if changed {
        settings::save(&settings)?;
    }
    Ok(Outcome::FormWrap {
        wrap: settings.form_wrap.unwrap_or(true),
    })
}

fn icons(style: Option<IconStyle>, reset: bool) -> anyhow::Result<Outcome> {
    let mut settings = load_settings(reset)?;
    let changed = if reset {
        settings.icons = None;
        true
    } else if let Some(style) = style {
        settings.icons = Some(style);
        true
    } else {
        false
    };
    if changed {
        settings::save(&settings)?;
    }
    Ok(Outcome::Icons {
        style: settings.icons.unwrap_or_default(),
    })
}
