//! Tests for [`crate::detectors::dotnet::detector`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::detector::Detector;
use crate::detectors::dotnet::detector::*;
use crate::domain::Tracker;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-dotnet-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn dotnet_info(dir: &Path) -> crate::detectors::dotnet::DotnetInfo {
    let tracker = DotnetDetector
        .detect(dir)
        .expect("should detect")
        .expect("should recognize the project");
    match tracker {
        Tracker::Dotnet(info) => info,
        other => panic!("expected Tracker::Dotnet, got {other:?}"),
    }
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = DotnetDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_records_a_csproj_by_its_filename() {
    let dir = temp_dir("csproj");
    fs::write(
        dir.join("App.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\" />\n",
    )
    .expect("should write App.csproj");

    assert_eq!(dotnet_info(&dir).marker, "App.csproj");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_solution_wins_over_a_project() {
    let dir = temp_dir("sln");
    fs::write(dir.join("App.csproj"), "<Project />\n").expect("should write App.csproj");
    fs::write(
        dir.join("App.sln"),
        "Microsoft Visual Studio Solution File\n",
    )
    .expect("should write App.sln");

    // A solution names the whole thing, so it is the stronger marker.
    assert_eq!(dotnet_info(&dir).marker, "App.sln");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_global_json_alone_is_enough() {
    let dir = temp_dir("global");
    fs::write(
        dir.join("global.json"),
        "{ \"sdk\": { \"version\": \"8.0.0\" } }\n",
    )
    .expect("should write global.json");

    assert_eq!(dotnet_info(&dir).marker, "global.json");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_dotnet() {
    assert_eq!(DotnetDetector.kind(), "dotnet");
}
