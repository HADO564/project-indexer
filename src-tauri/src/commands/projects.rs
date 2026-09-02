use std::sync::Arc;

use tauri::State;

use indexer_core::application::ProjectService;
use indexer_core::domain::sorting::SortOptions;
use indexer_core::domain::{Project, Tracker, UpdateProject};
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

/// Re-runs project-type detection (git, and whatever else is registered)
/// against an existing project's directory and persists the result.
///
/// Unlike the best-effort detection in [`create_project`], this is
/// all-or-nothing: any detector failure is returned to the caller and the
/// stored trackers are left untouched. It's an explicit, user-triggered
/// retry, so a half-applied refresh — a persisted tracker set silently
/// missing whatever the failing detector produces — is worse than a visible
/// failure. This is a recorded decision, not incidental; the alternative
/// (persist successes, surface per-detector errors) is documented in
/// `docs/architecture.md` and guarded by a runner test.
#[tauri::command]
pub fn refresh_project_trackers(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.refresh_trackers(&id)
}

/// Runs detection against a directory that isn't a project yet — nothing is
/// read from or written to the store. Lets the frontend preview what a
/// directory looks like (e.g. to suggest a name from its git remote) before
/// the user commits to [`create_project`].
///
/// Advisory, so it's best-effort: a detector that fails just contributes
/// nothing to the preview rather than failing the whole call.
#[tauri::command]
pub fn detect_project_trackers(
    service: State<'_, Arc<ProjectService>>,
    directory: String,
) -> Vec<Tracker> {
    service.preview_detection(&directory)
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

#[tauri::command]
pub fn update_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
    update: UpdateProject,
) -> Result<Project, ProjectError> {
    service.update(&id, update)
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

#[tauri::command]
pub fn get_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.get(&id)
}

/// Returns non-deleted projects for the main list view, ordered per
/// `options` (default: alphabetical, ascending — see [`SortOptions::default`]).
#[tauri::command]
pub fn get_all_projects(
    service: State<'_, Arc<ProjectService>>,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    service.list(options.unwrap_or_default())
}

/// Returns soft-deleted projects for the bin view, ordered per `options`
/// (default: alphabetical, ascending — see [`SortOptions::default`]).
#[tauri::command]
pub fn get_deleted_projects(
    service: State<'_, Arc<ProjectService>>,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    service.list_deleted(options.unwrap_or_default())
}

/// Returns favorited, non-deleted projects, ordered per `options` (default:
/// alphabetical, ascending).
#[tauri::command]
pub fn get_favorite_projects(
    service: State<'_, Arc<ProjectService>>,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    service.list_favorites(options.unwrap_or_default())
}

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
