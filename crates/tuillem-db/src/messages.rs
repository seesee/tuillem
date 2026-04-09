use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{Db, DbError};

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Role::User),
            "assistant" => Some(Role::Assistant),
            "system" => Some(Role::System),
            "tool" => Some(Role::Tool),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Text,
    Thinking,
    ToolCall,
    ToolResult,
}

impl BlockType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockType::Text => "text",
            BlockType::Thinking => "thinking",
            BlockType::ToolCall => "tool_call",
            BlockType::ToolResult => "tool_result",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "text" => Some(BlockType::Text),
            "thinking" => Some(BlockType::Thinking),
            "tool_call" => Some(BlockType::ToolCall),
            "tool_result" => Some(BlockType::ToolResult),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: Role,
    pub content: Option<String>,
    pub model_id: Option<String>,
    pub provider_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub token_usage_in: Option<i64>,
    pub token_usage_out: Option<i64>,
    pub latency_ms: Option<i64>,
    pub parent_message_id: Option<String>,
    pub blocks: Vec<MessageBlock>,
}

#[derive(Debug, Clone)]
pub struct MessageBlock {
    pub id: String,
    pub message_id: String,
    pub block_type: BlockType,
    pub content: Option<String>,
    pub sequence: i32,
    pub compressed: bool,
}

pub struct NewMessage<'a> {
    pub session_id: &'a str,
    pub role: &'a str,
    pub content: Option<&'a str>,
    pub model_id: Option<&'a str>,
    pub provider_name: Option<&'a str>,
    pub parent_message_id: Option<&'a str>,
}

pub struct NewBlock<'a> {
    pub block_type: &'a str,
    pub content: &'a str,
    pub sequence: i32,
}

