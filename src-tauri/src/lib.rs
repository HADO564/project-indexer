pub mod commands;
pub mod detectors;
pub mod errors;
pub mod migrations;
pub mod models;
pub mod store;
pub mod utils;

use tauri::Manager;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    disable_dmabuf_renderer_on_nvidia();

    tauri::Builder::default()
        // The detector set is built once and shared: commands pull it back
        // out with `State<DetectorRunner>` rather than constructing detectors
        // per call. See `detectors::registry` to register a new detector.
        .manage(detectors::DetectorRunner::default())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Err(e) = store::ProjectStore::flush(window.app_handle()) {
                    eprintln!("Failed to flush project store on close: {}", e);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::get_project,
            commands::projects::get_all_projects,
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
            commands::projects::detect_project_trackers
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
