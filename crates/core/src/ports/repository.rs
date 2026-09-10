use crate::domain::Project;
use crate::error::RepositoryError;

/// Read access to stored projects. Split from [`ProjectRepository`] so an
/// external consumer (devmon) can depend on reads without the write surface.
pub trait ProjectReader: Send + Sync {
    fn get(&self, id: &str) -> Result<Option<Project>, RepositoryError>;
    /// Every project, deleted included, no ordering guarantee.
    fn list(&self) -> Result<Vec<Project>, RepositoryError>;
    /// IDs of every **live** project — binned ones excluded, unlike [`list`](Self::list).
    ///
    /// Exists so a caller can work through projects one at a time instead of
    /// holding them all in memory: the re-detect sweep loads, updates and drops
    /// each project in turn, so its peak memory is one project whatever the
    /// database holds.
    fn list_ids(&self) -> Result<Vec<String>, RepositoryError>;

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

use crate::domain::Group;

/// Read access to stored groups. Split from [`GroupRepository`] for the same
/// reason [`ProjectReader`] is split from [`ProjectRepository`]: an external
/// consumer can depend on reads without the write surface.
pub trait GroupReader: Send + Sync {
    fn get_group(&self, id: &str) -> Result<Option<Group>, RepositoryError>;
    /// Every group, ordered by `position` ascending.
    fn list_groups(&self) -> Result<Vec<Group>, RepositoryError>;
}

pub trait GroupRepository: GroupReader {
    /// Insert or replace by `group.id`.
    fn save_group(&self, group: &Group) -> Result<(), RepositoryError>;

    /// Deletes the group **and** clears membership from every project that
    /// belonged to it, in one transaction. Idempotent — a missing id is `Ok(())`.
    ///
    /// The blob and the column are cleared together deliberately: the
    /// `ON DELETE SET NULL` constraint on the column would leave a stale
    /// `group_id` inside the JSON, which is the authoritative copy.
    fn delete_group(&self, id: &str) -> Result<(), RepositoryError>;

    /// Rewrites `position` so it matches the given order, in one transaction.
    /// The single path that renumbers; ids not listed are left untouched.
    fn set_group_positions(&self, ordered_ids: &[String]) -> Result<(), RepositoryError>;
}
