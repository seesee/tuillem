pub mod anthropic;
pub mod ollama;
pub mod openai;

use std::pin::Pin;
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::Stream;
use pin_project_lite::pin_project;
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

// ---------------------------------------------------------------------------
// Buffered SSE stream helper
// ---------------------------------------------------------------------------

pin_project! {
    /// Wraps a raw byte stream and buffers incomplete lines across chunk boundaries.
    /// Complete lines are passed to `parse_line` which returns zero or more `StreamDelta`s.
    struct BufferedSseStream<S, F> {
        #[pin]
        inner: S,
        parse_line: F,
        buffer: String,
        pending: Vec<Result<StreamDelta, ProviderError>>,
    }
}

impl<S, F> Stream for BufferedSseStream<S, F>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>>,
    F: Fn(&str) -> Vec<StreamDelta>,
{
    type Item = Result<StreamDelta, ProviderError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Drain any pending deltas first.
        if let Some(item) = this.pending.pop() {
            return Poll::Ready(Some(item));
        }

        loop {
            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    this.buffer.push_str(&String::from_utf8_lossy(&bytes));

                    // Process all complete lines (split on '\n').
                    while let Some(newline_pos) = this.buffer.find('\n') {
                        let line: String = this.buffer.drain(..=newline_pos).collect();
                        let line = line.trim_end_matches('\n').trim_end_matches('\r');
                        if line.is_empty() {
                            continue;
                        }
                        let deltas = (this.parse_line)(line);
                        for d in deltas {
                            this.pending.push(Ok(d));
                        }
                    }

                    // Reverse so we can pop from the end in order.
                    this.pending.reverse();

                    if let Some(item) = this.pending.pop() {
                        return Poll::Ready(Some(item));
                    }
                    // No complete lines yet — poll for more bytes.
                    continue;
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(ProviderError::Http(e))));
                }
                Poll::Ready(None) => {
                    // Stream ended — flush any remaining buffer content.
                    if !this.buffer.is_empty() {
                        let remaining = this.buffer.drain(..).collect::<String>();
                        let line = remaining.trim();
                        if !line.is_empty() {
                            let deltas = (this.parse_line)(line);
                            for d in deltas {
                                this.pending.push(Ok(d));
                            }
                            this.pending.reverse();
                            if let Some(item) = this.pending.pop() {
                                return Poll::Ready(Some(item));
                            }
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Buffer SSE byte chunks into complete lines, yielding `StreamDelta`s.
///
/// `parse_line` converts a single complete line into zero or more `StreamDelta`s.
/// Incomplete lines that span chunk boundaries are buffered until the next chunk
/// arrives with the terminating newline.
pub fn buffered_sse_stream(
    byte_stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    parse_line: impl Fn(&str) -> Vec<StreamDelta> + Send + 'static,
) -> ChatResponseStream {
    Box::pin(BufferedSseStream {
        inner: byte_stream,
        parse_line,
        buffer: String::new(),
        pending: Vec::new(),
    })
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
