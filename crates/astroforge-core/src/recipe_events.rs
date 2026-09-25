//! CR-08 §23 Recipe Events.
//!
//! A typed, sqlite-backed append-only event log
//! for Recipe lifecycle changes. Each row records
//! one event with a stable `event_id` (UUID-like
//! monotonic), the event kind (a closed enum so
//! the kind set cannot drift), the optional
//! `profile_id` + `version` anchor, and a freeform
//! JSON payload. The store is read-only from the
//! IPC layer: `recipe_list_events` queries it
//! with optional filters; the emitters live in
//! the IPC handlers (see `src-tauri/src/main.rs`).
//!
//! Schema lives in `db::RECIPE_EVENTS_SCHEMA_SQL`.
//! The store is opened alongside `RecipeStore`
//! in the same SQLite file so the events share
//! the same `recipes` lifecycle.
//!
//! Slice F scope:
//! - 15 §23 event kinds defined.
//! - 11 are wired into IPC handlers (Slice F).
//! - 3 (`RecipeValidated`, `RecipeAdaptationProposed`,
//!   `RecipeAdaptationAccepted`) are NOT wired in
//!   Slice F because no surface to hook them to
//!   exists yet: `RecipeValidated` has no dedicated
//!   validator IPC (§20 `validate_compatibility`
//!   lives inside `recipe_apply` and is implicit),
//!   and `RecipeAdaptationProposed/Accepted` require
//!   the §28 adapt-recipe IPC (§22 `adapt_recipe`
//!   row is ⚠️ Partial — `derive_adaptive_parameters`
//!   is engine-side only; no IPC).
//! - 1 (`ProvenanceCreated`) lives in the
//!   `astroforge-core::provenance` module, not
//!   here, because the `provenance_events` table
//!   predates §23.
//!
//! This module is purely additive: no existing
//! struct or table is touched. Existing
//! `RecipeStore` + `DomainStore` callers are
//! unchanged.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// SQLite schema for the `recipe_events` table
/// plus the supporting indexes. Append-only; rows
/// are never updated or deleted.
pub const RECIPE_EVENTS_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS recipe_events (
    event_id      INTEGER PRIMARY KEY AUTOINCREMENT,
    occurred_at   TEXT NOT NULL,
    kind          TEXT NOT NULL,
    profile_id    TEXT,
    version       INTEGER,
    payload_json  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recipe_events_occurred_at
    ON recipe_events(occurred_at);

CREATE INDEX IF NOT EXISTS idx_recipe_events_kind
    ON recipe_events(kind);

CREATE INDEX IF NOT EXISTS idx_recipe_events_profile
    ON recipe_events(profile_id, version);
"#;

/// The 15 closed event kinds defined by CR-08 §23.
/// New variants are added at the end (do NOT
/// reorder: `recipe_events.kind` rows use the
/// stable `as_ref()` string).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeEventKind {
    RecipeCreated,
    RecipeUpdated,
    RecipeVersionCreated,
    RecipeValidated,
    RecipeApplicabilityEvaluated,
    RecipeAdaptationProposed,
    RecipeAdaptationAccepted,
    RecipeApplied,
    RecipeImportStarted,
    RecipeImportCompleted,
    RecipeImportFailed,
    RecipeExported,
    PipelineSavedAsRecipe,
    ProvenanceCreated,
    ReproducibilityRecordCreated,
}

impl RecipeEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RecipeCreated => "recipe_created",
            Self::RecipeUpdated => "recipe_updated",
            Self::RecipeVersionCreated => "recipe_version_created",
            Self::RecipeValidated => "recipe_validated",
            Self::RecipeApplicabilityEvaluated => "recipe_applicability_evaluated",
            Self::RecipeAdaptationProposed => "recipe_adaptation_proposed",
            Self::RecipeAdaptationAccepted => "recipe_adaptation_accepted",
            Self::RecipeApplied => "recipe_applied",
            Self::RecipeImportStarted => "recipe_import_started",
            Self::RecipeImportCompleted => "recipe_import_completed",
            Self::RecipeImportFailed => "recipe_import_failed",
            Self::RecipeExported => "recipe_exported",
            Self::PipelineSavedAsRecipe => "pipeline_saved_as_recipe",
            Self::ProvenanceCreated => "provenance_created",
            Self::ReproducibilityRecordCreated => "reproducibility_record_created",
        }
    }
}

/// One row in the `recipe_events` table. The
/// `payload_json` carries the event-specific
/// fields the consumer cares about (see the
/// `payload_for_*` constructors below).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeEvent {
    pub event_id: i64,
    pub occurred_at: String,
    pub kind: String,
    pub profile_id: Option<String>,
    pub version: Option<u32>,
    pub payload: serde_json::Value,
}

/// Filters for `RecipeEventStore::list_events`.
/// Every field is optional so callers supply only
/// the dimensions they know. New dimensions are
/// additive (old callers render unknown ids as
/// "Unknown filter" and the list still works).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecipeEventFilter {
    pub profile_id: Option<String>,
    pub kind: Option<String>,
    pub since: Option<String>,
    pub limit: Option<u32>,
}

