//! Creating, editing and reading back a single project.

use std::sync::Arc;

use tauri::State;

use indexer_core::application::ProjectService;
use indexer_core::domain::{Project, UpdateProject};
use indexer_core::error::ProjectError;

#[tauri::command]
pub fn create_project(
    service: State<'_, Arc<ProjectService>>,
    name: String,
    directory: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Project, ProjectError> {
    service.create(name, directory, description, tags)
}

#[tauri::command]
pub fn update_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
    update: UpdateProject,
) -> Result<Project, ProjectError> {
    service.update(&id, update)
}

#[tauri::command]
pub fn get_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.get(&id)
}

/// Suggests a project name for a directory that isn't a project yet, from
/// its detected trackers (e.g. a git remote) falling back to the directory
/// name. Backs the name pre-fill in the create form's Browse action.
#[tauri::command]
pub fn suggest_project_name(
    service: State<'_, Arc<ProjectService>>,
    directory: String,
) -> Option<String> {
    indexer_core::domain::naming::suggest_project_name(
        &service.preview_detection(&directory),
        &directory,
    )
}
