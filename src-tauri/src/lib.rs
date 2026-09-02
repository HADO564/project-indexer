mod adapters;
pub mod commands;

use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use indexer_core::application::ProjectService;
use indexer_core::detectors::DetectorRunner;
use indexer_core::infra::SqliteRepository;

use crate::adapters::OpenerLauncher;

/// Works around WebKitGTK's DMABUF renderer failing on NVIDIA's proprietary
/// driver, where it can't allocate GBM buffers. The window then either comes
/// up blank ("Failed to create GBM buffer") or, under Wayland, the app dies
/// during startup with "Error 71 (Protocol error) dispatching to Wayland
/// display". Disabling the DMABUF renderer falls back to a software path that
/// works on both X11 and Wayland.
///
/// Gated on the proprietary driver so that Mesa, nouveau and everything else
/// keep the accelerated path, and skipped when the variable is already set so
/// a user can still force either behaviour. Must run before GTK/WebKit start.
#[cfg(target_os = "linux")]
fn disable_dmabuf_renderer_on_nvidia() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

    if std::env::var_os(VAR).is_some() {
        return;
    }

    // Both paths are created by the proprietary kernel module only, so this
    // is distro-independent — no package or driver-version probing needed.
    let nvidia_loaded = std::path::Path::new("/proc/driver/nvidia/version").exists()
        || std::path::Path::new("/sys/module/nvidia/version").exists();

    if nvidia_loaded {
        std::env::set_var(VAR, "1");
    }
}

/// Resolve the config dir and open the SQLite-backed project store. Every
/// failure here is one the user must be told about rather than crash on.
fn open_repository(app: &tauri::App) -> Result<SqliteRepository, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("could not locate the app config directory: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    SqliteRepository::open(&dir.join("projects.db"))
        .map_err(|e| format!("failed to open the project database: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    disable_dmabuf_renderer_on_nvidia();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // Persistence and orchestration live in `indexer_core`. The whole
        // service — a SQLite-backed repository, the app launcher adapter and
        // the shared detector set — is built once here and handed to the
        // commands as managed `Arc<ProjectService>` state.
        .setup(|app| {
            // Opening the store can fail for reasons the user needs to see —
            // most importantly the version-skew guard ("database is from a
            // newer version of Project Indexer"). Propagating the error out of
            // `setup` unwinds into `run()`'s `.expect(...)` and, in a release
            // GUI build, that's a window that never appears with no message.
            // So surface it in a blocking modal and exit non-zero instead.
            let repo = match open_repository(app) {
                Ok(repo) => repo,
                Err(e) => {
                    app.dialog()
                        .message(format!("Project Indexer can't start:\n\n{e}"))
                        .kind(MessageDialogKind::Error)
                        .blocking_show();
                    std::process::exit(1);
                }
            };
            let service = ProjectService::new(
                Arc::new(repo),
                Arc::new(OpenerLauncher),
                Arc::new(DetectorRunner::default()),
            );
            app.manage(Arc::new(service));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::get_project,
            commands::projects::get_all_projects,
            commands::projects::list_missing_directories,
            commands::projects::get_deleted_projects,
            commands::projects::get_favorite_projects,
            commands::projects::delete_project,
            commands::projects::delete_project_directory,
            commands::projects::untrack_project,
            commands::projects::restore_project,
            commands::system::list_installed_apps,
            commands::projects::open_project,
            commands::projects::open_project_in_explorer,
            commands::projects::refresh_project_trackers,
            commands::projects::detect_project_trackers,
            commands::projects::suggest_project_name,
            commands::inspect::inspect_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