/// Errors raised by the event store. The
/// variants mirror `RecipeStoreError` so the
/// IPC layer can collapse them into a single
/// `CommandError::from` conversion.
#[derive(Debug)]
pub enum RecipeEventStoreError {
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
}

impl std::fmt::Display for RecipeEventStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "recipe event sqlite: {e}"),
            Self::Json(e) => write!(f, "recipe event json: {e}"),
            Self::Io(e) => write!(f, "recipe event io: {e}"),
        }
    }
}

impl std::error::Error for RecipeEventStoreError {}

impl From<rusqlite::Error> for RecipeEventStoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

impl From<serde_json::Error> for RecipeEventStoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<std::io::Error> for RecipeEventStoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// The recipe event store. Opens the SQLite
/// connection on the same file as `RecipeStore`
/// so the event log + the recipes share the same
/// DB lifecycle. The store is thread-safe via the
/// internal `Mutex<Connection>`.
pub struct RecipeEventStore {
    conn: Mutex<Connection>,
}

impl RecipeEventStore {
    pub fn new(db_path: &PathBuf) -> Result<Self, RecipeEventStoreError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(RECIPE_EVENTS_SCHEMA_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Append one event to the log. Returns the
    /// `event_id` SQLite assigned (monotonic).
    /// Pure I/O; the caller (IPC handler) supplies
    /// the `occurred_at` timestamp so the event
    /// store stays free of wall-clock coupling.
    pub fn record_event(
        &self,
        kind: RecipeEventKind,
        profile_id: Option<&str>,
        version: Option<u32>,
        payload: serde_json::Value,
        occurred_at: &str,
    ) -> Result<i64, RecipeEventStoreError> {
        let conn = self.conn.lock().expect("recipe events db mutex poisoned");
        let payload_json = serde_json::to_string(&payload)?;
        conn.execute(
            "INSERT INTO recipe_events \
             (occurred_at, kind, profile_id, version, payload_json) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                occurred_at,
                kind.as_str(),
                profile_id,
                version.map(|v| v as i64),
                payload_json,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// List events, optionally filtered by
    /// `profile_id`, `kind`, `since` (ISO timestamp
    /// string), and `limit` (default 100; max 1000
    /// to bound reads). Returns rows newest-first
    /// (highest `event_id` first).
    pub fn list_events(
        &self,
        filter: &RecipeEventFilter,
    ) -> Result<Vec<RecipeEvent>, RecipeEventStoreError> {
        let conn = self.conn.lock().expect("recipe events db mutex poisoned");

        // Build the WHERE clause incrementally so
        // we only bind the filters that are set.
        let mut sql = String::from(
            "SELECT event_id, occurred_at, kind, profile_id, version, payload_json \
             FROM recipe_events WHERE 1=1",
        );
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(pid) = &filter.profile_id {
            sql.push_str(" AND profile_id = ?");
            binds.push(Box::new(pid.clone()));
        }
        if let Some(k) = &filter.kind {
            sql.push_str(" AND kind = ?");
            binds.push(Box::new(k.clone()));
        }
        if let Some(s) = &filter.since {
            sql.push_str(" AND occurred_at >= ?");
            binds.push(Box::new(s.clone()));
        }
        sql.push_str(" ORDER BY event_id DESC");
        let limit = filter.limit.unwrap_or(100).min(1000);
        sql.push_str(" LIMIT ?");
        binds.push(Box::new(limit as i64));

        let binds_refs: Vec<&dyn rusqlite::ToSql> = binds
            .iter()
            .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
            .collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&*binds_refs, |row| {
            let payload_str: String = row.get(5)?;
            Ok(RecipeEvent {
                event_id: row.get(0)?,
                occurred_at: row.get(1)?,
                kind: row.get(2)?,
                profile_id: row.get(3)?,
                version: row.get::<_, Option<i64>>(4)?.map(|v| v as u32),
                payload: serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null),
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Test-only: count events. Not exposed via
    /// IPC; the list endpoint is the public surface.
    #[doc(hidden)]
    pub fn count_events(&self) -> Result<i64, RecipeEventStoreError> {
        let conn = self.conn.lock().expect("recipe events db mutex poisoned");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM recipe_events", [], |row| row.get(0))?;
        Ok(n)
    }
}

// ─── Payload constructors ───────────────────────────────────────────────
//
// Each event kind has a small constructor that
// produces the canonical `serde_json::Value`
// payload. Keeping the constructors here (next
// to the store) ensures the IPC handler payload
// shape stays consistent with what the consumer
// reads back via `recipe_list_events`.

/// `RecipeCreated` payload: `{"name": ..., "target_type": ..., "version": N}`.
pub fn payload_recipe_created(name: &str, target_type: &str, version: u32) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "target_type": target_type,
        "version": version,
    })
}

/// `RecipeVersionCreated` payload: `{"parent_version": N}`.
pub fn payload_recipe_version_created(parent_version: Option<u32>) -> serde_json::Value {
    serde_json::json!({
        "parent_version": parent_version,
    })
}

/// `RecipeUpdated` payload: `{"fields": ["name", "description"]}` (best-effort).
pub fn payload_recipe_updated(fields: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "fields": fields,
    })
}

/// `RecipeValidated` payload: `{"verdict": "ok"|"warn"|"fail", "notes": [...]}`.
pub fn payload_recipe_validated(verdict: &str, notes: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "verdict": verdict,
        "notes": notes,
    })
}

