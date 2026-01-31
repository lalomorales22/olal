//! Digest command - Generate AI summaries of content ingested over a time period.

use super::get_database;
use anyhow::{Context, Result};
use olal_config::Config;
use olal_ollama::{GenerateOptions, GenerateRequest, OllamaClient};
use chrono::{Duration, NaiveDate, Utc};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::runtime::Runtime;

/// Time period for digest generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestPeriod {
    Day,
    Week,
    Month,
}

impl DigestPeriod {
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "day" | "daily" => Some(Self::Day),
            "week" | "weekly" => Some(Self::Week),
            "month" | "monthly" => Some(Self::Month),
            _ => None,
        }
    }

    /// Get the duration for this period.
    pub fn duration(&self) -> Duration {
        match self {
            Self::Day => Duration::days(1),
            Self::Week => Duration::weeks(1),
            Self::Month => Duration::days(30),
        }
    }

    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Day => "daily",
            Self::Week => "weekly",
            Self::Month => "monthly",
        }
    }
}

/// Run the digest command.
pub fn run(
    period: &str,
    since: Option<String>,
    output: Option<PathBuf>,
    model: Option<String>,
) -> Result<()> {
    let db = get_database()?;
    let config = Config::load().context("Failed to load configuration")?;

    // Determine start date
    let start_date = if let Some(ref date_str) = since {
        // Parse custom date
        let parsed = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .context("Invalid date format. Use YYYY-MM-DD.")?;
        parsed.and_hms_opt(0, 0, 0).unwrap().and_utc()
    } else {
        // Use period
        let digest_period = DigestPeriod::from_str(period).unwrap_or(DigestPeriod::Day);
        Utc::now() - digest_period.duration()
    };

    let period_desc = if since.is_some() {
        format!("since {}", since.as_ref().unwrap())
    } else {
        DigestPeriod::from_str(period)
            .unwrap_or(DigestPeriod::Day)
            .description()
            .to_string()
    };

    println!(
        "{} {}",
        "Generating".cyan().bold(),
        format!("{} digest", period_desc).white()
    );
    println!(
        "{} {}",
        "Period:".cyan(),
        format!("{} to now", start_date.format("%Y-%m-%d %H:%M UTC"))
    );
    println!("{}", "─".repeat(70));
    println!();

    // Query items
    let items = db
        .items_since(start_date)
        .context("Failed to query items")?;

    if items.is_empty() {
        println!(
            "{} No items found for this time period.",
            "Note:".yellow()
        );
        println!();
        println!("Suggestions:");
        println!("  - Try a longer time period (--period week or --period month)");
        println!("  - Ingest some content first with 'olal ingest <path>'");
        return Ok(());
    }

    println!(
        "{} {} items",
        "Found:".cyan(),
        items.len().to_string().green()
    );

    // Group items by type (use string keys since ItemType doesn't impl Hash)
    let mut by_type: HashMap<&str, Vec<_>> = HashMap::new();
    for item in &items {
        by_type
            .entry(item.item_type.as_str())
            .or_default()
            .push(item);
    }

    // Show breakdown
    for (item_type, type_items) in &by_type {
        println!(
            "  {} {} {}",
            "•".dimmed(),
            type_items.len(),
            item_type
        );
    }
    println!();

    // Collect summaries and excerpts
    let mut content_parts: Vec<String> = Vec::new();

    for item in &items {
        let mut item_content = format!("## {} ({})\n", item.title, item.item_type.as_str());

        // Add summary if available
        if let Some(ref summary) = item.summary {
            item_content.push_str(&format!("Summary: {}\n", summary));
        }

        // Get first chunk for excerpt
        if let Ok(chunks) = db.get_chunks_by_item(&item.id) {
            if let Some(first_chunk) = chunks.first() {
                let excerpt = if first_chunk.content.len() > 300 {
                    format!("{}...", &first_chunk.content[..300])
                } else {
                    first_chunk.content.clone()
                };
                item_content.push_str(&format!("Excerpt: {}\n", excerpt));
            }
        }

        content_parts.push(item_content);
    }

    let combined_content = content_parts.join("\n---\n\n");

    // Truncate if too long
    let combined_content = if combined_content.len() > 12000 {
        format!(
            "{}...\n[Content truncated - {} items total]",
            &combined_content[..12000],
            items.len()
        )
    } else {
        combined_content
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

    // Generate digest
    print!("{}", "Generating digest...".dimmed());
    io::stdout().flush()?;

    let digest = generate_digest(&rt, &client, model_name, &combined_content, &period_desc)?;

    println!("\r{}", " ".repeat(30));
    println!();

    // Output
    if let Some(ref output_path) = output {
        // Write to file
        let markdown = format_digest_markdown(&digest, &period_desc, items.len());
        fs::write(output_path, &markdown).context("Failed to write output file")?;
        println!(
            "{} {}",
            "Saved to:".green().bold(),
            output_path.display()
        );
    } else {
        // Display to stdout
        println!("{}", "Digest:".green().bold());
        println!();
        println!("{}", digest);
    }

    Ok(())
}

fn generate_digest(
    rt: &Runtime,
    client: &OllamaClient,
    model: &str,
    content: &str,
    period_desc: &str,
) -> Result<String> {
    let prompt = format!(
        r#"Generate a {} digest/summary of the following content that was ingested into a personal knowledge base.

Structure your response as:
1. **Overview** - A brief paragraph summarizing the key themes
2. **Key Items** - The most notable pieces of content (3-5 bullet points)
3. **Insights** - Connections or patterns you notice across the content
4. **Action Items** - Suggested next steps or things to revisit (if applicable)

Be concise but informative. Focus on what's most valuable to remember.

Content:
{}

Generate the digest now:"#,
        period_desc, content
    );

    let request = GenerateRequest::new(model, &prompt)
        .with_options(GenerateOptions::new().with_temperature(0.7));

    let response = rt.block_on(client.generate(request)).map_err(|e| {
        anyhow::anyhow!("Failed to generate digest: {}", e)
    })?;

    Ok(response.response.trim().to_string())
}

fn format_digest_markdown(digest: &str, period_desc: &str, item_count: usize) -> String {
    let now = Utc::now();
    format!(
        r#"# {} Digest

*Generated: {}*
*Items processed: {}*

---

{}

---

*Generated by Olal*
"#,
        period_desc.chars().next().unwrap().to_uppercase().to_string() + &period_desc[1..],
        now.format("%Y-%m-%d %H:%M UTC"),
        item_count,
        digest
    )
}
