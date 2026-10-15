//! Tests for [`crate::detectors::elixir::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::elixir::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-elixir-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn elixir_info(dir: &Path) -> crate::detectors::elixir::ElixirInfo {
    let tracker = ElixirDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Elixir(info) => info,
        other => panic!("expected Tracker::Elixir, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = ElixirDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_the_app_name() {
    let dir = temp_dir("app");
    fs::write(
        dir.join("mix.exs"),
        "defmodule MyApp.MixProject do\n  use Mix.Project\n\n  def project do\n    [\n      app: :my_app,\n      version: \"0.1.0\"\n    ]\n  end\nend\n",
    )
    .expect("should write mix.exs");

    assert_eq!(elixir_info(&dir).app.as_deref(), Some("my_app"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_degrades_a_manifest_with_no_app_to_none() {
    let dir = temp_dir("no-app");
    fs::write(
        dir.join("mix.exs"),
        "defmodule MyApp.MixProject do\n  use Mix.Project\nend\n",
    )
    .expect("should write mix.exs");

    assert_eq!(elixir_info(&dir).app, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_elixir() {
    assert_eq!(ElixirDetector.kind(), "elixir");
}
