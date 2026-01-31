//! Project CRUD operations.

use crate::database::Database;
use crate::error::{DbError, DbResult};
use olal_core::{Project, ProjectId, ProjectStatus};
use chrono::{DateTime, Utc};
use rusqlite::params;

impl Database {
    /// Create a new project.
    pub fn create_project(&self, project: &Project) -> DbResult<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO projects (id, name, description, status, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                project.id,
                project.name,
                project.description,
                project.status.as_str(),
                project.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get a project by ID.
    pub fn get_project(&self, id: &ProjectId) -> DbResult<Project> {
        let conn = self.conn()?;
        let project = conn.query_row(
            "SELECT id, name, description, status, created_at FROM projects WHERE id = ?1",
            params![id],
            row_to_project,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Project not found: {}", id)),
            _ => DbError::from(e),
        })?;

        Ok(project)
    }

    /// Get a project by name.
    pub fn get_project_by_name(&self, name: &str) -> DbResult<Option<Project>> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, name, description, status, created_at FROM projects WHERE name = ?1",
            params![name],
            row_to_project,
        );

        match result {
            Ok(project) => Ok(Some(project)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Update a project.
    pub fn update_project(&self, project: &Project) -> DbResult<()> {
        let conn = self.conn()?;
        let rows = conn.execute(
            r#"
            UPDATE projects
            SET name = ?2, description = ?3, status = ?4
            WHERE id = ?1
            "#,
            params![
                project.id,
                project.name,
                project.description,
                project.status.as_str(),
            ],
        )?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Project not found: {}", project.id)));
        }

        Ok(())
    }

    /// Delete a project by ID.
    pub fn delete_project(&self, id: &ProjectId) -> DbResult<()> {
        let conn = self.conn()?;
        let rows = conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Project not found: {}", id)));
        }

        Ok(())
    }

    /// List all projects.
    pub fn list_projects(&self, status: Option<ProjectStatus>) -> DbResult<Vec<Project>> {
        let conn = self.conn()?;

        let projects = match status {
            Some(s) => {
                let mut stmt = conn.prepare(
                    "SELECT id, name, description, status, created_at
                     FROM projects WHERE status = ?1 ORDER BY name",
                )?;
                let rows = stmt.query_map(params![s.as_str()], row_to_project)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id, name, description, status, created_at FROM projects ORDER BY name",
                )?;
                let rows = stmt.query_map([], row_to_project)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
        };

        Ok(projects)
    }
}

fn row_to_project(row: &rusqlite::Row) -> rusqlite::Result<Project> {
    let status_str: String = row.get(3)?;
    let created_at_str: String = row.get(4)?;

    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        status: ProjectStatus::from_str(&status_str).unwrap_or(ProjectStatus::Active),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_crud() {
        let db = Database::open_in_memory().unwrap();

        // Create
        let project = Project::new("Test Project").with_description("A test project");
        db.create_project(&project).unwrap();

        // Read
        let fetched = db.get_project(&project.id).unwrap();
        assert_eq!(fetched.name, "Test Project");
        assert_eq!(fetched.description, Some("A test project".to_string()));

        // Read by name
        let by_name = db.get_project_by_name("Test Project").unwrap();
        assert!(by_name.is_some());

        // Update
        let mut updated = fetched;
        updated.name = "Updated Project".to_string();
        db.update_project(&updated).unwrap();

        let fetched = db.get_project(&project.id).unwrap();
        assert_eq!(fetched.name, "Updated Project");

        // List
        let all = db.list_projects(None).unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        db.delete_project(&project.id).unwrap();
        assert!(db.get_project(&project.id).is_err());
    }
}
