use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};

use crate::domain::normalize::normalize_directory;
use crate::domain::Project;
use crate::error::RepositoryError;
use crate::ports::{ProjectReader, ProjectRepository};

/// The schema version this binary understands. `open` migrates up to this and
/// refuses any database already past it.
pub const CURRENT_SCHEMA_VERSION: i64 = 1;

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
             COMMIT;",
        )
        .map_err(be)?;
        conn.pragma_update(None, "user_version", 1).map_err(be)?;
    }
    Ok(())
}

impl ProjectReader for SqliteRepository {
    fn get(&self, id: &str) -> Result<Option<Project>, RepositoryError> {
        let conn = self.conn.lock().unwrap();
        let data: Option<String> = conn
            .query_row("SELECT data FROM projects WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()
            .map_err(be)?;
        data.map(|d| parse(&d)).transpose()
    }

    fn list(&self) -> Result<Vec<Project>, RepositoryError> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let data: Option<String> = conn
            .query_row(
                "SELECT data FROM projects WHERE directory_normalized = ?1 LIMIT 1",
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

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(be)?;
        tx.execute(
            "INSERT INTO projects (id, data, is_deleted, directory_normalized, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
               data = excluded.data,
               is_deleted = excluded.is_deleted,
               directory_normalized = excluded.directory_normalized,
               updated_at = excluded.updated_at",
            rusqlite::params![
                project.id,
                data,
                project.is_deleted as i64,
                dir_norm,
                project.updated_at.to_rfc3339(),
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = ?1", [id])
            .map_err(be)?;
        Ok(())
    }
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
    use crate::domain::Project;

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

    #[test]
    fn delete_is_idempotent_and_cascades_tags() {
        let repo = SqliteRepository::in_memory().unwrap();
        repo.save(&sample("id-1", &tmp())).unwrap();
        repo.delete("id-1").unwrap();
        repo.delete("id-1").unwrap(); // no error
        assert!(repo.get("id-1").unwrap().is_none());
        let conn = repo.conn.lock().unwrap();
        let tag_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tag_count, 0);
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
