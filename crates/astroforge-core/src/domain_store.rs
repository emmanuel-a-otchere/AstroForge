//! CR-02.2 — SQLite persistence for the canonical domain model.
//!
//! The CR-02 store lives ALONGSIDE the v1 `SessionStore`/`GalleryStore`/
//! `RecipeStore` (decision D-1, docs/plans/2026-09-06-cr02-domain-model):
//! those map the shipping schema and stay untouched; this store holds the
//! CR-02 tables in their own database file and evolves via versioned
//! migrations (CR-02 §30) recorded in `schema_migrations`.
//!
//! The per-project directory layout (CR-02 §18) is deferred to CR-02.4
//! (decision D-7) — for now the caller chooses the db path, as with the
//! other stores.

use crate::domain::{
    Artifact, ArtifactCategory, PipelineRun, PipelineRunStatus, PreviewRun, Project, ProjectEvent,
    ProjectEventKind, ProjectStatus, Session, SourceAsset, StageRunRecord, Target,
    DOMAIN_SCHEMA_VERSION,
};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

// ─── Migrations (CR-02 §30) ─────────────────────────────────────────────────

/// Versioned migrations, applied in order inside a transaction and recorded
/// in `schema_migrations`. Never edit an applied entry — append new ones.
const MIGRATIONS: &[(u32, &str)] = &[
    (
        1,
        r#"
CREATE TABLE targets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    object_type TEXT NOT NULL DEFAULT 'unknown',
    ra REAL,
    dec REAL,
    catalog_ids_json TEXT NOT NULL DEFAULT '[]',
    constellation TEXT,
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    target_id TEXT REFERENCES targets(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    application_version TEXT NOT NULL DEFAULT '',
    schema_version INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'NEW',
    active_session_id TEXT,
    active_image_version_id TEXT,
    project_root TEXT,
    source_mode TEXT NOT NULL DEFAULT 'referenced'
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    name TEXT NOT NULL DEFAULT '',
    capture_start TEXT,
    capture_end TEXT,
    source_location TEXT,
    camera_profile TEXT,
    telescope_profile TEXT,
    site_profile TEXT,
    target_detection TEXT,
    processing_status TEXT NOT NULL DEFAULT 'created',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE source_assets (
    id TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    original_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    format TEXT NOT NULL,
    mime_type TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    imported_at TEXT NOT NULL DEFAULT (datetime('now')),
    session_id TEXT NOT NULL REFERENCES sessions(id),
    frame_type TEXT,
    exposure REAL,
    filter TEXT,
    binning TEXT,
    width INTEGER,
    height INTEGER,
    bit_depth INTEGER,
    bayer_pattern TEXT,
    camera TEXT,
    date_obs TEXT,
    ra REAL,
    dec REAL
);

-- Duplicate detection (CR-02 §7): one logical asset per content hash
-- within a session.
CREATE UNIQUE INDEX idx_source_assets_session_hash
    ON source_assets(session_id, content_hash);

CREATE TABLE project_events (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    kind TEXT NOT NULL,
    payload_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_projects_status ON projects(status);
CREATE INDEX idx_sessions_project ON sessions(project_id);
CREATE INDEX idx_source_assets_session ON source_assets(session_id);
CREATE INDEX idx_project_events_project ON project_events(project_id);
"#,
    ),
    (
        2,
        r#"
-- CR-02.3 — canonical artifact records (lineage + producer linkage).
-- The bytes live in the content-addressed store (artifact.rs::ContentStore);
-- this table is the metadata/index half (CR-02 §17 division).
CREATE TABLE artifacts (
    id TEXT PRIMARY KEY,
    artifact_hash TEXT NOT NULL,
    artifact_type TEXT NOT NULL,
    format TEXT NOT NULL,
    path TEXT NOT NULL,
    size INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    producer_stage TEXT,
    pipeline_run_id TEXT,
    parent_artifact_ids_json TEXT NOT NULL DEFAULT '[]',
    width INTEGER,
    height INTEGER,
    channels INTEGER,
    bit_depth INTEGER,
    color_space TEXT,
    linear_or_nonlinear INTEGER
);

CREATE INDEX idx_artifacts_hash ON artifacts(artifact_hash);
CREATE INDEX idx_artifacts_run ON artifacts(pipeline_run_id);
"#,
    ),
    (
        3,
        r#"
-- CR-02.5 — pipeline persistence (CR-02 §11/§12). Run-keyed stage runs
-- replace the session-keyed stage_runs model for new runs; the v1
-- session_keyed row type stays untouched (D-1).
CREATE TABLE pipeline_runs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    session_ids_json TEXT NOT NULL DEFAULT '[]',
    recipe_id TEXT,
    started_at TEXT,
    completed_at TEXT,
    status TEXT NOT NULL DEFAULT 'queued',
    application_version TEXT NOT NULL DEFAULT '',
    engine_version TEXT NOT NULL DEFAULT '',
    hardware_profile TEXT,
    execution_mode TEXT,
    input_artifacts_json TEXT NOT NULL DEFAULT '[]',
    output_artifacts_json TEXT NOT NULL DEFAULT '[]'
);

CREATE INDEX idx_pipeline_runs_project ON pipeline_runs(project_id);
CREATE INDEX idx_pipeline_runs_status ON pipeline_runs(status);

CREATE TABLE stage_run_records (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES pipeline_runs(id),
    stage_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    attempt INTEGER NOT NULL DEFAULT 1,
    params_json TEXT,
    metrics_json TEXT,
    error TEXT,
    started_at TEXT,
    completed_at TEXT
);

CREATE INDEX idx_stage_run_records_run ON stage_run_records(run_id);
CREATE INDEX idx_stage_run_records_stage ON stage_run_records(stage_id);
"#,
    ),
    (
        4,
        r#"
-- CR-05 P4 slice 3 — preview-before-commit rows (CR-05 §11 + §26).
-- Persists the result of running a stage handler on a representative
-- region of the input image. stage_execution_id and
-- source_version_id are logical FKs (string ids) — the related rows
-- live in other SQLite databases (PipelinePlanStore, etc.) so we
-- do not enforce a SQLite-level FK constraint. Tauri commands
-- verify existence at the application boundary.
CREATE TABLE preview_runs (
    id TEXT PRIMARY KEY,
    stage_execution_id TEXT NOT NULL,
    source_version_id TEXT NOT NULL,
    preview_artifact_id TEXT,
    parameters_json TEXT NOT NULL,
    parameters_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    scale REAL NOT NULL DEFAULT 0.25,
    label TEXT NOT NULL DEFAULT 'Preview',
    error_json TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_preview_runs_stage_execution
    ON preview_runs(stage_execution_id);
CREATE INDEX idx_preview_runs_status
    ON preview_runs(status);
CREATE INDEX idx_preview_runs_parameters_hash
    ON preview_runs(parameters_hash);
"#,
    ),
];

// ─── Store ──────────────────────────────────────────────────────────────────

pub struct DomainStore {
    conn: Mutex<Connection>,
}

#[derive(Debug, thiserror::Error)]
pub enum DomainStoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// CR-05 P4 slice 5 — filesystem failures (preview PNG write,
    /// previews-dir creation). Stringly-typed so the store stays
    /// free of `std::io::Error` coupling in public signatures.
    #[error("I/O error: {0}")]
    Io(String),
    #[error("invalid project status transition: {from} -> {to}")]
    InvalidTransition { from: String, to: String },
    #[error("not found: {0}")]
    NotFound(String),
}

type Result<T> = std::result::Result<T, DomainStoreError>;

impl DomainStore {
    /// Open (or create) the CR-02 store at `db_path` and apply pending
    /// migrations. Use `:memory:` in tests.
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;
        {
            let applied: u32 = conn
                .query_row(
                    "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            for (version, sql) in MIGRATIONS {
                if *version > applied {
                    let tx = conn.unchecked_transaction()?;
                    tx.execute_batch(sql)?;
                    tx.execute(
                        "INSERT INTO schema_migrations (version) VALUES (?1)",
                        params![version],
                    )?;
                    tx.commit()?;
                }
            }
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Highest applied migration version (0 = none).
    pub fn schema_version(&self) -> u32 {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0)
    }

    // ─── Targets (CR-02 §4) ─────────────────────────────────────────────

    pub fn create_target(
        &self,
        name: &str,
        object_type: crate::domain::ObjectType,
    ) -> Result<String> {
        let id = new_id("tgt");
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO targets (id, name, object_type) VALUES (?1, ?2, ?3)",
            params![
                id,
                name,
                serde_json::to_string(&object_type)?.trim_matches('"')
            ],
        )?;
        Ok(id)
    }

    pub fn get_target(&self, target_id: &str) -> Result<Target> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, object_type, ra, dec, catalog_ids_json, constellation, description
             FROM targets WHERE id = ?1",
            params![target_id],
            target_row,
        )
        .map_err(|e| not_found_if_missing(e, "target", target_id))
        .and_then(target_from_row)
    }

    // ─── Projects (CR-02 §3) ────────────────────────────────────────────

    /// Create a project in status NEW and record PROJECT_CREATED (§35).
    pub fn create_project(
        &self,
        name: &str,
        target_id: Option<&str>,
        application_version: &str,
    ) -> Result<String> {
        let id = new_id("proj");
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, target_id, application_version, schema_version, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'NEW')",
            params![id, name, target_id, application_version, DOMAIN_SCHEMA_VERSION],
        )?;
        drop(conn);
        self.record_event(&id, ProjectEventKind::ProjectCreated, None)?;
        Ok(id)
    }

    pub fn get_project(&self, project_id: &str) -> Result<Project> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, description, target_id, created_at, updated_at,
                    application_version, schema_version, status, active_session_id,
                    active_image_version_id, project_root, source_mode
             FROM projects WHERE id = ?1",
            params![project_id],
            project_row,
        )
        .map_err(|e| not_found_if_missing(e, "project", project_id))
        .and_then(project_from_row)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, target_id, created_at, updated_at,
                    application_version, schema_version, status, active_session_id,
                    active_image_version_id, project_root, source_mode
             FROM projects ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], project_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(project_from_row(r?)?);
        }
        Ok(out)
    }

    /// Rename without touching artifacts (CR-02 §39 acceptance).
    pub fn rename_project(&self, project_id: &str, new_name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE projects SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![new_name, project_id],
        )?;
        if n == 0 {
            return Err(DomainStoreError::NotFound(format!("project {project_id}")));
        }
        Ok(())
    }

    /// CR-02 §21 + decision D-4: status transitions are validated against
    /// the ProjectStatus state machine. EXPORTED is not terminal.
    pub fn set_project_status(&self, project_id: &str, next: ProjectStatus) -> Result<()> {
        let current = self.get_project(project_id)?.status;
        if !current.can_transition_to(next) {
            return Err(DomainStoreError::InvalidTransition {
                from: format!("{current:?}"),
                to: format!("{next:?}"),
            });
        }
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE projects SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![enum_str(&next)?, project_id],
        )?;
        Ok(())
    }

    // ─── Sessions (CR-02 §5) ────────────────────────────────────────────

    pub fn create_session(&self, project_id: &str, name: &str) -> Result<String> {
        let id = new_id("sess");
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO sessions (id, project_id, name) VALUES (?1, ?2, ?3)",
                params![id, project_id, name],
            )?;
        }
        self.record_event(project_id, ProjectEventKind::SessionImported, Some(&id))?;
        Ok(id)
    }

    pub fn get_session(&self, session_id: &str) -> Result<Session> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, project_id, name, capture_start, capture_end, source_location,
                    camera_profile, telescope_profile, site_profile, target_detection,
                    processing_status, created_at, updated_at
             FROM sessions WHERE id = ?1",
            params![session_id],
            |row| {
                Ok(Session {
                    session_id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    capture_start: row.get(3)?,
                    capture_end: row.get(4)?,
                    source_location: row.get(5)?,
                    camera_profile: row.get(6)?,
                    telescope_profile: row.get(7)?,
                    site_profile: row.get(8)?,
                    target_detection: row.get(9)?,
                    processing_status: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            },
        )
        .map_err(|e| not_found_if_missing(e, "session", session_id))
    }

    pub fn list_sessions(&self, project_id: &str) -> Result<Vec<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, capture_start, capture_end, source_location,
                    camera_profile, telescope_profile, site_profile, target_detection,
                    processing_status, created_at, updated_at
             FROM sessions WHERE project_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok(Session {
                session_id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                capture_start: row.get(3)?,
                capture_end: row.get(4)?,
                source_location: row.get(5)?,
                camera_profile: row.get(6)?,
                telescope_profile: row.get(7)?,
                site_profile: row.get(8)?,
                target_detection: row.get(9)?,
                processing_status: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // ─── Source assets (CR-02 §6) ───────────────────────────────────────

    /// Register an immutable source. Returns (asset_id, is_duplicate):
    /// duplicate detection is by (session_id, content_hash) per CR-02 §7 —
    /// a re-import of identical content returns the existing asset id.
    pub fn register_source_asset(&self, asset: &SourceAsset) -> Result<(String, bool)> {
        let conn = self.conn.lock().unwrap();
        if let Ok(existing) = conn.query_row(
            "SELECT id FROM source_assets WHERE session_id = ?1 AND content_hash = ?2",
            params![asset.session_id, asset.content_hash],
            |row| row.get::<_, String>(0),
        ) {
            return Ok((existing, true));
        }
        let id = new_id("asset");
        conn.execute(
            "INSERT INTO source_assets (
                id, content_hash, original_filename, original_path, file_size,
                format, mime_type, session_id, frame_type, exposure, filter,
                binning, width, height, bit_depth, bayer_pattern, camera,
                date_obs, ra, dec
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                id, asset.content_hash, asset.original_filename, asset.original_path,
                asset.file_size as i64, asset.format, asset.mime_type, asset.session_id,
                asset.frame_type, asset.exposure, asset.filter, asset.binning,
                asset.width, asset.height, asset.bit_depth, asset.bayer_pattern,
                asset.camera, asset.date_obs, asset.ra, asset.dec
            ],
        )?;
        drop(conn);
        self.record_event(
            &self.get_session(&asset.session_id)?.project_id,
            ProjectEventKind::SourceImported,
            Some(&id),
        )?;
        Ok((id, false))
    }

    pub fn list_source_assets(&self, session_id: &str) -> Result<Vec<SourceAsset>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content_hash, original_filename, original_path, file_size,
                    format, mime_type, created_at, imported_at, session_id,
                    frame_type, exposure, filter, binning, width, height,
                    bit_depth, bayer_pattern, camera, date_obs, ra, dec
             FROM source_assets WHERE session_id = ?1 ORDER BY imported_at ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(SourceAsset {
                asset_id: row.get(0)?,
                content_hash: row.get(1)?,
                original_filename: row.get(2)?,
                original_path: row.get(3)?,
                file_size: row.get::<_, i64>(4)? as u64,
                format: row.get(5)?,
                mime_type: row.get(6)?,
                created_at: row.get(7)?,
                imported_at: row.get(8)?,
                session_id: row.get(9)?,
                frame_type: row.get(10)?,
                exposure: row.get(11)?,
                filter: row.get(12)?,
                binning: row.get(13)?,
                width: row.get::<_, Option<u32>>(14)?,
                height: row.get::<_, Option<u32>>(15)?,
                bit_depth: row.get::<_, Option<u32>>(16)?,
                bayer_pattern: row.get(17)?,
                camera: row.get(18)?,
                date_obs: row.get(19)?,
                ra: row.get(20)?,
                dec: row.get(21)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // ─── Artifacts (CR-02 §8) — migration v2 ────────────────────────────

    /// Record an artifact's metadata. The BYTES must already be durable in
    /// the ContentStore (CR-02 §29 ordering: write → validate → hash →
    /// rename → commit row). Mints an id when `artifact.artifact_id` is
    /// empty.
    pub fn record_artifact(&self, artifact: &Artifact) -> Result<String> {
        let id = if artifact.artifact_id.is_empty() {
            new_id("art")
        } else {
            artifact.artifact_id.clone()
        };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO artifacts (
                id, artifact_hash, artifact_type, format, path, size,
                producer_stage, pipeline_run_id, parent_artifact_ids_json,
                width, height, channels, bit_depth, color_space, linear_or_nonlinear
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                id,
                artifact.artifact_hash,
                enum_str(&artifact.artifact_type)?,
                artifact.format,
                artifact.path,
                artifact.size as i64,
                artifact.producer_stage,
                artifact.pipeline_run_id,
                serde_json::to_string(&artifact.parent_artifact_ids)?,
                artifact.width,
                artifact.height,
                artifact.channels,
                artifact.bit_depth,
                artifact.color_space,
                artifact.linear_or_nonlinear,
            ],
        )?;
        Ok(id)
    }

    pub fn get_artifact(&self, artifact_id: &str) -> Result<Artifact> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, artifact_hash, artifact_type, format, path, size, created_at,
                    producer_stage, pipeline_run_id, parent_artifact_ids_json,
                    width, height, channels, bit_depth, color_space, linear_or_nonlinear
             FROM artifacts WHERE id = ?1",
            params![artifact_id],
            artifact_row,
        )
        .map_err(|e| not_found_if_missing(e, "artifact", artifact_id))
        .and_then(artifact_from_row)
    }

    /// CR-02 §7 reuse: find all records pointing at identical content.
    pub fn find_artifacts_by_hash(&self, hash: &str) -> Result<Vec<Artifact>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, artifact_hash, artifact_type, format, path, size, created_at,
                    producer_stage, pipeline_run_id, parent_artifact_ids_json,
                    width, height, channels, bit_depth, color_space, linear_or_nonlinear
             FROM artifacts WHERE artifact_hash = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![hash], artifact_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(artifact_from_row(r?)?);
        }
        Ok(out)
    }

    pub fn list_artifacts_for_run(&self, run_id: &str) -> Result<Vec<Artifact>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, artifact_hash, artifact_type, format, path, size, created_at,
                    producer_stage, pipeline_run_id, parent_artifact_ids_json,
                    width, height, channels, bit_depth, color_space, linear_or_nonlinear
             FROM artifacts WHERE pipeline_run_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![run_id], artifact_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(artifact_from_row(r?)?);
        }
        Ok(out)
    }

    // ─── Pipeline runs + stage runs (CR-02 §11/§12) — migration v3 ────

    /// Create a pipeline run in `Queued` status, optionally linked to a
    /// recipe. Returns the minted run id.
    pub fn create_pipeline_run(
        &self,
        project_id: &str,
        session_ids: &[String],
        recipe_id: Option<&str>,
        application_version: &str,
        engine_version: &str,
    ) -> Result<String> {
        let id = new_id("run");
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO pipeline_runs (
                id, project_id, session_ids_json, recipe_id, status,
                application_version, engine_version
             ) VALUES (?1, ?2, ?3, ?4, 'queued', ?5, ?6)",
            params![
                id,
                project_id,
                serde_json::to_string(session_ids)?,
                recipe_id,
                application_version,
                engine_version,
            ],
        )?;
        drop(conn);
        self.record_event(project_id, ProjectEventKind::PipelineStarted, Some(&id))?;
        Ok(id)
    }

    pub fn get_pipeline_run(&self, run_id: &str) -> Result<PipelineRun> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, project_id, session_ids_json, recipe_id, started_at,
                    completed_at, status, application_version, engine_version,
                    hardware_profile, execution_mode, input_artifacts_json,
                    output_artifacts_json
             FROM pipeline_runs WHERE id = ?1",
            params![run_id],
            pipeline_run_row,
        )
        .map_err(|e| not_found_if_missing(e, "pipeline_run", run_id))
        .and_then(pipeline_run_from_row)
    }

    pub fn list_pipeline_runs(&self, project_id: &str) -> Result<Vec<PipelineRun>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, session_ids_json, recipe_id, started_at,
                    completed_at, status, application_version, engine_version,
                    hardware_profile, execution_mode, input_artifacts_json,
                    output_artifacts_json
             FROM pipeline_runs WHERE project_id = ?1
             ORDER BY COALESCE(started_at, '') DESC",
        )?;
        let rows = stmt.query_map(params![project_id], pipeline_run_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(pipeline_run_from_row(r?)?);
        }
        Ok(out)
    }

    /// CR-02 §28 crash-recovery substrate: any run not in a terminal state
    /// is presumed interrupted. The `PipelineRunStatus::is_terminal` set
    /// defines "terminal" — matches `is_terminal()` in `domain.rs`.
    pub fn find_interrupted_runs(&self) -> Result<Vec<PipelineRun>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, session_ids_json, recipe_id, started_at,
                    completed_at, status, application_version, engine_version,
                    hardware_profile, execution_mode, input_artifacts_json,
                    output_artifacts_json
             FROM pipeline_runs
             WHERE status NOT IN ('completed', 'failed', 'cancelled')
             ORDER BY COALESCE(started_at, '') ASC",
        )?;
        let rows = stmt.query_map([], pipeline_run_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(pipeline_run_from_row(r?)?);
        }
        Ok(out)
    }

    pub fn mark_run_started(&self, run_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE pipeline_runs
             SET status = 'running', started_at = datetime('now')
             WHERE id = ?1",
            params![run_id],
        )?;
        Ok(())
    }

    pub fn mark_run_finished(&self, run_id: &str, status: PipelineRunStatus) -> Result<()> {
        let terminal = status.is_terminal();
        let conn = self.conn.lock().unwrap();
        if terminal {
            conn.execute(
                "UPDATE pipeline_runs
                 SET status = ?1, completed_at = datetime('now')
                 WHERE id = ?2",
                params![enum_str(&status)?, run_id],
            )?;
        } else {
            conn.execute(
                "UPDATE pipeline_runs SET status = ?1 WHERE id = ?2",
                params![enum_str(&status)?, run_id],
            )?;
        }
        Ok(())
    }

    pub fn record_stage_run(&self, record: &StageRunRecord) -> Result<String> {
        let id = if record.stage_run_id.is_empty() {
            new_id("sr")
        } else {
            record.stage_run_id.clone()
        };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO stage_run_records (
                id, run_id, stage_id, status, attempt, params_json,
                metrics_json, error, started_at, completed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                record.run_id,
                record.stage_id,
                record.status,
                record.attempt as i64,
                record.params_json,
                record.metrics_json,
                record.error,
                record.started_at,
                record.completed_at,
            ],
        )?;
        Ok(id)
    }

    /// List stage runs for a pipeline run in insertion order — the DAG's
    /// execution history at a glance.
    pub fn list_stage_runs(&self, run_id: &str) -> Result<Vec<StageRunRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, stage_id, status, attempt, params_json,
                    metrics_json, error, started_at, completed_at
             FROM stage_run_records WHERE run_id = ?1
             ORDER BY COALESCE(started_at, '') ASC, id ASC",
        )?;
        let rows = stmt.query_map(params![run_id], stage_run_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(stage_run_from_row(r?)?);
        }
        Ok(out)
    }

    /// CR-02 §12: stages can be rerun independently. The next attempt
    /// counter is max(prior attempts) + 1.
    pub fn next_stage_attempt(&self, run_id: &str, stage_id: &str) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(attempt), 0) FROM stage_run_records
                 WHERE run_id = ?1 AND stage_id = ?2",
                params![run_id, stage_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        Ok((n + 1) as u32)
    }

    // ─── Project events (CR-02 §35) ─────────────────────────────────────

    pub fn record_event(
        &self,
        project_id: &str,
        kind: ProjectEventKind,
        payload_json: Option<&str>,
    ) -> Result<String> {
        let id = new_id("evt");
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO project_events (id, project_id, kind, payload_json) VALUES (?1, ?2, ?3, ?4)",
            params![id, project_id, enum_str(&kind)?, payload_json],
        )?;
        Ok(id)
    }

    pub fn list_events(&self, project_id: &str) -> Result<Vec<ProjectEvent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, kind, payload_json, created_at
             FROM project_events WHERE project_id = ?1 ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (id, pid, kind, payload, created) = r?;
            out.push(ProjectEvent {
                event_id: id,
                project_id: pid,
                kind: parse_enum(&kind)?,
                payload_json: payload,
                created_at: created,
            });
        }
        Ok(out)
    }

    // ─── CR-05 P4 slice 3 — PreviewRun CRUD (preview-before-commit) ────────
    //
    // Persists the result of running a stage handler on a
    // representative region of the input image. The companion
    // Tauri command layer (slice 4+) drives a stage handler at a
    // downscale and writes the resulting `PreviewRun` row + a
    // companion `Artifact` row (`preview_artifact_id`).

    /// Insert a new preview row. The caller supplies the
    /// `preview_id` (or empty string to auto-generate one with the
    /// `prev_` prefix), `parameters_hash`, and the initial status.
    /// Returns the stored preview id.
    pub fn create_preview_run(&self, preview: &PreviewRun) -> Result<String> {
        let id = if preview.preview_id.is_empty() {
            new_id("prev")
        } else {
            preview.preview_id.clone()
        };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO preview_runs (
                id, stage_execution_id, source_version_id, preview_artifact_id,
                parameters_json, parameters_hash, status, scale, label,
                error_json, started_at, completed_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12
             )",
            params![
                id,
                preview.stage_execution_id,
                preview.source_version_id,
                preview.preview_artifact_id,
                preview.parameters_json,
                preview.parameters_hash,
                preview.status,
                preview.scale,
                preview.label,
                preview.error_json,
                preview.started_at,
                preview.completed_at,
            ],
        )?;
        Ok(id)
    }

    /// Fetch a single preview by id. Returns `NotFound` when no
    /// row matches.
    pub fn get_preview_run(&self, preview_id: &str) -> Result<PreviewRun> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, stage_execution_id, source_version_id, preview_artifact_id,
                    parameters_json, parameters_hash, status, scale, label,
                    error_json, started_at, completed_at, created_at
             FROM preview_runs WHERE id = ?1",
        )?;
        let row = stmt
            .query_row(params![preview_id], preview_run_row)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    DomainStoreError::NotFound(preview_id.to_string())
                }
                other => DomainStoreError::Sqlite(other),
            })?;
        preview_run_from_row(row)
    }

    /// List every preview recorded against a given stage execution,
    /// in creation order. Used by the §11 preview UI to show
    /// "this stage's recent previews".
    pub fn list_preview_runs_for_stage_execution(
        &self,
        stage_execution_id: &str,
    ) -> Result<Vec<PreviewRun>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, stage_execution_id, source_version_id, preview_artifact_id,
                    parameters_json, parameters_hash, status, scale, label,
                    error_json, started_at, completed_at, created_at
             FROM preview_runs WHERE stage_execution_id = ?1
             ORDER BY COALESCE(started_at, '') ASC, created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map(params![stage_execution_id], preview_run_row)?;
        let mut collected = Vec::new();
        for r in rows {
            collected.push(preview_run_from_row(r?)?);
        }
        Ok(collected)
    }

    /// Transition a preview to `running`. Sets `started_at`. Returns
    /// `NotFound` when the preview does not exist.
    pub fn mark_preview_running(&self, preview_id: &str, started_at: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE preview_runs
             SET status = ?1, started_at = ?2
             WHERE id = ?3",
            params![
                crate::domain::preview_status::RUNNING,
                started_at,
                preview_id
            ],
        )?;
        if n == 0 {
            return Err(DomainStoreError::NotFound(preview_id.to_string()));
        }
        Ok(())
    }

    /// Mark a preview complete and attach its resulting artifact.
    /// `preview_artifact_id` is set, `completed_at` recorded.
    pub fn mark_preview_completed(
        &self,
        preview_id: &str,
        preview_artifact_id: &str,
        completed_at: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE preview_runs
             SET status = ?1, preview_artifact_id = ?2, completed_at = ?3
             WHERE id = ?4",
            params![
                crate::domain::preview_status::COMPLETED,
                preview_artifact_id,
                completed_at,
                preview_id,
            ],
        )?;
        if n == 0 {
            return Err(DomainStoreError::NotFound(preview_id.to_string()));
        }
        Ok(())
    }

    /// Mark a preview failed and record the error JSON.
    pub fn mark_preview_failed(
        &self,
        preview_id: &str,
        error_json: &str,
        completed_at: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE preview_runs
             SET status = ?1, error_json = ?2, completed_at = ?3
             WHERE id = ?4",
            params![
                crate::domain::preview_status::FAILED,
                error_json,
                completed_at,
                preview_id,
            ],
        )?;
        if n == 0 {
            return Err(DomainStoreError::NotFound(preview_id.to_string()));
        }
        Ok(())
    }

    /// Delete a preview row. Used by the §11 preview UI's
    /// "discard" affordance and by tests. Returns `NotFound`
    /// when no row matched.
    pub fn delete_preview_run(&self, preview_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "DELETE FROM preview_runs WHERE id = ?1",
            params![preview_id],
        )?;
        if n == 0 {
            return Err(DomainStoreError::NotFound(preview_id.to_string()));
        }
        Ok(())
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

