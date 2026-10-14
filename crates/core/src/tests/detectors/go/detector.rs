//! Tests for [`crate::detectors::go::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::go::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-go-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn go_info(dir: &Path) -> crate::detectors::go::GoInfo {
    let tracker = GoDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Go(info) => info,
        other => panic!("expected Tracker::Go, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = GoDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_the_module_path() {
    let dir = temp_dir("module");
    fs::write(
        dir.join("go.mod"),
        "module github.com/user/thing\n\ngo 1.22\n",
    )
    .expect("should write go.mod");

    assert_eq!(
        go_info(&dir).module.as_deref(),
        Some("github.com/user/thing")
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_recognizes_a_go_mod_with_no_module_directive() {
    let dir = temp_dir("no-module");
    fs::write(dir.join("go.mod"), "go 1.22\n").expect("should write go.mod");

    // A `go.mod` that is present but declares no module is still a Go project.
    assert_eq!(go_info(&dir).module, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_directive_merely_starting_with_module_is_not_mistaken_for_one() {
    let dir = temp_dir("lookalike");
    fs::write(dir.join("go.mod"), "modulefoo = bar\n").expect("should write go.mod");

    assert_eq!(go_info(&dir).module, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_go() {
    assert_eq!(GoDetector.kind(), "go");
}
