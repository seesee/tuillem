use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tracing::{debug, error, info};

use tuillem_db::Db;
use tuillem_db::messages::{NewBlock, NewMessage};
use tuillem_plugin::PluginHost;
use tuillem_provider::{ChatMessage, ChatRequest, Provider, StreamDelta};

use crate::actions::{Action, BlockView, Event, MessageView, SearchResultView, SessionSummary};

pub struct Coordinator {
    db: Db,
    providers: HashMap<String, Box<dyn Provider>>,
    plugin_host: PluginHost,
    current_provider: String,
    current_model: String,
    system_prompt: Option<String>,
    active_session_id: Option<String>,
    cancel_flag: Arc<AtomicBool>,
}

impl Coordinator {
    pub fn new(
        db: Db,
        providers: HashMap<String, Box<dyn Provider>>,
        plugin_host: PluginHost,
        default_provider: String,
        default_model: String,
        system_prompt: Option<String>,
    ) -> Self {
        Self {
            db,
            providers,
            plugin_host,
            current_provider: default_provider,
            current_model: default_model,
            system_prompt,
            active_session_id: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a handle to the cancel flag. Set to true to cancel active streaming.
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancel_flag.clone()
    }

    pub async fn run(
        mut self,
        mut action_rx: mpsc::UnboundedReceiver<Action>,
        event_tx: mpsc::UnboundedSender<Event>,
    ) {
        info!("Coordinator run() started, waiting for actions...");
        // On startup, load sessions and select the most recent one
        if let Ok(sessions) = self.db.list_sessions() {
            debug!("Loaded {} existing sessions from DB", sessions.len());
            let summaries: Vec<SessionSummary> = sessions
                .iter()
                .map(|s| {
                    let preview = self.db.get_session_last_message(&s.id).ok().flatten();
                    session_to_summary(s, preview)
                })
                .collect();
            let _ = event_tx.send(Event::SessionsLoaded {
                sessions: summaries,
            });
            if let Some(first) = sessions.first() {
                self.active_session_id = Some(first.id.clone());
                let _ = event_tx.send(Event::SessionSelected {
                    id: first.id.clone(),
                });
                self.send_messages_loaded(&first.id, &event_tx);
                self.restore_session_model(&first.id, &event_tx);
            }
        }

        while let Some(action) = action_rx.recv().await {
            debug!("Coordinator received action: {:?}", action);
            match action {
                Action::CreateSession { title } => match self.db.create_session(&title) {
                    Ok(session) => {
                        self.active_session_id = Some(session.id.clone());
                        let _ = event_tx.send(Event::SessionCreated {
                            id: session.id.clone(),
                            title: session.title,
                        });
                        let _ = event_tx.send(Event::MessagesLoaded {
                            messages: Vec::new(),
                        });
                    }
                    Err(e) => {
                        error!("Failed to create session: {e}");
                    }
                },

                Action::SelectSession { id } => {
                    self.active_session_id = Some(id.clone());
                    let _ = event_tx.send(Event::SessionSelected { id: id.clone() });
                    self.send_messages_loaded(&id, &event_tx);
                    self.restore_session_model(&id, &event_tx);
                }

                Action::DeleteSession { id } => {
                    if let Err(e) = self.db.delete_session(&id) {
                        error!("Failed to delete session: {e}");
                    } else {
                        if self.active_session_id.as_deref() == Some(&id) {
                            self.active_session_id = None;
                        }
                        let _ = event_tx.send(Event::SessionDeleted { id });
                    }
                }

                Action::RenameSession { id, title } => {
                    if let Err(e) = self.db.update_session_title(&id, &title) {
                        error!("Failed to rename session: {e}");
                    } else {
                        let _ = event_tx.send(Event::SessionRenamed { id, title });
                    }
                }

                Action::AddTag { session_id, tag } => {
                    if let Err(e) = self.db.add_session_tag(&session_id, &tag) {
                        error!("Failed to add tag: {e}");
                    }
                }

                Action::RemoveTag { session_id, tag } => {
                    if let Err(e) = self.db.remove_session_tag(&session_id, &tag) {
                        error!("Failed to remove tag: {e}");
                    }
                }

                Action::SendMessage { content } => {
                    debug!(
                        "SendMessage: content='{}', active_session={:?}",
                        &content[..content.len().min(50)],
                        self.active_session_id
                    );
                    // Auto-create a session if none is active
                    if self.active_session_id.is_none() {
                        let title = truncate_for_title(&content);
                        match self.db.create_session(&title) {
                            Ok(session) => {
                                self.active_session_id = Some(session.id.clone());
                                let _ = event_tx.send(Event::SessionCreated {
                                    id: session.id.clone(),
                                    title: session.title,
                                });
                            }
                            Err(e) => {
                                error!("Failed to auto-create session: {e}");
                                let _ = event_tx.send(Event::ResponseError {
                                    error: format!("Failed to create session: {e}"),
                                });
                                continue;
                            }
                        }
                    }
                    let Some(session_id) = self.active_session_id.clone() else {
                        error!("No active session after auto-create attempt");
                        continue;
                    };
                    self.handle_send_message(&session_id, &content, &event_tx)
                        .await;
                }

                Action::CancelStream => {
                    debug!("CancelStream received");
                    self.cancel_flag.store(true, Ordering::Relaxed);
                }

                Action::RegenerateLastResponse => {
                    if let Some(session_id) = self.active_session_id.clone() {
                        self.handle_regenerate(&session_id, &event_tx).await;
                    }
                }

                Action::SwitchModel { provider, model } => {
                    self.current_provider = provider.clone();
                    self.current_model = model.clone();
                    let _ = event_tx.send(Event::ModelSwitched { provider, model });
                }

                Action::Search { query } => match self.db.search_messages(&query) {
                    Ok(results) => {
                        let views = results
                            .into_iter()
                            .map(|r| SearchResultView {
                                session_id: r.session_id,
                                session_title: r.session_title,
                                snippet: r.content_snippet,
                            })
                            .collect();
                        let _ = event_tx.send(Event::SearchResults { results: views });
                    }
                    Err(e) => {
                        error!("Search failed: {e}");
                    }
                },

                Action::ConfirmToolCall { approved: _ } => {
                    // TODO: handle tool call confirmation
                }

                Action::SaveTranscript => {
                    if let Some(ref session_id) = self.active_session_id {
                        self.handle_save_transcript(session_id);
                    }
                }

                Action::Quit => {
                    info!("Quit action received, shutting down coordinator");
                    break;
                }
            }
        }
    }

    async fn handle_send_message(
        &self,
        session_id: &str,
        content: &str,
        event_tx: &mpsc::UnboundedSender<Event>,
    ) {
        debug!(
            "handle_send_message: session={}, provider={}, model={}",
            session_id, self.current_provider, self.current_model
        );
        // 1. Store user message in DB
        let user_msg = NewMessage {
            session_id,
            role: "user",
            content: Some(content),
            model_id: None,
            provider_name: None,
            parent_message_id: None,
        };
        let user_blocks = [NewBlock {
            block_type: "text",
            content,
            sequence: 0,
        }];
        if let Err(e) = self.db.create_message(&user_msg, &user_blocks) {
            error!("Failed to store user message: {e}");
            let _ = event_tx.send(Event::ResponseError {
                error: format!("Failed to store message: {e}"),
            });
            return;
        }

        debug!("User message stored, reloading messages for UI");
        // Show user message immediately in the UI
        self.send_messages_loaded(session_id, event_tx);

        // Signal that we're about to stream
        let _ = event_tx.send(Event::StreamStarted);

        // 2. Build ChatRequest from history
        let request = match self.build_chat_request(session_id, event_tx) {
            Some(r) => r,
            None => return,
        };

        // 3. Stream the response
        debug!(
            "Calling provider.send() with {} messages",
            request.messages.len()
        );
        if let Some(msg_id) = self.stream_response(session_id, request, event_tx).await {
            // 4. Save model/provider to session metadata
            let meta = serde_json::json!({
                "provider": self.current_provider,
                "model": self.current_model,
            });
            let _ = self
                .db
                .update_session_metadata(session_id, &meta.to_string());

            // 5. Auto-rename session if this is the first exchange
            let _ = msg_id;
            self.maybe_auto_rename_session(session_id, event_tx).await;
        }
    }

    async fn handle_regenerate(&self, session_id: &str, event_tx: &mpsc::UnboundedSender<Event>) {
        // Find the last assistant message and delete it
        let messages = match self.db.get_session_messages(session_id) {
            Ok(m) => m,
            Err(_) => return,
        };

        // Find last assistant message
        let last_assistant = messages
            .iter()
            .rev()
            .find(|m| m.role.as_str() == "assistant");

        if let Some(msg) = last_assistant {
            debug!("Regenerating: deleting last assistant message {}", msg.id);
            let _ = self.db.delete_message(&msg.id);
        } else {
            debug!("Regenerate: no assistant message to delete");
            return;
        }

        // Verify there is a user message to regenerate from
        let has_user = messages.iter().rev().any(|m| m.role.as_str() == "user");

        if !has_user {
            return;
        }

        // Reload messages (without the deleted assistant response)
        self.send_messages_loaded(session_id, event_tx);

        // Signal streaming start
        let _ = event_tx.send(Event::StreamStarted);

        // Build ChatRequest from updated history
        let request = match self.build_chat_request(session_id, event_tx) {
            Some(r) => r,
            None => return,
        };

        // Stream the response (no auto-rename or metadata save for regenerate)
        self.stream_response(session_id, request, event_tx).await;
    }

    /// Build a `ChatRequest` from the current session's message history.
    /// Returns `None` (and sends an error event) on failure.
    fn build_chat_request(
        &self,
        session_id: &str,
        event_tx: &mpsc::UnboundedSender<Event>,
    ) -> Option<ChatRequest> {
        let db_messages = match self.db.get_session_messages(session_id) {
            Ok(msgs) => msgs,
            Err(e) => {
                error!("Failed to load messages: {e}");
                let _ = event_tx.send(Event::ResponseError {
                    error: format!("Failed to load message history: {e}"),
                });
                return None;
            }
        };

        let chat_messages: Vec<ChatMessage> = db_messages
            .iter()
            .filter(|m| m.role.as_str() == "user" || m.role.as_str() == "assistant")
            .map(|m| ChatMessage {
                role: m.role.as_str().to_string(),
                content: m.content.clone().unwrap_or_default(),
            })
            .collect();

        Some(ChatRequest {
            model: self.current_model.clone(),
            messages: chat_messages,
            system: self.system_prompt.clone(),
            max_tokens: None,
            temperature: None,
        })
    }

    /// Stream a response from the provider, store it, and notify the UI.
    /// Returns the message ID if successful, `None` if cancelled or failed.
    async fn stream_response(
        &self,
        session_id: &str,
        request: ChatRequest,
        event_tx: &mpsc::UnboundedSender<Event>,
    ) -> Option<String> {
        // Look up the provider
        let provider = match self.providers.get(&self.current_provider) {
            Some(p) => p,
            None => {
                error!(
                    "Provider '{}' not found in providers map (available: {:?})",
                    self.current_provider,
                    self.providers.keys().collect::<Vec<_>>()
                );
                let _ = event_tx.send(Event::ResponseError {
                    error: format!(
                        "Provider '{}' not found (available: {:?})",
                        self.current_provider,
                        self.providers.keys().collect::<Vec<_>>()
                    ),
                });
                return None;
            }
        };

        let mut stream = match provider.send(request).await {
            Ok(s) => {
                debug!("Provider returned stream successfully");
                s
            }
            Err(e) => {
                error!("Provider.send() failed: {e}");
                let _ = event_tx.send(Event::ResponseError {
                    error: format!("Provider error: {e}"),
                });
                return None;
            }
        };

        // Stream through deltas (check cancel flag each iteration)
        self.cancel_flag.store(false, Ordering::Relaxed);
        let mut full_text = String::new();
        let mut full_thinking = String::new();
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;
        let start = std::time::Instant::now();
        let mut cancelled = false;

        while let Some(result) = stream.next().await {
            if self.cancel_flag.load(Ordering::Relaxed) {
                debug!("Stream cancelled by user");
                cancelled = true;
                break;
            }
            match result {
                Ok(delta) => match delta {
                    StreamDelta::Text(text) => {
                        let _ = event_tx.send(Event::StreamDelta { text: text.clone() });
                        full_text.push_str(&text);
                    }
                    StreamDelta::Thinking(text) => {
                        let _ = event_tx.send(Event::ThinkingDelta { text: text.clone() });
                        full_thinking.push_str(&text);
                    }
                    StreamDelta::Usage {
                        input_tokens: i,
                        output_tokens: o,
                    } => {
                        input_tokens = i;
                        output_tokens = o;
                    }
                    StreamDelta::ToolCallStart { name, .. } => {
                        let requires_confirm = self.plugin_host.requires_confirmation(&name);
                        let _ = event_tx.send(Event::ToolCallRequested {
                            tool_name: name,
                            input: serde_json::Value::Null,
                            requires_confirm,
                        });
                    }
                    StreamDelta::Done => break,
                    _ => {}
                },
                Err(e) => {
                    let _ = event_tx.send(Event::ResponseError {
                        error: format!("Stream error: {e}"),
                    });
                    return None;
                }
            }
        }

        let latency_ms = start.elapsed().as_millis() as i64;

        if cancelled {
            // Store partial response if we got any text
            if !full_text.is_empty() {
                full_text.push_str("\n\n*[response cancelled]*");
            }
            let _ = event_tx.send(Event::StreamDone {
                message_id: String::new(),
            });
            if full_text.is_empty() {
                self.send_messages_loaded(session_id, event_tx);
                return None;
            }
        }

        // Store assistant message with blocks
        let mut blocks = Vec::new();
        let mut seq = 0;
        if !full_text.is_empty() {
            blocks.push(NewBlock {
                block_type: "text",
                content: &full_text,
                sequence: seq,
            });
            seq += 1;
        }
        if !full_thinking.is_empty() {
            blocks.push(NewBlock {
                block_type: "thinking",
                content: &full_thinking,
                sequence: seq,
            });
        }

        let assistant_msg = NewMessage {
            session_id,
            role: "assistant",
            content: if full_text.is_empty() {
                None
            } else {
                Some(&full_text)
            },
            model_id: Some(&self.current_model),
            provider_name: Some(&self.current_provider),
            parent_message_id: None,
        };

        match self.db.create_message(&assistant_msg, &blocks) {
            Ok(msg) => {
                if input_tokens > 0 || output_tokens > 0 {
                    let _ = self.db.update_message_usage(
                        &msg.id,
                        input_tokens as i64,
                        output_tokens as i64,
                        latency_ms,
                    );
                }

                let _ = event_tx.send(Event::StreamDone {
                    message_id: msg.id.clone(),
                });
                self.send_messages_loaded(session_id, event_tx);
                Some(msg.id)
            }
            Err(e) => {
                error!("Failed to store assistant message: {e}");
                let _ = event_tx.send(Event::ResponseError {
                    error: format!("Failed to store response: {e}"),
                });
                None
            }
        }
    }

    async fn maybe_auto_rename_session(
        &self,
        session_id: &str,
        event_tx: &mpsc::UnboundedSender<Event>,
    ) {
        // Check if title needs auto-renaming
        let session = match self.db.get_session(session_id) {
            Ok(s) => s,
            Err(e) => {
                debug!("Auto-rename: failed to load session: {e}");
                return;
            }
        };

        // Only rename sessions with auto-generated titles
        let title = &session.title;
        let messages = match self.db.get_session_messages(session_id) {
            Ok(m) => m,
            Err(_) => return,
        };
        let first_user = messages
            .iter()
            .find(|m| m.role.as_str() == "user")
            .and_then(|m| m.content.as_deref())
            .unwrap_or("");
        let is_auto_title = title == "New Chat" || title == truncate_for_title(first_user).as_str();
        if !is_auto_title {
            debug!("Auto-rename: skipping, title '{}' looks user-set", title);
            return;
        }

        debug!("Auto-rename: title '{}' needs renaming", title);

        // Get context from last user + assistant exchange
        let user_content = messages
            .iter()
            .rev()
            .find(|m| m.role.as_str() == "user")
            .and_then(|m| m.content.as_deref())
            .unwrap_or("");
        let assistant_content = messages
            .iter()
            .rev()
            .find(|m| m.role.as_str() == "assistant")
            .and_then(|m| m.content.as_deref())
            .unwrap_or("");

        // Ask the model to generate a title
        let provider = match self.providers.get(&self.current_provider) {
            Some(p) => p,
            None => return,
        };

        let summary_request = ChatRequest {
            model: self.current_model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Summarize this conversation in 3-6 words for a sidebar title. Reply with ONLY the title, no quotes, no punctuation at the end.\n\nUser: {}\nAssistant: {}",
                    user_content.chars().take(200).collect::<String>(),
                    assistant_content.chars().take(200).collect::<String>()
                ),
            }],
            system: Some("You generate short conversation titles. Reply with only the title text, nothing else.".to_string()),
            max_tokens: Some(20),
            temperature: Some(0.3),
        };

        debug!("Auto-renaming session, requesting title from model");
        match provider.send(summary_request).await {
            Ok(mut stream) => {
                let mut title = String::new();
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(StreamDelta::Text(t)) => title.push_str(&t),
                        Ok(StreamDelta::Done) => break,
                        Err(_) => return,
                        _ => {}
                    }
                }
                let title = title.trim().to_string();
                if !title.is_empty() && title.len() < 80 {
                    debug!("Auto-rename: '{}'", title);
                    if self.db.update_session_title(session_id, &title).is_ok() {
                        let _ = event_tx.send(Event::SessionRenamed {
                            id: session_id.to_string(),
                            title,
                        });
                    }
                }
            }
            Err(e) => {
                debug!("Auto-rename failed: {e}");
            }
        }
    }

    fn handle_save_transcript(&self, session_id: &str) {
        let session = match self.db.get_session(session_id) {
            Ok(s) => s,
            Err(e) => {
                error!("SaveTranscript: failed to load session: {e}");
                return;
            }
        };
        let messages = match self.db.get_session_messages(session_id) {
            Ok(m) => m,
            Err(e) => {
                error!("SaveTranscript: failed to load messages: {e}");
                return;
            }
        };

        let title_slug: String = session
            .title
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let filename = format!("tuillem-{}-{}.md", title_slug, timestamp);

        let downloads = dirs::download_dir().unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join("Downloads"))
                .unwrap_or_else(|| std::path::PathBuf::from("."))
        });
        let path = downloads.join(&filename);

        let mut content = format!("# {}\n\n", session.title);
        for m in &messages {
            let role_label = match m.role.as_str() {
                "user" => "**User**",
                "assistant" => "**Assistant**",
                _ => "**System**",
            };
            content.push_str(&format!("## {}\n\n", role_label));
            if let Some(ref text) = m.content {
                content.push_str(text);
                content.push_str("\n\n");
            }
        }

        match std::fs::write(&path, &content) {
            Ok(_) => {
                info!("Transcript saved to {}", path.display());
            }
            Err(e) => {
                error!("SaveTranscript: failed to write file: {e}");
            }
        }
    }

    fn restore_session_model(&mut self, session_id: &str, event_tx: &mpsc::UnboundedSender<Event>) {
        if let Ok(session) = self.db.get_session(session_id)
            && let Some(meta) = &session.metadata
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(meta)
        {
            if let Some(p) = v["provider"].as_str()
                && self.providers.contains_key(p)
            {
                self.current_provider = p.to_string();
            }
            if let Some(m) = v["model"].as_str() {
                self.current_model = m.to_string();
            }
            debug!(
                "Restored session model: {}:{}",
                self.current_provider, self.current_model
            );
            let _ = event_tx.send(Event::ModelSwitched {
                provider: self.current_provider.clone(),
                model: self.current_model.clone(),
            });
        }
    }

    fn send_messages_loaded(&self, session_id: &str, event_tx: &mpsc::UnboundedSender<Event>) {
        match self.db.get_session_messages(session_id) {
            Ok(msgs) => {
                let views = msgs.into_iter().map(|m| message_to_view(&m)).collect();
                let _ = event_tx.send(Event::MessagesLoaded { messages: views });
            }
            Err(e) => {
                error!("Failed to load messages: {e}");
            }
        }
    }
}

