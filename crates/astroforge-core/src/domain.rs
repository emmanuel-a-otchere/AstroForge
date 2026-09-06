//! CR-02 canonical domain model — Project / Target / Session / SourceAsset /
//! Artifact / ImageVersion / PipelineRun / StageRunRecord / AIOperation /
//! Export / ProjectEvent.
//!
//! These types are the durable, user-facing domain vocabulary of AstroForge
//! ("what AstroForge owns"). They land ALONGSIDE the existing `db.rs` row
//! structs (decision D-1, docs/plans/2026-09-06-cr02-domain-model/PLAN.md):
//! `db.rs` maps the live v1 schema used by shipping IPC commands, while this
//! module defines the CR-02 target model that CR-02.2 (persistence) will
//! migrate toward.
//!
//! Recipe is intentionally NOT redefined here (decision D-2) —
//! `crate::recipe::Recipe` + `recipe_store` already implement the versioned
//! recipe model; domain types reference recipes by `recipe_id`.

use serde::{Deserialize, Serialize};

/// Domain schema version (CR-02 §30). Persisted in the `schema_migrations`
/// table from CR-02.2 onward; migrations must be automatic and silent.
pub const DOMAIN_SCHEMA_VERSION: u32 = 1;

// ─── Enums ──────────────────────────────────────────────────────────────────

/// CR-02 §4 — astronomical subject classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    DeepSky,
    Planet,
    Lunar,
    Solar,
    Unknown,
}

impl Default for ObjectType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// CR-02 §19 — how source files are held: copied into project storage
/// (managed) or left in place (referenced).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMode {
    Managed,
    Referenced,
}

/// CR-02 §8.1 — artifact categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactCategory {
    Source,
    Derived,
    Preview,
    Mask,
    Metadata,
    QualityMetric,
    Checkpoint,
    Export,
}

/// CR-02 §11 — pipeline run lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineRunStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Recovering,
}

impl PipelineRunStatus {
    /// True when the run is in a terminal state (no further transitions).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// CR-02 §21 — project state machine. EXPORTED is NOT terminal: the user
/// can return EXPORTED → REVIEW → ENHANCING → EXPORTED indefinitely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectStatus {
    New,
    Imported,
    Analyzing,
    Ready,
    Processing,
    Paused,
    Failed,
    Recovering,
    Processed,
    Enhancing,
    Review,
    Exported,
}

impl ProjectStatus {
    /// CR-02 §21 transition table (decision D-4). Enforcement at the type
    /// layer keeps invalid transitions out of CR-02.4 lifecycle code.
    pub fn can_transition_to(self, next: ProjectStatus) -> bool {
        use ProjectStatus::*;
        matches!(
            (self, next),
            (New, Imported)
                | (Imported, Analyzing)
                | (Analyzing, Ready)
                | (Ready, Processing)
                | (Processing, Paused)
                | (Paused, Processing)
                | (Processing, Failed)
                | (Failed, Recovering)
                | (Recovering, Processing)
                | (Recovering, Ready)
                | (Processing, Processed)
                | (Processed, Enhancing)
                | (Enhancing, Review)
                | (Review, Enhancing)
                | (Review, Exported)
                // EXPORTED is not terminal (CR-02 §21).
                | (Exported, Review)
                | (Exported, Enhancing)
        )
    }
}

/// CR-02 §35 — append-oriented project history event kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectEventKind {
    ProjectCreated,
    SessionImported,
    SourceImported,
    AnalysisCompleted,
    RecipeSelected,
    PipelineStarted,
    StageCompleted,
    AiOperationApplied,
    VersionCreated,
    ExportCreated,
}

// ─── Core domain objects ────────────────────────────────────────────────────

/// CR-02 §3.1 — the durable unit of work. A folder of images is merely an
/// input source; the Project is what the user owns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub target_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// AstroForge app version that last wrote the project (CR-02 §31).
    pub application_version: String,
    /// Domain schema version (CR-02 §30) — see DOMAIN_SCHEMA_VERSION.
    pub schema_version: u32,
    pub status: ProjectStatus,
    pub active_session_id: Option<String>,
    pub active_image_version_id: Option<String>,
    /// Application-managed project directory root (CR-02 §18). Meaningful
    /// from CR-02.4 onward (decision D-7).
    pub project_root: Option<String>,
    pub source_mode: SourceMode,
}

