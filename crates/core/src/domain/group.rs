use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::palette::is_valid_color;
use crate::domain::update_group::UpdateGroup;
use crate::error::ProjectError;

/// A user-defined band of projects, surfaced as one entry in the sidebar.
///
/// Membership is exclusive — a project has zero or one group — which is what
/// makes every project appear under exactly one entry. Tags remain the
/// non-exclusive mechanism.
///
/// `icon` names a glyph from the frontend's bundled set. It is validated only
/// as non-empty here: core does not own that list, and an unknown name falls
/// back to a default glyph at render rather than failing, the same
/// forward-compatibility rule the `--json` contract uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub position: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Group {
    fn validate_name(name: &str) -> Result<(), ProjectError> {
        if name.trim().is_empty() {
            return Err(ProjectError::InvalidGroupName);
        }
        Ok(())
    }

    fn validate_color(color: &str) -> Result<(), ProjectError> {
        if !is_valid_color(color) {
            return Err(ProjectError::UnknownSwatch(color.to_string()));
        }
        Ok(())
    }

    fn validate_icon(icon: &str) -> Result<(), ProjectError> {
        if icon.trim().is_empty() {
            return Err(ProjectError::InvalidGroupIcon);
        }
        Ok(())
    }

    pub fn new(
        name: String,
        color: String,
        icon: String,
        position: i64,
    ) -> Result<Self, ProjectError> {
        Self::validate_name(&name)?;
        Self::validate_color(&color)?;
        Self::validate_icon(&icon)?;

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            color,
            icon: icon.trim().to_string(),
            position,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn check_for_duplicate_name(name: &str, existing: &[Group]) -> Result<(), ProjectError> {
        if existing
            .iter()
            .any(|g| g.name.trim().eq_ignore_ascii_case(name.trim()))
        {
            return Err(ProjectError::DuplicateGroupName(name.trim().to_string()));
        }
        Ok(())
    }

    /// Validates every present field *before* assigning any of them, so a
    /// rejected update leaves the group exactly as it was.
    pub fn update(&mut self, update: UpdateGroup) -> Result<(), ProjectError> {
        if let Some(name) = &update.name {
            Self::validate_name(name)?;
        }
        if let Some(color) = &update.color {
            Self::validate_color(color)?;
        }
        if let Some(icon) = &update.icon {
            Self::validate_icon(icon)?;
        }

        if let Some(name) = update.name {
            self.name = name.trim().to_string();
        }
        if let Some(color) = update.color {
            self.color = color;
        }
        if let Some(icon) = update.icon {
            self.icon = icon.trim().to_string();
        }
        self.updated_at = Utc::now();
        Ok(())
    }
}
