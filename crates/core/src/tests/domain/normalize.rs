//! Tests for [`crate::domain::normalize`].

use crate::domain::normalize::*;

#[test]
fn normalizes_trailing_separator() {
    assert_eq!(
        normalize_directory("D:\\Projects\\Friction\\"),
        normalize_directory("D:\\Projects\\Friction"),
    );
}

#[test]
fn normalizes_mixed_separator_style() {
    assert_eq!(
        normalize_directory("D:/Projects/Friction"),
        normalize_directory("D:\\Projects\\Friction"),
    );
}

#[test]
fn preserves_root_path() {
    let sep = std::path::MAIN_SEPARATOR.to_string();
    assert_eq!(normalize_directory(&sep), sep);
}

#[cfg(target_os = "windows")]
#[test]
fn preserves_drive_root() {
    assert_eq!(normalize_directory("C:\\"), "C:\\");
    assert_eq!(normalize_directory("C:/"), "C:\\");
}

#[test]
fn remove_spaces_replaces_with_underscores() {
    assert_eq!(remove_spaces("my project"), "my_project");
}

#[test]
fn normalize_tag_trims_and_title_cases() {
    assert_eq!(normalize_tag("  rUST  "), "Rust");
}

#[test]
fn normalize_tags_drops_empty_and_duplicates() {
    assert_eq!(
        normalize_tags(vec![
            "rust".to_string(),
            "  ".to_string(),
            "RUST".to_string(),
            "web".to_string(),
        ]),
        vec!["Rust".to_string(), "Web".to_string()],
    );
}
