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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::SqliteRepository;

    fn service() -> GroupService {
        GroupService::new(Arc::new(
            SqliteRepository::in_memory().expect("in-memory db"),
        ))
    }

    #[test]
    fn create_appends_after_the_last_group() {
        let svc = service();
        svc.create("First".into(), "cyan".into(), "briefcase".into())
            .unwrap();
        let second = svc
            .create("Second".into(), "gold".into(), "star".into())
            .unwrap();
        assert_eq!(second.position, 1);
    }

    #[test]
    fn create_rejects_a_duplicate_name_ignoring_case() {
        let svc = service();
        svc.create("Client work".into(), "cyan".into(), "briefcase".into())
            .unwrap();
        let result = svc.create("CLIENT WORK".into(), "gold".into(), "star".into());
        assert!(matches!(result, Err(ProjectError::DuplicateGroupName(_))));
    }

    #[test]
    fn update_renames_a_group() {
        let svc = service();
        let g = svc
            .create("Old".into(), "cyan".into(), "briefcase".into())
            .unwrap();
        let updated = svc
            .update(
                &g.id,
                UpdateGroup {
                    name: Some("New".into()),
                    color: None,
                    icon: None,
                },
            )
            .unwrap();
        assert_eq!(updated.name, "New");
    }

    #[test]
    fn update_rejects_a_name_another_group_already_has() {
        let svc = service();
        svc.create("Taken".into(), "cyan".into(), "briefcase".into())
            .unwrap();
        let g = svc
            .create("Mine".into(), "gold".into(), "star".into())
            .unwrap();
        let result = svc.update(
            &g.id,
            UpdateGroup {
                name: Some("taken".into()),
                color: None,
                icon: None,
            },
        );
        assert!(matches!(result, Err(ProjectError::DuplicateGroupName(_))));
    }

    #[test]
    fn update_allows_a_group_to_keep_its_own_name() {
        let svc = service();
        let g = svc
            .create("Mine".into(), "cyan".into(), "briefcase".into())
            .unwrap();
        let updated = svc
            .update(
                &g.id,
                UpdateGroup {
                    name: Some("Mine".into()),
                    color: Some("gold".into()),
                    icon: None,
                },
            )
            .unwrap();
        assert_eq!(updated.color, "gold");
    }

    #[test]
    fn update_of_a_missing_group_is_not_found() {
        let svc = service();
        let result = svc.update(
            "nope",
            UpdateGroup {
                name: Some("x".into()),
                color: None,
                icon: None,
            },
        );
        assert!(matches!(result, Err(ProjectError::GroupNotFound(_))));
    }

    #[test]
    fn reorder_returns_the_new_order() {
        let svc = service();
        let a = svc
            .create("A".into(), "cyan".into(), "briefcase".into())
            .unwrap();
        let b = svc
            .create("B".into(), "gold".into(), "star".into())
            .unwrap();
        let names: Vec<String> = svc
            .reorder(vec![b.id.clone(), a.id.clone()])
            .unwrap()
            .into_iter()
            .map(|g| g.name)
            .collect();
        assert_eq!(names, vec!["B", "A"]);
    }
}
