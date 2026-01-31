//! Vector search operations for semantic search.

use crate::database::Database;
use crate::error::DbResult;
use olal_core::Chunk;
use rusqlite::params;

/// Result of a similarity search.
#[derive(Debug, Clone)]
pub struct SimilarityResult {
    /// The matching chunk.
    pub chunk: Chunk,
    /// Cosine similarity score (0.0 to 1.0).
    pub similarity: f32,
    /// ID of the parent item.
    pub item_id: String,
    /// Title of the parent item.
    pub item_title: String,
}

/// Calculate cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot_product = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        return 0.0;
    }

    dot_product / denominator
}

impl Database {
    /// Find similar chunks using cosine similarity.
    ///
    /// This performs a brute-force search over all embeddings, which is
    /// efficient for personal knowledge bases (<100K chunks).
    pub fn vector_search(
        &self,
        query_vector: &[f32],
        limit: usize,
        min_similarity: Option<f32>,
    ) -> DbResult<Vec<SimilarityResult>> {
        let conn = self.conn()?;
        let min_sim = min_similarity.unwrap_or(0.0);

        // Get all embeddings with their chunk and item info
        let mut stmt = conn.prepare(
            r#"
            SELECT
                c.id, c.item_id, c.chunk_index, c.content, c.start_time, c.end_time,
                e.vector, e.dimensions,
                i.title
            FROM embeddings e
            JOIN chunks c ON c.id = e.chunk_id
            JOIN items i ON i.id = c.item_id
            "#,
        )?;

        let mut results: Vec<SimilarityResult> = Vec::new();

        let rows = stmt.query_map([], |row| {
            let chunk = Chunk {
                id: row.get(0)?,
                item_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                start_time: row.get(4)?,
                end_time: row.get(5)?,
            };

            let vector_bytes: Vec<u8> = row.get(6)?;
            let dimensions: i32 = row.get(7)?;
            let item_title: String = row.get(8)?;

            Ok((chunk, vector_bytes, dimensions, item_title))
        })?;

        for row_result in rows {
            let (chunk, vector_bytes, dimensions, item_title) = row_result?;

            // Deserialize the vector
            let vector: Vec<f32> = vector_bytes
                .chunks(4)
                .take(dimensions as usize)
                .map(|bytes| {
                    if bytes.len() == 4 {
                        f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                    } else {
                        0.0
                    }
                })
                .collect();

            // Calculate similarity
            let similarity = cosine_similarity(query_vector, &vector);

            if similarity >= min_sim {
                results.push(SimilarityResult {
                    item_id: chunk.item_id.clone(),
                    item_title,
                    chunk,
                    similarity,
                });
            }
        }

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

        // Limit results
        results.truncate(limit);

        Ok(results)
    }

