use indexer_core::error::LauncherError;
use indexer_core::platform::open_with_app_available;
use indexer_core::ports::AppLauncher;

/// Tauri-side [`AppLauncher`] adapter. Wraps the platform-specific launch
/// logic that used to live in `commands/system.rs::open_in_app` and exposes it
/// through the frontend-agnostic core port.
pub struct OpenerLauncher;

impl AppLauncher for OpenerLauncher {
    fn open(&self, directory: &str, open_with: Option<&str>) -> Result<(), LauncherError> {
        open_in_app(directory, open_with).map_err(LauncherError)
    }

    fn is_available(&self, open_with: &str) -> bool {
        open_with_app_available(open_with)
    }
}

/// Opens `directory`, either with the application in `open_with` or with
/// the system default when that's `None`.
///
/// Windows and macOS hand the whole thing to the opener plugin, which
/// resolves an `.exe` path or an app name the way each platform expects.
/// Linux takes its own path because the picker stores a full `.desktop`
/// command line, which the plugin would treat as one long program name.
///
/// (Moved verbatim from `commands/system.rs`.)
fn open_in_app(directory: &str, open_with: Option<&str>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if let Some(command) = open_with.map(str::trim).filter(|c| !c.is_empty()) {
            return indexer_core::platform::app_discovery::open_with_command(directory, command);
        }
    }

    // Windows: launch a chosen executable ourselves rather than through
    // `ShellExecuteExW`, so we can scrub `ELECTRON_RUN_AS_NODE` from its
    // environment. When Project Indexer is itself started from a VS Code
    // terminal that variable is set and inherited, and it makes every Electron
    // binary (VS Code, Cursor, Slack, ...) run as plain Node — `Code.exe
    // <folder>` then tries to *require* the folder and exits instead of
    // opening it, while ShellExecute still reports success. Bare command names
    // (no path separator) fall through to the opener, which resolves them via
    // the registry's App Paths and PATHEXT.
    #[cfg(windows)]
    {
        if let Some(app) = open_with.map(str::trim).filter(|c| !c.is_empty()) {
            let looks_like_path =
                std::path::Path::new(app).is_absolute() || app.contains(['\\', '/']);
            if looks_like_path {
                use std::os::windows::process::CommandExt;
                const DETACHED_PROCESS: u32 = 0x0000_0008;
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                return std::process::Command::new(app)
                    .arg(directory)
                    .env_remove("ELECTRON_RUN_AS_NODE")
                    .env_remove("ELECTRON_NO_ATTACH_CONSOLE")
                    .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string());
            }
        }
    }

    tauri_plugin_opener::open_path(directory, open_with).map_err(|e| e.to_string())
}
