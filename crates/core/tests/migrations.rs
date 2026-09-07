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
    "client": "Acme Corp",
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

fn table_exists(path: &Path, name: &str) -> bool {
    Connection::open(path)
        .expect("reopen")
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |r| r.get::<_, i64>(0),
        )
        .expect("query sqlite_master")
        > 0
}

fn column_exists(path: &Path, table: &str, column: &str) -> bool {
    let conn = Connection::open(path).expect("reopen");
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("table_info");
    let mut rows = stmt.query([]).expect("query");
    while let Some(row) = rows.next().expect("row") {
        let name: String = row.get(1).expect("column name");
        if name == column {
            return true;
        }
    }
    false
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
    // seed_v1 stamps user_version = 1 and meta.schema_version = '1'. Now that
    // CURRENT_SCHEMA_VERSION is 2, the `from < 2` step and the meta-upsert
    // that follows it are both required to make these assertions pass — this
    // proves `open` actually runs the v2 migration and keeps the mirror in
    // sync, rather than the seed data satisfying the checks on its own.
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
    // seed_v1 stamps user_version = 1, so the first open runs the `from < 2`
    // step (from == 1) and the second open does not (from == 2). If the v2
    // migration weren't idempotent — e.g. re-running the CREATE TABLE or ADD
    // COLUMN — the second open would error instead of returning cleanly, so
    // this test now genuinely distinguishes the two code paths.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let _first = SqliteRepository::open(&path).expect("first open migrates");
    drop(_first);
    let repo = SqliteRepository::open(&path).expect("second open must be a no-op");

    assert_eq!(user_version(&path), CURRENT_SCHEMA_VERSION);
    assert!(repo.get("seeded-1").expect("read").is_some());
}

#[test]
fn v2_adds_groups_and_the_project_group_column() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let _repo = SqliteRepository::open(&path).expect("open");

    // Opening migrates all the way to current, so this asserts the constant
    // rather than the literal 2 — what this test is about is that the v2 step
    // ran, which the table and the column below are the evidence for.
    assert_eq!(user_version(&path), CURRENT_SCHEMA_VERSION);
    assert!(table_exists(&path, "groups"));
    assert!(column_exists(&path, "projects", "group_id"));
}

#[test]
fn v2_leaves_existing_projects_ungrouped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let repo = SqliteRepository::open(&path).expect("open");

    let project = repo.get("seeded-1").expect("read").expect("still there");
    assert!(
        project.group_id.is_none(),
        "a migrated project must land in Ungrouped, not in a group that does not exist"
    );

    let column: Option<String> = Connection::open(&path)
        .expect("reopen")
        .query_row(
            "SELECT group_id FROM projects WHERE id = 'seeded-1'",
            [],
            |r| r.get(0),
        )
        .expect("read column");
    assert!(
        column.is_none(),
        "the mirrored column must agree with the blob"
    );
}

#[test]
fn v3_lifts_the_retired_client_field_into_properties() {
    // `client` stopped being a field and became one possible key in the
    // open-ended `properties` map. Serde would silently ignore the old key and
    // the value would vanish on the record's next save, so the migration has
    // to move it across rather than let it strand.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let repo = SqliteRepository::open(&path).expect("open");
    let project = repo.get("seeded-1").expect("read").expect("still there");

    assert_eq!(
        project.properties.get("client").map(String::as_str),
        Some("Acme Corp"),
        "the seeded client value must survive as properties[\"client\"]"
    );

    // And the key is gone from the blob, not just shadowed by the new map.
    let data: String = Connection::open(&path)
        .expect("reopen")
        .query_row("SELECT data FROM projects WHERE id = 'seeded-1'", [], |r| {
            r.get(0)
        })
        .expect("read blob");
    let blob: serde_json::Value = serde_json::from_str(&data).expect("blob parses");
    assert!(
        blob.get("client").is_none(),
        "the retired field must be removed from the blob, not left beside its replacement"
    );
}

#[test]
fn v3_leaves_a_project_without_a_client_with_no_properties() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    // Overwrite the seeded row with one whose client is null, the common case.
    {
        let conn = Connection::open(&path).expect("reopen");
        conn.execute(
            "UPDATE projects SET data = ?1 WHERE id = 'seeded-1'",
            [V1_PROJECT.replace(r#""client": "Acme Corp""#, r#""client": null"#)],
        )
        .expect("seed a clientless project");
    }

    let repo = SqliteRepository::open(&path).expect("open");
    let project = repo.get("seeded-1").expect("read").expect("still there");
    assert!(
        project.properties.is_empty(),
        "a null client must not become an empty-string property"
    );
}
