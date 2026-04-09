pub mod anthropic;
pub mod ollama;
pub mod openai;

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use tuillem_config::{ProviderConfig, ProviderType};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error (status {status}): {message}")]
    Api { status: u16, message: String },

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Config error: {0}")]
    Config(String),
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum StreamDelta {
    Text(String),
    Thinking(String),
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallDelta(String),
    ToolCallEnd,
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    Done,
}

pub type ChatResponseStream =
    Pin<Box<dyn Stream<Item = Result<StreamDelta, ProviderError>> + Send>>;

// ---------------------------------------------------------------------------
// Model info
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub supports_streaming: bool,
    pub supports_thinking: bool,
    pub context_window: Option<u64>,
}

// ---------------------------------------------------------------------------
// Provider trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait Provider: Send + Sync {
    async fn send(&self, request: ChatRequest) -> Result<ChatResponseStream, ProviderError>;
    fn models(&self) -> Vec<ModelInfo>;
    fn name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn Provider>, ProviderError> {
    match config.provider_type {
        ProviderType::Anthropic => {
            let api_key = config
                .api_key
                .as_deref()
                .ok_or_else(|| ProviderError::Config("Anthropic requires an api_key".into()))?;
            Ok(Box::new(anthropic::AnthropicProvider::new(
                api_key,
                config.models.clone(),
            )))
        }
        ProviderType::Openai => {
            let api_key = config
                .api_key
                .as_deref()
                .ok_or_else(|| ProviderError::Config("OpenAI requires an api_key".into()))?;
            let base_url = config
                .base_url
                .as_deref()
                .unwrap_or("https://api.openai.com/v1");
            Ok(Box::new(openai::OpenAiProvider::new(
                "openai",
                api_key,
                base_url,
                config.models.clone(),
            )))
        }
        ProviderType::Openrouter => {
            let api_key = config
                .api_key
                .as_deref()
                .ok_or_else(|| ProviderError::Config("OpenRouter requires an api_key".into()))?;
            let base_url = config
                .base_url
                .as_deref()
                .unwrap_or("https://openrouter.ai/api/v1");
            Ok(Box::new(openai::OpenAiProvider::new(
                "openrouter",
                api_key,
                base_url,
                config.models.clone(),
            )))
        }
        ProviderType::Ollama => {
            let base_url = config
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:11434");
            Ok(Box::new(ollama::OllamaProvider::new(
                base_url,
                config.models.clone(),
            )))
        }
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