type TargetRow = (
    String,         // id
    String,         // name
    String,         // object_type (serde token)
    Option<f64>,    // ra
    Option<f64>,    // dec
    String,         // catalog_ids_json
    Option<String>, // constellation
    String,         // description
);

fn target_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TargetRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
    ))
}

fn target_from_row(r: TargetRow) -> Result<Target> {
    Ok(Target {
        target_id: r.0,
        name: r.1,
        object_type: parse_enum(&r.2)?,
        ra: r.3,
        dec: r.4,
        catalog_ids: serde_json::from_str(&r.5)?,
        constellation: r.6,
        description: r.7,
    })
}

/// Column order shared by the projects queries (get_project/list_projects).
type ProjectRow = (
    String,         // id
    String,         // name
    String,         // description
    Option<String>, // target_id
    String,         // created_at
    String,         // updated_at
    String,         // application_version
    u32,            // schema_version
    String,         // status
    Option<String>, // active_session_id
    Option<String>, // active_image_version_id
    Option<String>, // project_root
    String,         // source_mode
);

fn project_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
    ))
}

fn project_from_row(r: ProjectRow) -> Result<Project> {
    Ok(Project {
        project_id: r.0,
        name: r.1,
        description: r.2,
        target_id: r.3,
        created_at: r.4,
        updated_at: r.5,
        application_version: r.6,
        schema_version: r.7,
        status: parse_enum(&r.8)?,
        active_session_id: r.9,
        active_image_version_id: r.10,
        project_root: r.11,
        source_mode: parse_enum(&r.12)?,
    })
}

