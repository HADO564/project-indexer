use crate::commands::system::{open_with_app_available, remove_directory};
use crate::detectors::detect_project;
use crate::errors::ProjectError;
use crate::models::{Project, UpdateProject};
use crate::store::ProjectStore;
use crate::utils::{filter_deleted, filter_favorites, sort_projects, SortOptions};
use std::path::Path;
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub fn create_project(
    app: AppHandle,
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
    // what kind of project it is (a detector bug, a permissions hiccup on a
    // git call, etc). `refresh_project_trackers` lets the frontend retry
    // this explicitly, where a failure is worth surfacing to the user.
    match detect_project(Path::new(&project.directory)) {
        Ok(trackers) => project.trackers = trackers,
        Err(e) => eprintln!(
            "Failed to detect project type for '{}': {}",
            project.directory, e
        ),
    }

    store.save_project(&project)?;

    Ok(project)
}

/// Re-runs project-type detection (git, and whatever else is registered)
/// against an existing project's directory and persists the result.
///
/// Unlike the best-effort detection in [`create_project`], a detection
/// failure here is returned to the caller — this is an explicit,
/// user-triggered retry, so silently doing nothing would be confusing.
#[tauri::command]
pub fn refresh_project_trackers(app: AppHandle, id: String) -> Result<Project, ProjectError> {
    let store = ProjectStore::new(&app)?;

    let mut project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    project.trackers = detect_project(Path::new(&project.directory))
        .map_err(|e| ProjectError::Detection(e.to_string()))?;
    store.save_project(&project)?;

    Ok(project)
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
