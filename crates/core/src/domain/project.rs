use crate::domain::normalize::{normalize_directory, normalize_tags, remove_spaces};
use crate::domain::tracker::Tracker;
use crate::domain::update_project::UpdateProject;
use crate::error::ProjectError;
use crate::platform::filesystem::{check_directory_status, DirectoryStatus};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
/// A stored project.
///
/// Records written by older builds simply lack every field added since, so a
/// new field has to be absorbable on read: either `Option<T>`, which serde
/// reads as `None` when the key is missing, or `#[serde(default)]`. Adding a
/// bare `bool`, `String` or `Vec` without one of those makes every existing
/// record fail to load — the whole project disappears from the app, rather
/// than just the new field being empty.
///
/// The identifying fields are deliberately left strict. A record with no `id`
/// or `directory` is corrupt, and should fail loudly instead of loading as a
/// blank project.
///
/// A change that can't be absorbed this way — a rename, a type change, a field
/// split — needs a dedicated migration step and a schema-version bump.
/// `loads_a_record_missing_every_absorbable_field` in this module's tests
/// fails the moment a new field breaks the rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    #[serde(default)]
    pub is_deleted: bool,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub directory: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_opened_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
    pub open_with: Option<String>,
    pub notes: Option<String>,
    pub client: Option<String>,
    #[serde(default)]
    pub trackers: Vec<Tracker>,
}

/// Assigns each listed field from `$update` onto `$self` only when present
/// (`Some`), leaving unset fields unchanged.
macro_rules! apply_if_present {
    ($self:ident, $update:ident, $($field:ident),+ $(,)?) => {
        $(
            if let Some(value) = $update.$field {
                $self.$field = value;
            }
        )+
    };
}

impl Project {
    pub fn check_for_duplicate_name_or_dir(
        name: &str,
        directory: &str,
        existing: &[Project],
    ) -> Result<(), ProjectError> {
        let normalized_directory = normalize_directory(directory);
        if existing
            .iter()
            .any(|p| normalize_directory(&p.directory) == normalized_directory)
        {
            return Err(ProjectError::DuplicateDirectory(directory.to_string()));
        }

        if existing
            .iter()
            .any(|p| p.name.trim().eq_ignore_ascii_case(name.trim()))
        {
            return Err(ProjectError::DuplicateName(name.to_string()));
        }

        Ok(())
    }

    fn validate_name(name: &str) -> Result<(), ProjectError> {
        if name.trim().is_empty() {
            return Err(ProjectError::InvalidName);
        }
        Ok(())
    }

    fn validate_directory(directory: &str) -> Result<(), ProjectError> {
        match check_directory_status(directory) {
            DirectoryStatus::Exists => Ok(()),
            _ => Err(ProjectError::InvalidDirectory(directory.to_string())),
        }
    }

    /// Checks that an already-registered project's directory can still be
    /// opened, distinguishing "it's gone" (deleted, or moved somewhere else)
    /// from "it's there but we can't get into it" (permissions, an offline
    /// network drive, etc). Unlike [`validate_directory`](Self::validate_directory),
    /// which is used when a user picks a directory during create/update,
    /// this is for re-checking a directory that was valid when it was saved.
    pub fn check_directory_health(directory: &str) -> Result<(), ProjectError> {
        match check_directory_status(directory) {
            DirectoryStatus::Exists => Ok(()),
            DirectoryStatus::DoesNotExist | DirectoryStatus::NotADirectory => {
                Err(ProjectError::DirectoryDeletedOrMoved(directory.to_string()))
            }
            DirectoryStatus::Inaccessible | DirectoryStatus::PermissionDenied => {
                Err(ProjectError::DirectoryInaccessible(directory.to_string()))
            }
        }
    }

    pub fn new(
        name: String,
        directory: String,
        description: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<Self, ProjectError> {
        Self::validate_name(&name)?;
        let directory = normalize_directory(&directory);
        Self::validate_directory(&directory)?;
        let name = remove_spaces(&name);
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        Ok(Self {
            is_deleted: false,
            id,
            name,
            description: description.unwrap_or_default(),
            directory,
            created_at: now,
            updated_at: now,
            last_opened_at: None,
            tags: normalize_tags(tags.unwrap_or_default()),
            favorite: false,
            open_with: None,
            notes: None,
            client: None,
            trackers: Vec::new(),
        })
    }

    pub fn update(&mut self, update: UpdateProject) -> Result<(), ProjectError> {
        if let Some(name) = &update.name {
            Self::validate_name(name)?;
        }

        let normalized_directory = update.directory.as_deref().map(normalize_directory);
        if let Some(directory) = &normalized_directory {
            Self::validate_directory(directory)?;
        }

        if let Some(name) = update.name {
            self.name = remove_spaces(&name);
        }

        if let Some(directory) = normalized_directory {
            self.directory = directory;
        }

        if let Some(tags) = update.tags {
            self.tags = normalize_tags(tags);
        }

        apply_if_present!(
            self,
            update,
            description,
            favorite,
            open_with,
            notes,
            client
        );
        self.updated_at = Utc::now();

        Ok(())
    }

    pub fn mark_as_opened_recently(&mut self) {
        self.last_opened_at = Some(Utc::now());
    }

    /// Soft-deletes the project: its directory is gone from disk, but the
    /// tracked metadata stays in the store (shown in the bin) until the user
    /// either restores it or permanently purges it.
    pub fn mark_deleted(&mut self) {
        self.is_deleted = true;
        self.updated_at = Utc::now();
    }

    /// Restores a soft-deleted project so it shows up in the main list again.
    pub fn restore(&mut self) {
        self.is_deleted = false;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            client: None,
            trackers: Vec::new(),
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
}
