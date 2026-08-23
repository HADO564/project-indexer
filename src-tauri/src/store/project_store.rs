use crate::models::{Project, ProjectError};
use serde_json::{from_value, to_value};
use std::sync::Arc;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::{Store, StoreExt};

pub struct ProjectStore<R: tauri::Runtime> {
    store: Arc<Store<R>>,
}

impl<R: Runtime> ProjectStore<R> {
    pub fn new(app: &AppHandle<R>) -> Result<Self, ProjectError> {
        let store = app
            .store("projects.json")
            .map_err(|e| ProjectError::Store(format!("Failed to initialize project store: {}", e)))?;
        Ok(Self { store })
    }

    pub fn save_project(&self, project: &Project) -> Result<(), ProjectError> {
        let value = to_value(project)
            .map_err(|e| ProjectError::Store(format!("Failed to serialize project: {}", e)))?;

        self.store.set(project.id.clone(), value);

        self.store
            .save()
            .map_err(|e| ProjectError::Store(format!("Failed to save project store: {}", e)))?;

        Ok(())
    }

    pub fn get_project(&self, project_id: &str) -> Result<Option<Project>, ProjectError> {
        match self.store.get(project_id) {
            Some(value) => {
                let project: Project = from_value(value).map_err(|e| {
                    ProjectError::Store(format!("Failed to deserialize project: {}", e))
                })?;
                Ok(Some(project))
            }
            None => Ok(None),
        }
    }

    pub fn get_all_projects(&self) -> Result<Vec<Project>, ProjectError> {
        let mut projects = Vec::new();
        for value in self.store.values() {
            let project: Project = from_value(value).map_err(|e| {
                ProjectError::Store(format!("Failed to deserialize project: {}", e))
            })?;
            projects.push(project);
        }
        Ok(projects)
    }

    pub fn delete_project(&self, project_id: &str) -> Result<(), ProjectError> {
        self.store.delete(project_id);

        self.store
            .save()
            .map_err(|e| ProjectError::Store(format!("Failed to save project store: {}", e)))?;

        Ok(())
    }
}
