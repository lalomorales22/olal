//! Database connection and pool management.

use crate::error::{DbError, DbResult};
use crate::migrations;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;
use tracing::info;

/// Type alias for connection pool.
pub type ConnectionPool = Pool<SqliteConnectionManager>;
pub type PooledConn = PooledConnection<SqliteConnectionManager>;

/// Main database handle.
#[derive(Clone)]
pub struct Database {
    pool: ConnectionPool,
}

impl Database {
    /// Open a database at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> DbResult<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DbError::Other(e.to_string()))?;
        }

        info!("Opening database at: {}", path.display());

        let manager = SqliteConnectionManager::file(path)
            .with_init(|conn| {
                conn.execute_batch(
                    "PRAGMA journal_mode = WAL;
                     PRAGMA synchronous = NORMAL;
                     PRAGMA foreign_keys = ON;
                     PRAGMA cache_size = -64000;", // 64MB cache
                )?;
                Ok(())
            });

        let pool = Pool::builder()
            .max_size(10)
            .build(manager)?;

        // Initialize schema
        {
            let conn = pool.get()?;
            migrations::initialize_schema(&conn)?;
        }

        Ok(Self { pool })
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> DbResult<Self> {
        let manager = SqliteConnectionManager::memory()
            .with_init(|conn| {
                conn.execute_batch("PRAGMA foreign_keys = ON;")?;
                Ok(())
            });

        let pool = Pool::builder()
            .max_size(1) // Memory DB only supports single connection
            .build(manager)?;

        // Initialize schema
        {
            let conn = pool.get()?;
            migrations::initialize_schema(&conn)?;
        }

        Ok(Self { pool })
    }

    /// Get a connection from the pool.
    pub fn conn(&self) -> DbResult<PooledConn> {
        self.pool.get().map_err(DbError::from)
    }

    /// Get database file size in bytes.
    pub fn file_size<P: AsRef<Path>>(path: P) -> DbResult<i64> {
        let metadata = std::fs::metadata(path).map_err(|e| DbError::Other(e.to_string()))?;
        Ok(metadata.len() as i64)
    }

    /// Vacuum the database to reclaim space.
    pub fn vacuum(&self) -> DbResult<()> {
        let conn = self.conn()?;
        conn.execute("VACUUM", [])?;
        Ok(())
    }

    /// Run integrity check on the database.
    pub fn integrity_check(&self) -> DbResult<bool> {
        let conn = self.conn()?;
        let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        Ok(result == "ok")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = Database::open_in_memory();
        assert!(db.is_ok());
    }

    #[test]
    fn test_integrity_check() {
        let db = Database::open_in_memory().unwrap();
        assert!(db.integrity_check().unwrap());
    }
}
