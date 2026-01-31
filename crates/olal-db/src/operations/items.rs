//! Item CRUD operations.

use crate::database::Database;
use crate::error::{DbError, DbResult};
use olal_core::{Item, ItemType};
use chrono::{DateTime, Utc};
use rusqlite::params;

impl Database {
    /// Create a new item.
    pub fn create_item(&self, item: &Item) -> DbResult<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO items (id, item_type, title, source_path, content_hash, summary, created_at, processed_at, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                item.id,
                item.item_type.as_str(),
                item.title,
                item.source_path,
                item.content_hash,
                item.summary,
                item.created_at.to_rfc3339(),
                item.processed_at.map(|dt| dt.to_rfc3339()),
                item.metadata.to_string(),
            ],
        )?;
        Ok(())
    }

    /// Get an item by ID.
    pub fn get_item(&self, id: &str) -> DbResult<Item> {
        let conn = self.conn()?;
        let item = conn.query_row(
            "SELECT id, item_type, title, source_path, content_hash, summary, created_at, processed_at, metadata FROM items WHERE id = ?1",
            params![id],
            |row| {
                let item_type_str: String = row.get(1)?;
                let created_at_str: String = row.get(6)?;
                let processed_at_str: Option<String> = row.get(7)?;
                let metadata_str: String = row.get(8)?;

                Ok(Item {
                    id: row.get(0)?,
                    item_type: ItemType::from_str(&item_type_str).unwrap_or(ItemType::Document),
                    title: row.get(2)?,
                    source_path: row.get(3)?,
                    content_hash: row.get(4)?,
                    summary: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    processed_at: processed_at_str.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .ok()
                    }),
                    metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Item not found: {}", id)),
            _ => DbError::from(e),
        })?;

        Ok(item)
    }

    /// Update an item.
    pub fn update_item(&self, item: &Item) -> DbResult<()> {
        let conn = self.conn()?;
        let rows = conn.execute(
            r#"
            UPDATE items
            SET title = ?2, source_path = ?3, content_hash = ?4, summary = ?5,
                processed_at = ?6, metadata = ?7
            WHERE id = ?1
            "#,
            params![
                item.id,
                item.title,
                item.source_path,
                item.content_hash,
                item.summary,
                item.processed_at.map(|dt| dt.to_rfc3339()),
                item.metadata.to_string(),
            ],
        )?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Item not found: {}", item.id)));
        }

        Ok(())
    }

    /// Delete an item by ID.
    pub fn delete_item(&self, id: &str) -> DbResult<()> {
        let conn = self.conn()?;
        let rows = conn.execute("DELETE FROM items WHERE id = ?1", params![id])?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Item not found: {}", id)));
        }

        Ok(())
    }

    /// List items with optional filtering.
    pub fn list_items(&self, item_type: Option<ItemType>, limit: Option<i64>) -> DbResult<Vec<Item>> {
        let conn = self.conn()?;
        let limit = limit.unwrap_or(100);

        let sql = match item_type {
            Some(_) => {
                "SELECT id, item_type, title, source_path, content_hash, summary, created_at, processed_at, metadata
                 FROM items WHERE item_type = ?1 ORDER BY created_at DESC LIMIT ?2"
            }
            None => {
                "SELECT id, item_type, title, source_path, content_hash, summary, created_at, processed_at, metadata
                 FROM items ORDER BY created_at DESC LIMIT ?1"
            }
        };

        let mut stmt = conn.prepare(sql)?;

        let items = if let Some(ref it) = item_type {
            stmt.query_map(params![it.as_str(), limit], row_to_item)?
        } else {
            stmt.query_map(params![limit], row_to_item)?
        };

        items.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Find item by source path.
    pub fn find_item_by_path(&self, path: &str) -> DbResult<Option<Item>> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, item_type, title, source_path, content_hash, summary, created_at, processed_at, metadata
             FROM items WHERE source_path = ?1",
            params![path],
            row_to_item,
        );

        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Find item by content hash.
    pub fn find_item_by_hash(&self, hash: &str) -> DbResult<Option<Item>> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, item_type, title, source_path, content_hash, summary, created_at, processed_at, metadata
             FROM items WHERE content_hash = ?1",
            params![hash],
            row_to_item,
        );

        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Full-text search on items via chunks.
    pub fn search_items(&self, query: &str, limit: Option<i64>) -> DbResult<Vec<Item>> {
        let conn = self.conn()?;
        let limit = limit.unwrap_or(20);

        let mut stmt = conn.prepare(
            r#"
            SELECT DISTINCT i.id, i.item_type, i.title, i.source_path, i.content_hash,
                   i.summary, i.created_at, i.processed_at, i.metadata
            FROM items i
            INNER JOIN chunks c ON c.item_id = i.id
            INNER JOIN chunks_fts fts ON fts.rowid = c.rowid
            WHERE chunks_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
            "#,
        )?;

        let items = stmt.query_map(params![query, limit], row_to_item)?;
        items.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get recent items.
    pub fn recent_items(&self, limit: Option<i64>) -> DbResult<Vec<Item>> {
        self.list_items(None, limit)
    }

    /// Get items created since a specific date.
    pub fn items_since(&self, since: DateTime<Utc>) -> DbResult<Vec<Item>> {
        let conn = self.conn()?;
        let since_str = since.to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id, item_type, title, source_path, content_hash, summary,
                    created_at, processed_at, metadata
             FROM items WHERE created_at >= ?1 ORDER BY created_at DESC",
        )?;
        let items = stmt.query_map(params![since_str], row_to_item)?;
        items.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get items created between two dates.
    pub fn items_between(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> DbResult<Vec<Item>> {
        let conn = self.conn()?;
        let start_str = start.to_rfc3339();
        let end_str = end.to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id, item_type, title, source_path, content_hash, summary,
                    created_at, processed_at, metadata
             FROM items WHERE created_at >= ?1 AND created_at <= ?2 ORDER BY created_at DESC",
        )?;
        let items = stmt.query_map(params![start_str, end_str], row_to_item)?;
        items.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get an item by ID prefix.
    ///
    /// Useful for CLI where users can type partial IDs.
    pub fn get_item_by_prefix(&self, prefix: &str) -> DbResult<Item> {
        let conn = self.conn()?;

        // First try exact match
        if let Ok(item) = self.get_item(prefix) {
            return Ok(item);
        }

        // Then try prefix match
        let pattern = format!("{}%", prefix);
        let mut stmt = conn.prepare(
            "SELECT id, item_type, title, source_path, content_hash, summary, created_at, processed_at, metadata
             FROM items WHERE id LIKE ?1 LIMIT 2",
        )?;

        let items: Vec<Item> = stmt
            .query_map(params![pattern], row_to_item)?
            .collect::<Result<Vec<_>, _>>()?;

        match items.len() {
            0 => Err(DbError::NotFound(format!("Item not found: {}", prefix))),
            1 => Ok(items.into_iter().next().unwrap()),
            _ => Err(DbError::Other(format!(
                "Ambiguous ID prefix '{}': multiple items match",
                prefix
            ))),
        }
    }
}

fn row_to_item(row: &rusqlite::Row) -> rusqlite::Result<Item> {
    let item_type_str: String = row.get(1)?;
    let created_at_str: String = row.get(6)?;
    let processed_at_str: Option<String> = row.get(7)?;
    let metadata_str: String = row.get(8)?;

    Ok(Item {
        id: row.get(0)?,
        item_type: ItemType::from_str(&item_type_str).unwrap_or(ItemType::Document),
        title: row.get(2)?,
        source_path: row.get(3)?,
        content_hash: row.get(4)?,
        summary: row.get(5)?,
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        processed_at: processed_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        }),
        metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_crud() {
        let db = Database::open_in_memory().unwrap();

        // Create
        let item = Item::new(ItemType::Video, "Test Video")
            .with_source_path("/path/to/video.mp4");
        db.create_item(&item).unwrap();

        // Read
        let fetched = db.get_item(&item.id).unwrap();
        assert_eq!(fetched.title, "Test Video");
        assert_eq!(fetched.item_type, ItemType::Video);

        // Update
        let mut updated = fetched;
        updated.title = "Updated Title".to_string();
        db.update_item(&updated).unwrap();

        let fetched = db.get_item(&item.id).unwrap();
        assert_eq!(fetched.title, "Updated Title");

        // Delete
        db.delete_item(&item.id).unwrap();
        assert!(db.get_item(&item.id).is_err());
    }

    #[test]
    fn test_find_by_path() {
        let db = Database::open_in_memory().unwrap();

        let item = Item::new(ItemType::Note, "Test Note")
            .with_source_path("/path/to/note.md");
        db.create_item(&item).unwrap();

        let found = db.find_item_by_path("/path/to/note.md").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Test Note");

        let not_found = db.find_item_by_path("/nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_items_since() {
        use chrono::Duration;

        let db = Database::open_in_memory().unwrap();

        // Create items
        let item1 = Item::new(ItemType::Note, "Note 1");
        let item2 = Item::new(ItemType::Video, "Video 1");
        db.create_item(&item1).unwrap();
        db.create_item(&item2).unwrap();

        // Query items since an hour ago (should find both)
        let since = Utc::now() - Duration::hours(1);
        let items = db.items_since(since).unwrap();
        assert_eq!(items.len(), 2);

        // Query items since an hour from now (should find none)
        let since = Utc::now() + Duration::hours(1);
        let items = db.items_since(since).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_items_between() {
        use chrono::Duration;

        let db = Database::open_in_memory().unwrap();

        // Create items
        let item1 = Item::new(ItemType::Note, "Note 1");
        let item2 = Item::new(ItemType::Video, "Video 1");
        db.create_item(&item1).unwrap();
        db.create_item(&item2).unwrap();

        // Query items between an hour ago and an hour from now (should find both)
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now() + Duration::hours(1);
        let items = db.items_between(start, end).unwrap();
        assert_eq!(items.len(), 2);

        // Query items in the past (should find none)
        let start = Utc::now() - Duration::hours(2);
        let end = Utc::now() - Duration::hours(1);
        let items = db.items_between(start, end).unwrap();
        assert!(items.is_empty());
    }
}
