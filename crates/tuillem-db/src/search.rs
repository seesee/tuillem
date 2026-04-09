use crate::{Db, DbError};

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub session_id: String,
    pub session_title: String,
    pub message_id: String,
    pub content_snippet: String,
    pub role: String,
}

impl Db {
    pub fn search_messages(&self, query: &str) -> Result<Vec<SearchResult>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.session_id, s.title, m.id, snippet(messages_fts, 0, '**', '**', '...', 32), m.role
             FROM messages_fts
             JOIN messages m ON m.rowid = messages_fts.rowid
             JOIN sessions s ON s.id = m.session_id
             WHERE messages_fts MATCH ?1
             LIMIT 50",
        )?;

        let rows = stmt.query_map(rusqlite::params![query], |row| {
            Ok(SearchResult {
                session_id: row.get(0)?,
                session_title: row.get(1)?,
                message_id: row.get(2)?,
                content_snippet: row.get(3)?,
                role: row.get(4)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    pub fn search_sessions_by_tag(&self, tag: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT session_id FROM session_tags WHERE tag = ?1")?;

        let rows = stmt.query_map(rusqlite::params![tag], |row| row.get(0))?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }

        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use crate::Db;
    use crate::messages::{NewBlock, NewMessage};

    fn setup_with_data() -> Db {
        let db = Db::open_in_memory().unwrap();
        let session = db.create_session("Rust Chat").unwrap();
        db.add_session_tag(&session.id, "rust").unwrap();

        let msg = NewMessage {
            session_id: &session.id,
            role: "user",
            content: Some("How do I use iterators in Rust?"),
            model_id: None,
            provider_name: None,
            parent_message_id: None,
        };
        db.create_message(&msg, &[]).unwrap();

        let msg2 = NewMessage {
            session_id: &session.id,
            role: "assistant",
            content: Some("Iterators in Rust are lazy and composable."),
            model_id: None,
            provider_name: None,
            parent_message_id: None,
        };
        let blocks = vec![NewBlock {
            block_type: "text",
            content: "Iterators in Rust are lazy and composable.",
            sequence: 0,
        }];
        db.create_message(&msg2, &blocks).unwrap();

        db
    }

    #[test]
    fn test_fts_search() {
        let db = setup_with_data();

        let results = db.search_messages("iterators").unwrap();
        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .any(|r| r.content_snippet.to_lowercase().contains("iterator"))
        );
    }

    #[test]
    fn test_search_no_results() {
        let db = setup_with_data();

        let results = db.search_messages("quantum computing").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_by_tag() {
        let db = setup_with_data();

        let ids = db.search_sessions_by_tag("rust").unwrap();
        assert_eq!(ids.len(), 1);

        let ids = db.search_sessions_by_tag("python").unwrap();
        assert!(ids.is_empty());
    }
}
