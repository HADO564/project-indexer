//! Locates the database the desktop app uses.
//!
//! Tauri's `app_config_dir()` is `dirs::config_dir()` joined with the bundle
//! identifier, so deriving the same path here needs no Tauri. Getting it wrong
//! fails silently — two databases, neither tool seeing the other's projects —
//! which is why `tests/paths.rs` pins it.

use std::path::PathBuf;

use anyhow::Context as _;

/// The app's bundle identifier: `identifier` in `src-tauri/tauri.conf.json`.
pub const APP_IDENTIFIER: &str = "com.shaer.project-indexer";

/// The file name the app opens inside its config directory.
pub const DATABASE_FILE: &str = "projects.db";

/// The app's config directory, which holds `projects.db` and `icons/`.
pub fn config_dir() -> anyhow::Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join(APP_IDENTIFIER))
        .context("could not locate the config directory")
}

/// The CLI's own settings file. The app never reads it.
pub const SETTINGS_FILE: &str = "cli-settings.json";

/// The shared `projects.db`.
pub fn database_path() -> anyhow::Result<PathBuf> {
    Ok(config_dir()?.join(DATABASE_FILE))
}

/// The CLI's settings, beside the database.
pub fn settings_path() -> anyhow::Result<PathBuf> {
    Ok(config_dir()?.join(SETTINGS_FILE))
}
