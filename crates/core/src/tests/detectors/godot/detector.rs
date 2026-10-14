//! Tests for [`crate::detectors::godot::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::godot::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-godot-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn godot_info(dir: &Path) -> crate::detectors::godot::GodotInfo {
    let tracker = GodotDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Godot(info) => info,
        other => panic!("expected Tracker::Godot, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = GodotDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_the_project_name() {
    let dir = temp_dir("name");
    fs::write(
        dir.join("project.godot"),
        "config_version=5\n\n[application]\n\nconfig/name=\"My Game\"\nconfig/features=PackedStringArray(\"4.2\")\n",
    )
    .expect("should write project.godot");

    assert_eq!(godot_info(&dir).project_name.as_deref(), Some("My Game"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_recognizes_a_project_godot_without_a_name() {
    let dir = temp_dir("no-name");
    fs::write(
        dir.join("project.godot"),
        "config_version=5\n\n[rendering]\n",
    )
    .expect("should write project.godot");

    // The file's presence is the signal; a missing name is just absent.
    assert_eq!(godot_info(&dir).project_name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_name_outside_the_application_section_is_ignored() {
    let dir = temp_dir("wrong-section");
    fs::write(
        dir.join("project.godot"),
        "[autoload]\nconfig/name=\"not-the-project-name\"\n",
    )
    .expect("should write project.godot");

    assert_eq!(godot_info(&dir).project_name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_godot() {
    assert_eq!(GodotDetector.kind(), "godot");
}
