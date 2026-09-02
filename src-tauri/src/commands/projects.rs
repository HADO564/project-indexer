use crate::store::ProjectStore;
use indexer_core::detectors::DetectorRunner;
use indexer_core::domain::sorting::{filter_deleted, filter_favorites, sort_projects, SortOptions};
use indexer_core::domain::{Project, Tracker, UpdateProject};
use indexer_core::error::ProjectError;
use indexer_core::platform::{
    check_directory_status, open_with_app_available, remove_directory, DirectoryStatus,
};
use std::path::Path;
use tauri::{AppHandle, Runtime, State};

#[tauri::command]
pub fn create_project(
    app: AppHandle,
    detectors: State<'_, DetectorRunner>,
    name: String,
    directory: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Project, ProjectError> {
    let store = ProjectStore::new(&app)?;
    let existing = store.get_all_projects()?;

    Project::check_for_duplicate_name_or_dir(&name, &directory, &existing)?;

    let mut project = Project::new(name, directory, description, tags)?;

    // Best-effort: a project is still worth tracking even if we can't tell
    // what kind of project it is. Detection is resilient — whatever detectors
    // succeeded still count — and `refresh_project_trackers` lets the frontend
    // retry the rest explicitly, where a failure is worth surfacing.
    let detection = detectors.detect_project(Path::new(&project.directory));
    project.trackers = detection.trackers();
    for error in detection.errors() {
        eprintln!("Detector error for '{}': {}", project.directory, error);
    }

    store.save_project(&project)?;

    Ok(project)
}

/// Re-runs project-type detection (git, and whatever else is registered)
/// against an existing project's directory and persists the result.
///
/// Unlike the best-effort detection in [`create_project`], this is
/// all-or-nothing ([`indexer_core::detectors::Detection::into_result`]): any detector failure is
/// returned to the caller and the stored trackers are left untouched. It's an
/// explicit, user-triggered retry, so a half-applied refresh — a persisted
/// tracker set silently missing whatever the failing detector produces — is
/// worse than a visible failure. This is a recorded decision, not incidental;
/// the alternative (persist successes, surface per-detector errors) is
/// documented in `docs/architecture.md` and guarded by a runner test.
#[tauri::command]
pub fn refresh_project_trackers(
    app: AppHandle,
    detectors: State<'_, DetectorRunner>,
    id: String,
) -> Result<Project, ProjectError> {
    let store = ProjectStore::new(&app)?;

    let mut project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    // A directory that's since been deleted or moved gets a clear error of
    // its own, rather than surfacing as a raw I/O failure from whichever
    // detector happened to touch the filesystem first.
    Project::check_directory_health(&project.directory)?;

    project.trackers = detectors
        .detect_project(Path::new(&project.directory))
        .into_result()
        .map_err(|e| ProjectError::Detection(e.to_string()))?;
    store.save_project(&project)?;

    Ok(project)
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
    detectors: State<'_, DetectorRunner>,
    directory: String,
) -> Vec<Tracker> {
    let detection = detectors.detect_project(Path::new(&directory));
    for error in detection.errors() {
        eprintln!("Detector error previewing '{}': {}", directory, error);
    }
    detection.trackers()
}

#[tauri::command]
pub fn update_project(
    app: AppHandle,
    id: String,
    update: UpdateProject,
) -> Result<Project, ProjectError> {
    let store = ProjectStore::new(&app)?;

    let mut project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    project.update(update)?;
    store.save_project(&project)?;

    Ok(project)
}

/// IDs of live (non-deleted) projects whose directory is no longer on disk —
/// deleted or replaced by a file, i.e. moved out from under the store. Backs
/// the "directory gone" marker in the list. An *inaccessible* directory (an
/// offline network drive, a permissions hiccup) is deliberately not flagged:
/// that's transient, and calling it "gone" would be wrong.
#[tauri::command]
pub fn list_missing_directories(app: AppHandle) -> Result<Vec<String>, ProjectError> {
    let store = ProjectStore::new(&app)?;
    Ok(store
        .get_all_projects()?
        .into_iter()
        .filter(|p| {
            matches!(
                check_directory_status(&p.directory),
                DirectoryStatus::DoesNotExist | DirectoryStatus::NotADirectory
            )
        })
        .map(|p| p.id)
        .collect())
}

#[tauri::command]
pub fn get_project(app: AppHandle, id: String) -> Result<Project, ProjectError> {
    let store = ProjectStore::new(&app)?;

    store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))
}

/// Returns non-deleted projects for the main list view, ordered per
/// `options` (default: alphabetical, ascending — see [`SortOptions::default`]).
#[tauri::command]
pub fn get_all_projects(
    app: AppHandle,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    let store = ProjectStore::new(&app)?;
    let mut projects = store.get_all_projects()?;
    sort_projects(&mut projects, options.unwrap_or_default());
    Ok(projects)
}

