//! Task management commands.

use super::get_database;
use anyhow::Result;
use olal_core::{Task, TaskStatus};
use colored::Colorize;

pub fn add(description: &str, priority: i32, project: Option<String>) -> Result<()> {
    let db = get_database()?;

    let mut task = Task::new(description).with_priority(priority);

    // If project specified, find it
    if let Some(ref project_name) = project {
        let proj = db.get_project_by_name(project_name)?;
        if let Some(p) = proj {
            task = task.with_project(p.id);
        } else {
            anyhow::bail!("Project not found: {}", project_name);
        }
    }

    db.create_task(&task)?;

    println!(
        "{} Task added: {}",
        "✓".green(),
        description.white().bold()
    );
    println!(
        "  ID: {}",
        task.id.chars().take(8).collect::<String>().dimmed()
    );

    Ok(())
}

pub fn list(status_filter: Option<String>) -> Result<()> {
    let db = get_database()?;

    let status = status_filter
        .as_ref()
        .and_then(|s| TaskStatus::from_str(s));

    if status_filter.is_some() && status.is_none() {
        anyhow::bail!("Invalid status. Valid values: pending, in_progress, done, cancelled");
    }

    let tasks = db.list_tasks(status)?;

    if tasks.is_empty() {
        println!(
            "{}",
            "No tasks found. Use 'olal task add <description>' to create one.".dimmed()
        );
        return Ok(());
    }

    println!("{}", "Tasks".cyan().bold());
    println!("{}", "─".repeat(70));

    for task in tasks {
        let status_icon = match task.status {
            TaskStatus::Pending => "○".yellow(),
            TaskStatus::InProgress => "◐".blue(),
            TaskStatus::Done => "●".green(),
            TaskStatus::Cancelled => "✗".dimmed(),
        };

        let priority_indicator = if task.priority > 0 {
            format!(" [P{}]", task.priority).red().to_string()
        } else {
            String::new()
        };

        let id_short = task.id.chars().take(8).collect::<String>();

        let title = if task.status == TaskStatus::Done {
            task.title.dimmed().strikethrough().to_string()
        } else {
            task.title.white().to_string()
        };

        println!(
            "{} {} {} {}",
            status_icon,
            title,
            format!("[{}]", id_short).dimmed(),
            priority_indicator
        );

        if let Some(ref desc) = task.description {
            println!("  {}", desc.dimmed());
        }
    }

    Ok(())
}

pub fn done(id: &str) -> Result<()> {
    let db = get_database()?;

    // Try to find task by ID or prefix
    let task = db.get_task(id).or_else(|_| {
        // Try to find by prefix
        let tasks = db.list_tasks(None)?;
        tasks
            .into_iter()
            .find(|t| t.id.starts_with(id))
            .ok_or_else(|| olal_db::DbError::NotFound(format!("Task not found: {}", id)))
    })?;

    db.complete_task(&task.id)?;

    println!(
        "{} Task completed: {}",
        "✓".green(),
        task.title.strikethrough()
    );

    Ok(())
}

pub fn delete(id: &str) -> Result<()> {
    let db = get_database()?;

    // Try to find task by ID or prefix
    let task = db.get_task(id).or_else(|_| {
        let tasks = db.list_tasks(None)?;
        tasks
            .into_iter()
            .find(|t| t.id.starts_with(id))
            .ok_or_else(|| olal_db::DbError::NotFound(format!("Task not found: {}", id)))
    })?;

    db.delete_task(&task.id)?;

    println!(
        "{} Task deleted: {}",
        "✓".green(),
        task.title
    );

    Ok(())
}