impl Db {
    pub fn create_message(
        &self,
        msg: &NewMessage,
        blocks: &[NewBlock],
    ) -> Result<Message, DbError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        self.conn.execute(
            "INSERT INTO messages (id, session_id, role, content, model_id, provider_name, created_at, parent_message_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id,
                msg.session_id,
                msg.role,
                msg.content,
                msg.model_id,
                msg.provider_name,
                now_str,
                msg.parent_message_id,
            ],
        )?;

        // Update session updated_at
        self.conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now_str, msg.session_id],
        )?;

        let mut message_blocks = Vec::new();
        for block in blocks {
            let block_id = Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO message_blocks (id, message_id, block_type, content, sequence)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    block_id,
                    id,
                    block.block_type,
                    block.content,
                    block.sequence
                ],
            )?;
            message_blocks.push(MessageBlock {
                id: block_id,
                message_id: id.clone(),
                block_type: BlockType::parse(block.block_type).unwrap_or(BlockType::Text),
                content: Some(block.content.to_string()),
                sequence: block.sequence,
                compressed: false,
            });
        }

        Ok(Message {
            id,
            session_id: msg.session_id.to_string(),
            role: Role::parse(msg.role).unwrap_or(Role::User),
            content: msg.content.map(|s| s.to_string()),
            model_id: msg.model_id.map(|s| s.to_string()),
            provider_name: msg.provider_name.map(|s| s.to_string()),
            created_at: now,
            token_usage_in: None,
            token_usage_out: None,
            latency_ms: None,
            parent_message_id: msg.parent_message_id.map(|s| s.to_string()),
            blocks: message_blocks,
        })
    }

    pub fn update_message_usage(
        &self,
        message_id: &str,
        tokens_in: i64,
        tokens_out: i64,
        latency_ms: i64,
    ) -> Result<(), DbError> {
        let rows = self.conn.execute(
            "UPDATE messages SET token_usage_in = ?1, token_usage_out = ?2, latency_ms = ?3 WHERE id = ?4",
            rusqlite::params![tokens_in, tokens_out, latency_ms, message_id],
        )?;
        if rows == 0 {
            return Err(DbError::NotFound(format!("message {message_id}")));
        }
        Ok(())
    }

    pub fn get_session_messages(&self, session_id: &str) -> Result<Vec<Message>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, model_id, provider_name, created_at,
                    token_usage_in, token_usage_out, latency_ms, parent_message_id
             FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
        )?;

        let rows = stmt.query_map(rusqlite::params![session_id], |row| {
            let created_str: String = row.get(6)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                created_str,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<i64>>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, Option<String>>(10)?,
            ))
        })?;

        let mut messages = Vec::new();
        for row in rows {
            let r = row?;
            let blocks = self.get_message_blocks(&r.0)?;
            messages.push(Message {
                id: r.0,
                session_id: r.1,
                role: Role::parse(&r.2).unwrap_or(Role::User),
                content: r.3,
                model_id: r.4,
                provider_name: r.5,
                created_at: DateTime::parse_from_rfc3339(&r.6)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                token_usage_in: r.7,
                token_usage_out: r.8,
                latency_ms: r.9,
                parent_message_id: r.10,
                blocks,
            });
        }

        Ok(messages)
    }

    fn get_message_blocks(&self, message_id: &str) -> Result<Vec<MessageBlock>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, message_id, block_type, content, sequence, compressed
             FROM message_blocks WHERE message_id = ?1 ORDER BY sequence ASC",
        )?;

        let rows = stmt.query_map(rusqlite::params![message_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, bool>(5)?,
            ))
        })?;

        let mut blocks = Vec::new();
        for row in rows {
            let r = row?;
            blocks.push(MessageBlock {
                id: r.0,
                message_id: r.1,
                block_type: BlockType::parse(&r.2).unwrap_or(BlockType::Text),
                content: r.3,
                sequence: r.4,
                compressed: r.5,
            });
        }

        Ok(blocks)
    }

    /// Delete a message and its blocks by ID.
    pub fn delete_message(&self, message_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM messages WHERE id = ?1",
            rusqlite::params![message_id],
        )?;
        Ok(())
    }

    pub fn compress_thinking_blocks(&self, older_than_days: i64) -> Result<usize, DbError> {
        let cutoff = Utc::now() - chrono::Duration::days(older_than_days);
        let cutoff_str = cutoff.to_rfc3339();

        let count = self.conn.execute(
            "UPDATE message_blocks SET content = NULL, compressed = 1
             WHERE block_type = 'thinking' AND compressed = 0
             AND message_id IN (
                 SELECT id FROM messages WHERE created_at < ?1
             )",
            rusqlite::params![cutoff_str],
        )?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use crate::Db;
    use crate::messages::{NewBlock, NewMessage};

    fn setup() -> Db {
        Db::open_in_memory().unwrap()
    }

    #[test]
    fn test_create_message_with_blocks() {
        let db = setup();
        let session = db.create_session("Test").unwrap();

        let msg = NewMessage {
            session_id: &session.id,
            role: "user",
            content: Some("Hello world"),
            model_id: Some("claude-3"),
            provider_name: Some("anthropic"),
            parent_message_id: None,
        };

        let blocks = vec![
            NewBlock {
                block_type: "text",
                content: "Hello world",
                sequence: 0,
            },
            NewBlock {
                block_type: "thinking",
                content: "Let me think...",
                sequence: 1,
            },
        ];

        let message = db.create_message(&msg, &blocks).unwrap();
        assert_eq!(message.content, Some("Hello world".to_string()));
        assert_eq!(message.blocks.len(), 2);
        assert_eq!(message.blocks[0].sequence, 0);
        assert_eq!(message.blocks[1].sequence, 1);
    }

    #[test]
    fn test_get_session_messages_ordered() {
        let db = setup();
        let session = db.create_session("Test").unwrap();

        let msg1 = NewMessage {
            session_id: &session.id,
            role: "user",
            content: Some("First"),
            model_id: None,
            provider_name: None,
            parent_message_id: None,
        };
        db.create_message(&msg1, &[]).unwrap();

        let msg2 = NewMessage {
            session_id: &session.id,
            role: "assistant",
            content: Some("Second"),
            model_id: None,
            provider_name: None,
            parent_message_id: None,
        };
        db.create_message(&msg2, &[]).unwrap();

        let messages = db.get_session_messages(&session.id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, Some("First".to_string()));
        assert_eq!(messages[1].content, Some("Second".to_string()));
    }

    #[test]
    fn test_update_message_usage() {
        let db = setup();
        let session = db.create_session("Test").unwrap();

        let msg = NewMessage {
            session_id: &session.id,
            role: "assistant",
            content: Some("Response"),
            model_id: None,
            provider_name: None,
            parent_message_id: None,
        };
        let message = db.create_message(&msg, &[]).unwrap();

        db.update_message_usage(&message.id, 100, 200, 1500)
            .unwrap();

        let messages = db.get_session_messages(&session.id).unwrap();
        assert_eq!(messages[0].token_usage_in, Some(100));
        assert_eq!(messages[0].token_usage_out, Some(200));
        assert_eq!(messages[0].latency_ms, Some(1500));
    }

    #[test]
    fn test_compress_thinking_blocks() {
        let db = setup();
        let session = db.create_session("Test").unwrap();

        let msg = NewMessage {
            session_id: &session.id,
            role: "assistant",
            content: Some("Response"),
            model_id: None,
            provider_name: None,
            parent_message_id: None,
        };

        let blocks = vec![
            NewBlock {
                block_type: "text",
                content: "visible",
                sequence: 0,
            },
            NewBlock {
                block_type: "thinking",
                content: "internal thoughts",
                sequence: 1,
            },
        ];

        db.create_message(&msg, &blocks).unwrap();

        // Compress blocks older than 0 days (i.e., all)
        let count = db.compress_thinking_blocks(0).unwrap();
        assert_eq!(count, 1);

        let messages = db.get_session_messages(&session.id).unwrap();
        let thinking_block = messages[0]
            .blocks
            .iter()
            .find(|b| b.block_type == crate::messages::BlockType::Thinking)
            .unwrap();
        assert!(thinking_block.compressed);
        assert!(thinking_block.content.is_none());

        // Text block should be unaffected
        let text_block = messages[0]
            .blocks
            .iter()
            .find(|b| b.block_type == crate::messages::BlockType::Text)
            .unwrap();
        assert!(!text_block.compressed);
        assert_eq!(text_block.content, Some("visible".to_string()));
    }

    #[test]
    fn test_cascade_delete() {
        let db = setup();
        let session = db.create_session("Test").unwrap();

        let msg = NewMessage {
            session_id: &session.id,
            role: "user",
            content: Some("Hello"),
            model_id: None,
            provider_name: None,
            parent_message_id: None,
        };
        let blocks = vec![NewBlock {
            block_type: "text",
            content: "Hello",
            sequence: 0,
        }];
        db.create_message(&msg, &blocks).unwrap();

        // Delete session should cascade to messages and blocks
        db.delete_session(&session.id).unwrap();

        let messages = db.get_session_messages(&session.id).unwrap();
        assert!(messages.is_empty());
    }
}
