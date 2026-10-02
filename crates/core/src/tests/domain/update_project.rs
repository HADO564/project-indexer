use crate::domain::UpdateProject;

#[test]
fn the_default_update_is_empty() {
    assert!(UpdateProject::default().is_empty());
}

#[test]
fn any_field_set_is_not_empty() {
    let updates = [
        UpdateProject {
            description: Some(String::new()),
            ..Default::default()
        },
        UpdateProject {
            tags: Some(Vec::new()),
            ..Default::default()
        },
        UpdateProject {
            notes: Some(None),
            ..Default::default()
        },
        UpdateProject {
            icon: Some(Some("rocket".into())),
            ..Default::default()
        },
    ];
    for update in updates {
        assert!(!update.is_empty(), "{update:?}");
    }
}
