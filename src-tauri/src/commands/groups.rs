use std::sync::Arc;

use tauri::State;

use indexer_core::application::GroupService;
use indexer_core::domain::{Group, UpdateGroup};
use indexer_core::error::ProjectError;

#[tauri::command]
pub fn list_groups(service: State<'_, Arc<GroupService>>) -> Result<Vec<Group>, ProjectError> {
    service.list()
}

#[tauri::command]
pub fn create_group(
    service: State<'_, Arc<GroupService>>,
    name: String,
    color: String,
    icon: String,
) -> Result<Group, ProjectError> {
    service.create(name, color, icon)
}

#[tauri::command]
pub fn update_group(
    service: State<'_, Arc<GroupService>>,
    id: String,
    update: UpdateGroup,
) -> Result<Group, ProjectError> {
    service.update(&id, update)
}

/// Members become Ungrouped. No project is deleted by this.
#[tauri::command]
pub fn delete_group(service: State<'_, Arc<GroupService>>, id: String) -> Result<(), ProjectError> {
    service.delete(&id)
}

/// Rewrites the whole sidebar ordering and returns it, so the caller renders
/// what was persisted rather than what it hoped for.
#[tauri::command]
pub fn reorder_groups(
    service: State<'_, Arc<GroupService>>,
    ordered_ids: Vec<String>,
) -> Result<Vec<Group>, ProjectError> {
    service.reorder(ordered_ids)
}
