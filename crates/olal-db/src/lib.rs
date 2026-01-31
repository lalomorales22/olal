//! Olal DB - Database layer for Olal using SQLite.

mod database;
mod error;
mod migrations;
mod operations;

pub use database::Database;
pub use error::{DbError, DbResult};
pub use operations::vectors::{cosine_similarity, SimilarityResult};
