//! Ollama HTTP client.

use crate::error::{OllamaError, OllamaResult};
use crate::types::*;
use olal_config::OllamaConfig;
use futures_util::StreamExt;
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Client for interacting with Ollama's API.
#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    host: String,
    timeout: Duration,
}

impl OllamaClient {
    /// Create a new client from configuration.
    pub fn from_config(config: &OllamaConfig) -> OllamaResult<Self> {
        let timeout = Duration::from_secs(config.timeout_seconds);

        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(OllamaError::Http)?;

        Ok(Self {
            client,
            host: config.host.trim_end_matches('/').to_string(),
            timeout,
        })
    }

    /// Create a new client with default settings.
    pub fn new(host: impl Into<String>) -> OllamaResult<Self> {
        let host = host.into();
        let timeout = Duration::from_secs(120);

        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(OllamaError::Http)?;

        Ok(Self {
            client,
            host: host.trim_end_matches('/').to_string(),
            timeout,
        })
    }

    /// Check if Ollama server is available.
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.host);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// List all available models.
    pub async fn list_models(&self) -> OllamaResult<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.host);
        debug!("Listing models from {}", url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            if e.is_connect() {
                OllamaError::ServerNotRunning {
                    host: self.host.clone(),
                }
            } else if e.is_timeout() {
                OllamaError::Timeout {
                    seconds: self.timeout.as_secs(),
                }
            } else {
                OllamaError::Http(e)
            }
        })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(OllamaError::ApiError {
                status,
                message: text,
            });
        }

        let list: ListModelsResponse = response.json().await?;
        Ok(list.models)
    }

    /// Check if a specific model is available.
    pub async fn has_model(&self, model: &str) -> OllamaResult<bool> {
        let models = self.list_models().await?;
        // Check both exact match and model without tag
        Ok(models.iter().any(|m| {
            m.name == model || m.name.starts_with(&format!("{}:", model))
        }))
    }

    /// Generate embeddings for text.
    pub async fn embed(&self, model: &str, text: &str) -> OllamaResult<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.host);
        debug!("Generating embedding with model {} for text length {}", model, text.len());

        let request = EmbeddingRequest {
            model: model.to_string(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ServerNotRunning {
                        host: self.host.clone(),
                    }
                } else if e.is_timeout() {
                    OllamaError::Timeout {
                        seconds: self.timeout.as_secs(),
                    }
                } else {
                    OllamaError::Http(e)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();

            // Check for model not found
            if text.contains("not found") || status.as_u16() == 404 {
                return Err(OllamaError::ModelNotFound {
                    model: model.to_string(),
                });
            }

            return Err(OllamaError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let embedding_response: EmbeddingResponse = response.json().await?;
        info!("Generated embedding with {} dimensions", embedding_response.embedding.len());

        Ok(embedding_response.embedding)
    }

    /// Generate embeddings for multiple texts (batched).
    pub async fn embed_batch(
        &self,
        model: &str,
        texts: &[String],
    ) -> OllamaResult<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());

        for text in texts {
            let embedding = self.embed(model, text).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    /// Generate text (non-streaming).
    pub async fn generate(&self, request: GenerateRequest) -> OllamaResult<GenerateResponse> {
        let url = format!("{}/api/generate", self.host);
        debug!("Generating with model {}", request.model);

        // Ensure streaming is off for this method
        let mut request = request;
        request.stream = false;

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ServerNotRunning {
                        host: self.host.clone(),
                    }
                } else if e.is_timeout() {
                    OllamaError::Timeout {
                        seconds: self.timeout.as_secs(),
                    }
                } else {
                    OllamaError::Http(e)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();

            if text.contains("not found") || status.as_u16() == 404 {
                return Err(OllamaError::ModelNotFound {
                    model: request.model,
                });
            }

            return Err(OllamaError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let generate_response: GenerateResponse = response.json().await?;
        Ok(generate_response)
    }

    /// Generate text with streaming.
    /// Returns a channel receiver that yields response chunks.
    pub async fn generate_stream(
        &self,
        request: GenerateRequest,
    ) -> OllamaResult<mpsc::Receiver<String>> {
        let url = format!("{}/api/generate", self.host);
        debug!("Starting streaming generation with model {}", request.model);

        // Ensure streaming is on
        let mut request = request;
        request.stream = true;

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ServerNotRunning {
                        host: self.host.clone(),
                    }
                } else if e.is_timeout() {
                    OllamaError::Timeout {
                        seconds: self.timeout.as_secs(),
                    }
                } else {
                    OllamaError::Http(e)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();

            if text.contains("not found") || status.as_u16() == 404 {
                return Err(OllamaError::ModelNotFound {
                    model: request.model,
                });
            }

            return Err(OllamaError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let (tx, rx) = mpsc::channel(100);

        // Spawn task to read stream
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        // Each chunk is a JSON line
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            if line.is_empty() {
                                continue;
                            }

                            match serde_json::from_str::<StreamChunk>(line) {
                                Ok(chunk) => {
                                    if !chunk.response.is_empty() {
                                        if tx.send(chunk.response).await.is_err() {
                                            return; // Receiver dropped
                                        }
                                    }
                                    if chunk.done {
                                        return;
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to parse stream chunk: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Stream error: {}", e);
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = OllamaConfig::default();
        let client = OllamaClient::from_config(&config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_generate_request_builder() {
        let request = GenerateRequest::new("gpt-oss:20b", "Hello, world!")
            .with_system("You are a helpful assistant.")
            .with_options(GenerateOptions::new().with_temperature(0.7));

        assert_eq!(request.model, "gpt-oss:20b");
        assert_eq!(request.prompt, "Hello, world!");
        assert!(request.system.is_some());
        assert!(request.options.is_some());
    }
}