/// Returns soft-deleted projects for the bin view, ordered per `options`
/// (default: alphabetical, ascending — see [`SortOptions::default`]).
#[tauri::command]
pub fn get_deleted_projects(
    app: AppHandle,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    let store = ProjectStore::new(&app)?;
    let all = store.all_projects()?;
    Ok(filter_deleted(&all, options.unwrap_or_default()))
}

/// Returns favorited, non-deleted projects, ordered per `options` (default:
/// alphabetical, ascending).
#[tauri::command]
pub fn get_favorite_projects(
    app: AppHandle,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    let store = ProjectStore::new(&app)?;
    let active = store.get_all_projects()?;
    Ok(filter_favorites(&active, options.unwrap_or_default()))
}

/// Permanently purges a project's metadata. Only allowed on an already
/// soft-deleted project (from the bin) — deleting a project's directory goes
/// through [`delete_project_directory`] instead, which is the only path
/// that's supposed to touch disk. [`delete_project_directory`] with
/// `delete_metadata: true` bypasses this check by calling the store
/// directly, since purging immediately without a bin stop is exactly what
/// that flag means.
#[tauri::command]
pub fn delete_project(app: AppHandle, id: String) -> Result<(), ProjectError> {
    let store = ProjectStore::new(&app)?;

    let project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    if !project.is_deleted {
        return Err(ProjectError::ProjectNotInBin(id));
    }

    store.delete_project(&id)?;
    Ok(())
}

/// Removes a project's tracked metadata without touching its directory on
/// disk — "stop indexing this," as opposed to [`delete_project`] (only for
/// an already soft-deleted project) or [`delete_project_directory`] (which
/// always removes the directory too). Works on any project regardless of
/// `is_deleted`, since the folder is left exactly where it is; re-adding it
/// later is just [`create_project`] pointed at the same directory again.
#[tauri::command]
pub fn untrack_project(app: AppHandle, id: String) -> Result<(), ProjectError> {
    let store = ProjectStore::new(&app)?;

    store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    store.delete_project(&id)?;
    Ok(())
}

/// Deletes a project's directory from disk, then either purges its metadata
/// too (`delete_metadata: true`) or keeps it around soft-deleted so it shows
/// up in the bin (`delete_metadata: false`). This is the only path that
/// removes a directory. Dropping a project's tracked metadata *without*
/// touching its directory goes through [`untrack_project`] instead.
#[tauri::command]
pub fn delete_project_directory(
    app: AppHandle,
    id: String,
    delete_metadata: bool,
) -> Result<(), ProjectError> {
    let store = ProjectStore::new(&app)?;

    let mut project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    remove_directory(&project.directory).map_err(ProjectError::DirectoryInaccessible)?;

    if delete_metadata {
        store.delete_project(&id)?;
    } else {
        project.mark_deleted();
        store.save_project(&project)?;
    }

    Ok(())
}

/// Restores a soft-deleted project so it shows up in the main list again.
/// Note the directory itself isn't restored — it was already deleted from
/// disk when the project was soft-deleted.
#[tauri::command]
pub fn restore_project(app: AppHandle, id: String) -> Result<Project, ProjectError> {
    let store = ProjectStore::new(&app)?;

    let mut project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    project.restore();
    store.save_project(&project)?;

    Ok(project)
}

/// Opens a project with its stored `open_with` app (or the system default
/// when unset), after checking the app can still be found — a project set
/// up to open with an app that's since been uninstalled or moved fails with
/// [`ProjectError::OpenWithAppMissing`] instead of a generic launch failure,
/// so the frontend can offer to open in the file explorer or pick a
/// different app instead.
#[tauri::command]
pub fn open_project(app: AppHandle, id: String) -> Result<Project, ProjectError> {
    let store = ProjectStore::new(&app)?;

    let project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    Project::check_directory_health(&project.directory)?;

    let open_with = project
        .open_with
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if let Some(command) = &open_with {
        if !open_with_app_available(command) {
            return Err(ProjectError::OpenWithAppMissing(command.clone()));
        }
    }

    open_directory_and_mark_opened(&store, project, open_with.as_deref())
}

/// Opens a project's directory with the system's file explorer, ignoring
/// any `open_with` app configured for it. Used as a fallback when that app
/// can't be found.
#[tauri::command]
pub fn open_project_in_explorer(app: AppHandle, id: String) -> Result<Project, ProjectError> {
    let store = ProjectStore::new(&app)?;

    let project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    Project::check_directory_health(&project.directory)?;

    open_directory_and_mark_opened(&store, project, None)
}

fn open_directory_and_mark_opened<R: Runtime>(
    store: &ProjectStore<R>,
    mut project: Project,
    open_with: Option<&str>,
) -> Result<Project, ProjectError> {
    crate::commands::system::open_in_app(&project.directory, open_with)
        .map_err(ProjectError::OpenFailed)?;
    project.mark_as_opened_recently();
    store.save_project(&project)?;
    Ok(project)
}
