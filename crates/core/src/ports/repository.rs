use crate::domain::Project;
use crate::error::RepositoryError;

/// Read access to stored projects. Split from [`ProjectRepository`] so an
/// external consumer (devmon) can depend on reads without the write surface.
pub trait ProjectReader: Send + Sync {
    fn get(&self, id: &str) -> Result<Option<Project>, RepositoryError>;
    /// Every project, deleted included, no ordering guarantee.
    fn list(&self) -> Result<Vec<Project>, RepositoryError>;
    /// `normalized_directory` must already be `normalize_directory`'d.
    fn find_by_directory(
        &self,
        normalized_directory: &str,
    ) -> Result<Option<Project>, RepositoryError>;
}

pub trait ProjectRepository: ProjectReader {
    /// Insert or replace by `project.id`.
    fn save(&self, project: &Project) -> Result<(), RepositoryError>;
    /// Idempotent — a missing id is `Ok(())`.
    fn delete(&self, id: &str) -> Result<(), RepositoryError>;
}
