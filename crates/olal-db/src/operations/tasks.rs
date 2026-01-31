//! Task CRUD operations.

use crate::database::Database;
use crate::error::{DbError, DbResult};
use olal_core::{Task, TaskStatus};
use chrono::{DateTime, Utc};
use rusqlite::params;

impl Database {
    /// Create a new task.
    pub fn create_task(&self, task: &Task) -> DbResult<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO tasks (id, title, description, status, priority, project_id, due_date, created_at, completed_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                task.id,
                task.title,
                task.description,
                task.status.as_str(),
                task.priority,
                task.project_id,
                task.due_date.map(|dt| dt.to_rfc3339()),
                task.created_at.to_rfc3339(),
                task.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Get a task by ID.
    pub fn get_task(&self, id: &str) -> DbResult<Task> {
        let conn = self.conn()?;
        let task = conn.query_row(
            "SELECT id, title, description, status, priority, project_id, due_date, created_at, completed_at
             FROM tasks WHERE id = ?1",
            params![id],
            row_to_task,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Task not found: {}", id)),
            _ => DbError::from(e),
        })?;

        Ok(task)
    }

    /// Update a task.
    pub fn update_task(&self, task: &Task) -> DbResult<()> {
        let conn = self.conn()?;
        let rows = conn.execute(
            r#"
            UPDATE tasks
            SET title = ?2, description = ?3, status = ?4, priority = ?5,
                project_id = ?6, due_date = ?7, completed_at = ?8
            WHERE id = ?1
            "#,
            params![
                task.id,
                task.title,
                task.description,
                task.status.as_str(),
                task.priority,
                task.project_id,
                task.due_date.map(|dt| dt.to_rfc3339()),
                task.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Task not found: {}", task.id)));
        }

        Ok(())
    }

    /// Delete a task by ID.
    pub fn delete_task(&self, id: &str) -> DbResult<()> {
        let conn = self.conn()?;
        let rows = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Task not found: {}", id)));
        }

        Ok(())
    }

    /// List tasks with optional status filter.
    pub fn list_tasks(&self, status: Option<TaskStatus>) -> DbResult<Vec<Task>> {
        let conn = self.conn()?;

        let tasks = match status {
            Some(s) => {
                let mut stmt = conn.prepare(
                    "SELECT id, title, description, status, priority, project_id, due_date, created_at, completed_at
                     FROM tasks WHERE status = ?1 ORDER BY priority DESC, created_at",
                )?;
                let rows = stmt.query_map(params![s.as_str()], row_to_task)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id, title, description, status, priority, project_id, due_date, created_at, completed_at
                     FROM tasks ORDER BY priority DESC, created_at",
                )?;
                let rows = stmt.query_map([], row_to_task)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
        };

        Ok(tasks)
    }

    /// List tasks by project.
    pub fn list_tasks_by_project(&self, project_id: &str) -> DbResult<Vec<Task>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, title, description, status, priority, project_id, due_date, created_at, completed_at
             FROM tasks WHERE project_id = ?1 ORDER BY priority DESC, created_at",
        )?;

        let tasks = stmt.query_map(params![project_id], row_to_task)?;
        tasks.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Mark a task as done.
    pub fn complete_task(&self, id: &str) -> DbResult<()> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        let rows = conn.execute(
            "UPDATE tasks SET status = 'done', completed_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;

        if rows == 0 {
            return Err(DbError::NotFound(format!("Task not found: {}", id)));
        }

        Ok(())
    }
}

fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    let status_str: String = row.get(3)?;
    let due_date_str: Option<String> = row.get(6)?;
    let created_at_str: String = row.get(7)?;
    let completed_at_str: Option<String> = row.get(8)?;

    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        status: TaskStatus::from_str(&status_str).unwrap_or(TaskStatus::Pending),
        priority: row.get(4)?,
        project_id: row.get(5)?,
        due_date: due_date_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        }),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
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
    fn test_task_crud() {
        let db = Database::open_in_memory().unwrap();

        // Create
        let task = Task::new("Complete Phase 1").with_priority(1);
        db.create_task(&task).unwrap();

        // Read
        let fetched = db.get_task(&task.id).unwrap();
        assert_eq!(fetched.title, "Complete Phase 1");
        assert_eq!(fetched.status, TaskStatus::Pending);

        // Update
        let mut updated = fetched;
        updated.title = "Updated Task".to_string();
        updated.status = TaskStatus::InProgress;
        db.update_task(&updated).unwrap();

        let fetched = db.get_task(&task.id).unwrap();
        assert_eq!(fetched.title, "Updated Task");
        assert_eq!(fetched.status, TaskStatus::InProgress);

        // Complete
        db.complete_task(&task.id).unwrap();
        let fetched = db.get_task(&task.id).unwrap();
        assert_eq!(fetched.status, TaskStatus::Done);
        assert!(fetched.completed_at.is_some());

        // Delete
        db.delete_task(&task.id).unwrap();
        assert!(db.get_task(&task.id).is_err());
    }

    #[test]
    fn test_list_tasks_by_status() {
        let db = Database::open_in_memory().unwrap();

        let task1 = Task::new("Task 1");
        let mut task2 = Task::new("Task 2");
        task2.status = TaskStatus::InProgress;

        db.create_task(&task1).unwrap();
        db.create_task(&task2).unwrap();

        let pending = db.list_tasks(Some(TaskStatus::Pending)).unwrap();
        assert_eq!(pending.len(), 1);

        let in_progress = db.list_tasks(Some(TaskStatus::InProgress)).unwrap();
        assert_eq!(in_progress.len(), 1);

        let all = db.list_tasks(None).unwrap();
        assert_eq!(all.len(), 2);
    }
}
