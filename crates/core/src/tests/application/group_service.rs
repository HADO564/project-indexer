//! Tests for [`crate::application::group_service`].

use crate::domain::UpdateGroup;
use crate::error::ProjectError;
use std::sync::Arc;

use crate::application::group_service::*;
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
