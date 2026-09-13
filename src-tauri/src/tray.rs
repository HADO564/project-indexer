//! The system tray, and the window-visibility rules that depend on it.
//!
//! Split out of `lib.rs`, which was the app's composition root *and* its
//! tray implementation. `TRAY_AVAILABLE` is the load-bearing bit: with no
//! tray, closing the window must genuinely quit rather than hide it beyond
//! reach.

use std::panic::AssertUnwindSafe;
use std::sync::atomic::AtomicBool;
#[cfg(target_os = "macos")]
use std::sync::atomic::Ordering;

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

/// Hides the main window to the tray — the counterpart to `show_main_window`,
/// used by the `CloseRequested` handler. On macOS a fullscreen window lives in
/// its own Space, and hiding it there leaves that Space open and black. So it
/// leaves fullscreen first, and `watch_fullscreen_exit` hides it once macOS
/// reports the transition has finished.
pub(crate) fn hide_main_window(window: &tauri::Window) {
    #[cfg(target_os = "macos")]
    if window.is_fullscreen().unwrap_or(false) {
        HIDE_AFTER_FULLSCREEN_EXIT.store(true, Ordering::Relaxed);
        let _ = window.set_fullscreen(false);
        return;
    }
    let _ = window.hide();
}

/// Set when a close arrives while the main window is fullscreen (macOS), so
/// the fullscreen exit that follows ends in a hide. Exits the user starts
/// (green button, Ctrl+Cmd+F) leave it unset and the window stays visible.
#[cfg(target_os = "macos")]
static HIDE_AFTER_FULLSCREEN_EXIT: AtomicBool = AtomicBool::new(false);

/// Hides the main window once macOS reports that a fullscreen exit started by
/// `hide_main_window` has finished. Registered once, at startup.
///
/// This listens for `NSWindowDidExitFullScreenNotification` rather than using
/// Tauri's events: tao reports `is_fullscreen() == false` as soon as the exit
/// *starts*, and a hide during the animation is ignored, so no `Resized` event
/// reliably marks the end.
#[cfg(target_os = "macos")]
pub(crate) fn watch_fullscreen_exit(app: &tauri::AppHandle) {
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::NSWindowDidExitFullScreenNotification;
    use objc2_foundation::{NSNotification, NSNotificationCenter};

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(ns_window) = window.ns_window() else {
        return;
    };
    let block = RcBlock::new(move |_: NonNull<NSNotification>| {
        if HIDE_AFTER_FULLSCREEN_EXIT.swap(false, Ordering::Relaxed) {
            let _ = window.hide();
        }
    });
    // SAFETY: `ns_window` is the live NSWindow behind the main window. With no
    // queue, the block runs on the posting thread, which is AppKit's main thread.
    let observer = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(NSWindowDidExitFullScreenNotification),
            Some(&*ns_window.cast::<AnyObject>()),
            None,
            &block,
        )
    };
    // The main window lives as long as the app, so the observer does too.
    std::mem::forget(observer);
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
