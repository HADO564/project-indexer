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
    /// `pub(crate)` so the test module can assert against raw SQL without the
    /// `conn` field itself being reachable — and so those assertions go through
    /// the same poison-tolerant path the repository uses, rather than their own
    /// `.lock().unwrap()`.
    pub(crate) fn lock_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
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
