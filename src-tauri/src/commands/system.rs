use crate::models::InstalledApp;

/// Recursively deletes a directory from disk. Used when a project is
/// removed with the "also delete this directory" option.
#[tauri::command]
pub fn delete_directory(path: String) -> Result<(), String> {
    std::fs::remove_dir_all(&path).map_err(|e| format!("Failed to delete directory: {}", e))
}

/// Scans Start Menu shortcuts and registry App Paths for installed
/// applications, used by the "open with" app picker. Windows-only: both
/// data sources are Windows concepts, so other platforms just get an
/// empty list.
#[tauri::command]
pub fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    #[cfg(windows)]
    {
        Ok(windows_impl::list_installed_apps())
    }
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}

#[cfg(windows)]
mod windows_impl {
    use crate::models::InstalledApp;
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
