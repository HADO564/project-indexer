//! Tests for [`crate::detectors::swift::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::swift::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-swift-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn swift_info(dir: &Path) -> crate::detectors::swift::SwiftInfo {
    let tracker = SwiftDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Swift(info) => info,
        other => panic!("expected Tracker::Swift, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = SwiftDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_the_package_name() {
    let dir = temp_dir("name");
    fs::write(
        dir.join("Package.swift"),
        "// swift-tools-version:5.7\nimport PackageDescription\n\nlet package = Package(\n    name: \"MyLibrary\",\n    targets: [ .target(name: \"MyLibrary\") ]\n)\n",
    )
    .expect("should write Package.swift");

    assert_eq!(swift_info(&dir).name.as_deref(), Some("MyLibrary"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_degrades_a_manifest_with_no_name_to_none() {
    let dir = temp_dir("no-name");
    fs::write(
        dir.join("Package.swift"),
        "// swift-tools-version:5.7\nimport PackageDescription\n",
    )
    .expect("should write Package.swift");

    assert_eq!(swift_info(&dir).name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_swift() {
    assert_eq!(SwiftDetector.kind(), "swift");
}
