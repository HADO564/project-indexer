//! Opening a project in an app or in the file explorer.

use std::sync::Arc;

use tauri::State;

use indexer_core::application::ProjectService;
use indexer_core::domain::Project;
use indexer_core::error::ProjectError;

/// Opens a project with its stored `open_with` app (or the system default
/// when unset), after checking the app can still be found — a project set
/// up to open with an app that's since been uninstalled or moved fails with
/// [`ProjectError::OpenWithAppMissing`] instead of a generic launch failure,
/// so the frontend can offer to open in the file explorer or pick a
/// different app instead.
#[tauri::command]
pub fn open_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.open(&id)
}

/// Opens a project's directory with the system's file explorer, ignoring
/// any `open_with` app configured for it. Used as a fallback when that app
/// can't be found.
#[tauri::command]
pub fn open_project_in_explorer(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.open_in_explorer(&id)
}