/// CR-02 §4 — the astronomical subject, separated from Session so the same
/// object can be photographed over multiple nights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub target_id: String,
    pub name: String,
    pub object_type: ObjectType,
    pub ra: Option<f64>,
    pub dec: Option<f64>,
    pub catalog_ids: Vec<String>,
    pub constellation: Option<String>,
    pub description: String,
}

/// CR-02 §5.2 — normalized capture metadata. Original FITS headers are
/// preserved on the SourceAsset; this struct holds the normalized values.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CaptureMetadata {
    pub camera: Option<String>,
    pub sensor: Option<String>,
    pub resolution: Option<String>,
    pub pixel_size: Option<f64>,
    pub focal_length: Option<f64>,
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    pub temperature: Option<f64>,
    pub binning: Option<String>,
    pub filter: Option<String>,
    pub exposure: Option<f64>,
    pub date_obs: Option<String>,
    pub dithering: Option<bool>,
}

/// CR-02 §5 — one coherent acquisition set ("what I captured").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub project_id: String,
    pub name: String,
    pub capture_start: Option<String>,
    pub capture_end: Option<String>,
    pub source_location: Option<String>,
    pub camera_profile: Option<String>,
    pub telescope_profile: Option<String>,
    pub site_profile: Option<String>,
    pub target_detection: Option<String>,
    pub processing_status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// CR-02 §6 — an immutable original input. The processing engine must never
/// overwrite a SourceAsset; identity is by content hash (CR-02 §7, D-5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAsset {
    pub asset_id: String,
    /// SHA-256 hex digest of file contents (decision D-5).
    pub content_hash: String,
    pub original_filename: String,
    pub original_path: String,
    pub file_size: u64,
    pub format: String,
    pub mime_type: Option<String>,
    pub created_at: String,
    pub imported_at: String,
    pub session_id: String,
    // Astronomical metadata (normalized; raw FITS headers preserved on disk).
    pub frame_type: Option<String>,
    pub exposure: Option<f64>,
    pub filter: Option<String>,
    pub binning: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bit_depth: Option<u32>,
    pub bayer_pattern: Option<String>,
    pub camera: Option<String>,
    pub date_obs: Option<String>,
    pub ra: Option<f64>,
    pub dec: Option<f64>,
}

/// CR-02 §8 — any persistent output generated by AstroForge. An artifact is
/// an implementation object; the user-facing counterpart is ImageVersion.
///
/// Note: `artifact.rs::ArtifactRecord` remains the artifact-store row type;
/// this is the canonical domain shape with lineage and producer linkage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: String,
    /// SHA-256 hex digest of artifact contents (CR-02 §7).
    pub artifact_hash: String,
    pub artifact_type: ArtifactCategory,
    pub format: String,
    pub path: String,
    pub size: u64,
    pub created_at: String,
    /// Pipeline stage that produced this artifact, if any.
    pub producer_stage: Option<String>,
    pub pipeline_run_id: Option<String>,
    /// Provenance edges (CR-02 §16) — artifacts this one derives from.
    pub parent_artifact_ids: Vec<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub channels: Option<u32>,
    pub bit_depth: Option<u32>,
    pub color_space: Option<String>,
    /// True while the data is linear (pre-stretch), false after.
    pub linear_or_nonlinear: Option<bool>,
}

/// CR-02 §9 — a meaningful visual state of the user's image ("Version 8 —
/// AI Enhanced"), non-destructively movable between (CR-02 §10).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageVersion {
    pub version_id: String,
    pub project_id: String,
    /// User-facing label, e.g. "AI Enhanced".
    pub label: String,
    /// Monotonic sequence within the project (V0, V1, ...).
    pub sequence: u32,
    pub primary_artifact_id: String,
    /// Version this one branched from (CR-02 §10 copy-forward model).
    pub source_version_id: Option<String>,
    pub created_at: String,
    /// Hidden versions stay retained for provenance but leave the timeline
    /// (CR-02 §26 delete semantics).
    pub hidden: bool,
}

/// CR-02 §11 — one execution of a processing recipe. Distinct from Recipe:
/// Recipe → PipelineRun → Artifacts, never Recipe = PipelineRun.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    pub run_id: String,
    pub project_id: String,
    pub session_ids: Vec<String>,
    pub recipe_id: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub status: PipelineRunStatus,
    /// CR-02 §31 — application and engine versions recorded separately.
    pub application_version: String,
    pub engine_version: String,
    pub hardware_profile: Option<String>,
    pub execution_mode: Option<String>,
    pub input_artifacts: Vec<String>,
    pub output_artifacts: Vec<String>,
}

