//! Core domain types for Olal.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for items.
pub type ItemId = String;

/// Unique identifier for chunks.
pub type ChunkId = String;

/// Unique identifier for tasks.
pub type TaskId = String;

/// Unique identifier for projects.
pub type ProjectId = String;

/// Unique identifier for tags.
pub type TagId = String;

/// Generate a new unique ID.
pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

/// Type of content item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    Video,
    Audio,
    Document,
    Note,
    Bookmark,
    Code,
    Image,
}

impl ItemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ItemType::Video => "video",
            ItemType::Audio => "audio",
            ItemType::Document => "document",
            ItemType::Note => "note",
            ItemType::Bookmark => "bookmark",
            ItemType::Code => "code",
            ItemType::Image => "image",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "video" => Some(ItemType::Video),
            "audio" => Some(ItemType::Audio),
            "document" => Some(ItemType::Document),
            "note" => Some(ItemType::Note),
            "bookmark" => Some(ItemType::Bookmark),
            "code" => Some(ItemType::Code),
            "image" => Some(ItemType::Image),
            _ => None,
        }
    }

    /// Detect item type from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            // Video formats
            "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" => Some(ItemType::Video),
            // Audio formats
            "mp3" | "wav" | "m4a" | "flac" | "ogg" | "aac" => Some(ItemType::Audio),
            // Document formats
            "pdf" | "doc" | "docx" | "odt" | "rtf" => Some(ItemType::Document),
            // Note formats
            "md" | "markdown" | "txt" | "org" => Some(ItemType::Note),
            // Code formats
            "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h" | "java" | "rb" | "sh" | "zsh"
            | "bash" | "json" | "yaml" | "yml" | "toml" | "html" | "css" | "sql" => {
                Some(ItemType::Code)
            }
            // Image formats
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" => Some(ItemType::Image),
            _ => None,
        }
    }
}

impl std::fmt::Display for ItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A content item in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub item_type: ItemType,
    pub title: String,
    pub source_path: Option<String>,
    pub content_hash: Option<String>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

impl Item {
    pub fn new(item_type: ItemType, title: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            item_type,
            title: title.into(),
            source_path: None,
            content_hash: None,
            summary: None,
            created_at: Utc::now(),
            processed_at: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_source_path(mut self, path: impl Into<String>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    pub fn with_content_hash(mut self, hash: impl Into<String>) -> Self {
        self.content_hash = Some(hash.into());
        self
    }
}

/// A chunk of text content for RAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub item_id: ItemId,
    pub chunk_index: i32,
    pub content: String,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
}

impl Chunk {
    pub fn new(item_id: ItemId, chunk_index: i32, content: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            item_id,
            chunk_index,
            content: content.into(),
            start_time: None,
            end_time: None,
        }
    }

    pub fn with_timestamps(mut self, start: f64, end: f64) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }
}

/// Status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Done,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
            TaskStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Some(TaskStatus::Pending),
            "in_progress" => Some(TaskStatus::InProgress),
            "done" => Some(TaskStatus::Done),
            "cancelled" => Some(TaskStatus::Cancelled),
            _ => None,
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A task in the task management system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: i32,
    pub project_id: Option<ProjectId>,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            title: title.into(),
            description: None,
            status: TaskStatus::Pending,
            priority: 0,
            project_id: None,
            due_date: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_project(mut self, project_id: ProjectId) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub fn mark_done(&mut self) {
        self.status = TaskStatus::Done;
        self.completed_at = Some(Utc::now());
    }
}

/// Status of a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    #[default]
    Active,
    Archived,
    Completed,
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::Archived => "archived",
            ProjectStatus::Completed => "completed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "active" => Some(ProjectStatus::Active),
            "archived" => Some(ProjectStatus::Archived),
            "completed" => Some(ProjectStatus::Completed),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A project for organizing items and tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            name: name.into(),
            description: None,
            status: ProjectStatus::Active,
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// A tag for categorizing items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub color: Option<String>,
}

