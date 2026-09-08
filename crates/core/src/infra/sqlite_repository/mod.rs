use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::domain::{Group, Project};
use crate::error::RepositoryError;

/// The schema version this binary understands. `open` migrates up to this and
/// refuses any database already past it.
pub const CURRENT_SCHEMA_VERSION: i64 = 3;

pub struct SqliteRepository {
    conn: Mutex<Connection>,
}

// Split by what the code does to the database rather than by type: the
// migration runner, the project persistence and the group persistence change
// independently of each other. The struct, its `open`/connection handling and
// the row helpers all three share stay here.
mod groups;
mod migrations;
mod projects;

use migrations::run_migrations;

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
