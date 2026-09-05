//! Migration fixtures: seed a database at a known `user_version`, run `open`,
//! and assert the result of each step.
//!
//! `ROADMAP.md` gates this scaffold on `CURRENT_SCHEMA_VERSION` reaching 2.
//! The point is that a *new* binary opening an *old* database is routine — so
//! every step gets a fixture asserting the shape it produces and that existing
//! rows survive it.

use std::path::Path;

use indexer_core::infra::{SqliteRepository, CURRENT_SCHEMA_VERSION};
use indexer_core::ProjectReader;
use rusqlite::Connection;

/// The v1 schema, verbatim as the `from < 1` migration step wrote it. Copied
/// rather than referenced on purpose: a fixture must keep describing the old
/// shape even after the production code has moved on.
const V1_SCHEMA: &str = "
    CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    INSERT INTO meta (key, value) VALUES
      ('app', 'project-indexer'), ('schema_version', '1');
    CREATE TABLE projects (
      id                   TEXT PRIMARY KEY,
      data                 TEXT NOT NULL,
      is_deleted           INTEGER NOT NULL,
      directory_normalized TEXT NOT NULL,
      updated_at           TEXT NOT NULL
    );
    CREATE INDEX idx_projects_is_deleted ON projects(is_deleted);
    CREATE INDEX idx_projects_directory_normalized ON projects(directory_normalized);
    CREATE TABLE project_tags (
      project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
      tag        TEXT NOT NULL,
      PRIMARY KEY (project_id, tag)
    );
    CREATE INDEX idx_project_tags_tag ON project_tags(tag);
";

/// One project as a v1 binary would have written it — no group, colour or icon.
const V1_PROJECT: &str = r#"{
    "is_deleted": false,
    "id": "seeded-1",
    "name": "Seeded",
    "description": "",
    "directory": "/tmp/seeded",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z",
    "last_opened_at": null,
    "tags": ["rust"],
    "favorite": false,
    "open_with": null,
    "notes": null,
    "client": null,
    "trackers": []
}"#;

fn seed_v1(path: &Path) {
    let conn = Connection::open(path).expect("open fixture db");
    conn.execute_batch(V1_SCHEMA).expect("v1 schema");
    conn.execute(
        "INSERT INTO projects (id, data, is_deleted, directory_normalized, updated_at)
         VALUES ('seeded-1', ?1, 0, '/tmp/seeded', '2024-01-01T00:00:00Z')",
        [V1_PROJECT],
    )
    .expect("seed project");
    conn.execute(
        "INSERT INTO project_tags (project_id, tag) VALUES ('seeded-1', 'rust')",
        [],
    )
    .expect("seed tag");
    conn.pragma_update(None, "user_version", 1)
        .expect("stamp v1");
}

fn user_version(path: &Path) -> i64 {
    Connection::open(path)
        .expect("reopen")
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("read user_version")
}

fn meta_schema_version(path: &Path) -> String {
    Connection::open(path)
        .expect("reopen")
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .expect("read meta schema_version")
}

#[test]
fn a_v1_database_opens_and_keeps_its_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let repo = SqliteRepository::open(&path).expect("v1 database must open");

    let project = repo
        .get("seeded-1")
        .expect("read")
        .expect("the seeded project must survive migration");
    assert_eq!(project.name, "Seeded");
    assert_eq!(project.tags, vec!["rust".to_string()]);
}

#[test]
fn opening_stamps_the_current_schema_version_everywhere() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let _repo = SqliteRepository::open(&path).expect("open");

    assert_eq!(user_version(&path), CURRENT_SCHEMA_VERSION);
    assert_eq!(
        meta_schema_version(&path),
        CURRENT_SCHEMA_VERSION.to_string()
    );
}

#[test]
fn a_database_from_a_newer_binary_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);
    Connection::open(&path)
        .expect("open")
        .pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1)
        .expect("stamp a future version");

    assert!(
        SqliteRepository::open(&path).is_err(),
        "a database written by a newer binary must be refused, not migrated"
    );
}

#[test]
fn opening_an_already_current_database_is_a_no_op() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let _first = SqliteRepository::open(&path).expect("first open migrates");
    drop(_first);
    let repo = SqliteRepository::open(&path).expect("second open must be a no-op");

    assert_eq!(user_version(&path), CURRENT_SCHEMA_VERSION);
    assert!(repo.get("seeded-1").expect("read").is_some());
}
