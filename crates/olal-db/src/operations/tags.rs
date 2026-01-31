//! Tag CRUD operations.

use crate::database::Database;
use crate::error::{DbError, DbResult};
use olal_core::{ItemId, Tag, TagId};
use rusqlite::params;

impl Database {
    /// Create a new tag.
    pub fn create_tag(&self, tag: &Tag) -> DbResult<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO tags (id, name, color) VALUES (?1, ?2, ?3)",
            params![tag.id, tag.name, tag.color],
        )?;
        Ok(())
    }

    /// Get a tag by ID.
    pub fn get_tag(&self, id: &TagId) -> DbResult<Tag> {
        let conn = self.conn()?;
        let tag = conn.query_row(
            "SELECT id, name, color FROM tags WHERE id = ?1",
            params![id],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Tag not found: {}", id)),
            _ => DbError::from(e),
        })?;

        Ok(tag)
    }

    /// Get a tag by name.
    pub fn get_tag_by_name(&self, name: &str) -> DbResult<Option<Tag>> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, name, color FROM tags WHERE name = ?1",
            params![name],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            },
        );

        match result {
            Ok(tag) => Ok(Some(tag)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Get or create a tag by name.
    pub fn get_or_create_tag(&self, name: &str) -> DbResult<Tag> {
        if let Some(tag) = self.get_tag_by_name(name)? {
            return Ok(tag);
        }

        let tag = Tag::new(name);
        self.create_tag(&tag)?;
        Ok(tag)
    }

    /// Delete a tag by ID.
    pub fn delete_tag(&self, id: &TagId) -> DbResult<()> {
        let conn = self.conn()?;
        let rows = conn.execute("DELETE FROM tags WHERE id = ?1", params![id])?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Tag not found: {}", id)));
        }

        Ok(())
    }

    /// List all tags.
    pub fn list_tags(&self) -> DbResult<Vec<Tag>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT id, name, color FROM tags ORDER BY name")?;

        let tags = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?;

        tags.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Add a tag to an item.
    pub fn add_tag_to_item(&self, item_id: &ItemId, tag_id: &TagId) -> DbResult<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)",
            params![item_id, tag_id],
        )?;
        Ok(())
    }

    /// Add a tag to an item by tag name (creates tag if needed).
    pub fn tag_item(&self, item_id: &ItemId, tag_name: &str) -> DbResult<Tag> {
        let tag = self.get_or_create_tag(tag_name)?;
        self.add_tag_to_item(item_id, &tag.id)?;
        Ok(tag)
    }

    /// Remove a tag from an item.
    pub fn remove_tag_from_item(&self, item_id: &ItemId, tag_id: &TagId) -> DbResult<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM item_tags WHERE item_id = ?1 AND tag_id = ?2",
            params![item_id, tag_id],
        )?;
        Ok(())
    }

    /// Get all tags for an item.
    pub fn get_item_tags(&self, item_id: &ItemId) -> DbResult<Vec<Tag>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.color FROM tags t
             INNER JOIN item_tags it ON it.tag_id = t.id
             WHERE it.item_id = ?1 ORDER BY t.name",
        )?;

        let tags = stmt.query_map(params![item_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?;

        tags.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get all items with a specific tag.
    pub fn get_items_by_tag(&self, tag_id: &TagId) -> DbResult<Vec<ItemId>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT item_id FROM item_tags WHERE tag_id = ?1",
        )?;

        let items = stmt.query_map(params![tag_id], |row| row.get(0))?;
        items.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get tag usage counts.
    pub fn get_tag_counts(&self) -> DbResult<Vec<(Tag, i64)>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.color, COUNT(it.item_id) as count
             FROM tags t
             LEFT JOIN item_tags it ON it.tag_id = t.id
             GROUP BY t.id
             ORDER BY count DESC",
        )?;

        let results = stmt.query_map([], |row| {
            let tag = Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            };
            let count: i64 = row.get(3)?;
            Ok((tag, count))
        })?;

        results.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use olal_core::{Item, ItemType};

    #[test]
    fn test_tag_crud() {
        let db = Database::open_in_memory().unwrap();

        // Create
        let tag = Tag::new("rust").with_color("#FF5733");
        db.create_tag(&tag).unwrap();

        // Read
        let fetched = db.get_tag(&tag.id).unwrap();
        assert_eq!(fetched.name, "rust");
        assert_eq!(fetched.color, Some("#FF5733".to_string()));

        // Read by name
        let by_name = db.get_tag_by_name("rust").unwrap();
        assert!(by_name.is_some());

        // List
        let all = db.list_tags().unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        db.delete_tag(&tag.id).unwrap();
        assert!(db.get_tag(&tag.id).is_err());
    }

    #[test]
    fn test_tag_item_association() {
        let db = Database::open_in_memory().unwrap();

        // Create item and tag
        let item = Item::new(ItemType::Note, "Test Note");
        db.create_item(&item).unwrap();

        let tag = Tag::new("important");
        db.create_tag(&tag).unwrap();

        // Associate
        db.add_tag_to_item(&item.id, &tag.id).unwrap();

        // Get item's tags
        let tags = db.get_item_tags(&item.id).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "important");

        // Get items by tag
        let items = db.get_items_by_tag(&tag.id).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], item.id);

        // Remove association
        db.remove_tag_from_item(&item.id, &tag.id).unwrap();
        let tags = db.get_item_tags(&item.id).unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn test_tag_item_helper() {
        let db = Database::open_in_memory().unwrap();

        let item = Item::new(ItemType::Note, "Test Note");
        db.create_item(&item).unwrap();

        // Tag with name (creates tag automatically)
        let tag = db.tag_item(&item.id, "auto-created").unwrap();
        assert_eq!(tag.name, "auto-created");

        // Tag should exist now
        let existing = db.get_tag_by_name("auto-created").unwrap();
        assert!(existing.is_some());
    }
}
