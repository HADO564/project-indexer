use crate::models::{Project, UpdateProject};
use crate::store::ProjectStore;
use tauri::AppHandle;

#[tauri::command]
pub fn create_project(
    app: AppHandle,
    name: String,
    directory: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Project, String> {
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
) -> Result<Project, String> {
    let store = ProjectStore::new(&app)?;

    let mut project = store
        .get_project(&id)?
        .ok_or_else(|| format!("Project with id '{}' not found", id))?;

    project.update(update)?;
    store.save_project(&project)?;

    Ok(project)
}

#[tauri::command]
pub fn get_project(app: AppHandle, id: String) -> Result<Project, String> {
    let store = ProjectStore::new(&app)?;

    store
        .get_project(&id)?
        .ok_or_else(|| format!("Project with id '{}' not found", id))
}

#[tauri::command]
pub fn get_all_projects(app: AppHandle) -> Result<Vec<Project>, String> {
    let store = ProjectStore::new(&app)?;
    store.get_all_projects()
}

#[tauri::command]
pub fn delete_project(app: AppHandle, id: String) -> Result<(), String> {
    let store = ProjectStore::new(&app)?;
    store.delete_project(&id)?;
    Ok(())
}


#[tauri::command]
pub fn open_project(
    app: AppHandle,
    id: String,
) -> Result<Project, String> {
    let store = ProjectStore::new(&app)?;

    let mut project = store
        .get_project(&id)?
        .ok_or_else(|| format!("Project with id '{}' not found", id))?;

    crate::commands::system::open_in_app(&project.directory, project.open_with.as_deref())?;
    project.mark_as_opened_recently();
    store.save_project(&project)?;
    Ok(project)
}