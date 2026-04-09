use std::collections::HashMap;

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
        }
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
            let summaries: Vec<SessionSummary> =
                sessions.iter().map(session_to_summary).collect();
            let _ = event_tx.send(Event::SessionsLoaded {
                sessions: summaries,
            });
            if let Some(first) = sessions.first() {
                self.active_session_id = Some(first.id.clone());
                let _ = event_tx.send(Event::SessionSelected {
                    id: first.id.clone(),
                });
                self.send_messages_loaded(&first.id, &event_tx);
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
                    debug!("SendMessage: content='{}', active_session={:?}", &content[..content.len().min(50)], self.active_session_id);
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
                    let session_id = self.active_session_id.clone().unwrap();
                    self.handle_send_message(&session_id, &content, &event_tx)
                        .await;
                }

                Action::RegenerateLastResponse => {
                    // TODO: implement regeneration
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
        debug!("handle_send_message: session={}, provider={}, model={}", session_id, self.current_provider, self.current_model);
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

        // 2. Get message history from DB
        let db_messages = match self.db.get_session_messages(session_id) {
            Ok(msgs) => msgs,
            Err(e) => {
                error!("Failed to load messages: {e}");
                let _ = event_tx.send(Event::ResponseError {
                    error: format!("Failed to load message history: {e}"),
                });
                return;
            }
        };

        // 3. Build ChatRequest with history
        let chat_messages: Vec<ChatMessage> = db_messages
            .iter()
            .filter(|m| m.role.as_str() == "user" || m.role.as_str() == "assistant")
            .map(|m| ChatMessage {
                role: m.role.as_str().to_string(),
                content: m.content.clone().unwrap_or_default(),
            })
            .collect();

        let request = ChatRequest {
            model: self.current_model.clone(),
            messages: chat_messages,
            system: self.system_prompt.clone(),
            max_tokens: None,
            temperature: None,
        };

        // 4. Call provider.send()
        let provider = match self.providers.get(&self.current_provider) {
            Some(p) => p,
            None => {
                error!("Provider '{}' not found in providers map (available: {:?})", self.current_provider, self.providers.keys().collect::<Vec<_>>());
                let _ = event_tx.send(Event::ResponseError {
                    error: format!("Provider '{}' not found (available: {:?})", self.current_provider, self.providers.keys().collect::<Vec<_>>()),
                });
                return;
            }
        };

        debug!("Calling provider.send() with {} messages", request.messages.len());
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
                return;
            }
        };

        // 5. Stream through deltas
        let mut full_text = String::new();
        let mut full_thinking = String::new();
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;
        let start = std::time::Instant::now();

        while let Some(result) = stream.next().await {
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
                    return;
                }
            }
        }

        let latency_ms = start.elapsed().as_millis() as i64;

        // 6. Store assistant message with blocks
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
                // 7. Update usage stats
                if input_tokens > 0 || output_tokens > 0 {
                    let _ = self.db.update_message_usage(
                        &msg.id,
                        input_tokens as i64,
                        output_tokens as i64,
                        latency_ms,
                    );
                }

                // 8. Send StreamDone and reload messages
                let _ = event_tx.send(Event::StreamDone {
                    message_id: msg.id,
                });
                self.send_messages_loaded(session_id, event_tx);

                // 9. Auto-rename session if this is the first exchange
                self.maybe_auto_rename_session(session_id, event_tx).await;
            }
            Err(e) => {
                error!("Failed to store assistant message: {e}");
                let _ = event_tx.send(Event::ResponseError {
                    error: format!("Failed to store response: {e}"),
                });
            }
        }
    }

    async fn maybe_auto_rename_session(
        &self,
        session_id: &str,
        event_tx: &mpsc::UnboundedSender<Event>,
    ) {
        // Only auto-rename if this is the first assistant response
        let messages = match self.db.get_session_messages(session_id) {
            Ok(m) => m,
            Err(e) => {
                debug!("Auto-rename: failed to load messages: {e}");
                return;
            }
        };

        // Count assistant messages — only rename on the first one
        let assistant_count = messages.iter().filter(|m| m.role.as_str() == "assistant").count();
        if assistant_count != 1 {
            debug!("Auto-rename: skipping, {} assistant messages (need exactly 1)", assistant_count);
            return;
        }

        // Check if title looks auto-generated (from Ctrl+N or truncate_for_title)
        let session = match self.db.get_session(session_id) {
            Ok(s) => s,
            Err(_) => return,
        };
        // Skip if user has manually set a meaningful title (not "New Chat" or first-line truncation)
        let title = &session.title;
        let first_user = messages.first().and_then(|m| m.content.as_deref()).unwrap_or("");
        let is_auto_title = title == "New Chat" || title == truncate_for_title(first_user).as_str();
        if !is_auto_title {
            debug!("Auto-rename: skipping, title '{}' looks user-set", title);
            return;
        }

        // Get the user's first message for context
        let user_content = first_user;
        let assistant_content = messages
            .iter()
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
                    &user_content[..user_content.len().min(200)],
                    &assistant_content[..assistant_content.len().min(200)]
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
    if first_line.len() > 60 {
        format!("{}...", &first_line[..57])
    } else {
        first_line.to_string()
    }
}

fn session_to_summary(s: &tuillem_db::sessions::Session) -> SessionSummary {
    SessionSummary {
        id: s.id.clone(),
        title: s.title.clone(),
        updated_at: s.updated_at.to_rfc3339(),
        tags: s.tags.clone(),
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
