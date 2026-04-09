use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{Db, DbError};

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: Option<String>,
    pub tags: Vec<String>,
}

impl Db {
    pub fn create_session(&self, title: &str) -> Result<Session, DbError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        self.conn.execute(
            "INSERT INTO sessions (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, title, now_str, now_str],
        )?;

        Ok(Session {
            id,
            title: title.to_string(),
            created_at: now,
            updated_at: now,
            metadata: None,
            tags: Vec::new(),
        })
    }

    pub fn get_session(&self, id: &str) -> Result<Session, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, created_at, updated_at, metadata FROM sessions WHERE id = ?1",
        )?;

        let session = stmt
            .query_row(rusqlite::params![id], |row| {
                let created_str: String = row.get(2)?;
                let updated_str: String = row.get(3)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    created_str,
                    updated_str,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    DbError::NotFound(format!("session {id}"))
                }
                other => DbError::Sqlite(other),
            })?;

        let tags = self.get_session_tags(&session.0)?;

        Ok(Session {
            id: session.0,
            title: session.1,
            created_at: DateTime::parse_from_rfc3339(&session.2)
                .unwrap_or_default()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&session.3)
                .unwrap_or_default()
                .with_timezone(&Utc),
            metadata: session.4,
            tags,
        })
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, created_at, updated_at, metadata FROM sessions ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get(2)?;
            let updated_str: String = row.get(3)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                created_str,
                updated_str,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            let r = row?;
            let tags = self.get_session_tags(&r.0)?;
            sessions.push(Session {
                id: r.0,
                title: r.1,
                created_at: DateTime::parse_from_rfc3339(&r.2)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&r.3)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                metadata: r.4,
                tags,
            });
        }

        Ok(sessions)
    }

    pub fn update_session_title(&self, id: &str, title: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![title, now, id],
        )?;
        if rows == 0 {
            return Err(DbError::NotFound(format!("session {id}")));
        }
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> Result<(), DbError> {
        let rows = self.conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![id],
        )?;
        if rows == 0 {
            return Err(DbError::NotFound(format!("session {id}")));
        }
        Ok(())
    }

    pub fn add_session_tag(&self, session_id: &str, tag: &str) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO session_tags (session_id, tag) VALUES (?1, ?2)",
            rusqlite::params![session_id, tag],
        )?;
        Ok(())
    }

    pub fn remove_session_tag(&self, session_id: &str, tag: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM session_tags WHERE session_id = ?1 AND tag = ?2",
            rusqlite::params![session_id, tag],
        )?;
        Ok(())
    }

    fn get_session_tags(&self, session_id: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM session_tags WHERE session_id = ?1 ORDER BY tag")?;
        let tags = stmt
            .query_map(rusqlite::params![session_id], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use crate::Db;

    #[test]
    fn test_create_and_get_session() {
        let db = Db::open_in_memory().unwrap();
        let session = db.create_session("Test Session").unwrap();
        assert_eq!(session.title, "Test Session");

        let fetched = db.get_session(&session.id).unwrap();
        assert_eq!(fetched.id, session.id);
        assert_eq!(fetched.title, "Test Session");
    }

    #[test]
    fn test_list_sessions_ordered_by_updated() {
        let db = Db::open_in_memory().unwrap();
        let s1 = db.create_session("First").unwrap();
        let _s2 = db.create_session("Second").unwrap();

        // Update first session so it becomes most recently updated
        db.update_session_title(&s1.id, "First Updated").unwrap();

        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].title, "First Updated");
        assert_eq!(sessions[1].title, "Second");
    }

    #[test]
    fn test_session_tags() {
        let db = Db::open_in_memory().unwrap();
        let session = db.create_session("Tagged").unwrap();

        db.add_session_tag(&session.id, "rust").unwrap();
        db.add_session_tag(&session.id, "coding").unwrap();

        // Duplicate should be ignored
        db.add_session_tag(&session.id, "rust").unwrap();

        let fetched = db.get_session(&session.id).unwrap();
        assert_eq!(fetched.tags.len(), 2);
        assert!(fetched.tags.contains(&"rust".to_string()));
        assert!(fetched.tags.contains(&"coding".to_string()));

        // Remove tag
        db.remove_session_tag(&session.id, "rust").unwrap();
        let fetched = db.get_session(&session.id).unwrap();
        assert_eq!(fetched.tags.len(), 1);
        assert!(fetched.tags.contains(&"coding".to_string()));
    }

    #[test]
    fn test_delete_session() {
        let db = Db::open_in_memory().unwrap();
        let session = db.create_session("To Delete").unwrap();
        db.delete_session(&session.id).unwrap();

        let result = db.get_session(&session.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nonexistent_session() {
        let db = Db::open_in_memory().unwrap();
        let result = db.delete_session("nonexistent-id");
        assert!(result.is_err());
    }
}
