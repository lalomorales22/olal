//! Chunk CRUD operations.

use crate::database::Database;
use crate::error::{DbError, DbResult};
use olal_core::{Chunk, ChunkId, ItemId};
use rusqlite::params;

impl Database {
    /// Create a new chunk.
    pub fn create_chunk(&self, chunk: &Chunk) -> DbResult<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO chunks (id, item_id, chunk_index, content, start_time, end_time)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                chunk.id,
                chunk.item_id,
                chunk.chunk_index,
                chunk.content,
                chunk.start_time,
                chunk.end_time,
            ],
        )?;
        Ok(())
    }

    /// Create multiple chunks in a transaction.
    pub fn create_chunks(&self, chunks: &[Chunk]) -> DbResult<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;

        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO chunks (id, item_id, chunk_index, content, start_time, end_time)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
            )?;

            for chunk in chunks {
                stmt.execute(params![
                    chunk.id,
                    chunk.item_id,
                    chunk.chunk_index,
                    chunk.content,
                    chunk.start_time,
                    chunk.end_time,
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Get a chunk by ID.
    pub fn get_chunk(&self, id: &ChunkId) -> DbResult<Chunk> {
        let conn = self.conn()?;
        let chunk = conn.query_row(
            "SELECT id, item_id, chunk_index, content, start_time, end_time FROM chunks WHERE id = ?1",
            params![id],
            |row| {
                Ok(Chunk {
                    id: row.get(0)?,
                    item_id: row.get(1)?,
                    chunk_index: row.get(2)?,
                    content: row.get(3)?,
                    start_time: row.get(4)?,
                    end_time: row.get(5)?,
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Chunk not found: {}", id)),
            _ => DbError::from(e),
        })?;

        Ok(chunk)
    }

    /// Get all chunks for an item.
    pub fn get_chunks_by_item(&self, item_id: &ItemId) -> DbResult<Vec<Chunk>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, item_id, chunk_index, content, start_time, end_time
             FROM chunks WHERE item_id = ?1 ORDER BY chunk_index",
        )?;

        let chunks = stmt.query_map(params![item_id], |row| {
            Ok(Chunk {
                id: row.get(0)?,
                item_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                start_time: row.get(4)?,
                end_time: row.get(5)?,
            })
        })?;

        chunks.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Delete all chunks for an item.
    pub fn delete_chunks_by_item(&self, item_id: &ItemId) -> DbResult<i64> {
        let conn = self.conn()?;
        let count = conn.execute("DELETE FROM chunks WHERE item_id = ?1", params![item_id])?;
        Ok(count as i64)
    }

    /// Store embedding for a chunk.
    pub fn store_embedding(&self, chunk_id: &ChunkId, vector: &[f32], model: &str) -> DbResult<()> {
        let conn = self.conn()?;

        // Serialize vector to bytes
        let vector_bytes: Vec<u8> = vector
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO embeddings (chunk_id, vector, model, dimensions)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![chunk_id, vector_bytes, model, vector.len() as i32],
        )?;

        Ok(())
    }

    /// Get embedding for a chunk.
    pub fn get_embedding(&self, chunk_id: &ChunkId) -> DbResult<Option<Vec<f32>>> {
        let conn = self.conn()?;

        let result = conn.query_row(
            "SELECT vector, dimensions FROM embeddings WHERE chunk_id = ?1",
            params![chunk_id],
            |row| {
                let bytes: Vec<u8> = row.get(0)?;
                let dimensions: i32 = row.get(1)?;
                Ok((bytes, dimensions))
            },
        );

        match result {
            Ok((bytes, dimensions)) => {
                let vector: Vec<f32> = bytes
                    .chunks(4)
                    .take(dimensions as usize)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok(Some(vector))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Get chunks with embeddings for an item.
    pub fn get_chunks_with_embeddings(&self, item_id: &ItemId) -> DbResult<Vec<(Chunk, Option<Vec<f32>>)>> {
        let chunks = self.get_chunks_by_item(item_id)?;
        let mut results = Vec::with_capacity(chunks.len());

        for chunk in chunks {
            let embedding = self.get_embedding(&chunk.id)?;
            results.push((chunk, embedding));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use olal_core::{Item, ItemType};

    #[test]
    fn test_chunk_crud() {
        let db = Database::open_in_memory().unwrap();

        // Create item first
        let item = Item::new(ItemType::Note, "Test Note");
        db.create_item(&item).unwrap();

        // Create chunks
        let chunk1 = Chunk::new(item.id.clone(), 0, "First chunk content");
        let chunk2 = Chunk::new(item.id.clone(), 1, "Second chunk content");

        db.create_chunk(&chunk1).unwrap();
        db.create_chunk(&chunk2).unwrap();

        // Get chunks
        let chunks = db.get_chunks_by_item(&item.id).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].content, "First chunk content");
        assert_eq!(chunks[1].content, "Second chunk content");

        // Delete chunks
        let deleted = db.delete_chunks_by_item(&item.id).unwrap();
        assert_eq!(deleted, 2);

        let chunks = db.get_chunks_by_item(&item.id).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_embeddings() {
        let db = Database::open_in_memory().unwrap();

        let item = Item::new(ItemType::Note, "Test Note");
        db.create_item(&item).unwrap();

        let chunk = Chunk::new(item.id.clone(), 0, "Test content");
        db.create_chunk(&chunk).unwrap();

        // Store embedding
        let vector = vec![0.1, 0.2, 0.3, 0.4];
        db.store_embedding(&chunk.id, &vector, "test-model").unwrap();

        // Retrieve embedding
        let retrieved = db.get_embedding(&chunk.id).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.len(), 4);
        assert!((retrieved[0] - 0.1).abs() < 0.0001);
    }
}
