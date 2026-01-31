//! Database migrations and schema management.

use crate::error::DbResult;
use rusqlite::Connection;
use tracing::info;

/// Current schema version.
pub const SCHEMA_VERSION: i32 = 1;

/// Initialize the database schema.
pub fn initialize_schema(conn: &Connection) -> DbResult<()> {
    let current_version = get_schema_version(conn)?;

    if current_version == 0 {
        info!("Creating initial database schema...");
        create_initial_schema(conn)?;
        set_schema_version(conn, SCHEMA_VERSION)?;
    } else if current_version < SCHEMA_VERSION {
        info!(
            "Migrating database from version {} to {}",
            current_version, SCHEMA_VERSION
        );
        run_migrations(conn, current_version)?;
    }

    Ok(())
}

fn get_schema_version(conn: &Connection) -> DbResult<i32> {
    // Check if user_version is set
    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    Ok(version)
}

fn set_schema_version(conn: &Connection, version: i32) -> DbResult<()> {
    conn.pragma_update(None, "user_version", version)?;
    Ok(())
}

fn create_initial_schema(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- Core content storage
        CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            item_type TEXT NOT NULL,
            title TEXT NOT NULL,
            source_path TEXT,
            content_hash TEXT,
            summary TEXT,
            created_at TEXT NOT NULL,
            processed_at TEXT,
            metadata TEXT DEFAULT '{}'
        );

        CREATE INDEX IF NOT EXISTS idx_items_type ON items(item_type);
        CREATE INDEX IF NOT EXISTS idx_items_created ON items(created_at);
        CREATE INDEX IF NOT EXISTS idx_items_source ON items(source_path);
        CREATE INDEX IF NOT EXISTS idx_items_hash ON items(content_hash);

        -- Chunked text for RAG
        CREATE TABLE IF NOT EXISTS chunks (
            id TEXT PRIMARY KEY,
            item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
            chunk_index INTEGER NOT NULL,
            content TEXT NOT NULL,
            start_time REAL,
            end_time REAL
        );

        CREATE INDEX IF NOT EXISTS idx_chunks_item ON chunks(item_id);

        -- Vector embeddings (stored as BLOB)
        CREATE TABLE IF NOT EXISTS embeddings (
            chunk_id TEXT PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
            vector BLOB NOT NULL,
            model TEXT NOT NULL,
            dimensions INTEGER NOT NULL
        );

        -- Full-text search on chunks
        CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
            content,
            content='chunks',
            content_rowid='rowid'
        );

        -- Triggers to keep FTS in sync
        CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
            INSERT INTO chunks_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
        END;

        CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
            INSERT INTO chunks_fts(chunks_fts, rowid, content) VALUES('delete', OLD.rowid, OLD.content);
        END;

        CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
            INSERT INTO chunks_fts(chunks_fts, rowid, content) VALUES('delete', OLD.rowid, OLD.content);
            INSERT INTO chunks_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
        END;

        -- Task management
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT DEFAULT 'pending',
            priority INTEGER DEFAULT 0,
            project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
            due_date TEXT,
            created_at TEXT NOT NULL,
            completed_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
        CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);

        -- Projects for organization
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            status TEXT DEFAULT 'active',
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);

        -- Tagging system
        CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            color TEXT
        );

        CREATE TABLE IF NOT EXISTS item_tags (
            item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
            tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (item_id, tag_id)
        );

        CREATE INDEX IF NOT EXISTS idx_item_tags_item ON item_tags(item_id);
        CREATE INDEX IF NOT EXISTS idx_item_tags_tag ON item_tags(tag_id);

        -- Knowledge graph links
        CREATE TABLE IF NOT EXISTS links (
            source_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
            target_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
            link_type TEXT NOT NULL,
            strength REAL DEFAULT 1.0,
            PRIMARY KEY (source_id, target_id)
        );

        CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_id);
        CREATE INDEX IF NOT EXISTS idx_links_target ON links(target_id);

        -- Processing queue
        CREATE TABLE IF NOT EXISTS queue (
            id TEXT PRIMARY KEY,
            source_path TEXT NOT NULL,
            item_type TEXT NOT NULL,
            status TEXT DEFAULT 'pending',
            priority INTEGER DEFAULT 0,
            attempts INTEGER DEFAULT 0,
            error TEXT,
            created_at TEXT NOT NULL,
            started_at TEXT,
            completed_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_queue_status ON queue(status);
        CREATE INDEX IF NOT EXISTS idx_queue_priority ON queue(priority DESC);

        -- Enable foreign keys
        PRAGMA foreign_keys = ON;
        "#,
    )?;

    Ok(())
}

fn run_migrations(conn: &Connection, from_version: i32) -> DbResult<()> {
    // Future migrations go here
    // Example:
    // if from_version < 2 {
    //     migrate_v1_to_v2(conn)?;
    // }
    // if from_version < 3 {
    //     migrate_v2_to_v3(conn)?;
    // }

    let _ = (conn, from_version); // Silence unused warnings

    set_schema_version(conn, SCHEMA_VERSION)?;
    Ok(())
}

/// Drop all tables (for testing).
#[cfg(test)]
pub fn drop_all_tables(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        DROP TABLE IF EXISTS item_tags;
        DROP TABLE IF EXISTS links;
        DROP TABLE IF EXISTS embeddings;
        DROP TABLE IF EXISTS chunks_fts;
        DROP TABLE IF EXISTS chunks;
        DROP TABLE IF EXISTS queue;
        DROP TABLE IF EXISTS tasks;
        DROP TABLE IF EXISTS projects;
        DROP TABLE IF EXISTS tags;
        DROP TABLE IF EXISTS items;
        "#,
    )?;
    set_schema_version(conn, 0)?;
    Ok(())
}
