//! Tests for [`crate::domain::group`].

use crate::domain::update_group::UpdateGroup;
use crate::error::ProjectError;

use crate::domain::group::*;

fn sample() -> Group {
    Group::new("Client work".into(), "cyan".into(), "briefcase".into(), 0).expect("valid group")
}

#[test]
fn new_group_trims_its_name_and_stamps_timestamps() {
    let g = Group::new(
        "  Client work  ".into(),
        "cyan".into(),
        "briefcase".into(),
        3,
    )
    .expect("valid group");
    assert_eq!(g.name, "Client work");
    assert_eq!(g.position, 3);
    assert_eq!(g.created_at, g.updated_at);
    assert!(!g.id.is_empty());
}

#[test]
fn rejects_an_empty_name() {
    let result = Group::new("   ".into(), "cyan".into(), "briefcase".into(), 0);
    assert!(matches!(result, Err(ProjectError::InvalidGroupName)));
}

#[test]
fn rejects_an_unknown_colour() {
    let result = Group::new(
        "Personal".into(),
        "chartreuse".into(),
        "briefcase".into(),
        0,
    );
    assert!(matches!(result, Err(ProjectError::UnknownSwatch(_))));

    // Not a normalisation gap either: everything but the exact six-digit
    // form is refused, because the value reaches a `style` attribute.
    for bad in ["#ff0", "#gggggg", "red", "#ff0000; background: url(x)"] {
        let result = Group::new("Personal".into(), bad.into(), "briefcase".into(), 0);
        assert!(
            matches!(result, Err(ProjectError::UnknownSwatch(_))),
            "{bad} should not be a valid group colour"
        );
    }
}

#[test]
fn accepts_a_hex_literal_from_the_colour_picker() {
    // Groups take a picked colour like projects do. A palette name is
    // still the better answer — it is what lets a future theme recolour
    // everything coherently — but that is a reason to offer the palette
    // first, not to refuse a colour someone actually wants.
    let group = Group::new("Personal".into(), "#ff0000".into(), "briefcase".into(), 0)
        .expect("a hex literal is a valid group colour");
    assert_eq!(group.color, "#ff0000");
}

#[test]
fn rejects_an_empty_icon() {
    let result = Group::new("Personal".into(), "cyan".into(), "  ".into(), 0);
    assert!(matches!(result, Err(ProjectError::InvalidGroupIcon)));
}

#[test]
fn duplicate_name_check_ignores_case_and_surrounding_space() {
    let existing = vec![sample()];
    let result = Group::check_for_duplicate_name("  CLIENT WORK ", &existing);
    assert!(matches!(result, Err(ProjectError::DuplicateGroupName(_))));
}

#[test]
fn duplicate_name_check_allows_a_distinct_name() {
    let existing = vec![sample()];
    assert!(Group::check_for_duplicate_name("Personal", &existing).is_ok());
}

#[test]
fn update_applies_only_the_fields_present() {
    let mut g = sample();
    let before = g.color.clone();
    g.update(UpdateGroup {
        name: Some("Renamed".into()),
        color: None,
        icon: None,
    })
    .expect("valid update");
    assert_eq!(g.name, "Renamed");
    assert_eq!(g.color, before);
}

#[test]
fn update_rejects_an_unknown_colour_and_changes_nothing() {
    let mut g = sample();
    let result = g.update(UpdateGroup {
        name: Some("Renamed".into()),
        color: Some("chartreuse".into()),
        icon: None,
    });
    assert!(matches!(result, Err(ProjectError::UnknownSwatch(_))));
    assert_eq!(g.name, "Client work");
}
