//! Types for Ollama API requests and responses.

use serde::{Deserialize, Serialize};

/// Information about an available model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub digest: String,
    #[serde(default)]
    pub modified_at: Option<String>,
}

/// Response from the /api/tags endpoint (list models).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    pub models: Vec<ModelInfo>,
}

/// Request body for /api/embeddings endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub prompt: String,
}

/// Response from /api/embeddings endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
}

/// Request body for /api/generate endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<i32>>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerateOptions>,
}

impl GenerateRequest {
    /// Create a new generation request.
    pub fn new(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: prompt.into(),
            system: None,
            template: None,
            context: None,
            stream: false,
            options: None,
        }
    }

    /// Set the system prompt.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Enable streaming mode.
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Set generation options.
    pub fn with_options(mut self, options: GenerateOptions) -> Self {
        self.options = Some(options);
        self
    }
}

/// Options for text generation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerateOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
}

impl GenerateOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn with_num_predict(mut self, num_predict: i32) -> Self {
        self.num_predict = Some(num_predict);
        self
    }
}

/// Response from /api/generate endpoint (non-streaming).
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResponse {
    pub model: String,
    pub response: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub context: Option<Vec<i32>>,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub load_duration: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<u32>,
    #[serde(default)]
    pub prompt_eval_duration: Option<u64>,
    #[serde(default)]
    pub eval_count: Option<u32>,
    #[serde(default)]
    pub eval_duration: Option<u64>,
}

/// Streaming response chunk from /api/generate endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamChunk {
    pub model: String,
    pub response: String,
    #[serde(default)]
    pub done: bool,
}

/// Error response from Ollama API.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
