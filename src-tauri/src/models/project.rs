use crate::errors::ProjectError;
use crate::models::update_project::UpdateProject;
use crate::utils::filesystem::{check_directory_status, DirectoryStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Serialize, Deserialize)]

pub struct Project {
    pub is_deleted: bool,
    pub id: String,
    pub name: String,
    pub description: String,
    pub directory: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_opened_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub open_with: Option<String>,
    pub notes: Option<String>,
    pub client: Option<String>,
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
        let normalized_directory = Self::normalize_directory(directory);
        if existing
            .iter()
            .any(|p| Self::normalize_directory(&p.directory) == normalized_directory)
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

    fn remove_spaces(name: &str) -> String {
        name.replace(' ', "_")
    }

    fn normalize_tag(tag: &str) -> String {
        let trimmed = tag.trim();
        let mut chars = trimmed.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            None => String::new(),
        }
    }

    fn normalize_tags(tags: Vec<String>) -> Vec<String> {
        let mut normalized = Vec::new();
        for tag in tags {
            let tag = Self::normalize_tag(&tag);
            if !tag.is_empty() && !normalized.contains(&tag) {
                normalized.push(tag);
            }
        }
        normalized
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

    /// Normalizes a directory path so equivalent paths compare equal, e.g.
    /// `D:\Projects\Friction\` and `D:\Projects\Friction` (trailing separator)
    /// or `D:/Projects/Friction` (mixed slash style) all collapse to the same
    /// string. Drive roots (`C:\`) are left intact rather than being stripped
    /// down to `C:`, which would change their meaning.
    ///
    /// This does not normalize case, so `C:\Foo` and `c:\foo` are still
    /// treated as different directories.
    fn normalize_directory(directory: &str) -> String {
        let trimmed = directory.trim();
        let normalized_seps: String = trimmed
            .chars()
            .map(|c| {
                if c == '/' || c == '\\' {
                    std::path::MAIN_SEPARATOR
                } else {
                    c
                }
            })
            .collect();

        let stripped = normalized_seps.trim_end_matches(std::path::MAIN_SEPARATOR);

        if stripped.is_empty() {
            normalized_seps
        } else if stripped.ends_with(':') {
            format!("{stripped}{}", std::path::MAIN_SEPARATOR)
        } else {
            stripped.to_string()
        }
    }

    pub fn new(
        name: String,
        directory: String,
        description: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<Self, ProjectError> {
        Self::validate_name(&name)?;
        let directory = Self::normalize_directory(&directory);
        Self::validate_directory(&directory)?;
        let name = Self::remove_spaces(&name);
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
            tags: Self::normalize_tags(tags.unwrap_or_default()),
            favorite: false,
            open_with: None,
            notes: None,
            client: None,
        })
    }

    pub fn update(&mut self, update: UpdateProject) -> Result<(), ProjectError> {
        if let Some(name) = &update.name {
            Self::validate_name(name)?;
        }

        let normalized_directory = update.directory.as_deref().map(Self::normalize_directory);
        if let Some(directory) = &normalized_directory {
            Self::validate_directory(directory)?;
        }

        if let Some(name) = update.name {
            self.name = Self::remove_spaces(&name);
        }

        if let Some(directory) = normalized_directory {
            self.directory = directory;
        }

        if let Some(tags) = update.tags {
            self.tags = Self::normalize_tags(tags);
        }

        apply_if_present!(
            self, update, description, favorite, open_with, notes, client
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
        }
    }

    #[test]
    fn normalizes_trailing_separator() {
        assert_eq!(
            Project::normalize_directory("D:\\Projects\\Friction\\"),
            Project::normalize_directory("D:\\Projects\\Friction"),
        );
    }

    #[test]
    fn normalizes_mixed_separator_style() {
        assert_eq!(
            Project::normalize_directory("D:/Projects/Friction"),
            Project::normalize_directory("D:\\Projects\\Friction"),
        );
    }

    #[test]
    fn preserves_root_path() {
        let sep = std::path::MAIN_SEPARATOR.to_string();
        assert_eq!(Project::normalize_directory(&sep), sep);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn preserves_drive_root() {
        assert_eq!(Project::normalize_directory("C:\\"), "C:\\");
        assert_eq!(Project::normalize_directory("C:/"), "C:\\");
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
        assert!(matches!(result, Err(ProjectError::DirectoryDeletedOrMoved(_))));
    }

    #[test]
    fn directory_health_flags_a_file_in_place_of_the_directory_as_deleted_or_moved() {
        let file = std::env::temp_dir().join("project-indexer-tests-file-not-dir.txt");
        std::fs::write(&file, b"placeholder").expect("should write temp file");

        let result = Project::check_directory_health(file.to_str().unwrap());

        std::fs::remove_file(&file).expect("should clean up temp file");
        assert!(matches!(result, Err(ProjectError::DirectoryDeletedOrMoved(_))));
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
}
