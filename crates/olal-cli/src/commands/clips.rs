//! Clips command - AI-based clip detection from timestamped content.

use super::get_database;
use anyhow::{Context, Result};
use olal_config::Config;
use olal_core::ItemType;
use olal_ollama::{GenerateOptions, GenerateRequest, OllamaClient};
use colored::Colorize;
use tokio::runtime::Runtime;

/// A suggested clip from the content.
#[derive(Debug)]
pub struct ClipSuggestion {
    pub start_time: f64,
    pub end_time: f64,
    pub title: String,
    pub reason: String,
}

/// Run the clips command.
pub fn run(
    item_id: &str,
    count: usize,
    min_duration: u32,
    max_duration: u32,
    model: Option<String>,
) -> Result<()> {
    let db = get_database()?;
    let config = Config::load().context("Failed to load configuration")?;

    // Get the item
    let item = db.get_item_by_prefix(item_id)?;

    // Check if it's a video or audio (has timestamps)
    if item.item_type != ItemType::Video && item.item_type != ItemType::Audio {
        anyhow::bail!(
            "Clip detection requires video or audio content with timestamps. \
             Item '{}' is type '{}'.",
            item.title,
            item.item_type
        );
    }

    // Get chunks with timestamps
    let chunks = db.get_chunks_by_item(&item.id)?;

    // Check if we have timestamps
    let has_timestamps = chunks.iter().any(|c| c.start_time.is_some());
    if !has_timestamps {
        anyhow::bail!(
            "Item '{}' has no timestamp data. \
             The content may not have been fully processed.",
            item.title
        );
    }

    // Create Ollama client
    let client = OllamaClient::from_config(&config.ollama)
        .context("Failed to create Ollama client")?;

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

    println!(
        "{} {}",
        "Analyzing:".cyan().bold(),
        item.title
    );
    println!("{}", "─".repeat(70));
    println!(
        "Looking for {} engaging clips ({}-{}s each)...",
        count, min_duration, max_duration
    );
    println!();

    // Build timestamped content for the prompt
    let mut timestamped_content = String::new();
    for chunk in &chunks {
        if let (Some(start), Some(end)) = (chunk.start_time, chunk.end_time) {
            timestamped_content.push_str(&format!(
                "[{:.1}s - {:.1}s]: {}\n",
                start, end, chunk.content
            ));
        }
    }

    // Truncate if too long
    let content_for_prompt = if timestamped_content.len() > 6000 {
        format!("{}...", &timestamped_content[..6000])
    } else {
        timestamped_content
    };

    // Build prompt
    let prompt = format!(
        r#"Analyze the following timestamped transcript and identify {} engaging clips that would work well as short-form content (like YouTube Shorts or TikTok).

Requirements:
- Each clip should be between {} and {} seconds
- Look for: surprising moments, key insights, funny/entertaining moments, controversial statements, quotable phrases, dramatic reveals
- Provide start and end timestamps that capture complete thoughts
- Give each clip a catchy title

Transcript:
{}

Respond in this exact format for each clip (one per line):
CLIP: [start_seconds]-[end_seconds] | Title: [catchy title] | Reason: [why this is engaging]

Example:
CLIP: 45.0-75.0 | Title: The Shocking Truth About AI | Reason: Speaker reveals unexpected insight that challenges common assumptions"#,
        count, min_duration, max_duration, content_for_prompt
    );

    let request = GenerateRequest::new(model_name, prompt)
        .with_options(GenerateOptions::new().with_temperature(0.7).with_num_predict(1000));

    let response = rt
        .block_on(client.generate(request))
        .context("Failed to generate clip suggestions")?;

    // Parse the response
    let suggestions = parse_clip_response(&response.response);

    if suggestions.is_empty() {
        println!(
            "{}",
            "No suitable clips found. Try adjusting duration parameters.".yellow()
        );
        return Ok(());
    }

    // Display results
    println!("{} Suggested Clips:", suggestions.len().to_string().green());
    println!();

    for (i, clip) in suggestions.iter().enumerate() {
        println!(
            "{}. {} {}",
            (i + 1).to_string().cyan(),
            clip.title.white().bold(),
            format!(
                "[{} - {}]",
                format_time(clip.start_time),
                format_time(clip.end_time)
            )
            .dimmed()
        );
        println!(
            "   {} {:.1}s",
            "Duration:".dimmed(),
            clip.end_time - clip.start_time
        );
        println!("   {} {}", "Why:".dimmed(), clip.reason);
        println!();
    }

    // Show ffmpeg commands
    println!("{}", "─".repeat(70));
    println!("{}", "FFmpeg Commands:".cyan().bold());
    println!();

    let source = item.source_path.as_deref().unwrap_or("<source_file>");
    for (i, clip) in suggestions.iter().enumerate() {
        let output_name = clip
            .title
            .to_lowercase()
            .replace(' ', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .take(30)
            .collect::<String>();

        println!(
            "# Clip {}: {}",
            i + 1,
            clip.title
        );
        println!(
            "ffmpeg -i \"{}\" -ss {:.1} -t {:.1} -c copy \"clip_{}_{}.mp4\"",
            source,
            clip.start_time,
            clip.end_time - clip.start_time,
            i + 1,
            output_name
        );
        println!();
    }

    Ok(())
}

/// Parse the AI response into clip suggestions.
fn parse_clip_response(response: &str) -> Vec<ClipSuggestion> {
    let mut suggestions = Vec::new();

    for line in response.lines() {
        let line = line.trim();
        if !line.starts_with("CLIP:") {
            continue;
        }

        // Parse: CLIP: [start]-[end] | Title: [title] | Reason: [reason]
        let parts: Vec<&str> = line[5..].split('|').collect();
        if parts.len() < 3 {
            continue;
        }

        // Parse timestamps
        let time_part = parts[0].trim();
        let times: Vec<&str> = time_part.split('-').collect();
        if times.len() != 2 {
            continue;
        }

        let start_time = times[0].trim().parse::<f64>().ok();
        let end_time = times[1].trim().parse::<f64>().ok();

        if let (Some(start), Some(end)) = (start_time, end_time) {
            // Parse title
            let title = parts[1]
                .trim()
                .strip_prefix("Title:")
                .unwrap_or(parts[1])
                .trim()
                .to_string();

            // Parse reason
            let reason = parts[2]
                .trim()
                .strip_prefix("Reason:")
                .unwrap_or(parts[2])
                .trim()
                .to_string();

            suggestions.push(ClipSuggestion {
                start_time: start,
                end_time: end,
                title,
                reason,
            });
        }
    }

    suggestions
}

/// Format seconds as MM:SS.
fn format_time(seconds: f64) -> String {
    let mins = (seconds / 60.0) as u32;
    let secs = (seconds % 60.0) as u32;
    format!("{:02}:{:02}", mins, secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clip_response() {
        let response = r#"
CLIP: 45.0-75.0 | Title: The Shocking Truth | Reason: Great insight
CLIP: 120.5-150.0 | Title: Funny Moment | Reason: Entertaining
Invalid line
CLIP: bad format
"#;

        let clips = parse_clip_response(response);
        assert_eq!(clips.len(), 2);
        assert_eq!(clips[0].start_time, 45.0);
        assert_eq!(clips[0].end_time, 75.0);
        assert_eq!(clips[0].title, "The Shocking Truth");
    }

    #[test]
    fn test_format_time() {
        assert_eq!(format_time(0.0), "00:00");
        assert_eq!(format_time(65.0), "01:05");
        assert_eq!(format_time(3661.0), "61:01");
    }
}
