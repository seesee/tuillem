CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    metadata TEXT
);

CREATE TABLE session_tags (
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (session_id, tag)
);

CREATE INDEX idx_session_tags_tag ON session_tags(tag);

CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT,
    model_id TEXT,
    provider_name TEXT,
    created_at TEXT NOT NULL,
    token_usage_in INTEGER,
    token_usage_out INTEGER,
    latency_ms INTEGER,
    parent_message_id TEXT REFERENCES messages(id)
);

CREATE INDEX idx_messages_session ON messages(session_id, created_at);

CREATE TABLE message_blocks (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    block_type TEXT NOT NULL CHECK(block_type IN ('text', 'thinking', 'tool_call', 'tool_result')),
    content TEXT,
    sequence INTEGER NOT NULL,
    compressed INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_blocks_message ON message_blocks(message_id, sequence);

CREATE TABLE model_switches (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    from_model TEXT NOT NULL,
    to_model TEXT NOT NULL,
    at_message_id TEXT NOT NULL REFERENCES messages(id),
    context_strategy TEXT NOT NULL CHECK(context_strategy IN ('full', 'truncated', 'summary')),
    switched_at TEXT NOT NULL
);

CREATE VIRTUAL TABLE messages_fts USING fts5(content, content=messages, content_rowid=rowid);
CREATE VIRTUAL TABLE blocks_fts USING fts5(content, content=message_blocks, content_rowid=rowid);

-- FTS sync triggers for messages
CREATE TRIGGER messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER messages_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;
CREATE TRIGGER messages_au AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;

-- FTS sync triggers for message_blocks
CREATE TRIGGER blocks_ai AFTER INSERT ON message_blocks BEGIN
    INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER blocks_ad AFTER DELETE ON message_blocks BEGIN
    INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;
CREATE TRIGGER blocks_au AFTER UPDATE ON message_blocks BEGIN
    INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

INSERT INTO schema_version (version, applied_at) VALUES (1, datetime('now'));
