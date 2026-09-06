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
    Artifact, ArtifactCategory, Project, ProjectEvent, ProjectEventKind, ProjectStatus, Session,
    SourceAsset, Target, DOMAIN_SCHEMA_VERSION,
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
    use crate::domain::ObjectType;
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
        assert_eq!(s.schema_version(), 2);
        // Re-running the migration runner must not fail or re-apply.
        let s2 = DomainStore::new(&PathBuf::from(":memory:")).unwrap();
        assert_eq!(s2.schema_version(), 2);
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
}
