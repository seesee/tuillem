use async_trait::async_trait;
use reqwest::Client;

use crate::{
    ChatRequest, ChatResponseStream, ModelInfo, Provider, ProviderError, StreamDelta,
    buffered_sse_stream,
};

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    models: Vec<String>,
}

impl OllamaProvider {
    pub fn new(base_url: &str, models: Vec<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            models,
        }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    async fn send(&self, request: ChatRequest) -> Result<ChatResponseStream, ProviderError> {
        let mut messages = Vec::new();

        // Prepend system message if present.
        if let Some(ref system) = request.system {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system,
            }));
        }

        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content,
            }));
        }

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(temp) = request.temperature {
            body["options"] = serde_json::json!({ "temperature": temp });
        }

        let url = format!("{}/api/chat", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .body(serde_json::to_string(&body).map_err(|e| ProviderError::Stream(e.to_string()))?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".into());
            return Err(ProviderError::Api { status, message });
        }

        // Ollama uses newline-delimited JSON (not SSE).
        let stream = response.bytes_stream();

        Ok(buffered_sse_stream(stream, parse_ollama_line))
    }

    fn models(&self) -> Vec<ModelInfo> {
        self.models
            .iter()
            .map(|id| ModelInfo {
                id: id.clone(),
                name: id.clone(),
                supports_streaming: true,
                supports_thinking: false,
                context_window: None,
            })
            .collect()
    }

    fn name(&self) -> &str {
        "ollama"
    }
}

fn parse_ollama_line(line: &str) -> Vec<StreamDelta> {
    // Track <think> block state via thread-local (safe since each stream runs on one task)
    thread_local! {
        static IN_THINK_BLOCK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    let mut deltas = Vec::new();
    let line = line.trim();
    if line.is_empty() {
        return deltas;
    }
    let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) else {
        return deltas;
    };
    let done = obj.get("done").and_then(|d| d.as_bool()).unwrap_or(false);

    if let Some(message) = obj.get("message")
        && let Some(content) = message.get("content").and_then(|c| c.as_str())
        && !content.is_empty()
    {
        // Parse <think>...</think> tags used by reasoning models (DeepSeek, etc.)
        // Always strip tags to prevent them leaking into text; only emit when enabled
        let mut remaining = content;
        while !remaining.is_empty() {
            let in_think = IN_THINK_BLOCK.with(|c| c.get());
            if in_think {
                if let Some(end_pos) = remaining.find("</think>") {
                    let thinking = &remaining[..end_pos];
                    if !thinking.is_empty() {
                        deltas.push(StreamDelta::Thinking(thinking.to_string()));
                    }
                    IN_THINK_BLOCK.with(|c| c.set(false));
                    remaining = &remaining[end_pos + 8..];
                } else {
                    deltas.push(StreamDelta::Thinking(remaining.to_string()));
                    remaining = "";
                }
            } else if let Some(start_pos) = remaining.find("<think>") {
                let text = &remaining[..start_pos];
                if !text.is_empty() {
                    deltas.push(StreamDelta::Text(text.to_string()));
                }
                IN_THINK_BLOCK.with(|c| c.set(true));
                remaining = &remaining[start_pos + 7..];
            } else {
                deltas.push(StreamDelta::Text(remaining.to_string()));
                remaining = "";
            }
        }
    }

    if done {
        IN_THINK_BLOCK.with(|c| c.set(false));
        if let (Some(prompt), Some(eval)) = (
            obj.get("prompt_eval_count").and_then(|v| v.as_u64()),
            obj.get("eval_count").and_then(|v| v.as_u64()),
        ) {
            deltas.push(StreamDelta::Usage {
                input_tokens: prompt,
                output_tokens: eval,
            });
        }
        deltas.push(StreamDelta::Done);
    }
    deltas
}
