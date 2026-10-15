//! Tests for [`crate::detectors::docker::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::docker::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-docker-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn docker_info(dir: &Path) -> crate::detectors::docker::DockerInfo {
    let tracker = DockerDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Docker(info) => info,
        other => panic!("expected Tracker::Docker, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = DockerDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_records_a_dockerfile() {
    let dir = temp_dir("dockerfile");
    fs::write(dir.join("Dockerfile"), "FROM scratch\n").expect("should write Dockerfile");

    assert_eq!(docker_info(&dir).marker, "Dockerfile");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_records_a_compose_file() {
    let dir = temp_dir("compose");
    fs::write(dir.join("compose.yaml"), "services: {}\n").expect("should write compose.yaml");

    assert_eq!(docker_info(&dir).marker, "compose.yaml");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_dockerfile_wins_over_a_compose_file() {
    let dir = temp_dir("both");
    fs::write(dir.join("Dockerfile"), "FROM scratch\n").expect("should write Dockerfile");
    fs::write(dir.join("docker-compose.yml"), "services: {}\n").expect("should write compose");

    // The image recipe is the strongest signal, so it is reported first.
    assert_eq!(docker_info(&dir).marker, "Dockerfile");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_docker() {
    assert_eq!(DockerDetector.kind(), "docker");
}