impl Tag {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            name: name.into(),
            color: None,
        }
    }

    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// Status of a queue item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QueueStatus {
    #[default]
    Pending,
    Processing,
    Done,
    Failed,
}

impl QueueStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueueStatus::Pending => "pending",
            QueueStatus::Processing => "processing",
            QueueStatus::Done => "done",
            QueueStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Some(QueueStatus::Pending),
            "processing" => Some(QueueStatus::Processing),
            "done" => Some(QueueStatus::Done),
            "failed" => Some(QueueStatus::Failed),
            _ => None,
        }
    }
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An item in the processing queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub source_path: String,
    pub item_type: ItemType,
    pub status: QueueStatus,
    pub priority: i32,
    pub attempts: i32,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl QueueItem {
    pub fn new(source_path: impl Into<String>, item_type: ItemType) -> Self {
        Self {
            id: new_id(),
            source_path: source_path.into(),
            item_type,
            status: QueueStatus::Pending,
            priority: 0,
            attempts: 0,
            error: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// Type of link between items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    Related,
    References,
    Continues,
    Parent,
    Child,
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::Related => "related",
            LinkType::References => "references",
            LinkType::Continues => "continues",
            LinkType::Parent => "parent",
            LinkType::Child => "child",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "related" => Some(LinkType::Related),
            "references" => Some(LinkType::References),
            "continues" => Some(LinkType::Continues),
            "parent" => Some(LinkType::Parent),
            "child" => Some(LinkType::Child),
            _ => None,
        }
    }
}

/// A link between two items in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub source_id: ItemId,
    pub target_id: ItemId,
    pub link_type: LinkType,
    pub strength: f64,
}

impl Link {
    pub fn new(source_id: ItemId, target_id: ItemId, link_type: LinkType) -> Self {
        Self {
            source_id,
            target_id,
            link_type,
            strength: 1.0,
        }
    }

    pub fn with_strength(mut self, strength: f64) -> Self {
        self.strength = strength;
        self
    }
}

/// Statistics about the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_items: i64,
    pub items_by_type: std::collections::HashMap<String, i64>,
    pub total_chunks: i64,
    pub total_tasks: i64,
    pub pending_tasks: i64,
    pub total_projects: i64,
    pub total_tags: i64,
    pub queue_pending: i64,
    pub queue_processing: i64,
    pub queue_failed: i64,
    pub database_size_bytes: i64,
}

impl Default for DatabaseStats {
    fn default() -> Self {
        Self {
            total_items: 0,
            items_by_type: std::collections::HashMap::new(),
            total_chunks: 0,
            total_tasks: 0,
            pending_tasks: 0,
            total_projects: 0,
            total_tags: 0,
            queue_pending: 0,
            queue_processing: 0,
            queue_failed: 0,
            database_size_bytes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_type_from_extension() {
        assert_eq!(ItemType::from_extension("mp4"), Some(ItemType::Video));
        assert_eq!(ItemType::from_extension("mp3"), Some(ItemType::Audio));
        assert_eq!(ItemType::from_extension("MD"), Some(ItemType::Note));
        assert_eq!(ItemType::from_extension("rs"), Some(ItemType::Code));
        assert_eq!(ItemType::from_extension("pdf"), Some(ItemType::Document));
        assert_eq!(ItemType::from_extension("xyz"), None);
    }

    #[test]
    fn test_item_creation() {
        let item = Item::new(ItemType::Video, "Test Video")
            .with_source_path("/path/to/video.mp4")
            .with_content_hash("abc123");

        assert_eq!(item.title, "Test Video");
        assert_eq!(item.item_type, ItemType::Video);
        assert_eq!(item.source_path, Some("/path/to/video.mp4".to_string()));
        assert!(!item.id.is_empty());
    }

    #[test]
    fn test_task_workflow() {
        let mut task = Task::new("Complete Phase 1").with_priority(1);

        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.completed_at.is_none());

        task.mark_done();

        assert_eq!(task.status, TaskStatus::Done);
        assert!(task.completed_at.is_some());
    }
}
