//! Operations that reach the filesystem rather than the store.
//!
//! Both are advisory-then-authoritative: a health check first for a better
//! error message, with the real filesystem call owning the failure.

use super::*;

impl ProjectService {
    /// Deletes a project's directory from disk, then either purges its
    /// metadata (`delete_metadata: true`) or keeps it soft-deleted in the
    /// bin. The only path that removes a directory.
    pub fn delete_directory(&self, id: &str, delete_metadata: bool) -> Result<(), ProjectError> {
        let mut project = self.load(id)?;
        remove_directory(&project.directory).map_err(ProjectError::DirectoryInaccessible)?;
        if delete_metadata {
            self.repo.delete(id)?;
        } else {
            project.mark_deleted();
            self.repo.save(&project)?;
        }
        Ok(())
    }

    /// IDs of live projects whose directory is no longer on disk — deleted or
    /// replaced by a file. An *inaccessible* directory (offline network
    /// drive, permissions hiccup) is deliberately not flagged.
    pub fn list_missing_directories(&self) -> Result<Vec<String>, ProjectError> {
        Ok(self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .filter(|p| {
                matches!(
                    check_directory_status(&p.directory),
                    DirectoryStatus::DoesNotExist | DirectoryStatus::NotADirectory
                )
            })
            .map(|p| p.id)
            .collect())
    }
}
