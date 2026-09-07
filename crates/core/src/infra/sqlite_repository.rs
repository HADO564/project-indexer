use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};

use crate::domain::normalize::normalize_directory;
use crate::domain::{Group, Project};
use crate::error::RepositoryError;
use crate::ports::{GroupReader, GroupRepository, ProjectReader, ProjectRepository};

/// The schema version this binary understands. `open` migrates up to this and
/// refuses any database already past it.
pub const CURRENT_SCHEMA_VERSION: i64 = 3;

pub struct SqliteRepository {
    conn: Mutex<Connection>,
}

impl SqliteRepository {
    pub fn open(path: &Path) -> Result<Self, RepositoryError> {
        let conn = Connection::open(path).map_err(be)?;
        Self::from_connection(conn)
    }

    pub fn in_memory() -> Result<Self, RepositoryError> {
        Self::from_connection(Connection::open_in_memory().map_err(be)?)
    }

    /// Shared setup: pragmas, version-skew guard, migrations.
    pub fn from_connection(conn: Connection) -> Result<Self, RepositoryError> {
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(be)?;
        conn.pragma_update(None, "busy_timeout", 5000).map_err(be)?;
        conn.pragma_update(None, "foreign_keys", "ON").map_err(be)?;

        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(be)?;
        if version > CURRENT_SCHEMA_VERSION {
            return Err(RepositoryError::Backend(
                "database is from a newer version of Project Indexer".into(),
            ));
        }
        run_migrations(&conn, version)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Takes the connection lock, recovering from poisoning.
    ///
    /// A panic while another thread held this lock leaves SQLite itself
    /// consistent — an in-flight transaction rolls back when its guard drops —
    /// so the connection is still usable. Propagating the poison instead would
    /// turn one unrelated panic into permanently broken persistence for the
    /// rest of the process, which is a far worse failure than the one that
    /// caused it.
    fn lock_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn run_migrations(conn: &Connection, from: i64) -> Result<(), RepositoryError> {
    if from < 1 {
        conn.execute_batch(
            "BEGIN;
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
             PRAGMA user_version = 1;
             COMMIT;",
        )
        .map_err(be)?;
    }

    if from < 2 {
        // Groups, and the project's membership in one.
        //
        // `group_id` is added to `projects` as a mirror of the same key inside
        // the JSON blob — the house pattern, as `directory_normalized` already
        // is. `ON DELETE SET NULL` is a safety net only: `delete_group` clears
        // blob and column together in one transaction, because the constraint
        // alone would fix the column and leave the blob stale.
        //
        // SQLite permits a REFERENCES clause on ADD COLUMN as long as the
        // default is NULL, which it is.
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE groups (
               id         TEXT PRIMARY KEY,
               name       TEXT NOT NULL,
               color      TEXT NOT NULL,
               icon       TEXT NOT NULL,
               position   INTEGER NOT NULL,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             CREATE UNIQUE INDEX idx_groups_name_nocase ON groups(name COLLATE NOCASE);
             CREATE INDEX idx_groups_position ON groups(position);
             ALTER TABLE projects ADD COLUMN group_id TEXT
               REFERENCES groups(id) ON DELETE SET NULL;
             CREATE INDEX idx_projects_group_id ON projects(group_id);
             PRAGMA user_version = 2;
             COMMIT;",
        )
        .map_err(be)?;
    }

    if from < 3 {
        // `client` was a hardcoded field that only ever suited one kind of
        // user; it is replaced by the open-ended `properties` map. Serde would
        // simply ignore the old key, which loses the value the moment the
        // record is next saved — so this rewrites each blob, moving a non-empty
        // `client` to `properties["client"]` and dropping the key.
        //
        // This is a blob rewrite rather than a column change, which is exactly
        // what the numbered `user_version` runner exists for: a shape change
        // serde cannot absorb on its own.
        let tx = conn.unchecked_transaction().map_err(be)?;
        {
            let mut select = tx.prepare("SELECT id, data FROM projects").map_err(be)?;
            let rows: Vec<(String, String)> = select
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(be)?
                .collect::<Result<_, _>>()
                .map_err(be)?;

            let mut update = tx
                .prepare("UPDATE projects SET data = ?1 WHERE id = ?2")
                .map_err(be)?;

            for (id, data) in rows {
                // A blob that does not parse is left exactly as it is. It is
                // already broken, and a migration is the wrong place to decide
                // what to do about that — `get` surfaces it as a corrupt record.
                let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&data) else {
                    continue;
                };
                let Some(object) = value.as_object_mut() else {
                    continue;
                };

                let client = object.remove("client");
                let client = client
                    .as_ref()
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .trim();
                if !client.is_empty() {
                    let entry = object
                        .entry("properties")
                        .or_insert_with(|| serde_json::Value::Object(Default::default()));
                    if let Some(map) = entry.as_object_mut() {
                        // An existing `properties.client` wins: it was set
                        // deliberately, where this one is a leftover field.
                        map.entry("client")
                            .or_insert_with(|| serde_json::Value::String(client.to_string()));
                    }
                }

                let rewritten = serde_json::to_string(&value).map_err(|e| {
                    RepositoryError::Backend(format!("re-serializing project {id}: {e}"))
                })?;
                update
                    .execute(rusqlite::params![rewritten, id])
                    .map_err(be)?;
            }
        }
        tx.execute_batch("PRAGMA user_version = 3;").map_err(be)?;
        tx.commit().map_err(be)?;
    }

    // Keep the `meta` mirror of the schema version in lockstep with
    // `user_version` regardless of which migration steps ran — a future
    // step that doesn't touch this row would otherwise leave it stale.
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [CURRENT_SCHEMA_VERSION.to_string()],
    )
    .map_err(be)?;
    Ok(())
}

