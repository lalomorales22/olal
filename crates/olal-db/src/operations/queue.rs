//! Processing queue operations.

use crate::database::Database;
use crate::error::{DbError, DbResult};
use olal_core::{ItemType, QueueItem, QueueStatus};
use chrono::{DateTime, Utc};
use rusqlite::params;

impl Database {
    /// Add an item to the processing queue.
    pub fn enqueue(&self, item: &QueueItem) -> DbResult<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO queue (id, source_path, item_type, status, priority, attempts, error, created_at, started_at, completed_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                item.id,
                item.source_path,
                item.item_type.as_str(),
                item.status.as_str(),
                item.priority,
                item.attempts,
                item.error,
                item.created_at.to_rfc3339(),
                item.started_at.map(|dt| dt.to_rfc3339()),
                item.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Get a queue item by ID.
    pub fn get_queue_item(&self, id: &str) -> DbResult<QueueItem> {
        let conn = self.conn()?;
        let item = conn.query_row(
            "SELECT id, source_path, item_type, status, priority, attempts, error, created_at, started_at, completed_at
             FROM queue WHERE id = ?1",
            params![id],
            row_to_queue_item,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Queue item not found: {}", id)),
            _ => DbError::from(e),
        })?;

        Ok(item)
    }

    /// Dequeue the next item for processing (marks it as processing).
    pub fn dequeue(&self) -> DbResult<Option<QueueItem>> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        // Get the highest priority pending item
        let result = conn.query_row(
            "SELECT id, source_path, item_type, status, priority, attempts, error, created_at, started_at, completed_at
             FROM queue
             WHERE status = 'pending'
             ORDER BY priority DESC, created_at ASC
             LIMIT 1",
            [],
            row_to_queue_item,
        );

        let item = match result {
            Ok(item) => item,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(DbError::from(e)),
        };

        // Mark as processing
        conn.execute(
            "UPDATE queue SET status = 'processing', started_at = ?2, attempts = attempts + 1 WHERE id = ?1",
            params![item.id, now],
        )?;

        // Re-fetch the updated item using the same connection
        let updated = conn.query_row(
            "SELECT id, source_path, item_type, status, priority, attempts, error, created_at, started_at, completed_at
             FROM queue WHERE id = ?1",
            params![item.id],
            row_to_queue_item,
        )?;

        Ok(Some(updated))
    }

    /// Mark a queue item as completed.
    pub fn mark_completed(&self, id: &str) -> DbResult<()> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        let rows = conn.execute(
            "UPDATE queue SET status = 'done', completed_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Queue item not found: {}", id)));
        }

        Ok(())
    }

    /// Mark a queue item as failed.
    pub fn mark_failed(&self, id: &str, error: &str) -> DbResult<()> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        let rows = conn.execute(
            "UPDATE queue SET status = 'failed', error = ?2, completed_at = ?3 WHERE id = ?1",
            params![id, error, now],
        )?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Queue item not found: {}", id)));
        }

        Ok(())
    }

    /// Retry a failed queue item.
    pub fn retry(&self, id: &str) -> DbResult<()> {
        let conn = self.conn()?;

        let rows = conn.execute(
            "UPDATE queue SET status = 'pending', error = NULL, started_at = NULL, completed_at = NULL WHERE id = ?1",
            params![id],
        )?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Queue item not found: {}", id)));
        }

        Ok(())
    }

    /// List queue items by status.
    pub fn list_queue(&self, status: Option<QueueStatus>) -> DbResult<Vec<QueueItem>> {
        let conn = self.conn()?;

        let items = match status {
            Some(s) => {
                let mut stmt = conn.prepare(
                    "SELECT id, source_path, item_type, status, priority, attempts, error, created_at, started_at, completed_at
                     FROM queue WHERE status = ?1 ORDER BY priority DESC, created_at ASC",
                )?;
                let rows = stmt.query_map(params![s.as_str()], row_to_queue_item)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id, source_path, item_type, status, priority, attempts, error, created_at, started_at, completed_at
                     FROM queue ORDER BY priority DESC, created_at ASC",
                )?;
                let rows = stmt.query_map([], row_to_queue_item)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
        };

        Ok(items)
    }

    /// Check if a source path is already in the queue.
    pub fn is_queued(&self, source_path: &str) -> DbResult<bool> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM queue WHERE source_path = ?1 AND status IN ('pending', 'processing')",
            params![source_path],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Clear completed items from the queue.
    pub fn clear_completed(&self) -> DbResult<i64> {
        let conn = self.conn()?;
        let count = conn.execute("DELETE FROM queue WHERE status = 'done'", [])?;
        Ok(count as i64)
    }

    /// Clear failed items from the queue.
    pub fn clear_failed(&self) -> DbResult<i64> {
        let conn = self.conn()?;
        let count = conn.execute("DELETE FROM queue WHERE status = 'failed'", [])?;
        Ok(count as i64)
    }

    /// Get queue counts by status.
    pub fn queue_counts(&self) -> DbResult<(i64, i64, i64, i64)> {
        let conn = self.conn()?;

        let pending: i64 = conn.query_row(
            "SELECT COUNT(*) FROM queue WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )?;

        let processing: i64 = conn.query_row(
            "SELECT COUNT(*) FROM queue WHERE status = 'processing'",
            [],
            |row| row.get(0),
        )?;

        let done: i64 = conn.query_row(
            "SELECT COUNT(*) FROM queue WHERE status = 'done'",
            [],
            |row| row.get(0),
        )?;

        let failed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM queue WHERE status = 'failed'",
            [],
            |row| row.get(0),
        )?;

        Ok((pending, processing, done, failed))
    }
}

