//! Tests for [`crate::platform::app_discovery`]'s macOS scan, which runs
//! anywhere: it only reads folders, and these build fake `.app` bundles.

use std::fs;
use std::path::{Path, PathBuf};

use crate::platform::app_discovery::{is_app_bundle, scan_app_bundles};

fn bundle(dir: &Path, relative: &str) -> PathBuf {
    let path = dir.join(relative);
    fs::create_dir_all(path.join("Contents").join("MacOS")).unwrap();
    path
}

fn names(dirs: &[PathBuf]) -> Vec<String> {
    scan_app_bundles(dirs)
        .into_iter()
        .map(|app| app.name)
        .collect()
}

#[test]
fn bundles_are_named_without_app_and_stored_by_path() {
    let root = tempfile::tempdir().unwrap();
    let wezterm = bundle(root.path(), "WezTerm.app");
    let apps = scan_app_bundles(&[root.path().to_path_buf()]);
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].name, "WezTerm");
    assert_eq!(apps[0].path, wezterm.to_string_lossy());
}

#[test]
fn bundles_in_subfolders_are_found_two_levels_down() {
    let root = tempfile::tempdir().unwrap();
    bundle(root.path(), "Utilities/Terminal.app");
    bundle(root.path(), "Adobe/Photoshop 2026/Photoshop.app");
    bundle(root.path(), "a/b/c/TooDeep.app");
    assert_eq!(
        names(&[root.path().to_path_buf()]),
        ["Photoshop", "Terminal"]
    );
}

#[test]
fn a_bundle_is_never_looked_inside() {
    let root = tempfile::tempdir().unwrap();
    let outer = bundle(root.path(), "Xcode.app");
    bundle(&outer, "Contents/Applications/Simulator.app");
    assert_eq!(names(&[root.path().to_path_buf()]), ["Xcode"]);
}

#[test]
fn files_and_plain_folders_are_not_apps() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Notes.app"), "").unwrap();
    fs::create_dir(root.path().join("Projects")).unwrap();
    assert!(names(&[root.path().to_path_buf()]).is_empty());
}

#[test]
fn the_first_folder_wins_a_name_both_have_and_the_list_is_sorted() {
    let user = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    let mine = bundle(user.path(), "Firefox.app");
    bundle(system.path(), "Firefox.app");
    bundle(system.path(), "calculator.app");
    bundle(system.path(), "Zed.app");
    let apps = scan_app_bundles(&[user.path().to_path_buf(), system.path().to_path_buf()]);
    let listed: Vec<&str> = apps.iter().map(|app| app.name.as_str()).collect();
    assert_eq!(
        listed,
        ["calculator", "Firefox", "Zed"],
        "sorted ignoring case"
    );
    let firefox = apps.iter().find(|app| app.name == "Firefox").unwrap();
    assert_eq!(firefox.path, mine.to_string_lossy());
}

#[test]
fn a_missing_folder_is_skipped() {
    let root = tempfile::tempdir().unwrap();
    bundle(root.path(), "Zed.app");
    let dirs = [root.path().join("not-here"), root.path().to_path_buf()];
    assert_eq!(names(&dirs), ["Zed"]);
}

#[test]
fn an_app_bundle_is_a_folder_ending_in_app() {
    let root = tempfile::tempdir().unwrap();
    assert!(is_app_bundle(&bundle(root.path(), "Zed.app")));
    assert!(is_app_bundle(&bundle(root.path(), "Shouty.APP")));
    fs::write(root.path().join("file.app"), "").unwrap();
    assert!(!is_app_bundle(&root.path().join("file.app")));
    assert!(!is_app_bundle(root.path()));
}
