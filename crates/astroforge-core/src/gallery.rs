//! Local gallery store backed by rusqlite.
//!
//! Phase 4: replaces the placeholder data in `src/lib/gallery.ts` with
//! real local persistence. The DB lives at a path chosen by the Tauri
//! runtime (typically `<app_data_dir>/gallery.sqlite`) and is created
//! on first open. Placeholder rows are seeded automatically so the UI
//! is never empty after merge.
//!
//! Schema lives in `db::GALLERY_SCHEMA_SQL` (separate from the session
//! schema so the two stores can move independently). Migration is run
//! on `new()`.
//!
//! Design decisions:
//! - Owns its own DB connection — no coupling to SessionStore.
//! - Status is an enum-style string ('pending' | 'processing' |
//!   'completed') for forward compatibility; we can switch to a
//!   CHECK constraint later if needed.
//! - updated_at is set server-side (datetime('now')) on every upsert.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::db;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GalleryStatus {
    Pending,
    Processing,
    Completed,
}

impl GalleryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            GalleryStatus::Pending => "pending",
            GalleryStatus::Processing => "processing",
            GalleryStatus::Completed => "completed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(GalleryStatus::Pending),
            "processing" => Some(GalleryStatus::Processing),
            "completed" => Some(GalleryStatus::Completed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GalleryItem {
    pub id: String,
    pub name: String,
    pub target: String,
    pub integration_hours: f64,
    pub palette: String,
    pub status: GalleryStatus,
    pub updated_at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GalleryError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown status string: {0}")]
    UnknownStatus(String),
}

impl GalleryError {
    pub fn unknown_status(s: &str) -> Self {
        GalleryError::UnknownStatus(s.to_string())
    }
}

/// Parameters used when adding or updating a gallery item. `id` is
/// optional: when absent, the store will mint a new one. `updated_at`
/// is always server-side — caller-supplied values are ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalleryItemUpdate {
    pub id: Option<String>,
    pub name: String,
    pub target: String,
    pub integration_hours: f64,
    pub palette: String,
    pub status: GalleryStatus,
}

pub struct GalleryStore {
    conn: Mutex<Connection>,
}

fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<GalleryItem> {
    let status_str: String = row.get(5)?;
    let status = GalleryStatus::parse(&status_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Text,
            Box::new(GalleryError::unknown_status(&status_str)),
        )
    })?;
    Ok(GalleryItem {
        id: row.get(0)?,
        name: row.get(1)?,
        target: row.get(2)?,
        integration_hours: row.get(3)?,
        palette: row.get(4)?,
        status,
        updated_at: row.get(6)?,
    })
}

impl GalleryStore {
    pub fn new(db_path: &PathBuf) -> Result<Self, GalleryError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(db::GALLERY_SCHEMA_SQL)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        // Seed on first open so the UI is never empty.
        store.seed_if_empty()?;
        Ok(store)
    }

    pub fn list(&self) -> Result<Vec<GalleryItem>, GalleryError> {
        let conn = self.conn.lock().expect("gallery db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, name, target, integration_hours, palette, status, updated_at \
             FROM gallery_items \
             ORDER BY updated_at DESC, name ASC",
        )?;
        let rows = stmt
            .query_map([], row_to_item)?
            .collect::<Result<Vec<_>, rusqlite::Error>>()?;
        Ok(rows)
    }

    pub fn upsert(&self, update: GalleryItemUpdate) -> Result<GalleryItem, GalleryError> {
        let conn = self.conn.lock().expect("gallery db mutex poisoned");
        let id = match update.id.as_ref() {
            Some(id) if !id.is_empty() => id.clone(),
            _ => format!("gallery_{}", timestamp()),
        };
        conn.execute(
            "INSERT INTO gallery_items \
                (id, name, target, integration_hours, palette, status, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now')) \
             ON CONFLICT(id) DO UPDATE SET \
                name = excluded.name, \
                target = excluded.target, \
                integration_hours = excluded.integration_hours, \
                palette = excluded.palette, \
                status = excluded.status, \
                updated_at = datetime('now')",
            params![
                id,
                update.name,
                update.target,
                update.integration_hours,
                update.palette,
                update.status.as_str(),
            ],
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, name, target, integration_hours, palette, status, updated_at \
             FROM gallery_items WHERE id = ?1",
        )?;
        let row = stmt.query_row([&id], row_to_item)?;
        Ok(row)
    }

    pub fn delete(&self, id: &str) -> Result<(), GalleryError> {
        let conn = self.conn.lock().expect("gallery db mutex poisoned");
        conn.execute("DELETE FROM gallery_items WHERE id = ?1", [id])?;
        Ok(())
    }

    fn seed_if_empty(&self) -> Result<(), GalleryError> {
        let conn = self.conn.lock().expect("gallery db mutex poisoned");
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM gallery_items", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }
        // Mirror the previous Svelte-side placeholders so the user
        // sees the same five tiles on first launch.
        let seeds: [(&str, &str, &str, f64, &str, GalleryStatus); 5] = [
            (
                "placeholder-m42",
                "M42_L-PRO_BIN1",
                "M42",
                7.36,
                "LRGB",
                GalleryStatus::Completed,
            ),
            (
                "placeholder-m31",
                "M31_SHO_V2_FINAL",
                "M31",
                13.75,
                "SHO",
                GalleryStatus::Completed,
            ),
            (
                "placeholder-ngc7000",
                "NGC7000_NARROW",
                "NGC 7000",
                4.2,
                "H\u{03b1}",
                GalleryStatus::Processing,
            ),
            (
                "placeholder-ic1396",
                "IC1396_HA_OIII",
                "IC 1396",
                8.5,
                "HSO",
                GalleryStatus::Pending,
            ),
            (
                "placeholder-horsehead",
                "Horsehead_NB",
                "IC 434",
                5.23,
                "H\u{03b1} + OIII",
                GalleryStatus::Pending,
            ),
        ];
        for (id, name, target, hours, palette, status) in seeds {
            conn.execute(
                "INSERT INTO gallery_items \
                    (id, name, target, integration_hours, palette, status, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
                params![id, name, target, hours, palette, status.as_str()],
            )?;
        }
        Ok(())
    }
}

fn timestamp() -> String {
    // Millisecond-resolution monotonic counter.
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_millis())
}
