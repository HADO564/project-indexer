//! Everything that has to happen, or go wrong, before a window exists.
//!
//! A failure here cannot be surfaced in the UI, because there is no UI yet —
//! which is why `fatal_startup_error` opens a blocking dialog and exits
//! rather than propagating.

use tauri::Manager;

use indexer_core::{IconStore, SqliteRepository};

/// Works around WebKitGTK's DMABUF renderer failing on NVIDIA's proprietary
/// driver, where it can't allocate GBM buffers. The window then either comes
/// up blank ("Failed to create GBM buffer") or, under Wayland, the app dies
/// during startup with "Error 71 (Protocol error) dispatching to Wayland
/// display". Disabling the DMABUF renderer falls back to a software path that
/// works on both X11 and Wayland.
///
/// Gated on an NVIDIA kernel module being loaded so that Mesa, nouveau and
/// everything else keep the accelerated path, and skipped when the variable is
/// already set so a user can still force either behaviour. Must run before
/// GTK/WebKit start.
#[cfg(target_os = "linux")]
pub(crate) fn disable_dmabuf_renderer_on_nvidia() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

    if std::env::var_os(VAR).is_some() {
        return;
    }

    // Both paths are created by either NVIDIA kernel module, proprietary or
    // open, and catching both is deliberate: the open module still pairs with
    // the proprietary userspace GL stack that has the GBM allocation failure.
    // Distro-independent — no package or driver-version probing needed.
    let nvidia_loaded = std::path::Path::new("/proc/driver/nvidia/version").exists()
        || std::path::Path::new("/sys/module/nvidia/version").exists();

    if nvidia_loaded {
        std::env::set_var(VAR, "1");
    }
}

/// Reports a fatal startup problem and exits.
///
/// Deliberately *not* `tauri_plugin_dialog`: that plugin queues the dialog onto
/// the main-thread event loop (`run_on_main_thread`) and then blocks the caller
/// waiting for the result. `setup` runs on the main thread before the loop has
/// started, so the queued work would never run — the app would hang with no
/// window and no message, which is worse than the crash this replaced. `rfd`
/// renders the modal synchronously on the calling thread instead.
///
/// The message also goes to stderr, so a terminal launch or a captured log
/// still records it when no GUI is available at all.
pub(crate) fn fatal_startup_error(message: &str) -> ! {
    eprintln!("{message}");
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Project Indexer")
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    std::process::exit(1);
}

/// Resolve the config dir and open the SQLite-backed project store. Every
/// failure here is one the user must be told about rather than crash on.
pub(crate) fn open_repository(app: &tauri::App) -> Result<SqliteRepository, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("could not locate the app config directory: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    SqliteRepository::open(&dir.join("projects.db"))
        .map_err(|e| format!("failed to open the project database: {e}"))
}

/// The icon store lives beside `projects.db` in the app config directory. The
/// directory itself is created lazily on first import, so a missing one is not
/// a startup failure.
pub(crate) fn icon_store(app: &tauri::App) -> Result<IconStore, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("could not locate the app config directory: {e}"))?;
    Ok(IconStore::new(dir.join("icons")))
}
