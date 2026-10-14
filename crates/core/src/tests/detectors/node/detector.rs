//! Tests for [`crate::detectors::node::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::node::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-node-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn write_manifest(dir: &Path, contents: &str) {
    fs::write(dir.join("package.json"), contents).expect("should write package.json");
}

fn node_info(dir: &Path) -> crate::detectors::node::NodeInfo {
    let tracker = NodeDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Node(info) => info,
        other => panic!("expected Tracker::Node, got {other:?}"),
    }
}

#[test]
fn detect_recognizes_a_directory_with_a_package_json() {
    let dir = temp_dir("detect-true");
    write_manifest(&dir, "{}");

    let result = NodeDetector.detect(&dir).expect("should detect");

    assert!(matches!(result, Some(Tracker::Node(_))));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = NodeDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_name_and_version() {
    let dir = temp_dir("fields");
    write_manifest(
        &dir,
        r#"{ "name": "my-app", "version": "1.2.3", "private": true }"#,
    );

    let info = node_info(&dir);

    assert_eq!(info.name.as_deref(), Some("my-app"));
    assert_eq!(info.version.as_deref(), Some("1.2.3"));
    assert!(info.manifest_path.ends_with("package.json"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_treats_missing_fields_as_absent() {
    let dir = temp_dir("missing-fields");
    write_manifest(&dir, "{}");

    let info = node_info(&dir);

    assert_eq!(info.name, None);
    assert_eq!(info.version, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_degrades_a_malformed_manifest_to_empty_fields() {
    let dir = temp_dir("malformed");
    write_manifest(&dir, "{ this is not valid json");

    // The manifest's presence is the signal, so a broken file is still a Node
    // project — with no parsed fields — rather than a detection failure.
    let info = node_info(&dir);

    assert_eq!(info.name, None);
    assert_eq!(info.version, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_node() {
    assert_eq!(NodeDetector.kind(), "node");
}