/// `RecipeApplicabilityEvaluated` payload:
/// `{"verdict": "compatible"|"adaptable"|..., "warnings": [...]}`.
pub fn payload_recipe_applicability_evaluated(
    verdict: &str,
    warnings: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "verdict": verdict,
        "warnings": warnings,
    })
}

/// `RecipeAdaptationProposed` payload:
/// `{"stage_id": ..., "params": {...}}`.
pub fn payload_recipe_adaptation_proposed(
    stage_id: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "stage_id": stage_id,
        "params": params,
    })
}

/// `RecipeAdaptationAccepted` payload:
/// `{"stage_id": ..., "params": {...}}`.
pub fn payload_recipe_adaptation_accepted(
    stage_id: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "stage_id": stage_id,
        "params": params,
    })
}

/// `RecipeApplied` payload:
/// `{"image_version_id": ..., "available_models": [...]}`.
pub fn payload_recipe_applied(
    image_version_id: &str,
    available_models: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "image_version_id": image_version_id,
        "available_models": available_models,
    })
}

/// `RecipeImportStarted` payload: `{"source": "path-or-url"}`.
pub fn payload_recipe_import_started(source: &str) -> serde_json::Value {
    serde_json::json!({
        "source": source,
    })
}

/// `RecipeImportCompleted` payload:
/// `{"name": ..., "target_type": ..., "version": N}`.
pub fn payload_recipe_import_completed(
    name: &str,
    target_type: &str,
    version: u32,
) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "target_type": target_type,
        "version": version,
    })
}

/// `RecipeImportFailed` payload: `{"error": "..."}`.
pub fn payload_recipe_import_failed(error: &str) -> serde_json::Value {
    serde_json::json!({
        "error": error,
    })
}

/// `RecipeExported` payload:
/// `{"destination": "path-or-url", "bytes": N}`.
pub fn payload_recipe_exported(destination: &str, bytes: u64) -> serde_json::Value {
    serde_json::json!({
        "destination": destination,
        "bytes": bytes,
    })
}

/// `PipelineSavedAsRecipe` payload:
/// `{"source": "plan"|"stage_runs", "source_id": ..., "name": ...}`.
pub fn payload_pipeline_saved_as_recipe(
    source: &str,
    source_id: &str,
    name: &str,
) -> serde_json::Value {
    serde_json::json!({
        "source": source,
        "source_id": source_id,
        "name": name,
    })
}

/// `ReproducibilityRecordCreated` payload:
/// `{"version_id": ..., "verdict": "exact"|"material"|"indeterminate"}`.
pub fn payload_reproducibility_record_created(
    version_id: &str,
    verdict: &str,
) -> serde_json::Value {
    serde_json::json!({
        "version_id": version_id,
        "verdict": verdict,
    })
}

// ─── Pure helper ─────────────────────────────────────────────────────────
//
// `now_iso()` returns the current UTC timestamp
// in the canonical `YYYY-MM-DDTHH:MM:SSZ` form.
// Lives here (next to the store) so every event
// shares the same clock shape, and so the
// function can be mocked from tests via
// `with_clock` indirection (added in a follow-on
// slice if determinism matters for snapshots).
pub fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // Coarse UTC ISO-8601 timestamp:
    // `YYYY-MM-DDTHH:MM:SSZ`. Seconds-only is
    // sufficient for an event log; sub-second
    // precision is not promised by §23.
    let day_secs = secs.rem_euclid(86_400);
    let h = day_secs / 3600;
    let m = (day_secs % 3600) / 60;
    let s = day_secs % 60;
    let days = secs.div_euclid(86_400);
    civil_from_days(days)
        .map(|c| {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                c.year, c.month, c.day, h, m, s,
            )
        })
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
}

// Howard Hinnant's `days_from_civil` algorithm
// (public-domain). Used to convert a day count
// since the Unix epoch into a (year, month, day)
// tuple without pulling in `chrono`.
struct CivilDate {
    year: i64,
    month: u32,
    day: u32,
}

fn civil_from_days(z: i64) -> Option<CivilDate> {
    let z = z + 719_468;
    let era = z.div_euclid(400);
    let doe = z.rem_euclid(400);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    if !(1..=9999).contains(&y) {
        return None;
    }
    Some(CivilDate {
        year: y,
        month: m,
        day: d,
    })
}
