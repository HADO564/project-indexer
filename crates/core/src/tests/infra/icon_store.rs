//! Tests for [`crate::infra::icon_store`].

use crate::infra::icon_store::*;

const GOOD: &str = r#"<svg viewBox="0 0 24 24"><path d="M3 6h18"/></svg>"#;

fn store() -> (tempfile::TempDir, IconStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = IconStore::new(dir.path().join("icons"));
    (dir, store)
}

fn source(dir: &std::path::Path, file: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(file);
    std::fs::write(&path, body).expect("write source");
    path
}

#[test]
fn imports_sanitizes_and_lists_an_icon() {
    let (dir, store) = store();
    let src = source(dir.path(), "My Logo.svg", GOOD);

    let imported = store.import(&src).expect("import");
    assert_eq!(imported.name, "my-logo");
    assert!(imported.svg.contains("M3 6h18"));

    let listed = store.list().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "my-logo");
}

#[test]
fn import_rejects_a_hostile_svg_rather_than_storing_it() {
    let (dir, store) = store();
    let src = source(
        dir.path(),
        "bad.svg",
        r#"<svg viewBox="0 0 1 1"><script>alert(1)</script></svg>"#,
    );

    assert!(store.import(&src).is_err());
    assert!(
        store.list().expect("list").is_empty(),
        "nothing hostile may reach the store"
    );
}

#[test]
fn import_stores_the_sanitized_form_not_the_original() {
    let (dir, store) = store();
    let src = source(
        dir.path(),
        "mixed.svg",
        r#"<svg viewBox="0 0 1 1"><script>alert(1)</script><path d="M0 0"/></svg>"#,
    );

    store.import(&src).expect("import");
    let stored = &store.list().expect("list")[0];
    assert!(!stored.svg.contains("script"));
    assert!(!stored.svg.contains("alert"));
}

#[test]
fn a_second_icon_with_the_same_name_does_not_overwrite_the_first() {
    let (dir, store) = store();
    let a = source(dir.path(), "logo.svg", GOOD);
    std::fs::create_dir_all(dir.path().join("other")).expect("subdir");
    let b = source(&dir.path().join("other"), "logo.svg", GOOD);

    let first = store.import(&a).expect("first");
    let second = store.import(&b).expect("second");

    assert_eq!(first.name, "logo");
    assert_eq!(second.name, "logo-2");
    assert_eq!(store.list().expect("list").len(), 2);
}

#[test]
fn deletes_an_icon() {
    let (dir, store) = store();
    let src = source(dir.path(), "logo.svg", GOOD);
    store.import(&src).expect("import");

    store.delete("logo").expect("delete");

    assert!(store.list().expect("list").is_empty());
}

#[test]
fn delete_refuses_a_name_that_could_escape_the_store() {
    let (_dir, store) = store();
    assert!(store.delete("../projects.db").is_err());
    assert!(store.delete("nested/name").is_err());
    assert!(store.delete("..").is_err());
    // The native Windows separator, distinct from the forward-slash case
    // above: `dir.join("nested\\name.svg")` would otherwise reach into a
    // subdirectory just as readily as `/` does.
    assert!(store.delete("nested\\name").is_err());
    // An absolute, drive-rooted path: `PathBuf::join` replaces the whole
    // base when the joined component is itself absolute, so this is the
    // most direct way a name could address a file outside the store.
    assert!(store.delete("c:\\windows\\system32\\config").is_err());
}

#[test]
fn listing_an_absent_directory_is_empty_not_an_error() {
    let (_dir, store) = store();
    assert!(store
        .list()
        .expect("list must tolerate a store never written to")
        .is_empty());
}

#[test]
fn list_skips_a_directory_masquerading_as_an_icon_and_import_still_works() {
    let (dir, store) = store();
    let good = source(dir.path(), "logo.svg", GOOD);
    store.import(&good).expect("import the real icon");

    // A directory literally named `trap.svg` — same extension, same
    // apparent stem, but `read_to_string` on it fails ("Access is
    // denied" on Windows). It must be skipped, not propagated.
    let store_dir = dir.path().join("icons");
    std::fs::create_dir_all(store_dir.join("trap.svg")).expect("plant a trap directory");

    let listed = store.list().expect("list must skip the trap, not fail");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "logo");

    // `import` calls `list` via `unique_name`, so the trap must not
    // disable importing either.
    let other = source(dir.path(), "other.svg", GOOD);
    let imported = store
        .import(&other)
        .expect("import must not be disabled by a bad entry elsewhere in the store");
    assert_eq!(imported.name, "other");
}

#[test]
fn list_skips_a_non_utf8_svg_file_and_import_still_works() {
    let (dir, store) = store();
    let good = source(dir.path(), "logo.svg", GOOD);
    store.import(&good).expect("import the real icon");

    // A renamed PNG or a truncated write: valid filename, invalid UTF-8
    // contents. Must be skipped, not propagated.
    let store_dir = dir.path().join("icons");
    std::fs::write(store_dir.join("trap.svg"), [0xFF, 0xFE, 0xFD]).expect("plant a non-utf8 file");

    let listed = store
        .list()
        .expect("list must skip invalid utf-8, not fail");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "logo");

    let other = source(dir.path(), "other.svg", GOOD);
    let imported = store
        .import(&other)
        .expect("import must not be disabled by a bad entry elsewhere in the store");
    assert_eq!(imported.name, "other");
}

#[test]
fn list_does_not_offer_a_name_that_delete_would_refuse() {
    let (dir, store) = store();
    let store_dir = dir.path().join("icons");
    std::fs::create_dir_all(&store_dir).expect("create store dir");
    // Written directly to disk, bypassing `slugify` — the only way a
    // non-slug stem can land in the store.
    std::fs::write(store_dir.join("Weird Name.svg"), GOOD).expect("plant a non-slug stem");

    let listed = store.list().expect("list");
    assert!(
        listed.is_empty(),
        "a name delete would refuse must not be listed either"
    );
}
