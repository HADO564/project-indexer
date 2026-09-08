//! Opening a project — in an app, or in the system file explorer.

use super::*;

impl ProjectService {
    /// Opens a project with its stored `open_with` app (or the system default
    /// when unset), after checking the app can still be found.
    pub fn open(&self, id: &str) -> Result<Project, ProjectError> {
        let project = self.load(id)?;
        Project::check_directory_health(&project.directory)?;
        let open_with = project
            .open_with
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(command) = &open_with {
            if !self.launcher.is_available(command) {
                return Err(ProjectError::OpenWithAppMissing(command.clone()));
            }
        }
        self.open_and_mark(project, open_with.as_deref())
    }

    /// Opens a project's directory with the system's file explorer, ignoring
    /// any `open_with` app configured for it.
    pub fn open_in_explorer(&self, id: &str) -> Result<Project, ProjectError> {
        let project = self.load(id)?;
        Project::check_directory_health(&project.directory)?;
        self.open_and_mark(project, None)
    }

    fn open_and_mark(
        &self,
        mut project: Project,
        open_with: Option<&str>,
    ) -> Result<Project, ProjectError> {
        self.launcher.open(&project.directory, open_with)?;
        project.mark_as_opened_recently();
        self.repo.save(&project)?;
        Ok(project)
    }
}
