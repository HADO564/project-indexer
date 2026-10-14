//! Tests for [`crate::detectors::ruby::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::ruby::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-ruby-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn ruby_info(dir: &Path) -> crate::detectors::ruby::RubyInfo {
    let tracker = RubyDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Ruby(info) => info,
        other => panic!("expected Tracker::Ruby, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = RubyDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_recognizes_a_gemfile_without_a_ruby_directive() {
    let dir = temp_dir("no-version");
    fs::write(
        dir.join("Gemfile"),
        "source \"https://rubygems.org\"\ngem \"rails\"\n",
    )
    .expect("should write Gemfile");

    // A Gemfile that pins no interpreter is still a Ruby project.
    assert_eq!(ruby_info(&dir).ruby_version, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_the_pinned_ruby_version() {
    let dir = temp_dir("version");
    fs::write(
        dir.join("Gemfile"),
        "source \"https://rubygems.org\"\nruby \"3.2.0\"\ngem \"rails\"\n",
    )
    .expect("should write Gemfile");

    assert_eq!(ruby_info(&dir).ruby_version.as_deref(), Some("3.2.0"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_gem_named_like_ruby_is_not_mistaken_for_the_directive() {
    let dir = temp_dir("lookalike");
    fs::write(dir.join("Gemfile"), "gem \"ruby-lsp\"\n").expect("should write Gemfile");

    assert_eq!(ruby_info(&dir).ruby_version, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_ruby() {
    assert_eq!(RubyDetector.kind(), "ruby");
}