impl ProjectReader for SqliteRepository {
    fn get(&self, id: &str) -> Result<Option<Project>, RepositoryError> {
        let conn = self.lock_conn();
        let data: Option<String> = conn
            .query_row("SELECT data FROM projects WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()
            .map_err(be)?;
        data.map(|d| parse(&d)).transpose()
    }

    fn list(&self) -> Result<Vec<Project>, RepositoryError> {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare("SELECT data FROM projects").map_err(be)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(be)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(parse(&row.map_err(be)?)?);
        }
        Ok(out)
    }

    fn find_by_directory(
        &self,
        normalized_directory: &str,
    ) -> Result<Option<Project>, RepositoryError> {
        let conn = self.lock_conn();
        let data: Option<String> = conn
            .query_row(
                "SELECT data FROM projects WHERE directory_normalized = ?1 \
                 ORDER BY is_deleted ASC, updated_at DESC LIMIT 1",
                [normalized_directory],
                |r| r.get(0),
            )
            .optional()
            .map_err(be)?;
        data.map(|d| parse(&d)).transpose()
    }
}

impl ProjectRepository for SqliteRepository {
    fn save(&self, project: &Project) -> Result<(), RepositoryError> {
        let data = serde_json::to_string(project)
            .map_err(|e| RepositoryError::Backend(format!("serialize: {e}")))?;
        let dir_norm = normalize_directory(&project.directory);

        let mut conn = self.lock_conn();
        let tx = conn.transaction().map_err(be)?;
        tx.execute(
            "INSERT INTO projects (id, data, is_deleted, directory_normalized, updated_at, group_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               data = excluded.data,
               is_deleted = excluded.is_deleted,
               directory_normalized = excluded.directory_normalized,
               updated_at = excluded.updated_at,
               group_id = excluded.group_id",
            rusqlite::params![
                project.id,
                data,
                project.is_deleted as i64,
                dir_norm,
                project.updated_at.to_rfc3339(),
                project.group_id,
            ],
        )
        .map_err(be)?;
        tx.execute(
            "DELETE FROM project_tags WHERE project_id = ?1",
            [&project.id],
        )
        .map_err(be)?;
        {
            let mut ins = tx
                .prepare("INSERT INTO project_tags (project_id, tag) VALUES (?1, ?2)")
                .map_err(be)?;
            for tag in &project.tags {
                ins.execute(rusqlite::params![project.id, tag])
                    .map_err(be)?;
            }
        }
        tx.commit().map_err(be)?;
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), RepositoryError> {
        let conn = self.lock_conn();
        conn.execute("DELETE FROM projects WHERE id = ?1", [id])
            .map_err(be)?;
        Ok(())
    }
}

impl GroupReader for SqliteRepository {
    fn get_group(&self, id: &str) -> Result<Option<Group>, RepositoryError> {
        let conn = self.lock_conn();
        conn.query_row(
            "SELECT id, name, color, icon, position, created_at, updated_at
             FROM groups WHERE id = ?1",
            [id],
            group_from_row,
        )
        .optional()
        .map_err(be)?
        .transpose()
    }

