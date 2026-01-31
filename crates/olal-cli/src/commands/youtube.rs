//! YouTube command - Generate YouTube metadata from video content.

use super::get_database;
use anyhow::{Context, Result};
use olal_config::Config;
use olal_ollama::{GenerateOptions, GenerateRequest, OllamaClient};
use colored::Colorize;
use std::fmt;
use std::io::{self, Write};
use tokio::runtime::Runtime;

/// Content style for YouTube metadata generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentStyle {
    Tutorial,
    Review,
    Vlog,
    Educational,
}

impl ContentStyle {
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tutorial" => Some(Self::Tutorial),
            "review" => Some(Self::Review),
            "vlog" => Some(Self::Vlog),
            "educational" => Some(Self::Educational),
            _ => None,
        }
    }

    /// Get the style-specific prompt modifier.
    pub fn prompt_modifier(&self) -> &'static str {
        match self {
            Self::Tutorial => "This is a tutorial/how-to video. Focus on the learning objectives and steps covered.",
            Self::Review => "This is a review video. Focus on opinions, pros/cons, and recommendations.",
            Self::Vlog => "This is a vlog/personal video. Focus on the story, experiences, and personal insights.",
            Self::Educational => "This is an educational video. Focus on key concepts, facts, and learning outcomes.",
        }
    }
}

impl fmt::Display for ContentStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tutorial => write!(f, "tutorial"),
            Self::Review => write!(f, "review"),
            Self::Vlog => write!(f, "vlog"),
            Self::Educational => write!(f, "educational"),
        }
    }
}

/// Chapter marker for YouTube.
#[derive(Debug, Clone)]
pub struct Chapter {
    pub timestamp: String,
    pub title: String,
}

impl fmt::Display for Chapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.timestamp, self.title)
    }
}

/// YouTube metadata container.
#[derive(Debug, Clone, Default)]
pub struct YoutubeMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub chapters: Option<Vec<Chapter>>,
}

/// Output mode flags.
pub struct OutputMode {
    pub title_only: bool,
    pub description_only: bool,
    pub chapters_only: bool,
    pub tags_only: bool,
}

impl OutputMode {
    /// Returns true if all flags are false (generate everything).
    pub fn generate_all(&self) -> bool {
        !self.title_only && !self.description_only && !self.chapters_only && !self.tags_only
    }
}

