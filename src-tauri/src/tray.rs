//! The system tray, and the window-visibility rules that depend on it.
//!
//! Split out of `lib.rs`, which was the app's composition root *and* its
//! tray implementation. `TRAY_AVAILABLE` is the load-bearing bit: with no
//! tray, closing the window must genuinely quit rather than hide it beyond
//! reach.

use std::panic::AssertUnwindSafe;
use std::sync::atomic::AtomicBool;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

/// Brings the main window back to the foreground — used by the tray icon, the
/// tray menu, and a second launch of the app (single-instance).
pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Whether the tray icon was actually created. When it wasn't, closing the
/// window must really quit (see the `CloseRequested` handler) — hiding to a
/// tray that isn't there would strand the app with no way back.
pub(crate) static TRAY_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Builds the tray, downgrading any failure to "no tray" instead of taking the
/// app down with it.
///
/// On Linux the tray needs an appindicator shared library at runtime, and
/// `libappindicator-sys` *panics* rather than returning an error when it can't
/// load one — so `setup_tray(..)?` never sees that failure, and the process
/// dies during `setup` with a raw panic and no window. Catching the unwind is
/// the only way to observe it. The panic's own message still reaches stderr
/// via the default hook; this adds the part the user can act on.
pub(crate) fn setup_tray_or_warn(app: &tauri::AppHandle) -> bool {
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
