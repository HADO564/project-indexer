use crate::output::color::Color;
use crate::settings::{load_from, save_to, Settings};

#[test]
fn a_missing_file_is_all_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let settings = load_from(&dir.path().join("cli-settings.json")).unwrap();
    assert_eq!(settings, Settings::default());
}

#[test]
fn a_saved_folder_color_loads_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("cli-settings.json");
    let saved = Settings {
        folder_color: Some(Color::HotPink),
    };
    save_to(&path, &saved).unwrap();

    assert_eq!(load_from(&path).unwrap(), saved);
    // The same kebab-case name the flag accepts, so the file is hand-editable.
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"hot-pink\""), "{text}");
}

#[test]
fn unknown_keys_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cli-settings.json");
    std::fs::write(&path, r#"{"folder_color": "teal", "from_a_newer_cli": 1}"#).unwrap();
    assert_eq!(load_from(&path).unwrap().folder_color, Some(Color::Teal));
}

#[test]
fn a_corrupt_file_is_an_error_naming_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cli-settings.json");
    std::fs::write(&path, "not json").unwrap();
    let error = format!("{:#}", load_from(&path).unwrap_err());
    assert!(error.contains("cli-settings.json"), "{error}");
}