    fn list_groups(&self) -> Result<Vec<Group>, RepositoryError> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, color, icon, position, created_at, updated_at
                 FROM groups ORDER BY position ASC",
            )
            .map_err(be)?;
        let rows = stmt.query_map([], group_from_row).map_err(be)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(be)??);
        }
        Ok(out)
    }
}

impl GroupRepository for SqliteRepository {
    fn save_group(&self, group: &Group) -> Result<(), RepositoryError> {
        let conn = self.lock_conn();
        conn.execute(
            "INSERT INTO groups (id, name, color, icon, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               color = excluded.color,
               icon = excluded.icon,
               position = excluded.position,
               updated_at = excluded.updated_at",
            rusqlite::params![
                group.id,
                group.name,
                group.color,
                group.icon,
                group.position,
                group.created_at.to_rfc3339(),
                group.updated_at.to_rfc3339(),
            ],
        )
        .map_err(be)?;
        Ok(())
    }

    fn delete_group(&self, id: &str) -> Result<(), RepositoryError> {
        let mut conn = self.lock_conn();
        let tx = conn.transaction().map_err(be)?;

        // Read the members first: the blob is the authoritative copy of
        // `group_id`, so clearing the column alone would leave it stale.
        let members: Vec<(String, String)> = {
            let mut stmt = tx
                .prepare("SELECT id, data FROM projects WHERE group_id = ?1")
                .map_err(be)?;
            let rows = stmt
                .query_map([id], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(be)?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(be)?);
            }
            out
        };

        for (project_id, data) in members {
            let mut project = parse(&data)?;
            project.group_id = None;
            let updated = serde_json::to_string(&project)
                .map_err(|e| RepositoryError::Backend(format!("serialize: {e}")))?;
            tx.execute(
                "UPDATE projects SET data = ?2, group_id = NULL WHERE id = ?1",
                rusqlite::params![project_id, updated],
            )
            .map_err(be)?;
        }

        tx.execute("DELETE FROM groups WHERE id = ?1", [id])
            .map_err(be)?;
        tx.commit().map_err(be)?;
        Ok(())
    }

    fn set_group_positions(&self, ordered_ids: &[String]) -> Result<(), RepositoryError> {
        let mut conn = self.lock_conn();
        let tx = conn.transaction().map_err(be)?;
        for (position, id) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE groups SET position = ?2 WHERE id = ?1",
                rusqlite::params![id, position as i64],
            )
            .map_err(be)?;
        }
        tx.commit().map_err(be)?;
        Ok(())
    }
}

/// Row mapper shared by `get_group` and `list_groups`. The outer `Result` is
/// rusqlite's; the inner one carries a timestamp that failed to parse, which is
/// corruption rather than a backend fault.
fn group_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Group, RepositoryError>> {
    let created_at: String = row.get(5)?;
    let updated_at: String = row.get(6)?;
    Ok((|| {
        Ok(Group {
            id: row_string(row, 0)?,
            name: row_string(row, 1)?,
            color: row_string(row, 2)?,
            icon: row_string(row, 3)?,
            position: row
                .get(4)
                .map_err(|e| RepositoryError::Corrupt(e.to_string()))?,
            created_at: parse_timestamp(&created_at)?,
            updated_at: parse_timestamp(&updated_at)?,
        })
    })())
}

fn row_string(row: &rusqlite::Row<'_>, idx: usize) -> Result<String, RepositoryError> {
    row.get(idx)
        .map_err(|e| RepositoryError::Corrupt(e.to_string()))
}

fn parse_timestamp(raw: &str) -> Result<chrono::DateTime<chrono::Utc>, RepositoryError> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| RepositoryError::Corrupt(format!("timestamp {raw:?}: {e}")))
}

fn be(e: rusqlite::Error) -> RepositoryError {
    RepositoryError::Backend(e.to_string())
}

fn parse(data: &str) -> Result<Project, RepositoryError> {
    serde_json::from_str(data).map_err(|e| RepositoryError::Corrupt(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Group;
    use crate::ports::{GroupReader, GroupRepository};

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
            .conn
            .lock()
            .unwrap()
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
            .conn
            .lock()
            .unwrap()
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
        repo.conn
            .lock()
            .unwrap()
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
            let conn = repo.conn.lock().unwrap();
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
        let conn = repo.conn.lock().unwrap();

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
            let conn = repo.conn.lock().unwrap();
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
        let conn = repo.conn.lock().unwrap();
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
}
