//! Tests for [`crate::infra::sqlite_repository`].

use crate::domain::{Group, Project};
use crate::error::RepositoryError;
use crate::ports::{GroupReader, GroupRepository, ProjectReader, ProjectRepository};

use crate::infra::sqlite_repository::*;

fn sample(id: &str, dir: &str) -> Project {
    let mut p = Project::new("Name".into(), dir.into(), None, Some(vec!["Rust".into()]))
        .expect("dir must exist for Project::new");
    p.id = id.to_string();
    p
}

// Project::new validates the directory exists, so tests point at a real temp dir.
fn tmp() -> String {
    std::env::temp_dir().to_string_lossy().into_owned()
}

fn group(name: &str, position: i64) -> Group {
    Group::new(name.into(), "cyan".into(), "briefcase".into(), position).expect("valid group")
}

#[test]
fn round_trips_a_group() {
    let repo = SqliteRepository::in_memory().unwrap();
    let g = group("Client work", 0);
    repo.save_group(&g).unwrap();
    let got = repo.get_group(&g.id).unwrap().unwrap();
    assert_eq!(got.name, "Client work");
    assert_eq!(got.color, "cyan");
    assert_eq!(got.icon, "briefcase");
}

#[test]
fn lists_groups_in_position_order() {
    let repo = SqliteRepository::in_memory().unwrap();
    repo.save_group(&group("Third", 2)).unwrap();
    repo.save_group(&group("First", 0)).unwrap();
    repo.save_group(&group("Second", 1)).unwrap();

    let names: Vec<String> = repo
        .list_groups()
        .unwrap()
        .into_iter()
        .map(|g| g.name)
        .collect();
    assert_eq!(names, vec!["First", "Second", "Third"]);
}

