//! The numbered `user_version` migration runner.
//!
//! Separated from the repository because a migration is a one-time schema
//! change, not a query: this file grows by one `if from < N` block per schema
//! version and nothing else, and the reader/writer files never change with it.

use rusqlite::Connection;

use crate::error::RepositoryError;

use super::{be, CURRENT_SCHEMA_VERSION};

pub(super) fn run_migrations(conn: &Connection, from: i64) -> Result<(), RepositoryError> {
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
