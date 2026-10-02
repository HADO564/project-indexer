use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

/// Wraps a present-but-possibly-null JSON value in `Some`, so the outer
/// `Option` can distinguish "field omitted" (`None`) from "field explicitly
/// set to null" (`Some(None)`).
fn deserialize_some<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

/// A partial update to a [`Project`](crate::domain::Project): every field is
/// optional, and `None` means "leave this field alone".
///
/// `Default` is therefore the empty update, so a caller names only what it
/// changes — `UpdateProject { favorite: Some(true), ..Default::default() }`.
/// Guarded by `a_default_update_leaves_every_field_alone`.
#[derive(Debug, Serialize, Default, Deserialize)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub directory: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub favorite: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_some"
    )]
    pub open_with: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_some"
    )]
    pub notes: Option<Option<String>>,
    /// Replaces the whole map. Plain `Option`, not a double option: an empty
    /// map already means "no properties", so there is no absent-vs-null case
    /// to distinguish the way `open_with` and `notes` need.
    #[serde(default)]
    pub properties: Option<BTreeMap<String, String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_some"
    )]
    pub group_id: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_some"
    )]
    pub color: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_some"
    )]
    pub icon: Option<Option<String>>,
}

impl UpdateProject {
    /// Whether this update changes nothing — every field `None`, "leave
    /// alone". An editor that sends only what changed returns one of these
    /// for an untouched form, and saving it would still move `updated_at`.
    pub fn is_empty(&self) -> bool {
        // Destructured rather than compared field by field, so a field added
        // to the struct is a compile error here until it is checked too.
        let UpdateProject {
            name,
            directory,
            description,
            tags,
            favorite,
            open_with,
            notes,
            properties,
            group_id,
            color,
            icon,
        } = self;
        name.is_none()
            && directory.is_none()
            && description.is_none()
            && tags.is_none()
            && favorite.is_none()
            && open_with.is_none()
            && notes.is_none()
            && properties.is_none()
            && group_id.is_none()
            && color.is_none()
            && icon.is_none()
    }
}
