//! Finding the applications installed on this machine.
//!
//! Launching one is [`super::app_launching`]; the `.desktop` format both use
//! is [`super::desktop_entry`].

use crate::domain::InstalledApp;

/// Scans platform-specific sources for installed applications, used by
/// the "open with" app picker: Start Menu shortcuts and registry App
/// Paths on Windows, `.desktop` files on Linux. macOS isn't covered yet,
/// so it gets an empty list.
pub fn list_installed_apps() -> Vec<InstalledApp> {
    #[cfg(windows)]
    {
        windows_impl::list_installed_apps()
    }
    #[cfg(target_os = "linux")]
    {
        linux_impl::list_installed_apps()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
mod windows_impl {
    use crate::domain::InstalledApp;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use winreg::{HKCU, HKLM};

    /// Keyed by lowercased resolved exe path so Start Menu and App Paths
    /// entries for the same app collapse into one, with Start Menu's
    /// friendlier name winning since it's scanned first.
    pub fn list_installed_apps() -> Vec<InstalledApp> {
        let mut apps: HashMap<String, InstalledApp> = HashMap::new();

        for dir in start_menu_dirs() {
            scan_start_menu_dir(&dir, &mut apps);
        }

        scan_app_paths(HKCU, &mut apps);
        scan_app_paths(HKLM, &mut apps);

        let mut list: Vec<InstalledApp> = apps.into_values().collect();
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        list
    }

    fn start_menu_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Ok(program_data) = std::env::var("ProgramData") {
            dirs.push(PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs"));
        }
        if let Ok(app_data) = std::env::var("AppData") {
            dirs.push(PathBuf::from(app_data).join(r"Microsoft\Windows\Start Menu\Programs"));
        }
        dirs
    }

    fn scan_start_menu_dir(dir: &Path, apps: &mut HashMap<String, InstalledApp>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                scan_start_menu_dir(&path, apps);
                continue;
            }

            let is_lnk = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
            if !is_lnk {
                continue;
            }

            let Ok(lnk) = parselnk::Lnk::try_from(path.as_path()) else {
                continue;
            };
            let Some(target) = lnk
                .link_info
                .local_base_path
                .clone()
                .or_else(|| lnk.link_info.local_base_path_unicode.clone())
            else {
                continue;
            };
            if !target.to_lowercase().ends_with(".exe") {
                continue;
            }

            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };

            apps.entry(target.to_lowercase()).or_insert(InstalledApp {
                name: name.to_string(),
                path: target,
            });
        }
    }

    fn scan_app_paths(hive: &winreg::RegKey, apps: &mut HashMap<String, InstalledApp>) {
        let Ok(app_paths) =
            hive.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths")
        else {
            return;
        };

        for key_name in app_paths.enum_keys().flatten() {
            let Ok(subkey) = app_paths.open_subkey(&key_name) else {
                continue;
            };
            let Ok(path) = subkey.get_value::<String, _>("") else {
                continue;
            };
            if !path.to_lowercase().ends_with(".exe") {
                continue;
            }

            let name = key_name
                .trim_end_matches(".exe")
                .trim_end_matches(".EXE")
                .to_string();

            apps.entry(path.to_lowercase())
                .or_insert(InstalledApp { name, path });
        }
    }
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::super::desktop_entry::parse_desktop_entry;
    use crate::domain::InstalledApp;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    /// Keyed by desktop file ID (the basename without `.desktop`, which the
    /// Desktop Entry spec already guarantees unique per application) rather
    /// than by launch command. Several distinct apps legitimately share a
    /// leading command — every Flatpak launches through `flatpak`, every Wine
    /// association through `env` — so keying on the command collapsed them
    /// into a single picker entry. Earlier directories win, which gives a
    /// user's `~/.local/share/applications` override precedence over the
    /// system copy, matching XDG lookup order.
    pub(crate) fn list_installed_apps() -> Vec<InstalledApp> {
        let mut apps: HashMap<String, InstalledApp> = HashMap::new();

        for dir in application_dirs() {
            scan_applications_dir(&dir, &mut apps);
        }

        let mut list: Vec<InstalledApp> = apps.into_values().collect();
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        list
    }

    /// Directories searched for `.desktop` files, per the XDG Base
    /// Directory / Desktop Entry specifications. Covers the Flatpak and Snap
    /// exports that Ubuntu and Fedora add to `XDG_DATA_DIRS` too, since those
    /// are ordinary entries in an `applications` subdirectory.
    pub(crate) fn application_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(data_home).join("applications"));
        } else if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".local/share/applications"));
        }

        let data_dirs = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
        for dir in data_dirs.split(':').filter(|d| !d.is_empty()) {
            dirs.push(PathBuf::from(dir).join("applications"));
        }

        dirs
    }

    pub(crate) fn scan_applications_dir(dir: &Path, apps: &mut HashMap<String, InstalledApp>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                scan_applications_dir(&path, apps);
                continue;
            }

            let is_desktop_file = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("desktop"));
            if !is_desktop_file {
                continue;
            }

            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            // Cheap to check before reading, and keeps a system entry from
            // overwriting the user override that was scanned first.
            if apps.contains_key(id) {
                continue;
            }

            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };

            if let Some(app) = parse_desktop_entry(&contents) {
                apps.insert(id.to_string(), app);
            }
        }
    }
}
