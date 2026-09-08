use std::collections::BTreeMap;

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
    /// User-defined key/value facts — "client", "engine", "priority", whatever
    /// this particular set of projects needs. Replaces the single hardcoded
    /// `client` field, which only ever suited one kind of user; the v3
    /// migration lifts an existing `client` value into `properties["client"]`.
    ///
    /// A `BTreeMap` rather than a `HashMap` so the order is stable: the blob
    /// serializes identically across saves, and the UI lists keys the same way
    /// every render without sorting at the call site.
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
    #[serde(default)]
    pub trackers: Vec<Tracker>,
    /// The group this project belongs to, or `None` for Ungrouped.
    /// Mirrored into the `projects.group_id` column for querying.
    #[serde(default)]
    pub group_id: Option<String>,
    /// Secondary palette colour — distinguishes this project from others.
    #[serde(default)]
    pub color: Option<String>,
    /// Bundled icon name (`"gamepad"`) or a custom one (`"custom:my-logo"`).
    #[serde(default)]
    pub icon: Option<String>,
}

/// Trims each key and rejects the two shapes that would make a property
/// unreachable.
///
/// A key that is empty after trimming names nothing. A key containing `:`
/// cannot be searched: the search bar reads `key: value`, splitting on the
/// first colon, so `a:b` would only ever be looked up as `a`. Rejecting both
/// on write keeps every stored property findable, rather than letting the UI
/// accept something the search can never reach.
///
/// Values are left exactly as typed — a value may contain anything.
pub fn normalize_properties(
    properties: BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, ProjectError> {
    let mut out = BTreeMap::new();
    for (key, value) in properties {
        let key = key.trim().to_string();
        if key.is_empty() {
            return Err(ProjectError::InvalidPropertyKey(
                "a property name cannot be empty".into(),
            ));
        }
        if key.contains(':') {
            return Err(ProjectError::InvalidPropertyKey(format!(
                "a property name cannot contain ':' (got '{key}') — the search bar reads 'name: value'"
            )));
        }
        out.insert(key, value);
    }
    Ok(out)
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
            properties: BTreeMap::new(),
            trackers: Vec::new(),
            group_id: None,
            color: None,
            icon: None,
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

        if let Some(Some(color)) = &update.color {
            if !crate::domain::palette::is_valid_color(color) {
                return Err(ProjectError::UnknownSwatch(color.clone()));
            }
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

        if let Some(properties) = update.properties {
            self.properties = normalize_properties(properties)?;
        }

        apply_if_present!(
            self,
            update,
            description,
            favorite,
            open_with,
            notes,
            group_id,
            color,
            icon
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
