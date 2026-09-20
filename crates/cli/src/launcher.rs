//! The CLI's [`AppLauncher`]: opens a project's directory in an application.
//!
//! **Why it lives here and not in `indexer_core::platform`.** Core already
//! holds the genuinely platform-specific halves — [`open_with_app_available`]
//! decides whether a configured app can still be launched, and
//! [`open_with_command`] runs a Linux `.desktop` command line — so what was
//! left was "hand a path to the system opener", which the `open` crate does on
//! every platform. Core would have to take that dependency for one frontend:
//! the app cannot switch off `tauri-plugin-opener` without being retested on
//! three platforms. Once it does, this is the implementation to move.

use indexer_core::platform::open_with_app_available;
use indexer_core::{AppLauncher, LauncherError};

/// Opens directories through the operating system's own opener — `open` on
/// macOS, `start` on Windows, the XDG handler on Linux.
pub struct SystemLauncher;

impl AppLauncher for SystemLauncher {
    fn open(&self, directory: &str, open_with: Option<&str>) -> Result<(), LauncherError> {
        let app = open_with.map(str::trim).filter(|app| !app.is_empty());

        // Linux stores a whole `.desktop` command line, which the opener would
        // treat as one very long program name. Core splits and spawns it; this
        // is the same call the app's `OpenerLauncher` makes.
        #[cfg(target_os = "linux")]
        if let Some(app) = app {
            return indexer_core::platform::open_with_command(directory, app)
                .map_err(LauncherError);
        }

        // Detached on purpose: `indexer open` returns as soon as the editor is
        // launched. Without it the CLI would sit waiting for a GUI application
        // to exit, and closing the terminal would take the editor with it.
        let result = match app {
            Some(app) => open::with_detached(directory, app),
            None => open::that_detached(directory),
        };
        result.map_err(|e| LauncherError(format!("could not open {directory}: {e}")))
    }

    fn is_available(&self, open_with: &str) -> bool {
        open_with_app_available(open_with)
    }
}
