use crate::models::{Project, ProjectError, UpdateProject};
use crate::store::ProjectStore;
use tauri::AppHandle;

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
pub fn delete_project(app: AppHandle, id: String) -> Result<(), ProjectError> {
    let store = ProjectStore::new(&app)?;
    store.delete_project(&id)?;
    Ok(())
}


#[tauri::command]
pub fn open_project(
    app: AppHandle,
    id: String,
) -> Result<Project, ProjectError> {
    let store = ProjectStore::new(&app)?;

    let mut project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    crate::commands::system::open_in_app(&project.directory, project.open_with.as_deref())
        .map_err(ProjectError::OpenFailed)?;
    project.mark_as_opened_recently();
    store.save_project(&project)?;
    Ok(project)
}