//! A placeholder `AppLauncher`.
//!
//! The app's `OpenerLauncher` depends on `tauri-plugin-opener`, so the CLI
//! cannot reuse it. Whether a Tauri-free launcher moves into
//! `indexer_core::platform` or lives here is decided at the `open` milestone.

use indexer_core::{AppLauncher, LauncherError};

pub struct UnsupportedLauncher;

impl AppLauncher for UnsupportedLauncher {
    fn open(&self, _directory: &str, _open_with: Option<&str>) -> Result<(), LauncherError> {
        Err(LauncherError(
            "opening projects from the CLI is not implemented yet".to_string(),
        ))
    }

    fn is_available(&self, _open_with: &str) -> bool {
        false
    }
}
