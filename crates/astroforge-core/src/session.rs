use crate::db;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct SessionStore {
    conn: Mutex<Connection>,
}

impl SessionStore {
    pub fn new(db_path: &PathBuf) -> Result<Self, SessionError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(db::SESSION_SCHEMA_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn create_project(
        &self,
        name: &str,
        target_type: Option<&str>,
    ) -> Result<String, SessionError> {
        let id = format!("proj_{}_{}", timestamp(), unique_nonce());
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, target_type) VALUES (?1, ?2, ?3)",
            params![id, name, target_type],
        )?;
        Ok(id)
    }

    pub fn create_session(
        &self,
        project_id: &str,
        source_dir: Option<&str>,
        verbosity: &str,
    ) -> Result<String, SessionError> {
        let id = format!("sess_{}_{}", timestamp(), unique_nonce());
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, project_id, source_dir, verbosity, status) VALUES (?1, ?2, ?3, ?4, 'created')",
            params![id, project_id, source_dir, verbosity],
        )?;
        Ok(id)
    }

    pub fn record_stage_run(
        &self,
        session_id: &str,
        stage_id: &str,
        status: &str,
        params_json: Option<&str>,
        metrics_json: Option<&str>,
        error: Option<&str>,
    ) -> Result<i64, SessionError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO stage_runs (session_id, stage_id, status, params_json, metrics_json, error, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
            params![session_id, stage_id, status, params_json, metrics_json, error],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn complete_stage_run(&self, run_id: i64, status: &str) -> Result<(), SessionError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE stage_runs SET status = ?1, completed_at = datetime('now') WHERE id = ?2",
            params![status, run_id],
        )?;
        Ok(())
    }

    pub fn save_checkpoint(
        &self,
        session_id: &str,
        stage_id: &str,
        artifact_path: &str,
    ) -> Result<i64, SessionError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO checkpoints (session_id, stage_id, artifact_path) VALUES (?1, ?2, ?3)",
            params![session_id, stage_id, artifact_path],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_latest_checkpoint(&self, session_id: &str) -> Option<CheckpointInfo> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, stage_id, artifact_path, created_at
             FROM checkpoints WHERE session_id = ?1 ORDER BY id DESC LIMIT 1",
            )
            .ok()?;

        stmt.query_row(params![session_id], |row| {
            Ok(CheckpointInfo {
                id: row.get(0)?,
                session_id: row.get(1)?,
                stage_id: row.get(2)?,
                artifact_path: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .ok()
    }

    pub fn get_session_status(&self, session_id: &str) -> Option<SessionStatus> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, status FROM sessions WHERE id = ?1")
            .ok()?;

        stmt.query_row(params![session_id], |row| {
            Ok(SessionStatus {
                session_id: row.get(0)?,
                status: row.get(1)?,
            })
        })
        .ok()
    }

    pub fn set_session_status(&self, session_id: &str, status: &str) -> Result<(), SessionError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![status, session_id],
        )?;
        Ok(())
    }

    pub fn find_interrupted_sessions(&self) -> Vec<String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare("SELECT id FROM sessions WHERE status = 'running'") {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let rows = stmt.query_map([], |row| row.get::<_, String>(0));
        match rows {
            Ok(r) => r.filter_map(|r| r.ok()).collect(),
            Err(_) => vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    pub id: i64,
    pub session_id: String,
    pub stage_id: String,
    pub artifact_path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatus {
    pub session_id: String,
    pub status: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

fn timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// Monotonic counter for IDs that are created within the same millisecond
// (which the system clock cannot disambiguate). Combined with the timestamp,
// it guarantees uniqueness even when two sessions/projects are created back-to-back.
fn unique_nonce() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NONCE: AtomicU32 = AtomicU32::new(0);
    NONCE.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_store_crud() {
        let db_path = PathBuf::from(":memory:");
        let store = SessionStore::new(&db_path).unwrap();

        let proj_id = store
            .create_project("Test Project", Some("deep_sky"))
            .unwrap();
        assert!(proj_id.starts_with("proj_"));

        let sess_id = store
            .create_session(&proj_id, Some("/test/dir"), "beginner")
            .unwrap();
        assert!(sess_id.starts_with("sess_"));

        let run_id = store
            .record_stage_run(&sess_id, "ingest", "running", None, None, None)
            .unwrap();
        assert!(run_id > 0);

        store.complete_stage_run(run_id, "completed").unwrap();

        let ckpt_id = store
            .save_checkpoint(&sess_id, "ingest", "/artifacts/ingest.fits")
            .unwrap();
        assert!(ckpt_id > 0);

        let latest = store.get_latest_checkpoint(&sess_id);
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().stage_id, "ingest");

        let status = store.get_session_status(&sess_id).unwrap();
        assert_eq!(status.status, "created");

        store.set_session_status(&sess_id, "running").unwrap();
        let interrupted = store.find_interrupted_sessions();
        assert!(interrupted.contains(&sess_id));
    }

    #[test]
    fn test_crash_recovery_detection() {
        let db_path = PathBuf::from(":memory:");
        let store = SessionStore::new(&db_path).unwrap();

        let proj_id = store.create_project("Test", None).unwrap();
        let sess1 = store.create_session(&proj_id, None, "beginner").unwrap();
        let sess2 = store.create_session(&proj_id, None, "beginner").unwrap();

        store.set_session_status(&sess1, "running").unwrap();
        store.set_session_status(&sess2, "completed").unwrap();

        let interrupted = store.find_interrupted_sessions();
        assert_eq!(interrupted.len(), 1);
        assert_eq!(interrupted[0], sess1);
    }
}
