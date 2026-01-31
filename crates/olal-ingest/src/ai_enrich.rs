//! AI-based enrichment for items (summarization, auto-tagging).

use olal_config::Config;
use olal_db::Database;
use olal_ollama::{GenerateOptions, GenerateRequest, OllamaClient};
use tokio::runtime::Runtime;
use tracing::{debug, info, warn};

/// AI enricher for generating summaries and suggesting tags.
pub struct AiEnricher {
    client: OllamaClient,
    model: String,
    rt: Runtime,
}

impl AiEnricher {
    /// Create a new AI enricher from config.
    pub fn from_config(config: &Config) -> Result<Self, String> {
        let client = OllamaClient::from_config(&config.ollama)
            .map_err(|e| format!("Failed to create Ollama client: {}", e))?;

        let rt = Runtime::new().map_err(|e| format!("Failed to create async runtime: {}", e))?;

        // Check if Ollama is available
        let is_available = rt.block_on(client.is_available());
        if !is_available {
            return Err(format!(
                "Ollama is not running at {}",
                config.ollama.host
            ));
        }

        Ok(Self {
            client,
            model: config.ollama.model.clone(),
            rt,
        })
    }

    /// Generate a summary for the given content.
    pub fn generate_summary(&self, content: &str) -> Result<String, String> {
        // Truncate content if too long (aim for ~4000 chars to leave room for prompt)
        let truncated = if content.len() > 4000 {
            format!("{}...", &content[..4000])
        } else {
            content.to_string()
        };

        let prompt = format!(
            "Summarize the following content in 2-3 concise sentences. Focus on the main topics and key points. Do not include any preamble like 'Here is a summary' - just provide the summary directly.\n\nContent:\n{}",
            truncated
        );

        let request = GenerateRequest::new(&self.model, prompt)
            .with_options(GenerateOptions::new().with_temperature(0.3).with_num_predict(200));

        let response = self
            .rt
            .block_on(self.client.generate(request))
            .map_err(|e| format!("Failed to generate summary: {}", e))?;

        let summary = response.response.trim().to_string();
        debug!("Generated summary: {} chars", summary.len());

        Ok(summary)
    }

    /// Suggest tags for the given content.
    pub fn suggest_tags(&self, content: &str, title: &str) -> Result<Vec<String>, String> {
        // Truncate content if too long
        let truncated = if content.len() > 3000 {
            format!("{}...", &content[..3000])
        } else {
            content.to_string()
        };

        let prompt = format!(
            "Based on the following content, suggest 3-5 relevant tags (single words or short phrases) that categorize this content. Return only the tags, one per line, without numbers or bullets.\n\nTitle: {}\n\nContent:\n{}",
            title,
            truncated
        );

        let request = GenerateRequest::new(&self.model, prompt)
            .with_options(GenerateOptions::new().with_temperature(0.5).with_num_predict(100));

        let response = self
            .rt
            .block_on(self.client.generate(request))
            .map_err(|e| format!("Failed to suggest tags: {}", e))?;

        // Parse the response into tags
        let tags: Vec<String> = response
            .response
            .lines()
            .map(|line| {
                // Clean up the line (remove bullets, numbers, etc.)
                line.trim()
                    .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == '-' || c == '*')
                    .trim()
                    .to_lowercase()
            })
            .filter(|tag| !tag.is_empty() && tag.len() < 50)
            .take(5)
            .collect();

        debug!("Suggested tags: {:?}", tags);

        Ok(tags)
    }
}

/// Enrich an item with AI-generated summary and tags.
///
/// This function will:
/// 1. Generate a summary if `config.processing.generate_summary` is true
/// 2. Suggest and apply tags if `config.processing.auto_tag` is true
///
/// Errors are logged but don't cause the function to fail (graceful degradation).
pub fn enrich_item(
    db: &Database,
    item: &mut olal_core::Item,
    content: &str,
    config: &Config,
) -> Result<(), String> {
    // Skip if content is too short
    if content.len() < 100 {
        debug!("Content too short for AI enrichment");
        return Ok(());
    }

    // Create enricher
    let enricher = match AiEnricher::from_config(config) {
        Ok(e) => e,
        Err(e) => {
            warn!("AI enrichment unavailable: {}", e);
            return Err(e);
        }
    };

    info!("Enriching item {} with AI", item.id);

    // Generate summary if enabled and not already present
    if config.processing.generate_summary && item.summary.is_none() {
        match enricher.generate_summary(content) {
            Ok(summary) => {
                item.summary = Some(summary);
                if let Err(e) = db.update_item(item) {
                    warn!("Failed to save summary: {}", e);
                } else {
                    info!("Generated summary for item {}", item.id);
                }
            }
            Err(e) => {
                warn!("Failed to generate summary: {}", e);
            }
        }
    }

    // Auto-tag if enabled
    if config.processing.auto_tag {
        match enricher.suggest_tags(content, &item.title) {
            Ok(tags) => {
                for tag_name in tags {
                    if let Err(e) = db.tag_item(&item.id, &tag_name) {
                        warn!("Failed to add tag '{}': {}", tag_name, e);
                    } else {
                        debug!("Added tag '{}' to item {}", tag_name, item.id);
                    }
                }
                info!("Auto-tagged item {}", item.id);
            }
            Err(e) => {
                warn!("Failed to suggest tags: {}", e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_parsing() {
        // Test that tag parsing handles various formats
        let response = "1. rust\n- programming\n* software\ncoding\n  technology  ";
        let tags: Vec<String> = response
            .lines()
            .map(|line| {
                line.trim()
                    .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == '-' || c == '*')
                    .trim()
                    .to_lowercase()
            })
            .filter(|tag| !tag.is_empty() && tag.len() < 50)
            .take(5)
            .collect();

        assert_eq!(tags, vec!["rust", "programming", "software", "coding", "technology"]);
    }
}
