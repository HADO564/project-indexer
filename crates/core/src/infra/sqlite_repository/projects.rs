//! `ProjectReader` / `ProjectRepository` — persistence for projects.

use rusqlite::OptionalExtension;

use crate::domain::normalize::normalize_directory;
use crate::domain::Project;
use crate::error::RepositoryError;
use crate::ports::{ProjectReader, ProjectRepository};

use super::{be, parse, SqliteRepository};

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

    fn list_ids(&self) -> Result<Vec<String>, RepositoryError> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT id FROM projects WHERE is_deleted = 0")
            .map_err(be)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(be)?;
        rows.collect::<Result<Vec<String>, _>>().map_err(be)
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
