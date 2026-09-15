//! The CLI's own preferences, kept in `cli-settings.json` beside the app's
//! database.
//!
//! Deliberately not in `projects.db`: that file's schema belongs to core and is
//! the compatibility contract with the app, and nothing here concerns the app.
//!
//! A missing file means "all defaults". Unknown keys are ignored, so a newer
//! CLI's settings file still loads in an older one.

use std::io::ErrorKind;
use std::path::Path;

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

use crate::output::color::Color;
use crate::paths;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// The folder colour used when `--folder-color` is not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_color: Option<Color>,

    /// The table header colour used when `--header-color` is not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_color: Option<Color>,
}

pub fn load() -> anyhow::Result<Settings> {
    load_from(&paths::settings_path()?)
}

pub fn save(settings: &Settings) -> anyhow::Result<()> {
    save_to(&paths::settings_path()?, settings)
}

pub fn load_from(path: &Path) -> anyhow::Result<Settings> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .with_context(|| format!("{} is not a valid settings file", path.display())),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(e).with_context(|| format!("could not read {}", path.display())),
    }
}

pub fn save_to(path: &Path, settings: &Settings) -> anyhow::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("could not create {}", dir.display()))?;
    }
    let mut text = serde_json::to_string_pretty(settings)?;
    text.push('\n');
    std::fs::write(path, text).with_context(|| format!("could not write {}", path.display()))
}