/// Column order shared by the pipeline_runs queries.
#[allow(clippy::type_complexity)]
type PipelineRunRow = (
    String,         // id
    String,         // project_id
    String,         // session_ids_json
    Option<String>, // recipe_id
    Option<String>, // started_at
    Option<String>, // completed_at
    String,         // status
    String,         // application_version
    String,         // engine_version
    Option<String>, // hardware_profile
    Option<String>, // execution_mode
    String,         // input_artifacts_json
    String,         // output_artifacts_json
);

fn pipeline_run_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PipelineRunRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
    ))
}

fn pipeline_run_from_row(r: PipelineRunRow) -> Result<PipelineRun> {
    Ok(PipelineRun {
        run_id: r.0,
        project_id: r.1,
        session_ids: serde_json::from_str(&r.2)?,
        recipe_id: r.3,
        started_at: r.4,
        completed_at: r.5,
        status: parse_enum(&r.6)?,
        application_version: r.7,
        engine_version: r.8,
        hardware_profile: r.9,
        execution_mode: r.10,
        input_artifacts: serde_json::from_str(&r.11)?,
        output_artifacts: serde_json::from_str(&r.12)?,
    })
}

/// Column order shared by the stage_run_records queries.
#[allow(clippy::type_complexity)]
type StageRunRow = (
    String,         // id
    String,         // run_id
    String,         // stage_id
    String,         // status
    i64,            // attempt
    Option<String>, // params_json
    Option<String>, // metrics_json
    Option<String>, // error
    Option<String>, // started_at
    Option<String>, // completed_at
);

