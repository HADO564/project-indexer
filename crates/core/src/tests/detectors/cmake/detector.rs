//! Tests for [`crate::detectors::cmake::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::cmake::detector::*;
use crate::detectors::detector::Detector;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-cmake-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn cmake_info(dir: &Path) -> crate::detectors::cmake::CmakeInfo {
    let tracker = CmakeDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Cmake(info) => info,
        other => panic!("expected Tracker::Cmake, got {other:?}"),
    }
}

#[test]
fn detect_recognizes_a_directory_with_a_cmakelists() {
    let dir = temp_dir("detect-true");
    fs::write(
        dir.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.10)\n",
    )
    .expect("should write CMakeLists.txt");

    let result = CmakeDetector.detect(&dir).expect("should detect");

    assert!(matches!(result, Some(Tracker::Cmake(_))));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = CmakeDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_the_project_name() {
    let dir = temp_dir("project-name");
    fs::write(
        dir.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.10)\nproject(MyApp VERSION 1.0 LANGUAGES CXX)\n",
    )
    .expect("should write CMakeLists.txt");

    assert_eq!(cmake_info(&dir).project_name.as_deref(), Some("MyApp"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_commented_project_command_is_not_mistaken_for_one() {
    let dir = temp_dir("commented");
    fs::write(
        dir.join("CMakeLists.txt"),
        "# project(NotReal)\ncmake_minimum_required(VERSION 3.10)\n",
    )
    .expect("should write CMakeLists.txt");

    // The file is present, so it is still a CMake project — with no name.
    assert_eq!(cmake_info(&dir).project_name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_degrades_a_listfile_with_no_project_to_no_name() {
    let dir = temp_dir("no-project");
    fs::write(dir.join("CMakeLists.txt"), "not even cmake\n").expect("should write CMakeLists.txt");

    assert_eq!(cmake_info(&dir).project_name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_cmake() {
    assert_eq!(CmakeDetector.kind(), "cmake");
}
