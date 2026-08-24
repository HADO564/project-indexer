use crate::commands::system::{open_with_app_available, remove_directory};
use crate::errors::ProjectError;
use crate::models::{Project, UpdateProject};
use crate::store::ProjectStore;
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

    let project = Project::new(name, directory, description, tags)?;
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

#[tauri::command]
pub fn get_all_projects(app: AppHandle) -> Result<Vec<Project>, ProjectError> {
    let store = ProjectStore::new(&app)?;
    store.get_all_projects()
}

#[tauri::command]
pub fn get_deleted_projects(app: AppHandle) -> Result<Vec<Project>, ProjectError> {
    let store = ProjectStore::new(&app)?;
    store.get_deleted_projects()
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

/// Deletes a project's directory from disk, then either purges its metadata
/// too (`delete_metadata: true`) or keeps it around soft-deleted so it shows
/// up in the bin (`delete_metadata: false`). This is the only path that
/// removes a directory — there's no way to drop a project's tracked metadata
/// without also deleting the directory it points at.
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
