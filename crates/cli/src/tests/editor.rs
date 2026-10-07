//! The directory typed into the edit form, resolved before it is saved.

use std::path::PathBuf;

use indexer_core::UpdateProject;

use crate::editor::{expand_home, resolve_appearance, resolve_directory};

fn typed(directory: &str) -> UpdateProject {
    UpdateProject {
        directory: Some(directory.to_string()),
        ..Default::default()
    }
}

#[test]
fn a_leading_tilde_is_the_home_folder() {
    let home = Some(PathBuf::from("/home/me"));
    assert_eq!(expand_home("~", home.clone()), PathBuf::from("/home/me"));
    assert_eq!(
        expand_home("~/work/app", home.clone()),
        PathBuf::from("/home/me/work/app")
    );
}

#[test]
fn a_tilde_anywhere_else_is_left_alone() {
    let home = Some(PathBuf::from("/home/me"));
    // `~bob` is another user's home, which a shell would resolve and this
    // does not pretend to.
    assert_eq!(
        expand_home("~bob/app", home.clone()),
        PathBuf::from("~bob/app")
    );
    assert_eq!(expand_home("work/~", home), PathBuf::from("work/~"));
    assert_eq!(expand_home("~/app", None), PathBuf::from("~/app"));
}

#[test]
fn an_untouched_directory_is_left_unresolved() {
    let mut changes = UpdateProject::default();
    resolve_directory(&mut changes).unwrap();
    assert_eq!(changes.directory, None);
}

#[test]
fn a_typed_directory_is_stored_absolute_and_resolved() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("app")).unwrap();
    let roundabout = dir.path().join("app").join("..").join("app");
    let mut changes = typed(&roundabout.to_string_lossy());
    resolve_directory(&mut changes).unwrap();
    let expected = std::fs::canonicalize(dir.path().join("app")).unwrap();
    assert_eq!(
        changes.directory.as_deref(),
        Some(&*expected.to_string_lossy())
    );
}

#[test]
fn an_emptied_box_is_refused_with_a_reason() {
    let mut changes = typed("   ");
    let error = resolve_directory(&mut changes).unwrap_err().to_string();
    assert_eq!(error, "a project needs a directory");
}

#[test]
fn a_missing_folder_is_refused_as_a_move() {
    let dir = tempfile::tempdir().unwrap();
    let mut changes = typed(&dir.path().join("nope").to_string_lossy());
    let error = format!("{:#}", resolve_directory(&mut changes).unwrap_err());
    assert!(error.starts_with("cannot move to "), "{error}");
}

// Colour and icon, checked on save so a bad one keeps the form open.

#[test]
fn appearance_is_spelled_as_stored_and_clears_pass_through() {
    let mut changes = UpdateProject {
        color: Some(Some("Cyan".into())),
        icon: Some(None),
        ..Default::default()
    };
    resolve_appearance(&mut changes).unwrap();
    assert_eq!(changes.color, Some(Some("cyan".into())));
    assert_eq!(changes.icon, Some(None));
}

#[test]
fn a_bad_colour_or_icon_is_the_reason_not_saved() {
    let mut changes = UpdateProject {
        color: Some(Some("red".into())),
        ..Default::default()
    };
    let err = resolve_appearance(&mut changes).unwrap_err().to_string();
    assert!(err.starts_with("not a colour: \"red\""), "{err}");

    let mut changes = UpdateProject {
        icon: Some(Some("rockt".into())),
        ..Default::default()
    };
    assert!(resolve_appearance(&mut changes).is_err());
}
