//! Tests for [`crate::detectors::unreal::unreal`].

use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use std::fs;
use std::path::{Path, PathBuf};

use crate::detectors::unreal::unreal::*;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-unreal-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn write_uproject(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(format!("{name}.uproject"));
    fs::write(&path, contents).expect("should write .uproject file");
    path
}

#[test]
fn find_project_file_locates_a_uproject_in_the_directory() {
    let dir = temp_dir("find-project-file");
    let uproject = write_uproject(&dir, "MyGame", "{}");

    let found = find_project_file(&dir).expect("should read dir");

    assert_eq!(found, Some(uproject));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn find_project_file_returns_none_without_a_uproject() {
    let dir = temp_dir("find-project-file-none");

    let found = find_project_file(&dir).expect("should read dir");

    assert_eq!(found, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_recognizes_a_directory_with_a_uproject() {
    let dir = temp_dir("detect-true");
    write_uproject(&dir, "MyGame", "{}");

    let result = UnrealDetector.detect(&dir).expect("should detect");

    assert!(matches!(result, Some(Tracker::Unreal(_))));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_returns_none_for_a_plain_directory() {
    let dir = temp_dir("detect-false");

    let result = UnrealDetector.detect(&dir).expect("should detect");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_returns_none_without_a_uproject() {
    let dir = temp_dir("get-info-none");

    let result = UnrealDetector.detect(&dir).expect("should get info");

    assert!(result.is_none());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_parses_descriptor_fields() {
    let dir = temp_dir("get-info-fields");
    write_uproject(
        &dir,
        "MyGame",
        r#"{
            "EngineAssociation": "5.3",
            "Category": "Shooter",
            "Description": "A test project",
            "Modules": [{"Name": "MyGame", "Type": "Runtime", "LoadingPhase": "Default"}],
            "Plugins": [
                {"Name": "EnabledPlugin", "Enabled": true},
                {"Name": "DisabledPlugin", "Enabled": false}
            ]
        }"#,
    );

    let tracker = UnrealDetector
        .detect(&dir)
        .expect("should get info")
        .expect("should find project");

    let Tracker::Unreal(info) = tracker else {
        panic!("expected Tracker::Unreal");
    };

    assert_eq!(info.project_name, "MyGame");
    assert_eq!(info.engine_association.as_deref(), Some("5.3"));
    assert_eq!(info.category.as_deref(), Some("Shooter"));
    assert_eq!(info.description.as_deref(), Some("A test project"));
    assert_eq!(info.modules, vec!["MyGame".to_string()]);
    assert_eq!(info.plugins, vec!["EnabledPlugin".to_string()]);
    assert_eq!(info.vcs_provider, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_treats_missing_optional_fields_as_absent() {
    let dir = temp_dir("get-info-missing-fields");
    write_uproject(&dir, "MyGame", "{}");

    let tracker = UnrealDetector
        .detect(&dir)
        .expect("should get info")
        .expect("should find project");

    let Tracker::Unreal(info) = tracker else {
        panic!("expected Tracker::Unreal");
    };

    assert_eq!(info.engine_association, None);
    assert_eq!(info.category, None);
    assert_eq!(info.description, None);
    assert!(info.modules.is_empty());
    assert!(info.plugins.is_empty());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_reads_the_configured_source_control_provider() {
    let dir = temp_dir("get-info-vcs");
    write_uproject(&dir, "MyGame", "{}");

    let config_dir = dir.join("Saved").join("Config").join("WindowsEditor");
    fs::create_dir_all(&config_dir).expect("should create config dir");
    fs::write(
        config_dir.join("SourceControlSettings.ini"),
        "[SourceControl.SourceControlSettings]\nProvider=Perforce\n",
    )
    .expect("should write ini");

    let tracker = UnrealDetector
        .detect(&dir)
        .expect("should get info")
        .expect("should find project");

    let Tracker::Unreal(info) = tracker else {
        panic!("expected Tracker::Unreal");
    };

    assert_eq!(info.vcs_provider.as_deref(), Some("Perforce"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn detect_treats_an_explicit_none_provider_as_absent() {
    let dir = temp_dir("get-info-vcs-none");
    write_uproject(&dir, "MyGame", "{}");

    let config_dir = dir.join("Saved").join("Config").join("WindowsEditor");
    fs::create_dir_all(&config_dir).expect("should create config dir");
    fs::write(
        config_dir.join("SourceControlSettings.ini"),
        "[SourceControl.SourceControlSettings]\nProvider=None\n",
    )
    .expect("should write ini");

    let tracker = UnrealDetector
        .detect(&dir)
        .expect("should get info")
        .expect("should find project");

    let Tracker::Unreal(info) = tracker else {
        panic!("expected Tracker::Unreal");
    };

    assert_eq!(info.vcs_provider, None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kind_is_unreal() {
    assert_eq!(UnrealDetector.kind(), "unreal");
}