fn truncate_for_title(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or(content);
    if first_line.chars().count() > 60 {
        let truncated: String = first_line.chars().take(57).collect();
        format!("{}...", truncated)
    } else {
        first_line.to_string()
    }
}

fn session_to_summary(
    s: &tuillem_db::sessions::Session,
    preview: Option<String>,
) -> SessionSummary {
    // Parse last model from metadata JSON
    let last_model = s.metadata.as_deref().and_then(|m| {
        serde_json::from_str::<serde_json::Value>(m)
            .ok()
            .and_then(|v| v["model"].as_str().map(|s| s.to_string()))
    });

    SessionSummary {
        id: s.id.clone(),
        title: s.title.clone(),
        updated_at: s.updated_at.to_rfc3339(),
        tags: s.tags.clone(),
        preview: preview.map(|p| {
            let trimmed = p.trim();
            if trimmed.chars().count() > 60 {
                let truncated: String = trimmed.chars().take(57).collect();
                format!("{}...", truncated)
            } else {
                trimmed.to_string()
            }
        }),
        last_model,
    }
}

fn message_to_view(m: &tuillem_db::messages::Message) -> MessageView {
    MessageView {
        id: m.id.clone(),
        role: m.role.as_str().to_string(),
        content: m.content.clone(),
        model_id: m.model_id.clone(),
        provider_name: m.provider_name.clone(),
        blocks: m
            .blocks
            .iter()
            .map(|b| BlockView {
                block_type: b.block_type.as_str().to_string(),
                content: b.content.clone(),
                compressed: b.compressed,
            })
            .collect(),
        token_usage_in: m.token_usage_in,
        token_usage_out: m.token_usage_out,
    }
}
