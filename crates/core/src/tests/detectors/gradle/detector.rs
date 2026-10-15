//! Tests for [`crate::detectors::gradle::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::gradle::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-gradle-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn gradle_info(dir: &Path) -> crate::detectors::gradle::GradleInfo {
    let tracker = GradleDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Gradle(info) => info,
        other => panic!("expected Tracker::Gradle, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = GradleDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_records_the_groovy_build_script() {
    let dir = temp_dir("groovy");
    fs::write(dir.join("build.gradle"), "plugins { id 'java' }\n").expect("should write script");

    assert_eq!(gradle_info(&dir).marker, "build.gradle");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_records_the_kotlin_build_script() {
    let dir = temp_dir("kotlin");
    fs::write(dir.join("build.gradle.kts"), "plugins { java }\n").expect("should write script");

    assert_eq!(gradle_info(&dir).marker, "build.gradle.kts");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_gradle() {
    assert_eq!(GradleDetector.kind(), "gradle");
}