#[test]
fn save_writes_group_id_to_both_the_blob_and_the_column() {
    let repo = SqliteRepository::in_memory().unwrap();
    let g = group("Client work", 0);
    repo.save_group(&g).unwrap();

    let mut p = sample("id-1", &tmp());
    p.group_id = Some(g.id.clone());
    repo.save(&p).unwrap();

    assert_eq!(
        repo.get("id-1").unwrap().unwrap().group_id,
        Some(g.id.clone())
    );
    let column: Option<String> = repo
        .lock_conn()
        .query_row("SELECT group_id FROM projects WHERE id = 'id-1'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(column, Some(g.id));
}

#[test]
fn deleting_a_group_ungroups_its_projects_in_blob_and_column() {
    let repo = SqliteRepository::in_memory().unwrap();
    let g = group("Client work", 0);
    repo.save_group(&g).unwrap();
    let mut p = sample("id-1", &tmp());
    p.group_id = Some(g.id.clone());
    repo.save(&p).unwrap();

    repo.delete_group(&g.id).unwrap();

    // The project survives — deleting a group never deletes a project.
    let got = repo.get("id-1").unwrap().expect("project must survive");
    assert!(
        got.group_id.is_none(),
        "the blob must be cleared, not just the column"
    );
    let column: Option<String> = repo
        .lock_conn()
        .query_row("SELECT group_id FROM projects WHERE id = 'id-1'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(column.is_none());
    assert!(repo.get_group(&g.id).unwrap().is_none());
}

#[test]
fn deleting_a_missing_group_is_idempotent() {
    let repo = SqliteRepository::in_memory().unwrap();
    assert!(repo.delete_group("nope").is_ok());
}

#[test]
fn set_group_positions_rewrites_the_whole_ordering() {
    let repo = SqliteRepository::in_memory().unwrap();
    let a = group("A", 0);
    let b = group("B", 1);
    let c = group("C", 2);
    for g in [&a, &b, &c] {
        repo.save_group(g).unwrap();
    }

    repo.set_group_positions(&[c.id.clone(), a.id.clone(), b.id.clone()])
        .unwrap();

    let names: Vec<String> = repo
        .list_groups()
        .unwrap()
        .into_iter()
        .map(|g| g.name)
        .collect();
    assert_eq!(names, vec!["C", "A", "B"]);
}

#[test]
fn round_trips_a_project() {
    let repo = SqliteRepository::in_memory().unwrap();
    let p = sample("id-1", &tmp());
    repo.save(&p).unwrap();
    let got = repo.get("id-1").unwrap().unwrap();
    assert_eq!(got.id, "id-1");
    assert_eq!(got.tags, vec!["Rust".to_string()]);
}

#[test]
fn save_replaces_on_conflict() {
    let repo = SqliteRepository::in_memory().unwrap();
    let mut p = sample("id-1", &tmp());
    repo.save(&p).unwrap();
    p.name = "Renamed".into();
    repo.save(&p).unwrap();
    assert_eq!(repo.list().unwrap().len(), 1);
    assert_eq!(repo.get("id-1").unwrap().unwrap().name, "Renamed");
}

fn tag_count(repo: &SqliteRepository) -> i64 {
    repo.lock_conn()
        .query_row("SELECT COUNT(*) FROM project_tags", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn delete_is_idempotent_and_cascades_tags() {
    let repo = SqliteRepository::in_memory().unwrap();
    repo.save(&sample("id-1", &tmp())).unwrap();
    // `sample` carries one tag — prove `save` actually wrote it, so the
    // post-delete count of 0 means the cascade fired (not that nothing
    // was ever mirrored).
    assert_eq!(tag_count(&repo), 1);
    repo.delete("id-1").unwrap();
    repo.delete("id-1").unwrap(); // no error
    assert!(repo.get("id-1").unwrap().is_none());
    assert_eq!(tag_count(&repo), 0);
}

#[test]
fn save_populates_and_replaces_tag_mirror() {
    let repo = SqliteRepository::in_memory().unwrap();
    let mut p = sample("id-1", &tmp());
    p.tags = vec!["Rust".into(), "Cli".into()];
    repo.save(&p).unwrap();

    let mirrored = |repo: &SqliteRepository| -> Vec<String> {
        let conn = repo.lock_conn();
        let mut stmt = conn
            .prepare("SELECT tag FROM project_tags WHERE project_id = ?1 ORDER BY tag")
            .unwrap();
        let rows = stmt
            .query_map(["id-1"], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect::<Vec<_>>();
        rows
    };
    assert_eq!(mirrored(&repo), vec!["Cli".to_string(), "Rust".to_string()]);

    p.tags = vec!["Web".into()];
    repo.save(&p).unwrap();
    assert_eq!(mirrored(&repo), vec!["Web".to_string()]);
}

#[test]
fn find_by_directory_prefers_the_live_most_recent_row() {
    // Soft-deleting a project at dir D then creating a new one at the same
    // D is supported (the dup-check ignores deleted rows), so two rows can
    // share `directory_normalized`. `find_by_directory` must return the
    // live one.
    let repo = SqliteRepository::in_memory().unwrap();
    let dir = tmp();
    let normalized = crate::domain::normalize::normalize_directory(&dir);

    let mut a = sample("id-a", &dir);
    repo.save(&a).unwrap();
    a.is_deleted = true;
    a.updated_at = chrono::Utc::now();
    repo.save(&a).unwrap();

    let b = sample("id-b", &dir);
    repo.save(&b).unwrap();

    let found = repo.find_by_directory(&normalized).unwrap().unwrap();
    assert_eq!(found.id, "id-b");
    assert!(!found.is_deleted);
}

#[test]
fn open_creates_schema_on_a_file_db() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let repo = SqliteRepository::open(&path).unwrap();
    let conn = repo.lock_conn();

    let journal: String = conn
        .pragma_query_value(None, "journal_mode", |r| r.get(0))
        .unwrap();
    assert_eq!(journal.to_lowercase(), "wal");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(user_version, CURRENT_SCHEMA_VERSION);

    let schema_version: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(schema_version, CURRENT_SCHEMA_VERSION.to_string());
}

#[test]
fn list_includes_deleted() {
    let repo = SqliteRepository::in_memory().unwrap();
    let mut p = sample("id-1", &tmp());
    p.is_deleted = true;
    repo.save(&p).unwrap();
    assert_eq!(repo.list().unwrap().len(), 1);
}

#[test]
fn find_by_directory_matches_normalized() {
    let repo = SqliteRepository::in_memory().unwrap();
    let dir = tmp();
    repo.save(&sample("id-1", &dir)).unwrap();
    let normalized = crate::domain::normalize::normalize_directory(&dir);
    assert_eq!(
        repo.find_by_directory(&normalized).unwrap().unwrap().id,
        "id-1"
    );
    assert!(repo.find_by_directory("/nope").unwrap().is_none());
}

#[test]
fn corrupt_blob_is_reported() {
    let repo = SqliteRepository::in_memory().unwrap();
    {
        let conn = repo.lock_conn();
        conn.execute(
            "INSERT INTO projects (id, data, is_deleted, directory_normalized, updated_at)
             VALUES ('bad', '{not json', 0, '/x', '2024-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    }
    assert!(matches!(repo.get("bad"), Err(RepositoryError::Corrupt(_))));
}

#[test]
fn fresh_db_is_at_current_schema_version() {
    let repo = SqliteRepository::in_memory().unwrap();
    let conn = repo.lock_conn();
    let v: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(v, CURRENT_SCHEMA_VERSION);
    let app: String = conn
        .query_row("SELECT value FROM meta WHERE key = 'app'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(app, "project-indexer");
}

#[test]
fn refuses_a_newer_database() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1)
        .unwrap();
    assert!(matches!(
        SqliteRepository::from_connection(conn),
        Err(RepositoryError::Backend(_))
    ));
}
