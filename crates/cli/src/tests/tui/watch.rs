//! The live-refresh watch against a real database file, set up the way the
//! app's is (WAL, migrated by core), with a second connection standing in for
//! the GUI or another shell.

use indexer_core::SqliteRepository;
use rusqlite::Connection;

use crate::tui::watch::Watch;

#[test]
fn a_write_from_another_connection_is_noticed_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("projects.db");
    // Core opens and migrates it, as `Context::open` does; kept open, like the
    // TUI's own connection.
    let _repo = SqliteRepository::open(&path).unwrap();
    let mut watch = Watch::open(&path).unwrap();

    assert!(!watch.changed().unwrap(), "nothing written yet");

    let other = Connection::open(&path).unwrap();
    other
        .execute_batch("CREATE TABLE scratch (x); INSERT INTO scratch VALUES (1);")
        .unwrap();
    assert!(watch.changed().unwrap(), "another connection committed");
    assert!(
        !watch.changed().unwrap(),
        "the same change is reported once"
    );

    other.execute("INSERT INTO scratch VALUES (2)", []).unwrap();
    assert!(watch.changed().unwrap(), "a later commit is noticed too");
}

#[test]
fn a_read_from_another_connection_is_not_a_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("projects.db");
    let _repo = SqliteRepository::open(&path).unwrap();
    let mut watch = Watch::open(&path).unwrap();

    let other = Connection::open(&path).unwrap();
    let _: i64 = other
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))
        .unwrap();
    assert!(!watch.changed().unwrap());
}

#[test]
fn the_watch_is_read_only_and_never_creates_a_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("projects.db");
    // A writable open would create an empty file here; a read-only one can't.
    assert!(Watch::open(&path).is_err());
    assert!(!path.exists());
}
