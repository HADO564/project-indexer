use crate::models::InstalledApp;

/// Recursively deletes a directory from disk. Used when a project is
/// removed with the "also delete this directory" option.
#[tauri::command]
pub fn delete_directory(path: String) -> Result<(), String> {
    std::fs::remove_dir_all(&path).map_err(|e| format!("Failed to delete directory: {}", e))
}

/// Scans platform-specific sources for installed applications, used by
/// the "open with" app picker: Start Menu shortcuts and registry App
/// Paths on Windows, `.desktop` files on Linux. macOS isn't covered yet,
/// so it gets an empty list.
#[tauri::command]
pub fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    #[cfg(windows)]
    {
        Ok(windows_impl::list_installed_apps())
    }
    #[cfg(target_os = "linux")]
    {
        Ok(linux_impl::list_installed_apps())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
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

#[cfg(target_os = "linux")]
mod linux_impl {
    use crate::models::InstalledApp;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Keyed by the launch command so entries installed under multiple
    /// application directories (e.g. a system package and a user
    /// override) collapse into one.
    pub fn list_installed_apps() -> Vec<InstalledApp> {
        let mut apps: HashMap<String, InstalledApp> = HashMap::new();

        for dir in application_dirs() {
            scan_applications_dir(&dir, &mut apps);
        }

        let mut list: Vec<InstalledApp> = apps.into_values().collect();
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        list
    }

    /// Directories searched for `.desktop` files, per the XDG Base
    /// Directory / Desktop Entry specifications.
    fn application_dirs() -> Vec<PathBuf> {
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

    fn scan_applications_dir(dir: &std::path::Path, apps: &mut HashMap<String, InstalledApp>) {
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

            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };

            if let Some(app) = parse_desktop_entry(&contents) {
                apps.entry(app.path.to_lowercase()).or_insert(app);
            }
        }
    }

    /// Parses the `[Desktop Entry]` section of a `.desktop` file into an
    /// `InstalledApp`, skipping entries that shouldn't be launchable from
    /// a picker (`NoDisplay`/`Hidden`) or that lack a usable `Exec` line.
    fn parse_desktop_entry(contents: &str) -> Option<InstalledApp> {
        let mut in_entry_section = false;
        let mut name = None;
        let mut exec = None;

        for line in contents.lines() {
            let line = line.trim();

            if line.starts_with('[') {
                in_entry_section = line == "[Desktop Entry]";
                continue;
            }
            if !in_entry_section || line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "Name" => name = Some(value.trim().to_string()),
                "Exec" => exec = Some(value.trim().to_string()),
                "NoDisplay" | "Hidden" if value.trim().eq_ignore_ascii_case("true") => {
                    return None;
                }
                _ => {}
            }
        }

        let name = name?;
        let command = exec_command(&exec?)?;

        Some(InstalledApp {
            name,
            path: command,
        })
    }

    /// Extracts the launchable binary from an `Exec=` line: takes the
    /// first whitespace-separated token and strips desktop entry field
    /// codes (`%f`, `%U`, etc.), which `open::with` has no use for.
    fn exec_command(exec: &str) -> Option<String> {
        let command = exec.split_whitespace().next()?;
        if command.starts_with('%') {
            return None;
        }
        Some(command.trim_matches('"').to_string())
    }
}
