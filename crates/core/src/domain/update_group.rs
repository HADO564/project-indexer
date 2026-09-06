use serde::{Deserialize, Serialize};

/// Partial update: a field left `None` is unchanged.
///
/// Unlike `UpdateProject`, no field here is nullable — a group always has a
/// name, a colour and an icon — so these are plain `Option<T>` rather than the
/// double-option "absent vs explicit null" shape.
///
/// `position` is deliberately absent: reordering goes through
/// `GroupRepository::set_group_positions`, which rewrites the whole ordering,
/// so there is exactly one path that renumbers.
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateGroup {
    pub name: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}
