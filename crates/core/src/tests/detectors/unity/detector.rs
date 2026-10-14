//! Tests for [`crate::detectors::unity::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::unity::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-unity-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn write_version(dir: &Path, contents: &str) {
    let settings = dir.join("ProjectSettings");
    fs::create_dir_all(&settings).expect("should create ProjectSettings");
    fs::write(settings.join("ProjectVersion.txt"), contents).expect("should write version file");
}

fn unity_info(dir: &Path) -> crate::detectors::unity::UnityInfo {
    let tracker = UnityDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Unity(info) => info,
        other => panic!("expected Tracker::Unity, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = UnityDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_the_editor_version() {
    let dir = temp_dir("version");
    write_version(
        &dir,
        "m_EditorVersion: 2021.3.0f1\nm_EditorVersionWithRevision: 2021.3.0f1 (abc123)\n",
    );

    // Only `m_EditorVersion` is read — not `m_EditorVersionWithRevision`.
    assert_eq!(
        unity_info(&dir).editor_version.as_deref(),
        Some("2021.3.0f1")
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_recognizes_a_version_file_with_no_version_line() {
    let dir = temp_dir("no-version");
    write_version(&dir, "# empty\n");

    assert_eq!(unity_info(&dir).editor_version, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_unity() {
    assert_eq!(UnityDetector.kind(), "unity");
}
