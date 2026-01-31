//! Database statistics operations.

use crate::database::Database;
use crate::error::DbResult;
use olal_core::DatabaseStats;
use std::collections::HashMap;

impl Database {
    /// Get comprehensive database statistics.
    pub fn get_stats(&self) -> DbResult<DatabaseStats> {
        let conn = self.conn()?;

        // Total items
        let total_items: i64 = conn.query_row(
            "SELECT COUNT(*) FROM items",
            [],
            |row| row.get(0),
        )?;

        // Items by type
        let mut items_by_type = HashMap::new();
        {
            let mut stmt = conn.prepare(
                "SELECT item_type, COUNT(*) FROM items GROUP BY item_type",
            )?;
            let rows = stmt.query_map([], |row| {
                let item_type: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((item_type, count))
            })?;
            for row in rows {
                let (item_type, count) = row?;
                items_by_type.insert(item_type, count);
            }
        }

        // Total chunks
        let total_chunks: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks",
            [],
            |row| row.get(0),
        )?;

        // Total tasks
        let total_tasks: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks",
            [],
            |row| row.get(0),
        )?;

        // Pending tasks
        let pending_tasks: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )?;

        // Total projects
        let total_projects: i64 = conn.query_row(
            "SELECT COUNT(*) FROM projects",
            [],
            |row| row.get(0),
        )?;

        // Total tags
        let total_tags: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tags",
            [],
            |row| row.get(0),
        )?;

        // Queue stats (inline to use same connection)
        let queue_pending: i64 = conn.query_row(
            "SELECT COUNT(*) FROM queue WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )?;

        let queue_processing: i64 = conn.query_row(
            "SELECT COUNT(*) FROM queue WHERE status = 'processing'",
            [],
            |row| row.get(0),
        )?;

        let queue_failed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM queue WHERE status = 'failed'",
            [],
            |row| row.get(0),
        )?;

        // Database size (page_count * page_size)
        let page_count: i64 = conn.pragma_query_value(None, "page_count", |row| row.get(0))?;
        let page_size: i64 = conn.pragma_query_value(None, "page_size", |row| row.get(0))?;
        let database_size_bytes = page_count * page_size;

        Ok(DatabaseStats {
            total_items,
            items_by_type,
            total_chunks,
            total_tasks,
            pending_tasks,
            total_projects,
            total_tags,
            queue_pending,
            queue_processing,
            queue_failed,
            database_size_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use olal_core::{Item, ItemType, Task};

    #[test]
    fn test_get_stats() {
        let db = Database::open_in_memory().unwrap();

        // Add some data
        let item1 = Item::new(ItemType::Video, "Video 1");
        let item2 = Item::new(ItemType::Video, "Video 2");
        let item3 = Item::new(ItemType::Note, "Note 1");
        db.create_item(&item1).unwrap();
        db.create_item(&item2).unwrap();
        db.create_item(&item3).unwrap();

        let task = Task::new("Test task");
        db.create_task(&task).unwrap();

        // Get stats
        let stats = db.get_stats().unwrap();

        assert_eq!(stats.total_items, 3);
        assert_eq!(stats.items_by_type.get("video"), Some(&2));
        assert_eq!(stats.items_by_type.get("note"), Some(&1));
        assert_eq!(stats.total_tasks, 1);
        assert_eq!(stats.pending_tasks, 1);
        assert!(stats.database_size_bytes > 0);
    }
}
