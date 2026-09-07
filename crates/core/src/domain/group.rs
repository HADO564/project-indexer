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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Group {
        Group::new("Client work".into(), "cyan".into(), "briefcase".into(), 0).expect("valid group")
    }

    #[test]
    fn new_group_trims_its_name_and_stamps_timestamps() {
        let g = Group::new(
            "  Client work  ".into(),
            "cyan".into(),
            "briefcase".into(),
            3,
        )
        .expect("valid group");
        assert_eq!(g.name, "Client work");
        assert_eq!(g.position, 3);
        assert_eq!(g.created_at, g.updated_at);
        assert!(!g.id.is_empty());
    }

    #[test]
    fn rejects_an_empty_name() {
        let result = Group::new("   ".into(), "cyan".into(), "briefcase".into(), 0);
        assert!(matches!(result, Err(ProjectError::InvalidGroupName)));
    }

    #[test]
    fn rejects_an_unknown_colour() {
        let result = Group::new(
            "Personal".into(),
            "chartreuse".into(),
            "briefcase".into(),
            0,
        );
        assert!(matches!(result, Err(ProjectError::UnknownSwatch(_))));

        // Not a normalisation gap either: everything but the exact six-digit
        // form is refused, because the value reaches a `style` attribute.
        for bad in ["#ff0", "#gggggg", "red", "#ff0000; background: url(x)"] {
            let result = Group::new("Personal".into(), bad.into(), "briefcase".into(), 0);
            assert!(
                matches!(result, Err(ProjectError::UnknownSwatch(_))),
                "{bad} should not be a valid group colour"
            );
        }
    }

    #[test]
    fn accepts_a_hex_literal_from_the_colour_picker() {
        // Groups take a picked colour like projects do. A palette name is
        // still the better answer — it is what lets a future theme recolour
        // everything coherently — but that is a reason to offer the palette
        // first, not to refuse a colour someone actually wants.
        let group = Group::new("Personal".into(), "#ff0000".into(), "briefcase".into(), 0)
            .expect("a hex literal is a valid group colour");
        assert_eq!(group.color, "#ff0000");
    }

    #[test]
    fn rejects_an_empty_icon() {
        let result = Group::new("Personal".into(), "cyan".into(), "  ".into(), 0);
        assert!(matches!(result, Err(ProjectError::InvalidGroupIcon)));
    }

    #[test]
    fn duplicate_name_check_ignores_case_and_surrounding_space() {
        let existing = vec![sample()];
        let result = Group::check_for_duplicate_name("  CLIENT WORK ", &existing);
        assert!(matches!(result, Err(ProjectError::DuplicateGroupName(_))));
    }

    #[test]
    fn duplicate_name_check_allows_a_distinct_name() {
        let existing = vec![sample()];
        assert!(Group::check_for_duplicate_name("Personal", &existing).is_ok());
    }

    #[test]
    fn update_applies_only_the_fields_present() {
        let mut g = sample();
        let before = g.color.clone();
        g.update(UpdateGroup {
            name: Some("Renamed".into()),
            color: None,
            icon: None,
        })
        .expect("valid update");
        assert_eq!(g.name, "Renamed");
        assert_eq!(g.color, before);
    }

    #[test]
    fn update_rejects_an_unknown_colour_and_changes_nothing() {
        let mut g = sample();
        let result = g.update(UpdateGroup {
            name: Some("Renamed".into()),
            color: Some("chartreuse".into()),
            icon: None,
        });
        assert!(matches!(result, Err(ProjectError::UnknownSwatch(_))));
        assert_eq!(g.name, "Client work");
    }
}
