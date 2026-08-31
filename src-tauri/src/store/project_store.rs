use crate::errors::ProjectError;
use crate::migrations;
use crate::models::Project;
use crate::utils::sort_projects_by_recents;
use serde_json::{from_value, to_value};
use std::sync::Arc;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::{Store, StoreExt};

pub struct ProjectStore<R: tauri::Runtime> {
    store: Arc<Store<R>>,
}

impl<R: Runtime> ProjectStore<R> {
    pub fn new(app: &AppHandle<R>) -> Result<Self, ProjectError> {
        let store = app.store("projects.json").map_err(|e| {
            ProjectError::Store(format!("Failed to initialize project store: {}", e))
        })?;
        Ok(Self { store })
    }

    pub fn save_project(&self, project: &Project) -> Result<(), ProjectError> {
        let value = to_value(project)
            .map_err(|e| ProjectError::Store(format!("Failed to serialize project: {}", e)))?;
        let value = migrations::migrate(value);

        self.store.set(project.id.clone(), value);

        Ok(())
    }

    pub fn get_project(&self, project_id: &str) -> Result<Option<Project>, ProjectError> {
        match self.store.get(project_id) {
            Some(value) => {
                let value = migrations::migrate(value);
                let project: Project = from_value(value).map_err(|e| {
                    ProjectError::Store(format!("Failed to deserialize project: {}", e))
                })?;
                Ok(Some(project))
            }
            None => Ok(None),
        }
    }

    /// Returns every non-deleted project. This is the main list, and also
    /// what duplicate name/directory checks are run against, so a directory
    /// freed up by a soft-deleted project can be reused by a new one.
    pub fn get_all_projects(&self) -> Result<Vec<Project>, ProjectError> {
        Ok(self
            .all_projects()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .collect())
    }

    /// Returns every project regardless of deleted status. Commands that
    /// need a specific view of the full set (e.g. favorites, the bin) filter
    /// and re-sort this themselves via `utils::sorting`, rather than the
    /// store baking a fixed sort/filter into each view.
    pub fn all_projects(&self) -> Result<Vec<Project>, ProjectError> {
        let mut projects = Vec::new();
        for value in self.store.values() {
            let value = migrations::migrate(value);
            let project: Project = from_value(value).map_err(|e| {
                ProjectError::Store(format!("Failed to deserialize project: {}", e))
            })?;
            projects.push(project);
        }
        sort_projects_by_recents(&mut projects);
        Ok(projects)
    }

    pub fn delete_project(&self, project_id: &str) -> Result<(), ProjectError> {
        self.store.delete(project_id);

        Ok(())
    }

    /// Flushes any pending autosaved writes to disk immediately.
    ///
    /// Autosave debounces writes by 100ms, so a mutation made just before the
    /// app closes could otherwise be lost. Call this on shutdown (e.g. window
    /// `CloseRequested`). Does nothing if the store was never loaded.
    pub fn flush(app: &AppHandle<R>) -> Result<(), ProjectError> {
        if let Some(store) = app.get_store("projects.json") {
            store
                .save()
                .map_err(|e| ProjectError::Store(format!("Failed to save project store: {}", e)))?;
        }
        Ok(())
    }
}
