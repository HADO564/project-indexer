//! Tests for [`crate::detectors::dart::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::dart::detector::*;
use crate::detectors::detector::Detector;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-dart-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn dart_info(dir: &Path) -> crate::detectors::dart::DartInfo {
    let tracker = DartDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Dart(info) => info,
        other => panic!("expected Tracker::Dart, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = DartDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_name_and_version() {
    let dir = temp_dir("fields");
    fs::write(
        dir.join("pubspec.yaml"),
        "name: my_app\ndescription: A sample app.\nversion: 1.0.0+1\n\ndependencies:\n  flutter:\n    sdk: flutter\n",
    )
    .expect("should write pubspec.yaml");

    let info = dart_info(&dir);

    assert_eq!(info.name.as_deref(), Some("my_app"));
    assert_eq!(info.version.as_deref(), Some("1.0.0+1"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_indented_name_is_not_mistaken_for_the_top_level_one() {
    let dir = temp_dir("nested");
    fs::write(
        dir.join("pubspec.yaml"),
        "dependencies:\n  some_pkg:\n    name: not_the_project\n",
    )
    .expect("should write pubspec.yaml");

    // Only column-zero keys count, so the nested `name:` is ignored.
    assert_eq!(dart_info(&dir).name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_degrades_a_manifest_with_no_fields_to_none() {
    let dir = temp_dir("empty");
    fs::write(dir.join("pubspec.yaml"), "# nothing useful here\n")
        .expect("should write pubspec.yaml");

    let info = dart_info(&dir);

    assert_eq!(info.name, None);
    assert_eq!(info.version, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_dart() {
    assert_eq!(DartDetector.kind(), "dart");
}
