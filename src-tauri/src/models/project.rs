use crate::models::update_project::UpdateProject;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;
#[derive(Debug, Serialize, Deserialize)]

pub struct Project {
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
    pub fn new(
        name: String,
        directory: String,
        description: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<Self, String> {
        if name.trim().is_empty() {
            return Err(String::from("Project name cannot be empty"));
        }

        if !Path::new(&directory).is_dir() {
            return Err(String::from("Project directory does not exist"));
        }

        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        Ok(Self {
            id,
            name,
            description: description.unwrap_or_default(),
            directory,
            created_at: now,
            updated_at: now,
            last_opened_at: None,
            tags: tags.unwrap_or_default(),
            favorite: false,
            open_with: None,
            notes: None,
            client: None,
        })
    }

    pub fn update(&mut self, update: UpdateProject) -> Result<(), String> {
        if let Some(name) = &update.name {
            if name.trim().is_empty() {
                return Err(String::from("Project name cannot be empty"));
            }
        }

        if let Some(directory) = &update.directory {
            if !Path::new(directory).is_dir() {
                return Err(String::from("Project directory does not exist"));
            }
        }

        apply_if_present!(
            self, update, name, directory, description, tags, favorite, open_with, notes, client
        );
        self.updated_at = Utc::now();

        Ok(())
    }
}
