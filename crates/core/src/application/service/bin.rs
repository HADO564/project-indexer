//! The soft-delete lifecycle — bin, restore, and untrack.
//!
//! None of these touch the directory on disk; that is `directory.rs`.

use super::*;

impl ProjectService {
    /// Permanently purges a project's metadata. Only allowed on an already
    /// soft-deleted project (from the bin).
    pub fn delete(&self, id: &str) -> Result<(), ProjectError> {
        let project = self.load(id)?;
        if !project.is_deleted {
            return Err(ProjectError::ProjectNotInBin(id.to_string()));
        }
        self.repo.delete(id)?;
        Ok(())
    }

    /// Restores a soft-deleted project so it shows up in the main list again.
    pub fn restore(&self, id: &str) -> Result<Project, ProjectError> {
        let mut project = self.load(id)?;
        project.restore();
        self.repo.save(&project)?;
        Ok(project)
    }

    /// Removes a project's tracked metadata without touching its directory on
    /// disk — "stop indexing this". Works on any project regardless of
    /// `is_deleted`.
    pub fn untrack(&self, id: &str) -> Result<(), ProjectError> {
        self.load(id)?;
        self.repo.delete(id)?;
        Ok(())
    }
}
