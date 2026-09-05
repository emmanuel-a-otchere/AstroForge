use serde::{Deserialize, Serialize};

/// Session store schema (projects / sessions / stage_runs / checkpoints).
///
/// Kept separate from the gallery schema so the two stores can evolve
/// independently (different ownership, different write cadence).
/// Migration is run when `SessionStore::new` opens the DB.
pub const SESSION_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    target_type TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    source_dir TEXT,
    verbosity TEXT NOT NULL DEFAULT 'beginner',
    status TEXT NOT NULL DEFAULT 'created',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS stage_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    stage_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    params_json TEXT,
    metrics_json TEXT,
    error TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS checkpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    stage_id TEXT NOT NULL,
    artifact_path TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_id);
CREATE INDEX IF NOT EXISTS idx_stage_runs_session ON stage_runs(session_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_session ON checkpoints(session_id);
"#;

/// Gallery store schema. Kept separate from the session schema so the
/// two stores can evolve independently (different ownership, different
/// write cadence). Migration is run when `GalleryStore::new` opens the
/// DB.
pub const GALLERY_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS gallery_items (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    target TEXT NOT NULL,
    integration_hours REAL NOT NULL DEFAULT 0,
    palette TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'processing', 'completed')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_gallery_items_status
    ON gallery_items(status);
CREATE INDEX IF NOT EXISTS idx_gallery_items_updated_at
    ON gallery_items(updated_at DESC);
"#;

/// Recipe (pipeline profile) store schema. Each row is one version of a
/// profile; the `profile_id` groups versions of the same logical
/// profile (same name + target_type). The current/active head of each
/// profile is the row with the highest `version` for that `profile_id`.
///
/// Migration is run when `RecipeStore::new` opens the DB.
pub const RECIPE_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS recipes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    target_type TEXT NOT NULL,
    version INTEGER NOT NULL,
    parent_version INTEGER,
    branch TEXT NOT NULL DEFAULT 'main',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    payload_json TEXT NOT NULL,
    UNIQUE(profile_id, version, branch)
);

CREATE INDEX IF NOT EXISTS idx_recipes_profile
    ON recipes(profile_id);
CREATE INDEX IF NOT EXISTS idx_recipes_target_type
    ON recipes(target_type);
CREATE INDEX IF NOT EXISTS idx_recipes_name_target
    ON recipes(name, target_type);
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub target_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_id: String,
    pub source_dir: Option<String>,
    pub verbosity: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRun {
    pub id: i64,
    pub session_id: String,
    pub stage_id: String,
    pub status: String,
    pub params_json: Option<String>,
    pub metrics_json: Option<String>,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: i64,
    pub session_id: String,
    pub stage_id: String,
    pub artifact_path: String,
    pub created_at: String,
}
