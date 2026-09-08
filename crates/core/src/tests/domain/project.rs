//! Tests for [`crate::domain::project`].

use crate::domain::update_project::UpdateProject;
use crate::error::ProjectError;
use chrono::Utc;
use std::collections::BTreeMap;

use crate::domain::project::*;

fn project_with_dir(directory: &str) -> Project {
    Project {
        is_deleted: false,
        id: "id".to_string(),
        name: "name".to_string(),
        description: String::new(),
        directory: directory.to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_opened_at: None,
        tags: Vec::new(),
        favorite: false,
        open_with: None,
        notes: None,
        properties: BTreeMap::new(),
        trackers: Vec::new(),
        group_id: None,
        color: None,
        icon: None,
    }
}

#[test]
fn rejects_directory_that_only_differs_by_trailing_separator() {
    let existing = vec![project_with_dir("D:\\Projects\\Friction")];
    let result = Project::check_for_duplicate_name_or_dir(
        "OtherName",
        "D:\\Projects\\Friction\\",
        &existing,
    );
    assert!(matches!(result, Err(ProjectError::DuplicateDirectory(_))));
}

#[test]
fn allows_distinct_directories() {
    let existing = vec![project_with_dir("D:\\Projects\\Friction")];
    let result = Project::check_for_duplicate_name_or_dir(
        "OtherName",
        "D:\\Projects\\OtherProject",
        &existing,
    );
    assert!(result.is_ok());
}

#[test]
fn directory_health_flags_missing_directory_as_deleted_or_moved() {
    let missing = std::env::temp_dir().join("project-indexer-tests-missing-dir-xyz");
    let result = Project::check_directory_health(missing.to_str().unwrap());
    assert!(matches!(
        result,
        Err(ProjectError::DirectoryDeletedOrMoved(_))
    ));
}

#[test]
fn directory_health_flags_a_file_in_place_of_the_directory_as_deleted_or_moved() {
    let file = std::env::temp_dir().join("project-indexer-tests-file-not-dir.txt");
    std::fs::write(&file, b"placeholder").expect("should write temp file");

    let result = Project::check_directory_health(file.to_str().unwrap());

    std::fs::remove_file(&file).expect("should clean up temp file");
    assert!(matches!(
        result,
        Err(ProjectError::DirectoryDeletedOrMoved(_))
    ));
}

#[test]
fn directory_health_ok_for_a_real_directory() {
    let dir = std::env::temp_dir();
    let result = Project::check_directory_health(dir.to_str().unwrap());
    assert!(result.is_ok());
}
#[test]
fn mark_deleted_sets_the_flag() {
    let mut project = project_with_dir("D:\\Projects\\Friction");
    assert!(!project.is_deleted);

    project.mark_deleted();

    assert!(project.is_deleted);
}

#[test]
fn restore_clears_the_flag() {
    let mut project = project_with_dir("D:\\Projects\\Friction");
    project.mark_deleted();

    project.restore();

    assert!(!project.is_deleted);
}

/// The oldest record shape still in the wild: identity and timestamps only.
const LEGACY_RECORD: &str = r#"{
    "id": "e4df90f6",
    "name": "Legacy",
    "directory": "/tmp/legacy",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z"
}"#;

#[test]
fn loads_a_record_missing_every_absorbable_field() {
    let project: Project = serde_json::from_str(LEGACY_RECORD).expect(
        "a new field on Project must be Option<T> or #[serde(default)], \
         otherwise every already-stored project fails to load",
    );

    assert!(!project.is_deleted);
    assert!(!project.favorite);
    assert!(project.tags.is_empty());
    assert!(project.description.is_empty());
    assert!(project.last_opened_at.is_none());
    assert!(project.trackers.is_empty());
    assert!(project.group_id.is_none());
    assert!(project.color.is_none());
    assert!(project.icon.is_none());
}

#[test]
fn rejects_a_record_missing_its_identity() {
    // The other half of the contract: absorbable fields default, but a
    // record with no id is corrupt and must not load as a blank project.
    let corrupt = r#"{
        "name": "No id",
        "directory": "/tmp/x",
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }"#;

    assert!(serde_json::from_str::<Project>(corrupt).is_err());
}

#[test]
fn new_project_has_no_group_colour_or_icon() {
    let p = Project::new(
        "Name".into(),
        std::env::temp_dir().to_string_lossy().into_owned(),
        None,
        None,
    )
    .expect("temp dir exists");
    assert!(p.group_id.is_none());
    assert!(p.color.is_none());
    assert!(p.icon.is_none());
}

#[test]
fn update_sets_and_clears_group_colour_and_icon() {
    let mut p = project_with_dir(&std::env::temp_dir().to_string_lossy());

    p.update(UpdateProject {
        name: None,
        directory: None,
        description: None,
        tags: None,
        favorite: None,
        open_with: None,
        notes: None,
        properties: None,
        group_id: Some(Some("group-1".into())),
        color: Some(Some("gold".into())),
        icon: Some(Some("gamepad".into())),
    })
    .expect("valid update");
    assert_eq!(p.group_id.as_deref(), Some("group-1"));
    assert_eq!(p.color.as_deref(), Some("gold"));
    assert_eq!(p.icon.as_deref(), Some("gamepad"));

    // An explicit null clears; an absent key leaves it alone.
    p.update(UpdateProject {
        name: None,
        directory: None,
        description: None,
        tags: None,
        favorite: None,
        open_with: None,
        notes: None,
        properties: None,
        group_id: Some(None),
        color: None,
        icon: None,
    })
    .expect("valid update");
    assert!(p.group_id.is_none());
    assert_eq!(p.color.as_deref(), Some("gold"));
}

#[test]
fn update_rejects_an_unknown_colour() {
    let mut p = project_with_dir(&std::env::temp_dir().to_string_lossy());
    let result = p.update(UpdateProject {
        name: None,
        directory: None,
        description: None,
        tags: None,
        favorite: None,
        open_with: None,
        notes: None,
        properties: None,
        group_id: None,
        color: Some(Some("chartreuse".into())),
        icon: None,
    });
    assert!(matches!(result, Err(ProjectError::UnknownSwatch(_))));
    assert!(p.color.is_none());
}
