#[derive(Debug, Clone)]
pub enum Action {
    CreateSession { title: String },
    SelectSession { id: String },
    DeleteSession { id: String },
    RenameSession { id: String, title: String },
    AddTag { session_id: String, tag: String },
    RemoveTag { session_id: String, tag: String },
    SendMessage { content: String },
    RegenerateLastResponse,
    SwitchModel { provider: String, model: String },
    Search { query: String },
    ConfirmToolCall { approved: bool },
    Quit,
}

#[derive(Debug, Clone)]
pub enum Event {
    SessionCreated {
        id: String,
        title: String,
    },
    SessionSelected {
        id: String,
    },
    SessionDeleted {
        id: String,
    },
    SessionRenamed {
        id: String,
        title: String,
    },
    SessionsLoaded {
        sessions: Vec<SessionSummary>,
    },
    MessagesLoaded {
        messages: Vec<MessageView>,
    },
    StreamStarted,
    StreamDelta {
        text: String,
    },
    ThinkingDelta {
        text: String,
    },
    StreamDone {
        message_id: String,
    },
    ResponseError {
        error: String,
    },
    SearchResults {
        results: Vec<SearchResultView>,
    },
    ToolCallRequested {
        tool_name: String,
        input: serde_json::Value,
        requires_confirm: bool,
    },
    ToolCallResult {
        output: String,
    },
    ModelSwitched {
        provider: String,
        model: String,
    },
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    pub preview: Option<String>,
    pub last_model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MessageView {
    pub id: String,
    pub role: String,
    pub content: Option<String>,
    pub model_id: Option<String>,
    pub provider_name: Option<String>,
    pub blocks: Vec<BlockView>,
    pub token_usage_in: Option<i64>,
    pub token_usage_out: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct BlockView {
    pub block_type: String,
    pub content: Option<String>,
    pub compressed: bool,
}

#[derive(Debug, Clone)]
pub struct SearchResultView {
    pub session_id: String,
    pub session_title: String,
    pub snippet: String,
}
