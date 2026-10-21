//! `icon list|import|delete`: custom icons, against a temporary icon store and
//! the in-memory database.

use std::path::{Path, PathBuf};

use indexer_core::{IconStore, UpdateProject};
use tempfile::TempDir;

use super::support::{context, never_asked, run, Scripted};
use crate::commands::Outcome;
use crate::context::Context;

/// A context whose icon store is a fresh temporary folder. The `TempDir` must
/// outlive the context, or the folder is deleted under it.
fn with_icons(consent: bool) -> (TempDir, Context) {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(consent, Scripted(never_asked));
    ctx.icons = IconStore::new(dir.path().join("icons"));
    (dir, ctx)
}

/// A small valid SVG at `dir/<file>`, to import.
fn svg(dir: &Path, file: &str) -> PathBuf {
    let path = dir.join(file);
    std::fs::write(
        &path,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10"/></svg>"#,
    )
    .unwrap();
    path
}

fn import(ctx: &Context, path: &Path) -> anyhow::Result<Outcome> {
    run(ctx, &["dexily", "icon", "import", path.to_str().unwrap()])
}

/// `(name, projects using it)` for every icon `icon list` shows.
fn listed(ctx: &Context) -> Vec<(String, usize)> {
    match run(ctx, &["dexily", "icon", "list"]).unwrap() {
        Outcome::CustomIcons { icons } => icons,
        other => panic!("expected custom icons, got {other:?}"),
    }
}

/// Puts the custom icon `name` on the project `project`, straight through
/// core — `edit --icon` checks names against the real store, not the test's.
fn wear(ctx: &Context, project: &str, name: &str) {
    let id = format!("{project}-0000-4000-8000-000000000000");
    let update = UpdateProject {
        icon: Some(Some(format!("custom:{name}"))),
        ..UpdateProject::default()
    };
    ctx.projects.update(&id, update).unwrap();
}

#[test]
fn an_empty_store_lists_nothing() {
    let (_dir, ctx) = with_icons(true);
    assert!(listed(&ctx).is_empty());
}

#[test]
fn import_names_the_icon_after_its_file_and_never_overwrites() {
    let (dir, ctx) = with_icons(true);
    let first = svg(dir.path(), "Logo.svg");
    let other = dir.path().join("other");
    std::fs::create_dir(&other).unwrap();
    let second = svg(&other, "logo.svg");

    match import(&ctx, &first).unwrap() {
        Outcome::IconImported { name } => assert_eq!(name, "logo"),
        other => panic!("expected an imported icon, got {other:?}"),
    }
    // A second `logo.svg` from another folder gets its own name.
    match import(&ctx, &second).unwrap() {
        Outcome::IconImported { name } => assert_eq!(name, "logo-2"),
        other => panic!("expected an imported icon, got {other:?}"),
    }
    assert_eq!(
        listed(&ctx),
        [("logo".to_string(), 0), ("logo-2".to_string(), 0)]
    );
}

#[test]
fn import_refuses_a_file_that_is_not_an_svg() {
    let (dir, ctx) = with_icons(true);
    let path = dir.path().join("notes.svg");
    std::fs::write(&path, "just some text").unwrap();

    assert!(import(&ctx, &path).is_err());
    assert!(
        listed(&ctx).is_empty(),
        "nothing is stored from a refused file"
    );
}

#[test]
fn list_counts_the_live_projects_wearing_each_icon() {
    let (dir, ctx) = with_icons(true);
    import(&ctx, &svg(dir.path(), "logo.svg")).unwrap();
    import(&ctx, &svg(dir.path(), "mascot.svg")).unwrap();
    wear(&ctx, "app", "logo");

    assert_eq!(
        listed(&ctx),
        [("logo".to_string(), 1), ("mascot".to_string(), 0)]
    );
}

#[test]
fn delete_answered_no_keeps_the_icon() {
    let (dir, ctx) = with_icons(false);
    import(&ctx, &svg(dir.path(), "logo.svg")).unwrap();

    assert!(matches!(
        run(&ctx, &["dexily", "icon", "delete", "logo"]).unwrap(),
        Outcome::Cancelled
    ));
    assert_eq!(listed(&ctx), [("logo".to_string(), 0)]);
}

#[test]
fn delete_takes_the_name_with_or_without_custom_and_counts_binned_wearers() {
    let (dir, ctx) = with_icons(true);
    import(&ctx, &svg(dir.path(), "logo.svg")).unwrap();
    wear(&ctx, "app", "logo");
    wear(&ctx, "old", "logo"); // binned, but it still wears it

    match run(&ctx, &["dexily", "icon", "delete", "custom:Logo"]).unwrap() {
        Outcome::IconDeleted { name, users } => {
            assert_eq!(name, "logo");
            assert_eq!(users, 2);
        }
        other => panic!("expected a deleted icon, got {other:?}"),
    }
    assert!(listed(&ctx).is_empty());
}

#[test]
fn delete_of_an_unknown_icon_names_the_ones_there_are() {
    let (dir, ctx) = with_icons(true);
    let err = run(&ctx, &["dexily", "icon", "delete", "logo"]).unwrap_err();
    assert_eq!(
        err.to_string(),
        "no custom icon named \"logo\": there are none yet"
    );

    import(&ctx, &svg(dir.path(), "mascot.svg")).unwrap();
    let err = run(&ctx, &["dexily", "icon", "delete", "logo"]).unwrap_err();
    assert_eq!(
        err.to_string(),
        "no custom icon named \"logo\". Custom icons: mascot"
    );
}
