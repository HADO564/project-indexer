use std::path::PathBuf;

use crate::paths::{config_dir, database_path, APP_IDENTIFIER, DATABASE_FILE};

/// If the app's identifier changes, the app moves its database and the CLI
/// must move with it.
#[test]
fn identifier_matches_tauri_config() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../../../src-tauri/tauri.conf.json"))
            .expect("tauri.conf.json is valid JSON");
    assert_eq!(config["identifier"], APP_IDENTIFIER);
}

/// The directory Tauri's `app_config_dir()` resolves to on this OS, written
/// out by hand rather than through `dirs`, so a change in either shows up.
fn expected_app_config_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var("HOME").unwrap()).join("Library/Application Support");
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(std::env::var("APPDATA").unwrap());
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|dir| PathBuf::from(dir).is_absolute())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap()).join(".config"));
    base.join(APP_IDENTIFIER)
}

#[test]
fn config_dir_matches_the_app() {
    assert_eq!(config_dir().unwrap(), expected_app_config_dir());
}

#[test]
fn database_is_projects_db_in_the_config_dir() {
    assert_eq!(
        database_path().unwrap(),
        expected_app_config_dir().join(DATABASE_FILE)
    );
}