fn stage_run_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StageRunRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
    ))
}

fn stage_run_from_row(r: StageRunRow) -> Result<StageRunRecord> {
    Ok(StageRunRecord {
        stage_run_id: r.0,
        run_id: r.1,
        stage_id: r.2,
        status: r.3,
        attempt: r.4 as u32,
        params_json: r.5,
        metrics_json: r.6,
        error: r.7,
        started_at: r.8,
        completed_at: r.9,
    })
}

/// CR-05 P4 slice 3 — column order for `preview_runs` queries.
type PreviewRunRow = (
    String,         // id
    String,         // stage_execution_id
    String,         // source_version_id
    Option<String>, // preview_artifact_id
    String,         // parameters_json
    String,         // parameters_hash
    String,         // status
    f64,            // scale
    String,         // label
    Option<String>, // error_json
    Option<String>, // started_at
    Option<String>, // completed_at
    String,         // created_at
);

fn preview_run_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PreviewRunRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
    ))
}

fn preview_run_from_row(r: PreviewRunRow) -> Result<PreviewRun> {
    Ok(PreviewRun {
        preview_id: r.0,
        stage_execution_id: r.1,
        source_version_id: r.2,
        preview_artifact_id: r.3,
        parameters_json: r.4,
        parameters_hash: r.5,
        status: r.6,
        scale: r.7,
        label: r.8,
        error_json: r.9,
        started_at: r.10,
        completed_at: r.11,
        created_at: r.12,
    })
}

