//! Tests for [`crate::detectors::rust::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::rust::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-rust-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn rust_info(dir: &Path) -> crate::detectors::rust::RustInfo {
    let tracker = RustDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Rust(info) => info,
        other => panic!("expected Tracker::Rust, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = RustDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_the_package_name() {
    let dir = temp_dir("package-name");
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"my-crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("should write Cargo.toml");

    assert_eq!(rust_info(&dir).name.as_deref(), Some("my-crate"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_recognizes_a_workspace_root_with_no_package() {
    let dir = temp_dir("workspace");
    fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .expect("should write Cargo.toml");

    // A virtual manifest declares no `[package]`, but it is still Rust.
    assert_eq!(rust_info(&dir).name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_name_in_another_table_is_not_mistaken_for_the_package_name() {
    let dir = temp_dir("other-table");
    fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = []\n\n[workspace.dependencies]\nname = \"not-it\"\n",
    )
    .expect("should write Cargo.toml");

    assert_eq!(rust_info(&dir).name, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_rust() {
    assert_eq!(RustDetector.kind(), "rust");
}