    /// Hybrid search combining vector similarity and full-text search.
    ///
    /// The final score is: `vector_weight * vector_score + (1 - vector_weight) * fts_score`
    pub fn hybrid_search(
        &self,
        query: &str,
        query_vector: &[f32],
        limit: usize,
        vector_weight: f32,
    ) -> DbResult<Vec<SimilarityResult>> {
        // Get vector search results (more than limit to allow for combining)
        let vector_results = self.vector_search(query_vector, limit * 2, Some(0.1))?;

        // Get FTS results
        let conn = self.conn()?;
        let mut fts_stmt = conn.prepare(
            r#"
            SELECT c.id, c.item_id, c.chunk_index, c.content, c.start_time, c.end_time,
                   i.title, bm25(chunks_fts)
            FROM chunks_fts
            JOIN chunks c ON c.id = chunks_fts.rowid
            JOIN items i ON i.id = c.item_id
            WHERE chunks_fts MATCH ?1
            ORDER BY bm25(chunks_fts)
            LIMIT ?2
            "#,
        )?;

        let fts_results: Vec<(Chunk, String, f32)> = fts_stmt
            .query_map(params![query, limit * 2], |row| {
                let chunk = Chunk {
                    id: row.get(0)?,
                    item_id: row.get(1)?,
                    chunk_index: row.get(2)?,
                    content: row.get(3)?,
                    start_time: row.get(4)?,
                    end_time: row.get(5)?,
                };
                let item_title: String = row.get(6)?;
                let bm25_score: f64 = row.get(7)?;
                // BM25 scores are negative, normalize to 0-1 range
                let normalized_score = 1.0 / (1.0 + (-bm25_score as f32).exp());
                Ok((chunk, item_title, normalized_score))
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Combine results using a simple score fusion
        use std::collections::HashMap;

        let mut combined: HashMap<String, SimilarityResult> = HashMap::new();

        // Add vector results
        for result in vector_results {
            combined.insert(
                result.chunk.id.clone(),
                SimilarityResult {
                    similarity: result.similarity * vector_weight,
                    ..result
                },
            );
        }

        // Add/update with FTS results
        let fts_weight = 1.0 - vector_weight;
        for (chunk, item_title, fts_score) in fts_results {
            let chunk_id = chunk.id.clone();
            let item_id = chunk.item_id.clone();

            combined
                .entry(chunk_id)
                .and_modify(|e| {
                    e.similarity += fts_score * fts_weight;
                })
                .or_insert(SimilarityResult {
                    chunk,
                    similarity: fts_score * fts_weight,
                    item_id,
                    item_title,
                });
        }

        // Sort and limit
        let mut results: Vec<SimilarityResult> = combined.into_values().collect();
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(limit);

        Ok(results)
    }

    /// Get chunks that don't have embeddings yet.
    pub fn get_unembedded_chunks(&self, limit: usize) -> DbResult<Vec<Chunk>> {
        let conn = self.conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT c.id, c.item_id, c.chunk_index, c.content, c.start_time, c.end_time
            FROM chunks c
            LEFT JOIN embeddings e ON e.chunk_id = c.id
            WHERE e.chunk_id IS NULL
            ORDER BY c.item_id, c.chunk_index
            LIMIT ?1
            "#,
        )?;

        let chunks = stmt
            .query_map(params![limit as i64], |row| {
                Ok(Chunk {
                    id: row.get(0)?,
                    item_id: row.get(1)?,
                    chunk_index: row.get(2)?,
                    content: row.get(3)?,
                    start_time: row.get(4)?,
                    end_time: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(chunks)
    }

    /// Get embedding statistics: (embedded_count, total_count).
    pub fn embedding_stats(&self) -> DbResult<(i64, i64)> {
        let conn = self.conn()?;

        let total: i64 =
            conn.query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;

        let embedded: i64 =
            conn.query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;

        Ok((embedded, total))
    }

    /// Get all embeddings for vector operations.
    pub fn get_all_embeddings(&self) -> DbResult<Vec<(String, Vec<f32>)>> {
        let conn = self.conn()?;

        let mut stmt = conn.prepare(
            "SELECT chunk_id, vector, dimensions FROM embeddings"
        )?;

        let results = stmt
            .query_map([], |row| {
                let chunk_id: String = row.get(0)?;
                let vector_bytes: Vec<u8> = row.get(1)?;
                let dimensions: i32 = row.get(2)?;

                let vector: Vec<f32> = vector_bytes
                    .chunks(4)
                    .take(dimensions as usize)
                    .map(|bytes| {
                        if bytes.len() == 4 {
                            f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                        } else {
                            0.0
                        }
                    })
                    .collect();

                Ok((chunk_id, vector))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use olal_core::{Item, ItemType};

    #[test]
    fn test_cosine_similarity() {
        // Identical vectors
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.0001);

        // Orthogonal vectors
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.0001);

        // Opposite vectors
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - (-1.0)).abs() < 0.0001);

        // Empty vectors
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&a, &b), 0.0);

        // Different lengths
        let a = vec![1.0, 0.0];
        let b = vec![1.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_vector_search() {
        let db = Database::open_in_memory().unwrap();

        // Create an item
        let item = Item::new(ItemType::Note, "Test Note");
        db.create_item(&item).unwrap();

        // Create chunks
        let chunk1 = Chunk::new(item.id.clone(), 0, "First chunk about Rust programming");
        let chunk2 = Chunk::new(item.id.clone(), 1, "Second chunk about Python");

        db.create_chunk(&chunk1).unwrap();
        db.create_chunk(&chunk2).unwrap();

        // Store embeddings (simple test vectors)
        let vec1 = vec![1.0, 0.0, 0.0, 0.0];
        let vec2 = vec![0.0, 1.0, 0.0, 0.0];

        db.store_embedding(&chunk1.id, &vec1, "test-model").unwrap();
        db.store_embedding(&chunk2.id, &vec2, "test-model").unwrap();

        // Search with a query similar to vec1
        let query = vec![0.9, 0.1, 0.0, 0.0];
        let results = db.vector_search(&query, 10, None).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].chunk.id, chunk1.id); // More similar to query
    }

    #[test]
    fn test_unembedded_chunks() {
        let db = Database::open_in_memory().unwrap();

        let item = Item::new(ItemType::Note, "Test Note");
        db.create_item(&item).unwrap();

        let chunk1 = Chunk::new(item.id.clone(), 0, "Embedded chunk");
        let chunk2 = Chunk::new(item.id.clone(), 1, "Unembedded chunk");

        db.create_chunk(&chunk1).unwrap();
        db.create_chunk(&chunk2).unwrap();

        // Only embed chunk1
        db.store_embedding(&chunk1.id, &[1.0, 0.0], "test-model")
            .unwrap();

        let unembedded = db.get_unembedded_chunks(10).unwrap();
        assert_eq!(unembedded.len(), 1);
        assert_eq!(unembedded[0].id, chunk2.id);
    }

    #[test]
    fn test_embedding_stats() {
        let db = Database::open_in_memory().unwrap();

        let item = Item::new(ItemType::Note, "Test Note");
        db.create_item(&item).unwrap();

        let chunk1 = Chunk::new(item.id.clone(), 0, "Chunk 1");
        let chunk2 = Chunk::new(item.id.clone(), 1, "Chunk 2");
        let chunk3 = Chunk::new(item.id.clone(), 2, "Chunk 3");

        db.create_chunk(&chunk1).unwrap();
        db.create_chunk(&chunk2).unwrap();
        db.create_chunk(&chunk3).unwrap();

        // Embed only 2 chunks
        db.store_embedding(&chunk1.id, &[1.0], "test-model").unwrap();
        db.store_embedding(&chunk2.id, &[1.0], "test-model").unwrap();

        let (embedded, total) = db.embedding_stats().unwrap();
        assert_eq!(embedded, 2);
        assert_eq!(total, 3);
    }
}
