//! Tests for [`crate::detectors::python::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::python::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-python-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn python_info(dir: &Path) -> crate::detectors::python::PythonInfo {
    let tracker = PythonDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Python(info) => info,
        other => panic!("expected Tracker::Python, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = PythonDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_the_name_from_pyproject() {
    let dir = temp_dir("pyproject-name");
    fs::write(
        dir.join("pyproject.toml"),
        "[build-system]\nrequires = [\"setuptools\"]\n\n[project]\nname = \"my-pkg\"\nversion = \"0.1.0\"\n",
    )
    .expect("should write pyproject.toml");

    let info = python_info(&dir);

    assert_eq!(info.marker, "pyproject.toml");
    assert_eq!(info.name.as_deref(), Some("my-pkg"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_recognizes_setup_py_without_a_name() {
    let dir = temp_dir("setup-py");
    fs::write(
        dir.join("setup.py"),
        "from setuptools import setup\nsetup()\n",
    )
    .expect("should write setup.py");

    let info = python_info(&dir);

    // `setup.py` proves the project without declaring a name in a fixed place.
    assert_eq!(info.marker, "setup.py");
    assert_eq!(info.name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_recognizes_requirements_txt() {
    let dir = temp_dir("requirements");
    fs::write(dir.join("requirements.txt"), "requests==2.31.0\n")
        .expect("should write requirements.txt");

    let info = python_info(&dir);

    assert_eq!(info.marker, "requirements.txt");
    assert_eq!(info.name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn pyproject_wins_over_the_weaker_markers() {
    let dir = temp_dir("priority");
    fs::write(dir.join("requirements.txt"), "requests\n").expect("should write requirements.txt");
    fs::write(dir.join("setup.py"), "setup()\n").expect("should write setup.py");
    fs::write(dir.join("pyproject.toml"), "[project]\nname = \"winner\"\n")
        .expect("should write pyproject.toml");

    let info = python_info(&dir);

    assert_eq!(info.marker, "pyproject.toml");
    assert_eq!(info.name.as_deref(), Some("winner"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_degrades_a_malformed_pyproject_to_no_name() {
    let dir = temp_dir("malformed");
    fs::write(dir.join("pyproject.toml"), "this is not = valid [ toml")
        .expect("should write pyproject.toml");

    // The file's presence is the signal; an unreadable name is just absent.
    let info = python_info(&dir);

    assert_eq!(info.marker, "pyproject.toml");
    assert_eq!(info.name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_single_quoted_name_is_read_too() {
    let dir = temp_dir("single-quote");
    fs::write(dir.join("pyproject.toml"), "[project]\nname = 'quoted'\n")
        .expect("should write pyproject.toml");

    assert_eq!(python_info(&dir).name.as_deref(), Some("quoted"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_python() {
    assert_eq!(PythonDetector.kind(), "python");
}