/// Run the youtube command.
pub fn run(
    item_id: &str,
    style: Option<String>,
    model: Option<String>,
    title_only: bool,
    description_only: bool,
    chapters_only: bool,
    tags_only: bool,
) -> Result<()> {
    let db = get_database()?;
    let config = Config::load().context("Failed to load configuration")?;

    // Parse content style
    let content_style = style
        .as_deref()
        .map(|s| ContentStyle::from_str(s))
        .flatten()
        .unwrap_or(ContentStyle::Educational);

    // Get item by ID (with prefix matching)
    let item = db
        .get_item_by_prefix(item_id)
        .context("Failed to find item")?;

    println!(
        "{} {} {}",
        "Item:".cyan().bold(),
        item.title.white(),
        format!("[{}]", &item.id[..8]).dimmed()
    );
    println!("{} {}", "Type:".cyan(), item.item_type.as_str());
    println!("{} {}", "Style:".cyan(), content_style);
    println!("{}", "─".repeat(70));
    println!();

    // Get all chunks for the item
    let chunks = db
        .get_chunks_by_item(&item.id)
        .context("Failed to get chunks")?;

    if chunks.is_empty() {
        anyhow::bail!(
            "No content chunks found for this item. The item may not have been fully processed."
        );
    }

    // Combine chunk content
    let content: String = chunks
        .iter()
        .map(|c| {
            if let (Some(start), Some(_end)) = (c.start_time, c.end_time) {
                format!("[{:.0}s] {}", start, c.content)
            } else {
                c.content.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // Truncate if too long (keep first ~8000 chars for context window)
    let content = if content.len() > 8000 {
        format!("{}...\n[Content truncated]", &content[..8000])
    } else {
        content
    };

    // Create Ollama client
    let client = OllamaClient::from_config(&config.ollama)
        .context("Failed to create Ollama client")?;

    // Create async runtime
    let rt = Runtime::new().context("Failed to create async runtime")?;

    // Check if Ollama is available
    let is_available = rt.block_on(client.is_available());
    if !is_available {
        anyhow::bail!(
            "Ollama is not running at {}. Start it with 'ollama serve'.",
            config.ollama.host
        );
    }

    let model_name = model.as_deref().unwrap_or(&config.ollama.model);
    let output_mode = OutputMode {
        title_only,
        description_only,
        chapters_only,
        tags_only,
    };

    // Generate metadata
    let mut metadata = YoutubeMetadata::default();

    // Generate title
    if output_mode.generate_all() || output_mode.title_only {
        print!("{}", "Generating title...".dimmed());
        io::stdout().flush()?;
        let title = generate_title(&rt, &client, model_name, &content, content_style)?;
        println!("\r{}", " ".repeat(30));
        metadata.title = Some(title);
    }

    // Generate description
    if output_mode.generate_all() || output_mode.description_only {
        print!("{}", "Generating description...".dimmed());
        io::stdout().flush()?;
        let description = generate_description(&rt, &client, model_name, &content, content_style)?;
        println!("\r{}", " ".repeat(30));
        metadata.description = Some(description);
    }

    // Generate tags
    if output_mode.generate_all() || output_mode.tags_only {
        print!("{}", "Generating tags...".dimmed());
        io::stdout().flush()?;
        let tags = generate_tags(&rt, &client, model_name, &content, content_style)?;
        println!("\r{}", " ".repeat(30));
        metadata.tags = Some(tags);
    }

    // Generate chapters (only if content has timestamps)
    let has_timestamps = chunks.iter().any(|c| c.start_time.is_some());
    if has_timestamps && (output_mode.generate_all() || output_mode.chapters_only) {
        print!("{}", "Generating chapters...".dimmed());
        io::stdout().flush()?;
        let chapters = generate_chapters(&rt, &client, model_name, &content, content_style)?;
        println!("\r{}", " ".repeat(30));
        metadata.chapters = Some(chapters);
    }

    // Display output
    display_metadata(&metadata, &output_mode);

    Ok(())
}

fn generate_title(
    rt: &Runtime,
    client: &OllamaClient,
    model: &str,
    content: &str,
    style: ContentStyle,
) -> Result<String> {
    let prompt = format!(
        r#"Generate a compelling YouTube video title based on this content.

{}

Requirements:
- 50-60 characters maximum
- Attention-grabbing but not clickbait
- Include relevant keywords
- Match the content style

Content:
{}

Respond with ONLY the title, no quotes or extra text."#,
        style.prompt_modifier(),
        content
    );

    let request = GenerateRequest::new(model, &prompt)
        .with_options(GenerateOptions::new().with_temperature(0.7));

    let response = rt.block_on(client.generate(request)).map_err(|e| {
        anyhow::anyhow!("Failed to generate title: {}", e)
    })?;

    Ok(response.response.trim().to_string())
}

fn generate_description(
    rt: &Runtime,
    client: &OllamaClient,
    model: &str,
    content: &str,
    style: ContentStyle,
) -> Result<String> {
    let prompt = format!(
        r#"Generate a YouTube video description based on this content.

{}

Structure:
1. Opening hook (1-2 sentences that grab attention)
2. Main content summary as bullet points (3-5 points)
3. Call to action (subscribe, like, comment)

Requirements:
- Use emojis sparingly but effectively
- Include relevant keywords naturally
- Keep it scannable with line breaks
- 150-300 words total

Content:
{}

Respond with ONLY the description text, ready to paste into YouTube."#,
        style.prompt_modifier(),
        content
    );

    let request = GenerateRequest::new(model, &prompt)
        .with_options(GenerateOptions::new().with_temperature(0.7));

    let response = rt.block_on(client.generate(request)).map_err(|e| {
        anyhow::anyhow!("Failed to generate description: {}", e)
    })?;

    Ok(response.response.trim().to_string())
}

fn generate_tags(
    rt: &Runtime,
    client: &OllamaClient,
    model: &str,
    content: &str,
    style: ContentStyle,
) -> Result<Vec<String>> {
    let prompt = format!(
        r#"Generate YouTube tags for this video content.

{}

Requirements:
- 10-15 tags
- Mix of broad and specific terms
- Include relevant keywords from the content
- Consider search terms viewers might use

Content:
{}

Respond with ONLY a comma-separated list of tags, no numbering or extra text."#,
        style.prompt_modifier(),
        content
    );

    let request = GenerateRequest::new(model, &prompt)
        .with_options(GenerateOptions::new().with_temperature(0.5));

    let response = rt.block_on(client.generate(request)).map_err(|e| {
        anyhow::anyhow!("Failed to generate tags: {}", e)
    })?;

    let tags: Vec<String> = response
        .response
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    Ok(tags)
}

fn generate_chapters(
    rt: &Runtime,
    client: &OllamaClient,
    model: &str,
    content: &str,
    style: ContentStyle,
) -> Result<Vec<Chapter>> {
    let prompt = format!(
        r#"Generate YouTube chapter markers based on this timestamped content.

{}

Requirements:
- Use the timestamps from the content (format: MM:SS or H:MM:SS)
- Create 5-10 logical chapter breaks
- Each chapter title should be 2-5 words
- First chapter should start at 0:00 (Intro)

Content:
{}

Respond with chapters in this exact format, one per line:
0:00 Intro
1:23 Chapter Title
etc.

No extra text or formatting."#,
        style.prompt_modifier(),
        content
    );

    let request = GenerateRequest::new(model, &prompt)
        .with_options(GenerateOptions::new().with_temperature(0.3));

    let response = rt.block_on(client.generate(request)).map_err(|e| {
        anyhow::anyhow!("Failed to generate chapters: {}", e)
    })?;

    let chapters: Vec<Chapter> = response
        .response
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            // Parse "MM:SS Title" or "H:MM:SS Title" format
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                Some(Chapter {
                    timestamp: parts[0].to_string(),
                    title: parts[1].to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(chapters)
}

fn display_metadata(metadata: &YoutubeMetadata, mode: &OutputMode) {
    if let Some(ref title) = metadata.title {
        if mode.generate_all() || mode.title_only {
            println!("{}", "Title:".green().bold());
            println!("{}", title);
            println!();
        }
    }

    if let Some(ref description) = metadata.description {
        if mode.generate_all() || mode.description_only {
            println!("{}", "Description:".green().bold());
            println!("{}", description);
            println!();
        }
    }

    if let Some(ref tags) = metadata.tags {
        if mode.generate_all() || mode.tags_only {
            println!("{}", "Tags:".green().bold());
            println!("{}", tags.join(", "));
            println!();
        }
    }

    if let Some(ref chapters) = metadata.chapters {
        if mode.generate_all() || mode.chapters_only {
            println!("{}", "Chapters:".green().bold());
            for chapter in chapters {
                println!("{}", chapter);
            }
            println!();
        }
    }

    // Show copy hint
    if mode.generate_all() {
        println!("{}", "─".repeat(70));
        println!(
            "{}",
            "Tip: Use --title-only, --description-only, --tags-only, or --chapters-only to generate specific sections."
                .dimmed()
        );
    }
}
