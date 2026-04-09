use crate::actions::{Event, MessageView, SearchResultView, SessionSummary};

#[derive(Debug, Clone)]
pub struct PendingToolCall {
    pub tool_name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub sessions: Vec<SessionSummary>,
    pub active_session_id: Option<String>,
    pub messages: Vec<MessageView>,
    pub streaming_text: String,
    pub streaming_thinking: String,
    pub is_streaming: bool,
    pub current_provider: String,
    pub current_model: String,
    pub search_results: Vec<SearchResultView>,
    pub search_query: String,
    pub pending_tool_call: Option<PendingToolCall>,
    pub error: Option<String>,
    /// Transient status message (e.g. "Copied to clipboard"). Cleared on next action.
    pub status_message: Option<String>,
}

impl AppState {
    pub fn new(provider: String, model: String) -> Self {
        Self {
            current_provider: provider,
            current_model: model,
            ..Default::default()
        }
    }

    pub fn apply_event(&mut self, event: &Event) {
        match event {
            Event::SessionCreated { id, title } => {
                self.sessions.insert(
                    0,
                    SessionSummary {
                        id: id.clone(),
                        title: title.clone(),
                        updated_at: String::new(),
                        tags: Vec::new(),
                        preview: None,
                        last_model: None,
                    },
                );
                self.active_session_id = Some(id.clone());
            }
            Event::SessionSelected { id } => {
                self.active_session_id = Some(id.clone());
            }
            Event::SessionDeleted { id } => {
                let was_active = self.active_session_id.as_deref() == Some(id);
                self.sessions.retain(|s| s.id != *id);
                if was_active {
                    self.active_session_id = self.sessions.first().map(|s| s.id.clone());
                }
            }
            Event::SessionRenamed { id, title } => {
                if let Some(s) = self.sessions.iter_mut().find(|s| s.id == *id) {
                    s.title = title.clone();
                }
            }
            Event::SessionsLoaded { sessions } => {
                self.sessions = sessions.clone();
            }
            Event::MessagesLoaded { messages } => {
                self.messages = messages.clone();
                self.streaming_text.clear();
                self.streaming_thinking.clear();
                self.is_streaming = false;
            }
            Event::StreamStarted => {
                self.streaming_text.clear();
                self.streaming_thinking.clear();
                self.is_streaming = true;
                self.error = None;
            }
            Event::StreamDelta { text } => {
                self.streaming_text.push_str(text);
                self.is_streaming = true;
            }
            Event::ThinkingDelta { text } => {
                self.streaming_thinking.push_str(text);
                self.is_streaming = true;
            }
            Event::StreamDone { .. } => {
                self.streaming_text.clear();
                self.streaming_thinking.clear();
                self.is_streaming = false;
            }
            Event::ResponseError { error } => {
                self.streaming_text.clear();
                self.streaming_thinking.clear();
                self.is_streaming = false;
                self.error = Some(error.clone());
            }
            Event::SearchResults { results } => {
                self.search_results = results.clone();
            }
            Event::ToolCallRequested {
                tool_name,
                input,
                requires_confirm,
            } => {
                if *requires_confirm {
                    self.pending_tool_call = Some(PendingToolCall {
                        tool_name: tool_name.clone(),
                        input: input.clone(),
                    });
                }
            }
            Event::ToolCallResult { .. } => {
                self.pending_tool_call = None;
            }
            Event::ModelSwitched { provider, model } => {
                self.current_provider = provider.clone();
                self.current_model = model.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Event;

    #[test]
    fn test_session_created() {
        let mut state = AppState::new("anthropic".into(), "claude-3".into());
        state.apply_event(&Event::SessionCreated {
            id: "s1".into(),
            title: "First".into(),
        });

        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.sessions[0].id, "s1");
        assert_eq!(state.sessions[0].title, "First");
        assert_eq!(state.active_session_id, Some("s1".into()));

        // Insert another at front
        state.apply_event(&Event::SessionCreated {
            id: "s2".into(),
            title: "Second".into(),
        });
        assert_eq!(state.sessions.len(), 2);
        assert_eq!(state.sessions[0].id, "s2");
        assert_eq!(state.active_session_id, Some("s2".into()));
    }

    #[test]
    fn test_session_deleted_selects_next() {
        let mut state = AppState::new("anthropic".into(), "claude-3".into());
        state.apply_event(&Event::SessionCreated {
            id: "s1".into(),
            title: "First".into(),
        });
        state.apply_event(&Event::SessionCreated {
            id: "s2".into(),
            title: "Second".into(),
        });

        // s2 is at front, s1 is second. Active is s2.
        assert_eq!(state.active_session_id, Some("s2".into()));

        // Delete the active session
        state.apply_event(&Event::SessionDeleted { id: "s2".into() });
        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.active_session_id, Some("s1".into()));
    }

    #[test]
    fn test_streaming_deltas() {
        let mut state = AppState::new("anthropic".into(), "claude-3".into());

        state.apply_event(&Event::StreamDelta {
            text: "Hello ".into(),
        });
        assert_eq!(state.streaming_text, "Hello ");
        assert!(state.is_streaming);

        state.apply_event(&Event::StreamDelta {
            text: "world".into(),
        });
        assert_eq!(state.streaming_text, "Hello world");

        state.apply_event(&Event::ThinkingDelta {
            text: "thinking...".into(),
        });
        assert_eq!(state.streaming_thinking, "thinking...");

        state.apply_event(&Event::StreamDone {
            message_id: "m1".into(),
        });
        assert!(state.streaming_text.is_empty());
        assert!(state.streaming_thinking.is_empty());
        assert!(!state.is_streaming);
    }

    #[test]
    fn test_model_switch() {
        let mut state = AppState::new("anthropic".into(), "claude-3".into());
        state.apply_event(&Event::ModelSwitched {
            provider: "openai".into(),
            model: "gpt-4".into(),
        });
        assert_eq!(state.current_provider, "openai");
        assert_eq!(state.current_model, "gpt-4");
    }
}
