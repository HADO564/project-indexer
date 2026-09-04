mod adapters;
pub mod commands;

use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

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

/// Brings the main window back to the foreground — used by the tray icon, the
/// tray menu, and a second launch of the app (single-instance).
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Whether the tray icon was actually created. When it wasn't, closing the
/// window must really quit (see the `CloseRequested` handler) — hiding to a
/// tray that isn't there would strand the app with no way back.
static TRAY_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Builds the tray, downgrading any failure to "no tray" instead of taking the
/// app down with it.
///
/// On Linux the tray needs an appindicator shared library at runtime, and
/// `libappindicator-sys` *panics* rather than returning an error when it can't
/// load one — so `setup_tray(..)?` never sees that failure, and the process
/// dies during `setup` with a raw panic and no window. Catching the unwind is
/// the only way to observe it. The panic's own message still reaches stderr
/// via the default hook; this adds the part the user can act on.
fn setup_tray_or_warn(app: &tauri::AppHandle) -> bool {
    match std::panic::catch_unwind(AssertUnwindSafe(|| setup_tray(app))) {
        Ok(Ok(())) => true,
        Ok(Err(e)) => {
            eprintln!("Project Indexer: could not create the tray icon: {e}");
            false
        }
        Err(_) => {
            eprintln!(
                "Project Indexer: could not create the tray icon — no appindicator \
                 library is installed.\nThe app will keep running, but closing the \
                 window now quits instead of hiding to the tray.\nOn Arch, install \
                 `libayatana-appindicator`; see the README's Linux notes for other \
                 distributions."
            );
            false
        }
    }
}

/// Builds the system-tray icon: left-click restores the window, right-click
/// opens a small menu (Show / Quit). Closing the window only hides it (see the
/// `CloseRequested` handler), so the tray is how you get back — or quit.
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Project Indexer", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &PredefinedMenuItem::separator(app)?, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().expect("bundled app icon").clone())
        .tooltip("Project Indexer")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
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
fn fatal_startup_error(message: &str) -> ! {
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
            let service = ProjectService::new(
                Arc::new(repo),
                Arc::new(OpenerLauncher),
                Arc::new(DetectorRunner::default()),
            );
            app.manage(Arc::new(service));

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
            commands::inspect::inspect_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
