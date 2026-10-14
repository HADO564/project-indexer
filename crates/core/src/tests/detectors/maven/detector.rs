//! Tests for [`crate::detectors::maven::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::maven::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-maven-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn maven_info(dir: &Path) -> crate::detectors::maven::MavenInfo {
    let tracker = MavenDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Maven(info) => info,
        other => panic!("expected Tracker::Maven, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = MavenDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_artifact_id_and_version() {
    let dir = temp_dir("fields");
    fs::write(
        dir.join("pom.xml"),
        "<project>\n  <groupId>com.example</groupId>\n  <artifactId>my-app</artifactId>\n  <version>1.2.3</version>\n</project>\n",
    )
    .expect("should write pom.xml");

    let info = maven_info(&dir);

    assert_eq!(info.artifact_id.as_deref(), Some("my-app"));
    assert_eq!(info.version.as_deref(), Some("1.2.3"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_degrades_a_malformed_pom_to_empty_fields() {
    let dir = temp_dir("malformed");
    fs::write(dir.join("pom.xml"), "<project> not closed").expect("should write pom.xml");

    // The manifest's presence is the signal, so a broken file is still a Maven
    // project — with no parsed fields — rather than a detection failure.
    let info = maven_info(&dir);

    assert_eq!(info.artifact_id, None);
    assert_eq!(info.version, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_maven() {
    assert_eq!(MavenDetector.kind(), "maven");
}
