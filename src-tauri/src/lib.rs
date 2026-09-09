//! The app's composition root: build the services, register the commands,
//! wire the window events. The pieces it assembles live beside it.

mod adapters;
pub mod commands;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::{Manager, WindowEvent};

use indexer_core::application::{GroupService, ProjectService, ScanService};
use indexer_core::detectors::DetectorRunner;

use crate::adapters::OpenerLauncher;
mod startup;
mod tray;

use startup::{fatal_startup_error, icon_store, open_repository};
use tray::{setup_tray_or_warn, show_main_window, TRAY_AVAILABLE};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    startup::disable_dmabuf_renderer_on_nvidia();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch (e.g. from the Start Menu while hidden to tray)
            // just brings the running window forward.
            show_main_window(app);
        }))
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
                    fatal_startup_error(&format!("Project Indexer can't start:\n\n{e}"));
                }
            };
            // One repository, two services. `Arc<SqliteRepository>` coerces to
            // each port, so both views share a single connection and its lock.
            let repo = Arc::new(repo);
            let detectors = Arc::new(DetectorRunner::default());
            let service = Arc::new(ProjectService::new(
                repo.clone(),
                Arc::new(OpenerLauncher),
                detectors.clone(),
                repo.clone(),
            ));
            app.manage(Arc::new(ScanService::new(service.clone(), detectors)));
            app.manage(service);
            app.manage(Arc::new(GroupService::new(repo)));

            let icons = match icon_store(app) {
                Ok(store) => store,
                Err(e) => {
                    fatal_startup_error(&format!("Project Indexer can't start:\n\n{e}"));
                }
            };
            app.manage(Arc::new(icons));

            TRAY_AVAILABLE.store(setup_tray_or_warn(app.handle()), Ordering::Relaxed);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window hides it to the tray instead of quitting,
            // so the app keeps running in the background. "Quit" on the tray
            // menu is the real exit — so when there is no tray, let the close
            // through rather than hiding the window beyond reach.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && TRAY_AVAILABLE.load(Ordering::Relaxed) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
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
            commands::inspect::inspect_project,
            commands::scan::scan_folder,
            commands::scan::import_scanned,
            commands::scan::list_detector_kinds,
            commands::groups::list_groups,
            commands::groups::create_group,
            commands::groups::update_group,
            commands::groups::delete_group,
            commands::groups::reorder_groups,
            commands::icons::list_custom_icons,
            commands::icons::import_custom_icon,
            commands::icons::delete_custom_icon
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