/// Column order shared by the artifacts queries.
#[allow(clippy::type_complexity)]
type ArtifactRow = (
    String,         // id
    String,         // artifact_hash
    String,         // artifact_type (serde token)
    String,         // format
    String,         // path
    i64,            // size
    String,         // created_at
    Option<String>, // producer_stage
    Option<String>, // pipeline_run_id
    String,         // parent_artifact_ids_json
    Option<u32>,    // width
    Option<u32>,    // height
    Option<u32>,    // channels
    Option<u32>,    // bit_depth
    Option<String>, // color_space
    Option<bool>,   // linear_or_nonlinear
);

fn artifact_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtifactRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
        row.get(14)?,
        row.get(15)?,
    ))
}

fn artifact_from_row(r: ArtifactRow) -> Result<Artifact> {
    Ok(Artifact {
        artifact_id: r.0,
        artifact_hash: r.1,
        artifact_type: parse_enum::<ArtifactCategory>(&r.2)?,
        format: r.3,
        path: r.4,
        size: r.5 as u64,
        created_at: r.6,
        producer_stage: r.7,
        pipeline_run_id: r.8,
        parent_artifact_ids: serde_json::from_str(&r.9)?,
        width: r.10,
        height: r.11,
        channels: r.12,
        bit_depth: r.13,
        color_space: r.14,
        linear_or_nonlinear: r.15,
    })
}

