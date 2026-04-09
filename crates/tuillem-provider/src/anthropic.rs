use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;

use crate::{
    ChatRequest, ChatResponseStream, ModelInfo, Provider, ProviderError, StreamDelta,
};

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    models: Vec<String>,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, models: Vec<String>) -> Self {
        let models = if models.is_empty() {
            vec!["claude-sonnet-4-20250514".to_string()]
        } else {
            models
        };
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            models,
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn send(&self, request: ChatRequest) -> Result<ChatResponseStream, ProviderError> {
        let mut messages = Vec::new();
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
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(ref system) = request.system {
            body["system"] = serde_json::json!(system);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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

        let stream = response.bytes_stream();

        let mapped = stream
            .map(|chunk| {
                let chunk = chunk.map_err(ProviderError::Http)?;
                let text = String::from_utf8_lossy(&chunk);
                let mut deltas = Vec::new();

                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            deltas.push(StreamDelta::Done);
                            continue;
                        }
                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                            let event_type = event.get("type").and_then(|t| t.as_str());
                            match event_type {
                                Some("content_block_delta") => {
                                    if let Some(delta) = event.get("delta") {
                                        let delta_type =
                                            delta.get("type").and_then(|t| t.as_str());
                                        match delta_type {
                                            Some("text_delta") => {
                                                if let Some(text) =
                                                    delta.get("text").and_then(|t| t.as_str())
                                                {
                                                    deltas
                                                        .push(StreamDelta::Text(text.to_string()));
                                                }
                                            }
                                            Some("thinking_delta") => {
                                                if let Some(thinking) = delta
                                                    .get("thinking")
                                                    .and_then(|t| t.as_str())
                                                {
                                                    deltas.push(StreamDelta::Thinking(
                                                        thinking.to_string(),
                                                    ));
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Some("message_delta") => {
                                    if let Some(usage) = event.get("usage") {
                                        let input = usage
                                            .get("input_tokens")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);
                                        let output = usage
                                            .get("output_tokens")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);
                                        deltas.push(StreamDelta::Usage {
                                            input_tokens: input,
                                            output_tokens: output,
                                        });
                                    }
                                }
                                Some("message_stop") => {
                                    deltas.push(StreamDelta::Done);
                                }
                                _ => {}
                            }
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
                supports_thinking: true,
                context_window: Some(200_000),
            })
            .collect()
    }

    fn name(&self) -> &str {
        "anthropic"
    }
}
