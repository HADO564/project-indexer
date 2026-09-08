use std::sync::Arc;

use crate::domain::{Group, UpdateGroup};
use crate::error::ProjectError;
use crate::ports::GroupRepository;

/// Orchestration for groups: uniqueness, positioning, and the read-back that
/// gives callers the persisted value rather than the one they sent.
///
/// Assigning a *project* to a group is not here — that is an ordinary
/// `UpdateProject.group_id` and belongs to `ProjectService`.
pub struct GroupService {
    repo: Arc<dyn GroupRepository>,
}

/// Opaque by necessity — the field is a `dyn` port with no `Debug` bound.
/// Mirrors `ProjectService`, so types holding one can still derive `Debug`.
impl std::fmt::Debug for GroupService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupService").finish_non_exhaustive()
    }
}

impl GroupService {
    pub fn new(repo: Arc<dyn GroupRepository>) -> Self {
        Self { repo }
    }

    pub fn list(&self) -> Result<Vec<Group>, ProjectError> {
        Ok(self.repo.list_groups()?)
    }

    /// Appends after the last group. New groups never displace existing ones —
    /// `reorder` is the only thing that renumbers.
    pub fn create(&self, name: String, color: String, icon: String) -> Result<Group, ProjectError> {
        let existing = self.repo.list_groups()?;
        Group::check_for_duplicate_name(&name, &existing)?;

        let position = existing.iter().map(|g| g.position).max().unwrap_or(-1) + 1;
        // This read-then-write is safe only because Tauri's ExecutionContext::Blocking
        // serializes non-async commands: we list all groups to compute the next position,
        // then save. If commands were concurrent, another create could pick the same
        // position, causing a duplicate sidebar entry; the position index is non-unique.
        let group = Group::new(name, color, icon, position)?;
        self.repo.save_group(&group)?;
        Ok(group)
    }

    pub fn update(&self, id: &str, update: UpdateGroup) -> Result<Group, ProjectError> {
        let mut group = self
            .repo
            .get_group(id)?
            .ok_or_else(|| ProjectError::GroupNotFound(id.to_string()))?;

        // A group keeping its own name is not a duplicate, so exclude itself
        // from the check rather than comparing against every group.
        if let Some(name) = &update.name {
            let others: Vec<Group> = self
                .repo
                .list_groups()?
                .into_iter()
                .filter(|g| g.id != group.id)
                .collect();
            Group::check_for_duplicate_name(name, &others)?;
            // This check-then-write is safe only because Tauri's ExecutionContext::Blocking
            // serializes non-async commands: we verify the name against other groups, then
            // save. If commands were concurrent, another update could change a group's name
            // to our target between the check and the save; the database's case-insensitive
            // unique index is a backstop that prevents silent data corruption.
        }

        group.update(update)?;
        self.repo.save_group(&group)?;
        Ok(group)
    }

    /// Members become Ungrouped; no project is deleted.
    pub fn delete(&self, id: &str) -> Result<(), ProjectError> {
        self.repo.delete_group(id)?;
        Ok(())
    }

    pub fn reorder(&self, ordered_ids: Vec<String>) -> Result<Vec<Group>, ProjectError> {
        self.repo.set_group_positions(&ordered_ids)?;
        self.list()
    }
}
