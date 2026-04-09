use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;

use crate::{
    ChatRequest, ChatResponseStream, ModelInfo, Provider, ProviderError, StreamDelta,
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

        let mapped = stream
            .map(|chunk| {
                let chunk = chunk.map_err(ProviderError::Http)?;
                let text = String::from_utf8_lossy(&chunk);
                let mut deltas = Vec::new();

                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) {
                        // Check if done
                        let done = obj.get("done").and_then(|d| d.as_bool()).unwrap_or(false);

                        // Extract message content
                        if let Some(message) = obj.get("message") {
                            if let Some(content) =
                                message.get("content").and_then(|c| c.as_str())
                            {
                                if !content.is_empty() {
                                    deltas.push(StreamDelta::Text(content.to_string()));
                                }
                            }
                        }

                        if done {
                            // Extract usage if present
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
                    }
                }

                Ok(deltas)
            })
            .flat_map(|result: Result<Vec<StreamDelta>, ProviderError>| {
                let items: Vec<Result<StreamDelta, ProviderError>> = match result {
                    Ok(deltas) => deltas.into_iter().map(Ok).collect(),
                    Err(e) => vec![Err(e)],
                };
                futures::stream::iter(items)
            });

        Ok(Box::pin(mapped))
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
