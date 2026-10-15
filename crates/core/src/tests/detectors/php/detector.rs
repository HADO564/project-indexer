//! Tests for [`crate::detectors::php::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::php::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-php-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn php_info(dir: &Path) -> crate::detectors::php::PhpInfo {
    let tracker = PhpDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Php(info) => info,
        other => panic!("expected Tracker::Php, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = PhpDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_the_package_name() {
    let dir = temp_dir("name");
    fs::write(
        dir.join("composer.json"),
        r#"{ "name": "vendor/package", "require": {} }"#,
    )
    .expect("should write composer.json");

    assert_eq!(php_info(&dir).name.as_deref(), Some("vendor/package"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_treats_a_nameless_manifest_as_absent() {
    let dir = temp_dir("no-name");
    fs::write(dir.join("composer.json"), "{}").expect("should write composer.json");

    assert_eq!(php_info(&dir).name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_degrades_a_malformed_manifest_to_no_name() {
    let dir = temp_dir("malformed");
    fs::write(dir.join("composer.json"), "{ not valid json").expect("should write composer.json");

    // The manifest's presence is the signal, so a broken file is still a PHP
    // project — with no name — rather than a detection failure.
    assert_eq!(php_info(&dir).name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_php() {
    assert_eq!(PhpDetector.kind(), "php");
}
