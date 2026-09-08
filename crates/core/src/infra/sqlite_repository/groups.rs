//! `GroupReader` / `GroupRepository` — persistence for groups.
//!
//! Groups are a real table rather than a JSON blob (see `architecture.md`),
//! which is why this file reads as SQL where `projects.rs` reads as blob
//! round-tripping.

use rusqlite::OptionalExtension;

use crate::domain::Group;
use crate::error::RepositoryError;
use crate::ports::{GroupReader, GroupRepository};

use super::{be, group_from_row, parse, SqliteRepository};

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