fn row_to_queue_item(row: &rusqlite::Row) -> rusqlite::Result<QueueItem> {
    let item_type_str: String = row.get(2)?;
    let status_str: String = row.get(3)?;
    let created_at_str: String = row.get(7)?;
    let started_at_str: Option<String> = row.get(8)?;
    let completed_at_str: Option<String> = row.get(9)?;

    Ok(QueueItem {
        id: row.get(0)?,
        source_path: row.get(1)?,
        item_type: ItemType::from_str(&item_type_str).unwrap_or(ItemType::Document),
        status: QueueStatus::from_str(&status_str).unwrap_or(QueueStatus::Pending),
        priority: row.get(4)?,
        attempts: row.get(5)?,
        error: row.get(6)?,
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        started_at: started_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        }),
        completed_at: completed_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_workflow() {
        let db = Database::open_in_memory().unwrap();

        // Enqueue
        let item = QueueItem::new("/path/to/video.mp4", ItemType::Video);
        db.enqueue(&item).unwrap();

        // Check queued
        assert!(db.is_queued("/path/to/video.mp4").unwrap());

        // Dequeue
        let dequeued = db.dequeue().unwrap();
        assert!(dequeued.is_some());
        let dequeued = dequeued.unwrap();
        assert_eq!(dequeued.status, QueueStatus::Processing);
        assert_eq!(dequeued.attempts, 1);

        // Mark completed
        db.mark_completed(&dequeued.id).unwrap();
        let completed = db.get_queue_item(&dequeued.id).unwrap();
        assert_eq!(completed.status, QueueStatus::Done);
        assert!(completed.completed_at.is_some());
    }

    #[test]
    fn test_queue_failure_and_retry() {
        let db = Database::open_in_memory().unwrap();

        let item = QueueItem::new("/path/to/video.mp4", ItemType::Video);
        db.enqueue(&item).unwrap();

        // Dequeue and fail
        let dequeued = db.dequeue().unwrap().unwrap();
        db.mark_failed(&dequeued.id, "Processing error").unwrap();

        let failed = db.get_queue_item(&dequeued.id).unwrap();
        assert_eq!(failed.status, QueueStatus::Failed);
        assert_eq!(failed.error, Some("Processing error".to_string()));

        // Retry
        db.retry(&dequeued.id).unwrap();
        let retried = db.get_queue_item(&dequeued.id).unwrap();
        assert_eq!(retried.status, QueueStatus::Pending);
        assert!(retried.error.is_none());
    }

    #[test]
    fn test_queue_counts() {
        let db = Database::open_in_memory().unwrap();

        db.enqueue(&QueueItem::new("/a.mp4", ItemType::Video)).unwrap();
        db.enqueue(&QueueItem::new("/b.mp4", ItemType::Video)).unwrap();

        let (pending, processing, done, failed) = db.queue_counts().unwrap();
        assert_eq!(pending, 2);
        assert_eq!(processing, 0);
        assert_eq!(done, 0);
        assert_eq!(failed, 0);
    }
}
