use indexer_core::domain::InstalledApp;

/// Scans platform-specific sources for installed applications, used by the
/// "open with" app picker. Thin wrapper over
/// [`indexer_core::platform::list_installed_apps`] — the scan itself is
/// frontend-agnostic and lives in `indexer-core`.
#[tauri::command]
pub fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    Ok(indexer_core::platform::list_installed_apps())
}