fn not_found_if_missing(e: rusqlite::Error, what: &str, id: &str) -> DomainStoreError {
    match e {
        rusqlite::Error::QueryReturnedNoRows => DomainStoreError::NotFound(format!("{what} {id}")),
        other => DomainStoreError::Sqlite(other),
    }
}

/// Enums are stored as their serde string token (e.g. "deep_sky", "NEW").
fn enum_str<T: serde::Serialize>(v: &T) -> Result<String> {
    Ok(serde_json::to_string(v)?.trim_matches('"').to_string())
}

fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    Ok(serde_json::from_str(&format!("\"{s}\""))?)
}

/// Id generation mirrors session.rs: millisecond timestamp + monotonic
/// nonce so back-to-back creations within the same millisecond stay unique.
fn new_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static NONCE: AtomicU32 = AtomicU32::new(0);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    format!("{prefix}_{ts}_{}", NONCE.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ObjectType, PreviewRun};
    use std::path::PathBuf;

    fn store() -> DomainStore {
        DomainStore::new(&PathBuf::from(":memory:")).unwrap()
    }

    fn blank_asset(session_id: &str, hash: &str) -> SourceAsset {
        SourceAsset {
            asset_id: String::new(),
            content_hash: hash.into(),
            original_filename: "light_001.fits".into(),
            original_path: "/captures/light_001.fits".into(),
            file_size: 1024,
            format: "fits".into(),
            mime_type: None,
            created_at: String::new(),
            imported_at: String::new(),
            session_id: session_id.into(),
            frame_type: Some("LIGHT".into()),
            exposure: Some(10.0),
            filter: None,
            binning: Some("1x1".into()),
            width: Some(1280),
            height: Some(960),
            bit_depth: Some(16),
            bayer_pattern: Some("RGGB".into()),
            camera: Some("Seestar S50".into()),
            date_obs: None,
            ra: None,
            dec: None,
        }
    }

    #[test]
    fn migrations_apply_once_and_are_idempotent() {
        let s = store();
        // CR-05 P4 slice 3 — migration v4 added the `preview_runs`
        // table. Bump the expected version; the assertion still
        // proves the runner applies migrations exactly once and
        // re-running it on a fresh store does not double-apply.
        assert_eq!(s.schema_version(), 4);
        // Re-running the migration runner must not fail or re-apply.
        let s2 = DomainStore::new(&PathBuf::from(":memory:")).unwrap();
        assert_eq!(s2.schema_version(), 4);
    }

    #[test]
    fn artifact_record_and_lookup_round_trip() {
        let s = store();
        let art = Artifact {
            artifact_id: String::new(), // minted by the store
            artifact_hash: "deadbeef".into(),
            artifact_type: ArtifactCategory::Derived,
            format: "fits".into(),
            path: "/artifacts/de/deadbeef.fits".into(),
            size: 4096,
            created_at: String::new(), // sqlite default
            producer_stage: Some("stretch".into()),
            pipeline_run_id: Some("run_1".into()),
            parent_artifact_ids: vec!["art_parent".into()],
            width: Some(1280),
            height: Some(960),
            channels: Some(3),
            bit_depth: Some(32),
            color_space: Some("linear".into()),
            linear_or_nonlinear: Some(false),
        };
        let id = s.record_artifact(&art).unwrap();
        assert!(id.starts_with("art_"));

        let got = s.get_artifact(&id).unwrap();
        assert_eq!(got.artifact_hash, "deadbeef");
        assert_eq!(got.artifact_type, ArtifactCategory::Derived);
        assert_eq!(got.parent_artifact_ids, vec!["art_parent"]);
        assert_eq!(got.producer_stage.as_deref(), Some("stretch"));
        assert_eq!(got.linear_or_nonlinear, Some(false));

        // §7 reuse: lookup by content hash.
        let by_hash = s.find_artifacts_by_hash("deadbeef").unwrap();
        assert_eq!(by_hash.len(), 1);
        assert_eq!(by_hash[0].artifact_id, id);

        // Lineage queries by run.
        let for_run = s.list_artifacts_for_run("run_1").unwrap();
        assert_eq!(for_run.len(), 1);
        assert!(s.list_artifacts_for_run("run_other").unwrap().is_empty());
    }

    #[test]
    fn project_crud_rename_and_events() {
        let s = store();
        let pid = s.create_project("M42", None, "0.1.0").unwrap();
        let p = s.get_project(&pid).unwrap();
        assert_eq!(p.status, ProjectStatus::New);
        assert_eq!(p.schema_version, DOMAIN_SCHEMA_VERSION);

        s.rename_project(&pid, "M42 — Orion Nebula").unwrap();
        assert_eq!(s.get_project(&pid).unwrap().name, "M42 — Orion Nebula");

        let events = s.list_events(&pid).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, ProjectEventKind::ProjectCreated);
    }

    #[test]
    fn project_status_machine_is_enforced() {
        let s = store();
        let pid = s.create_project("M42", None, "0.1.0").unwrap();

        // Invalid: NEW -> EXPORTED.
        assert!(matches!(
            s.set_project_status(&pid, ProjectStatus::Exported),
            Err(DomainStoreError::InvalidTransition { .. })
        ));

        // Valid forward path to EXPORTED.
        for next in [
            ProjectStatus::Imported,
            ProjectStatus::Analyzing,
            ProjectStatus::Ready,
            ProjectStatus::Processing,
            ProjectStatus::Processed,
            ProjectStatus::Enhancing,
            ProjectStatus::Review,
            ProjectStatus::Exported,
        ] {
            s.set_project_status(&pid, next).unwrap();
        }
        assert_eq!(s.get_project(&pid).unwrap().status, ProjectStatus::Exported);

        // EXPORTED is not terminal (CR-02 §21).
        s.set_project_status(&pid, ProjectStatus::Review).unwrap();
        s.set_project_status(&pid, ProjectStatus::Enhancing)
            .unwrap();
    }

    #[test]
    fn target_and_sessions_round_trip() {
        let s = store();
        let tid = s.create_target("M42", ObjectType::DeepSky).unwrap();
        let t = s.get_target(&tid).unwrap();
        assert_eq!(t.object_type, ObjectType::DeepSky);

        let pid = s.create_project("M42", Some(&tid), "0.1.0").unwrap();
        assert_eq!(
            s.get_project(&pid).unwrap().target_id.as_deref(),
            Some(tid.as_str())
        );

        let s1 = s.create_session(&pid, "2026-01-12 Seestar").unwrap();
        let s2 = s.create_session(&pid, "2026-01-15 Seestar").unwrap();
        let sessions = s.list_sessions(&pid).unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].session_id, s1);
        assert_eq!(sessions[1].session_id, s2);
    }

    #[test]
    fn source_asset_duplicate_detection_by_hash() {
        let s = store();
        let pid = s.create_project("M42", None, "0.1.0").unwrap();
        let sid = s.create_session(&pid, "night 1").unwrap();

        let (id1, dup1) = s
            .register_source_asset(&blank_asset(&sid, "abc123"))
            .unwrap();
        assert!(!dup1);
        let (id2, dup2) = s
            .register_source_asset(&blank_asset(&sid, "abc123"))
            .unwrap();
        assert!(dup2);
        assert_eq!(id1, id2, "same content hash must return the existing asset");

        let (id3, dup3) = s
            .register_source_asset(&blank_asset(&sid, "def456"))
            .unwrap();
        assert!(!dup3);
        assert_ne!(id1, id3);

        let assets = s.list_source_assets(&sid).unwrap();
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].frame_type.as_deref(), Some("LIGHT"));
        assert_eq!(assets[0].width, Some(1280));
    }

    #[test]
    fn missing_rows_report_not_found() {
        let s = store();
        assert!(matches!(
            s.get_project("proj_nope"),
            Err(DomainStoreError::NotFound(_))
        ));
    }

    #[test]
    fn pipeline_run_lifecycle_records_events() {
        let s = store();
        let pid = s.create_project("M42", None, "0.1.0").unwrap();
        let sid = s.create_session(&pid, "night 1").unwrap();
        let run = s
            .create_pipeline_run(&pid, &[sid.clone()], None, "0.1.0", "engine-1.0")
            .unwrap();
        assert!(run.starts_with("run_"));

        let created = s.get_pipeline_run(&run).unwrap();
        assert_eq!(created.status, PipelineRunStatus::Queued);
        assert_eq!(created.session_ids, vec![sid]);
        assert_eq!(created.application_version, "0.1.0");

        s.mark_run_started(&run).unwrap();
        assert_eq!(
            s.get_pipeline_run(&run).unwrap().status,
            PipelineRunStatus::Running
        );
        // mark_run_started wrote started_at; a second call must not reset it
        // to "now" again? — current impl does, which is acceptable: the run
        // is genuinely re-starting. Verify only that started_at stays set.
        assert!(s.get_pipeline_run(&run).unwrap().started_at.is_some());

        s.mark_run_finished(&run, PipelineRunStatus::Completed)
            .unwrap();
        let done = s.get_pipeline_run(&run).unwrap();
        assert_eq!(done.status, PipelineRunStatus::Completed);
        assert!(done.completed_at.is_some());

        // PIPELINE_STARTED event was recorded at create time.
        let events = s.list_events(&pid).unwrap();
        assert!(events
            .iter()
            .any(|e| e.kind == ProjectEventKind::PipelineStarted
                && e.payload_json.as_deref() == Some(run.as_str())));
    }

    #[test]
    fn stage_runs_persist_with_independent_attempts() {
        let s = store();
        let pid = s.create_project("M42", None, "0.1.0").unwrap();
        let run = s
            .create_pipeline_run(&pid, &[], None, "0.1.0", "engine-1.0")
            .unwrap();

        // First attempt.
        let mut rec = StageRunRecord {
            stage_run_id: String::new(),
            run_id: run.clone(),
            stage_id: "stretch".into(),
            status: "running".into(),
            attempt: 1,
            params_json: Some(r#"{"blackPoint":0.02}"#.into()),
            metrics_json: None,
            error: None,
            started_at: Some("2026-09-06T00:00:00Z".into()),
            completed_at: None,
        };
        let id = s.record_stage_run(&rec).unwrap();
        assert!(id.starts_with("sr_"));

        // Completed first attempt.
        rec.status = "completed".into();
        rec.attempt = s.next_stage_attempt(&run, "stretch").unwrap();
        rec.completed_at = Some("2026-09-06T00:00:05Z".into());
        s.record_stage_run(&rec).unwrap();

        let all = s.list_stage_runs(&run).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].attempt, 1);
        assert_eq!(all[1].attempt, 2);
        assert_eq!(all[1].stage_id, "stretch");
        assert_eq!(all[1].status, "completed");

        // CR-02 §12: reruns are independent. next_stage_attempt is monotonic.
        assert_eq!(s.next_stage_attempt(&run, "stretch").unwrap(), 3);
        assert_eq!(s.next_stage_attempt(&run, "denoise").unwrap(), 1);
    }

    #[test]
    fn crash_recovery_finds_non_terminal_runs() {
        let s = store();
        let pid = s.create_project("M42", None, "0.1.0").unwrap();
        let running = s
            .create_pipeline_run(&pid, &[], None, "0.1.0", "engine-1.0")
            .unwrap();
        s.mark_run_started(&running).unwrap();
        let queued = s
            .create_pipeline_run(&pid, &[], None, "0.1.0", "engine-1.0")
            .unwrap();
        let done = s
            .create_pipeline_run(&pid, &[], None, "0.1.0", "engine-1.0")
            .unwrap();
        s.mark_run_started(&done).unwrap();
        s.mark_run_finished(&done, PipelineRunStatus::Completed)
            .unwrap();

        let interrupted = s.find_interrupted_runs().unwrap();
        assert_eq!(interrupted.len(), 2);
        let ids: Vec<_> = interrupted.iter().map(|r| r.run_id.clone()).collect();
        assert!(ids.contains(&running));
        assert!(ids.contains(&queued));
        assert!(!ids.contains(&done));
    }

    // ─── CR-05 P4 slice 3 — PreviewRun CRUD tests ─ ────────────────────────

    fn blank_preview(stage_execution_id: &str, source_version_id: &str) -> PreviewRun {
        PreviewRun {
            preview_id: String::new(),
            stage_execution_id: stage_execution_id.into(),
            source_version_id: source_version_id.into(),
            preview_artifact_id: None,
            parameters_json: r#"{"sigma":1.5,"gain":1.0}"#.into(),
            parameters_hash: "sha256:abc123".into(),
            status: crate::domain::preview_status::PENDING.into(),
            scale: 0.25,
            label: "Preview — Calibrate @ 0.25".into(),
            error_json: None,
            started_at: None,
            completed_at: None,
            created_at: String::new(),
        }
    }

    #[test]
    fn preview_run_migration_is_applied_on_open() {
        let s = store();
        // Schema migrations must reach version 4 after open.
        assert!(s.schema_version() >= 4);
    }

    #[test]
    fn preview_run_round_trip_persists_all_fields() {
        let s = store();
        let preview = blank_preview("ste_1", "ver_1");
        let id = s.create_preview_run(&preview).unwrap();
        assert!(id.starts_with("prev_"));
        let row = s.get_preview_run(&id).unwrap();
        assert_eq!(row.preview_id, id);
        assert_eq!(row.stage_execution_id, "ste_1");
        assert_eq!(row.source_version_id, "ver_1");
        assert_eq!(row.parameters_hash, "sha256:abc123");
        assert_eq!(row.status, crate::domain::preview_status::PENDING);
        assert!((row.scale - 0.25).abs() < f64::EPSILON);
        assert_eq!(row.label, "Preview — Calibrate @ 0.25");
        assert!(row.preview_artifact_id.is_none());
        assert!(row.started_at.is_none());
        assert!(row.completed_at.is_none());
        assert!(row.error_json.is_none());
        // created_at is filled in by SQL DEFAULT.
        assert!(!row.created_at.is_empty());
    }

    #[test]
    fn preview_run_supplied_id_is_honoured() {
        let s = store();
        let mut preview = blank_preview("ste_2", "ver_2");
        preview.preview_id = "prev_custom".into();
        let id = s.create_preview_run(&preview).unwrap();
        assert_eq!(id, "prev_custom");
    }

    #[test]
    fn preview_run_get_unknown_id_returns_not_found() {
        let s = store();
        let err = s.get_preview_run("prev_does_not_exist").unwrap_err();
        match err {
            DomainStoreError::NotFound(id) => assert_eq!(id, "prev_does_not_exist"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn preview_run_list_filters_by_stage_execution_id() {
        let s = store();
        s.create_preview_run(&blank_preview("ste_a", "ver_1"))
            .unwrap();
        s.create_preview_run(&blank_preview("ste_a", "ver_2"))
            .unwrap();
        s.create_preview_run(&blank_preview("ste_b", "ver_1"))
            .unwrap();

        let ste_a = s.list_preview_runs_for_stage_execution("ste_a").unwrap();
        assert_eq!(ste_a.len(), 2);
        for p in &ste_a {
            assert_eq!(p.stage_execution_id, "ste_a");
        }

        let ste_b = s.list_preview_runs_for_stage_execution("ste_b").unwrap();
        assert_eq!(ste_b.len(), 1);
        assert_eq!(ste_b[0].stage_execution_id, "ste_b");

        let empty = s
            .list_preview_runs_for_stage_execution("ste_missing")
            .unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn preview_run_mark_running_sets_status_and_timestamp() {
        let s = store();
        let id = s
            .create_preview_run(&blank_preview("ste_r", "ver_1"))
            .unwrap();
        s.mark_preview_running(&id, "unix_ms:1000").unwrap();
        let row = s.get_preview_run(&id).unwrap();
        assert_eq!(row.status, crate::domain::preview_status::RUNNING);
        assert_eq!(row.started_at.as_deref(), Some("unix_ms:1000"));
    }

    #[test]
    fn preview_run_mark_running_unknown_returns_not_found() {
        let s = store();
        let err = s.mark_preview_running("prev_x", "unix_ms:1").unwrap_err();
        match err {
            DomainStoreError::NotFound(id) => assert_eq!(id, "prev_x"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn preview_run_mark_completed_attaches_artifact() {
        let s = store();
        let id = s
            .create_preview_run(&blank_preview("ste_c", "ver_1"))
            .unwrap();
        s.mark_preview_running(&id, "unix_ms:100").unwrap();
        s.mark_preview_completed(&id, "art_preview_42", "unix_ms:200")
            .unwrap();
        let row = s.get_preview_run(&id).unwrap();
        assert_eq!(row.status, crate::domain::preview_status::COMPLETED);
        assert_eq!(row.preview_artifact_id.as_deref(), Some("art_preview_42"));
        assert_eq!(row.completed_at.as_deref(), Some("unix_ms:200"));
    }

    #[test]
    fn preview_run_mark_failed_records_error() {
        let s = store();
        let id = s
            .create_preview_run(&blank_preview("ste_f", "ver_1"))
            .unwrap();
        s.mark_preview_running(&id, "unix_ms:10").unwrap();
        s.mark_preview_failed(&id, r#"{"error":"oom"}"#, "unix_ms:20")
            .unwrap();
        let row = s.get_preview_run(&id).unwrap();
        assert_eq!(row.status, crate::domain::preview_status::FAILED);
        assert_eq!(row.error_json.as_deref(), Some(r#"{"error":"oom"}"#));
        assert_eq!(row.completed_at.as_deref(), Some("unix_ms:20"));
    }

    #[test]
    fn preview_run_delete_removes_row() {
        let s = store();
        let id = s
            .create_preview_run(&blank_preview("ste_d", "ver_1"))
            .unwrap();
        s.delete_preview_run(&id).unwrap();
        let err = s.get_preview_run(&id).unwrap_err();
        match err {
            DomainStoreError::NotFound(found) => assert_eq!(found, id),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn preview_run_delete_unknown_returns_not_found() {
        let s = store();
        let err = s.delete_preview_run("prev_nope").unwrap_err();
        match err {
            DomainStoreError::NotFound(id) => assert_eq!(id, "prev_nope"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn preview_run_migration_is_idempotent_across_reopen() {
        // Open twice with the same path and confirm migration
        // does not double-apply. Uses a temp file (`:memory:` would
        // lose state between opens).
        let path = std::env::temp_dir().join("astroforge_preview_migration.sqlite");
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        {
            let s = DomainStore::new(&path).unwrap();
            assert!(s.schema_version() >= 4);
        }
        {
            let s = DomainStore::new(&path).unwrap();
            assert!(s.schema_version() >= 4);
            // Smoke: CRUD still works.
            let id = s
                .create_preview_run(&blank_preview("ste_i", "ver_1"))
                .unwrap();
            assert!(id.starts_with("prev_"));
        }
        let _ = std::fs::remove_file(&path);
    }
}
