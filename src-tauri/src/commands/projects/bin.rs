//! The soft-delete lifecycle: bin, restore, untrack.

use std::sync::Arc;

use tauri::State;

use indexer_core::application::ProjectService;
use indexer_core::domain::Project;
use indexer_core::error::ProjectError;

/// Permanently purges a project's metadata. Only allowed on an already
/// soft-deleted project (from the bin) — deleting a project's directory goes
/// through [`delete_project_directory`] instead, which is the only path
/// that's supposed to touch disk.
#[tauri::command]
pub fn delete_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<(), ProjectError> {
    service.delete(&id)
}

/// Removes a project's tracked metadata without touching its directory on
/// disk — "stop indexing this," as opposed to [`delete_project`] (only for
/// an already soft-deleted project) or [`delete_project_directory`] (which
/// always removes the directory too).
#[tauri::command]
pub fn untrack_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<(), ProjectError> {
    service.untrack(&id)
}

/// Restores a soft-deleted project so it shows up in the main list again.
/// Note the directory itself isn't restored — it was already deleted from
/// disk when the project was soft-deleted.
#[tauri::command]
pub fn restore_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.restore(&id)
}