/// CR-02 §12 — one DAG node execution, persistent. Keyed by run_id
/// (decision D-3); `db.rs::StageRun` remains the session-keyed v1 row type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRunRecord {
    pub stage_run_id: String,
    pub run_id: String,
    pub stage_id: String,
    pub status: String,
    /// 1-based attempt counter — stages can be rerun independently.
    pub attempt: u32,
    pub params_json: Option<String>,
    pub metrics_json: Option<String>,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// CR-02 §15 — AI provenance. Deterministic is the default; stochastic /
/// generative operations are explicitly labeled and seed-recorded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiOperation {
    pub operation_id: String,
    pub stage_run_id: String,
    pub model_id: String,
    pub model_version: String,
    pub model_hash: Option<String>,
    pub runtime: Option<String>,
    pub backend: Option<String>,
    pub precision: Option<String>,
    pub parameters_json: Option<String>,
    pub seed: Option<u64>,
    pub deterministic: bool,
    pub experimental: bool,
    pub input_artifact_id: Option<String>,
    pub output_artifact_id: Option<String>,
}

/// CR-02 — what left AstroForge. Deleting an Export never affects
/// processing history (CR-02 §26).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub export_id: String,
    pub project_id: String,
    pub image_version_id: String,
    pub artifact_id: String,
    pub format: String,
    pub destination: String,
    pub created_at: String,
}

/// CR-02 §35 — append-oriented auditable history row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEvent {
    pub event_id: String,
    pub project_id: String,
    pub kind: ProjectEventKind,
    pub payload_json: Option<String>,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_status_forward_path() {
        use ProjectStatus::*;
        let path = [
            New, Imported, Analyzing, Ready, Processing, Processed, Enhancing, Review, Exported,
        ];
        for pair in path.windows(2) {
            assert!(
                pair[0].can_transition_to(pair[1]),
                "{:?} -> {:?} should be allowed",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn project_status_exported_is_not_terminal() {
        use ProjectStatus::*;
        assert!(Exported.can_transition_to(Review));
        assert!(Exported.can_transition_to(Enhancing));
        assert!(Review.can_transition_to(Enhancing));
    }

    #[test]
    fn project_status_rejects_invalid_transitions() {
        use ProjectStatus::*;
        assert!(!New.can_transition_to(Exported));
        assert!(!New.can_transition_to(Processing));
        assert!(!Exported.can_transition_to(New));
        assert!(!Failed.can_transition_to(Processed));
    }

    #[test]
    fn pipeline_run_status_terminal() {
        assert!(PipelineRunStatus::Completed.is_terminal());
        assert!(PipelineRunStatus::Failed.is_terminal());
        assert!(PipelineRunStatus::Cancelled.is_terminal());
        assert!(!PipelineRunStatus::Running.is_terminal());
        assert!(!PipelineRunStatus::Recovering.is_terminal());
    }

    #[test]
    fn domain_types_serde_round_trip() {
        let project = Project {
            project_id: "proj_1".into(),
            name: "M42 — Orion Nebula".into(),
            description: String::new(),
            target_id: Some("tgt_m42".into()),
            created_at: "2026-09-06T00:00:00Z".into(),
            updated_at: "2026-09-06T00:00:00Z".into(),
            application_version: "0.1.0".into(),
            schema_version: DOMAIN_SCHEMA_VERSION,
            status: ProjectStatus::New,
            active_session_id: None,
            active_image_version_id: None,
            project_root: None,
            source_mode: SourceMode::Referenced,
        };
        let json = serde_json::to_string(&project).expect("serialize");
        let back: Project = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.project_id, "proj_1");
        assert_eq!(back.status, ProjectStatus::New);
        assert_eq!(back.source_mode, SourceMode::Referenced);

        let op = AiOperation {
            operation_id: "ai_1".into(),
            stage_run_id: "sr_1".into(),
            model_id: "swinir-astro-denoise".into(),
            model_version: "1.2.0".into(),
            model_hash: Some("sha256:abc".into()),
            runtime: Some("onnx".into()),
            backend: Some("coreml".into()),
            precision: Some("int8".into()),
            parameters_json: None,
            seed: None,
            deterministic: true,
            experimental: false,
            input_artifact_id: None,
            output_artifact_id: None,
        };
        let json = serde_json::to_string(&op).expect("serialize");
        let back: AiOperation = serde_json::from_str(&json).expect("deserialize");
        assert!(back.deterministic);
        assert!(!back.experimental);
    }
}
