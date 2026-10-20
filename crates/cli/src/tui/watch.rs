//! Notices when the database has changed, so the browser can re-read it without
//! a restart — a project added in the GUI, a `dexily` run in another shell, the
//! observer tracking a fresh clone.
//!
//! SQLite keeps a counter per connection: `PRAGMA data_version` returns a number
//! that changes whenever *another* connection has committed since this one last
//! asked. So the TUI holds one small read-only connection of its own and asks
//! it on each idle tick. Every writer counts as "another" — including the TUI's
//! own commands, which write through `Context`'s connection, not this one.
//! Asking reads the WAL index header and touches no table, so a poll is cheaper
//! than watching the file, and exact where file timestamps are not.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

/// A read-only view of the database's commit counter.
pub(crate) struct Watch {
    conn: Connection,
    /// The `data_version` seen at the last check.
    seen: i64,
}

impl Watch {
    /// Opens the watching connection on the database at `path`, which must
    /// already exist (`Context::open` creates it). Read-only, so it can never
    /// write, migrate or take a write lock.
    pub(crate) fn open(path: &Path) -> rusqlite::Result<Self> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path, flags)?;
        let seen = data_version(&conn)?;
        Ok(Self { conn, seen })
    }

    /// Whether anything has been committed since the last call (or since
    /// `open`). Each call takes the current state as seen, so a change is
    /// reported once.
    pub(crate) fn changed(&mut self) -> rusqlite::Result<bool> {
        let now = data_version(&self.conn)?;
        let changed = now != self.seen;
        self.seen = now;
        Ok(changed)
    }
}

fn data_version(conn: &Connection) -> rusqlite::Result<i64> {
    conn.pragma_query_value(None, "data_version", |row| row.get(0))
}
