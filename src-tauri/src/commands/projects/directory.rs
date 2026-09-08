//! Commands that reach the filesystem rather than the store.

use std::sync::Arc;

use tauri::State;

use indexer_core::application::ProjectService;
use indexer_core::error::ProjectError;

/// Deletes a project's directory from disk, then either purges its metadata
/// too (`delete_metadata: true`) or keeps it around soft-deleted so it shows
/// up in the bin (`delete_metadata: false`). This is the only path that
/// removes a directory.
#[tauri::command]
pub fn delete_project_directory(
    service: State<'_, Arc<ProjectService>>,
    id: String,
    delete_metadata: bool,
) -> Result<(), ProjectError> {
    service.delete_directory(&id, delete_metadata)
}

/// IDs of live (non-deleted) projects whose directory is no longer on disk —
/// deleted or replaced by a file, i.e. moved out from under the store. Backs
/// the "directory gone" marker in the list. An *inaccessible* directory (an
/// offline network drive, a permissions hiccup) is deliberately not flagged:
/// that's transient, and calling it "gone" would be wrong.
#[tauri::command]
pub fn list_missing_directories(
    service: State<'_, Arc<ProjectService>>,
) -> Result<Vec<String>, ProjectError> {
    service.list_missing_directories()
}
