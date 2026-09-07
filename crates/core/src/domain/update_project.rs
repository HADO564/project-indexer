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

#[derive(Debug, Serialize, Deserialize)]
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
