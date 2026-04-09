use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;

use crate::{ChatRequest, ChatResponseStream, ModelInfo, Provider, ProviderError, StreamDelta};

pub struct OpenAiProvider {
    provider_name: String,
    client: Client,
    api_key: String,
    base_url: String,
    models: Vec<String>,
}

impl OpenAiProvider {
    pub fn new(name: &str, api_key: &str, base_url: &str, models: Vec<String>) -> Self {
        Self {
            provider_name: name.to_string(),
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            models,
        }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
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

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
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
                            if let Some(choices) = event.get("choices").and_then(|c| c.as_array())
                                && let Some(choice) = choices.first()
                            {
                                if let Some(delta) = choice.get("delta")
                                    && let Some(content) =
                                        delta.get("content").and_then(|c| c.as_str())
                                    && !content.is_empty()
                                {
                                    deltas.push(StreamDelta::Text(content.to_string()));
                                }
                                // Check for finish_reason
                                if let Some(finish) =
                                    choice.get("finish_reason").and_then(|f| f.as_str())
                                    && finish == "stop"
                                {
                                    deltas.push(StreamDelta::Done);
                                }
                            }
                            // Check for usage in the event
                            if let Some(usage) = event.get("usage") {
                                let input = usage
                                    .get("prompt_tokens")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let output = usage
                                    .get("completion_tokens")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                if input > 0 || output > 0 {
                                    deltas.push(StreamDelta::Usage {
                                        input_tokens: input,
                                        output_tokens: output,
                                    });
                                }
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
                supports_thinking: false,
                context_window: None,
            })
            .collect()
    }

    fn name(&self) -> &str {
        &self.provider_name
    }
}
