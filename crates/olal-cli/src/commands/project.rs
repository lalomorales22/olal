//! Project management commands.

use super::get_database;
use anyhow::Result;
use olal_core::{Project, ProjectStatus, TaskStatus};
use colored::Colorize;

pub fn create(name: &str, description: Option<String>) -> Result<()> {
    let db = get_database()?;

    // Check if project already exists
    if db.get_project_by_name(name)?.is_some() {
        anyhow::bail!("Project already exists: {}", name);
    }

    let mut project = Project::new(name);
    if let Some(desc) = description {
        project = project.with_description(desc);
    }

    db.create_project(&project)?;

    println!(
        "{} Project created: {}",
        "✓".green(),
        name.white().bold()
    );

    Ok(())
}

pub fn list() -> Result<()> {
    let db = get_database()?;

    let projects = db.list_projects(None)?;

    if projects.is_empty() {
        println!(
            "{}",
            "No projects found. Use 'olal project create <name>' to create one.".dimmed()
        );
        return Ok(());
    }

    println!("{}", "Projects".cyan().bold());
    println!("{}", "─".repeat(70));

    for project in projects {
        let status_icon = match project.status {
            ProjectStatus::Active => "●".green(),
            ProjectStatus::Completed => "✓".blue(),
            ProjectStatus::Archived => "○".dimmed(),
        };

        // Count tasks in project
        let tasks = db.list_tasks_by_project(&project.id)?;
        let pending = tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();
        let total = tasks.len();

        println!(
            "{} {} {}",
            status_icon,
            project.name.white().bold(),
            if total > 0 {
                format!("({}/{} tasks)", pending, total).dimmed().to_string()
            } else {
                String::new()
            }
        );

        if let Some(ref desc) = project.description {
            println!("  {}", desc.dimmed());
        }
    }

    Ok(())
}

pub fn show(name: &str) -> Result<()> {
    let db = get_database()?;

    let project = db
        .get_project_by_name(name)?
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", name))?;

    let status_icon = match project.status {
        ProjectStatus::Active => "●".green(),
        ProjectStatus::Completed => "✓".blue(),
        ProjectStatus::Archived => "○".dimmed(),
    };

    println!("{} {}", status_icon, project.name.white().bold());
    println!("{}", "─".repeat(70));

    println!(
        "  {}: {}",
        "ID".cyan(),
        project.id
    );
    println!(
        "  {}: {}",
        "Status".cyan(),
        project.status
    );
    println!(
        "  {}: {}",
        "Created".cyan(),
        project.created_at.format("%Y-%m-%d %H:%M")
    );

    if let Some(ref desc) = project.description {
        println!("  {}: {}", "Description".cyan(), desc);
    }

    // List tasks
    let tasks = db.list_tasks_by_project(&project.id)?;
    if !tasks.is_empty() {
        println!();
        println!("{}", "Tasks".white().bold());
        println!("{}", "─".repeat(70));

        for task in tasks {
            let status_icon = match task.status {
                TaskStatus::Pending => "○".yellow(),
                TaskStatus::InProgress => "◐".blue(),
                TaskStatus::Done => "●".green(),
                TaskStatus::Cancelled => "✗".dimmed(),
            };

            let title = if task.status == TaskStatus::Done {
                task.title.dimmed().strikethrough().to_string()
            } else {
                task.title.clone()
            };

            println!("  {} {}", status_icon, title);
        }
    }

    Ok(())
}
