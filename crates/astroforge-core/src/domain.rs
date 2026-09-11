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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    DeepSky,
    Planet,
    Lunar,
    Solar,
    #[default]
    Unknown,
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

/// CR-04 P8 — the import lifecycle state attached to a
/// session. Distinct from `processing_status` (which
/// tracks pipeline execution) — `import_state` tracks the
/// CR-04 §12 wizard stages (scanned → understanding →
/// confirmed → materialised).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportState {
    /// Created from `create_session`; no understanding yet.
    Created,
    /// Scan complete; assets grouped; awaiting understanding.
    Scanned,
    /// P3 + P4 + P5 + P6 + P7 ran; understanding + suggestion emitted.
    Understanding,
    /// Ambiguous classification surfaced; UI must prompt user
    /// for confirmation before materialising.
    Ambiguous,
    /// User confirmed the import (or accepted the override).
    Confirmed,
    /// Session + assets persisted; the project now owns this session.
    Materialised,
}

impl ImportState {
    pub fn as_str(self) -> &'static str {
        match self {
            ImportState::Created => "created",
            ImportState::Scanned => "scanned",
            ImportState::Understanding => "understanding",
            ImportState::Ambiguous => "ambiguous",
            ImportState::Confirmed => "confirmed",
            ImportState::Materialised => "materialised",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "scanned" => ImportState::Scanned,
            "understanding" => ImportState::Understanding,
            "ambiguous" => ImportState::Ambiguous,
            "confirmed" => ImportState::Confirmed,
            "materialised" => ImportState::Materialised,
            _ => ImportState::Created,
        }
    }
}

/// CR-04 P10 — the source of authority for the target
/// classification. Mirrors
/// `astroforge_ai::target_classify::TargetClassificationProvenance`
/// but lives in `domain.rs` so the schema migration can
/// record it directly on the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationProvenance {
    /// The deterministic P3..P7 classifier was authoritative.
    Deterministic,
    /// The AI stub was requested (per D-CR04-3) but
    /// returned the deterministic fallback. The UI
    /// surfaces "AI confirming…" when this is the
    /// provenance.
    AiStubRequested,
    /// A user override was applied (per §13 ambiguity UX).
    UserOverride,
}

impl ClassificationProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            ClassificationProvenance::Deterministic => "deterministic",
            ClassificationProvenance::AiStubRequested => "ai_stub_requested",
            ClassificationProvenance::UserOverride => "user_override",
        }
    }
}

/// CR-04 P8 — the per-session import understanding. The
/// classification pipeline (P3 frame classification, P4
/// target detection, P5 session grouping, P6 capture
/// analysis, P7 narrowband detection) emits this struct
/// which the IPC layer persists alongside the Session row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClassification {
    pub session_id: String,
    pub import_state: ImportState,
    /// P6 capture kind as a snake_case string:
    /// "deep_sky" / "planetary_lunar" / "ambiguous".
    pub capture_kind: Option<String>,
    /// P7 narrowband composition as a snake_case string:
    /// "none" / "mono" / "hoo" / "sho" / "lrgb" /
    /// "hoo_or_sho".
    pub narrowband_composition: Option<String>,
    /// 0.0..=1.0. Aggregate confidence from the pipeline.
    /// Below ~0.6 the UI surfaces the ambiguity dialog
    /// (CR-04 §13).
    pub classification_confidence: Option<f64>,
    /// CR-04 P10 — the source of authority for the
    /// target classification. `Deterministic` when the
    /// P4 detector was above the AI floor; `AiStubRequested`
    /// when the AI stub was invoked but returned the
    /// deterministic fallback; `UserOverride` when the
    /// user corrected the suggestion via the §13 panel.
    pub target_provenance: ClassificationProvenance,
    /// JSON blob of per-classifier observations (target,
    /// capture, narrowband). Serialized so the UI can
    /// render "Observation → Evidence → Confidence →
    /// Decision" without re-running the pipeline.
    pub classification_metadata: Option<String>,
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

// ─── CR-05 §26: PipelinePlan / PipelineStage / StageExecution ───────────────
//
// Three new persistent entities introduced by CR-05 P1. They sit alongside
// CR-02's PipelineRun + StageRunRecord: a Plan is the user-facing
// human-readable processing recipe adapted for a specific Session
// Understanding; a Stage is one node in that plan; a StageExecution is the
// actual run of that stage (a 1:N expansion of StageRunRecord keyed by plan
// rather than run, so the same plan can be branched — see CR-05 §17).
//
// P1 introduces the types and the plan generator. P2 adds the runner that
// writes StageExecution rows. P3–P5 extend StageExecution with quality
// metrics, preview runs, and resource usage.

/// CR-05 §26 — a concrete, dataset-aware processing plan. Generated from a
/// Session Understanding (CR-04) plus a Recipe (CR-02 §14); consumed by
/// P2's stage runner and P5's Expert DAG view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinePlan {
    pub plan_id: String,
    pub project_id: String,
    pub session_id: String,
    pub recipe_id: Option<String>,
    /// "auto" | "guided" | "expert" (per Decision D-CR05-7). The engine
    /// does not branch on mode; the UI renders more controls as mode
    /// escalates. Stored for provenance and reproducibility.
    pub mode: String,
    /// ObjectType at plan-creation time, for provenance.
    pub target_type: ObjectType,
    pub status: PipelinePlanStatus,
    /// Plan generation time (ISO-8601 UTC).
    pub created_at: String,
    /// Plan-level schema version; bump if PipelinePlan shape changes.
    pub schema_version: u32,
    pub stages: Vec<PipelineStage>,
}

/// CR-05 §26 — one node in a PipelinePlan. Sequence is 0-based, ascending.
/// `required` vs `optional` is the canonical distinction a Guided-mode UI
/// surfaces (per CR-05 §6.1 / §18).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    pub stage_id: String,
    pub plan_id: String,
    /// CR-05 §31 — `StageType` for the underlying engine call. Mirrors
    /// `pipeline::StageType` in the legacy trait surface so the P2 runner
    /// can dispatch without a translation table.
    pub stage_type: String,
    pub sequence: u32,
    /// User-visible label (Calibrate / Stack / Color / Stretch / ...).
    pub label: String,
    pub required: bool,
    pub enabled: bool,
    pub parameters_json: Option<String>,
    /// Per CR-05 §6 Zone A: this stage creates an Image Version when it
    /// commits. Optional stages that produce no visible artifact can
    /// leave this false.
    pub produces_image_version: bool,
    /// CR-05 §9 — whether the stage supports undo. Set false for stages
    /// that mutate the source asset (e.g. ingest). Decision D-CR05-12:
    /// this flag is the authoritative signal; wizard `undo()` becomes a
    /// wrapper over the runner after P2.
    pub undo_supported: bool,
}

/// CR-05 §26 — one execution of a PipelineStage. Distinct from CR-02
/// `StageRunRecord` (which is keyed by PipelineRun). Keyed by plan_id +
/// stage_id + attempt so the same plan can be branched and re-run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageExecution {
    pub stage_execution_id: String,
    pub plan_id: String,
    pub stage_id: String,
    /// 1-based attempt counter (per CR-05 §9 Re-run semantics).
    pub attempt: u32,
    pub status: String,
    pub input_version_id: Option<String>,
    pub output_artifact_id: Option<String>,
    pub parameters_json: Option<String>,
    /// SHA-256 of the canonicalised parameters; P4 PreviewRun uses this
    /// to guarantee that a full-resolution run re-validates against the
    /// same parameters that produced the preview (Decision D-CR05-5).
    pub parameters_hash: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub resource_usage_json: Option<String>,
    pub error_json: Option<String>,
    /// CR-05 P3 slice 1 — deterministic quality metrics captured at
    /// stage completion (SNR / FWHM / star_count / background
    /// gradient / mean / stddev). JSON for forward compatibility —
    /// the recommendation engine (P3 slice 2) reads this column.
    pub metric_snapshot_json: Option<String>,
    /// CR-05 P6.2 (§23 AI Boundary) — `AiBoundaryLabel` JSON blob
    /// derived from `stage_type` at insertion time. Persisted as a
    /// blob (not split into 4 columns) because:
    ///   1. the shape is small and read together as a unit;
    ///   2. pre-P6.2 rows have no values, so the column is nullable
    ///      and the absence deserialises to `AiBoundaryLabel::default()`
    ///      (classical deterministic processing);
    ///   3. future CR-06 stages may add fields without another
    ///      migration round.
    pub ai_label_json: Option<String>,
}

/// CR-05 P4 slice 3 — preview-before-commit row (CR-05 §11 + §26).
///
/// Persists the result of running a stage handler on a representative
/// region of the input image, so the user can evaluate a processing
/// choice before committing full-resolution compute. Pairs with
/// `StageExecution.parameters_hash` (Decision D-CR05-5): a full-
/// resolution run may not proceed unless its `parameters_hash`
/// matches the preview that the user approved.
///
/// `stage_execution_id` and `source_version_id` are **logical** foreign
/// keys (string ids) — `PreviewRun` rows live in the project
/// (DomainStore) SQLite while the related `stage_executions` row
/// lives in the plan (PipelinePlanStore) SQLite. We do not enforce a
/// SQLite-level FK across the two databases; integrity is verified
/// at the application boundary (Tauri command handlers reject
/// references to non-existent rows).
///
/// `preview_artifact_id` references a row in the same project
/// (DomainStore) `artifacts` table. Null while the preview is
/// pending or running; populated on completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewRun {
    pub preview_id: String,
    pub stage_execution_id: String,
    pub source_version_id: String,
    pub preview_artifact_id: Option<String>,
    pub parameters_json: String,
    pub parameters_hash: String,
    pub status: String,
    /// Downscale factor applied to the input (e.g. 0.25 for a
    /// quarter-size preview). Matches the spec §11 "Preview —
    /// reduced resolution" convention.
    pub scale: f64,
    /// User-facing label, e.g. "Preview — Calibrate @ 0.25".
    pub label: String,
    pub error_json: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

/// CR-05 P4 slice 3 — canonical status values for `PreviewRun.status`.
/// Mirrors the StageExecutionStatus vocabulary used in CR-05 P2
/// (slice 1 + 2.5) but is exposed as constants here so the
/// DomainStore CRUD layer can validate incoming values without
/// depending on the pipeline_plan module.
pub mod preview_status {
    pub const PENDING: &str = "pending";
    pub const RUNNING: &str = "running";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
}

/// CR-05 §26 — status of a PipelinePlan. Mirrors the §8 state vocabulary
/// (Not started / Ready / Running / Completed / Paused / Needs attention /
/// Failed / Recovering) but as a persisted, queryable enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PipelinePlanStatus {
    #[default]
    Draft,
    Ready,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// CR-02 §15 — AI provenance. Deterministic is the default; stochastic /
/// generative operations are explicitly labeled and seed-recorded.
///
/// CR-06 P1 — extended with a three-way `safety_classification` enum
/// (Deterministic / Perceptual / Generative) per CR-06 §16. The
/// `deterministic: bool` field is retained for backward compatibility
/// with callers that already check it; new code should branch on
/// `safety_classification` instead.
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
    /// Legacy CR-02 boolean. Prefer `safety_classification` for
    /// any new code path. The two are kept in sync at write time:
    /// `deterministic == (safety == Deterministic)`.
    pub deterministic: bool,
    /// CR-06 §16 — the safety classification surfaced to the
    /// user. Defaults to `Deterministic` when the field is
    /// missing on a pre-CR-06 row.
    #[serde(default)]
    pub safety_classification: AiSafetyClassification,
    pub experimental: bool,
    pub input_artifact_id: Option<String>,
    pub output_artifact_id: Option<String>,
    /// CR-06 §21 — engine version that produced this operation
    /// (e.g. `astroforge-ai-0.1.0`). Useful when a model ships
    /// new runtime behaviour.
    #[serde(default)]
    pub engine_version: Option<String>,
    /// CR-06 §21 — JSON blob describing the tile configuration
    /// (size, overlap, blending mode). `None` for non-tiled
    /// operations.
    #[serde(default)]
    pub tile_configuration: Option<String>,
    /// CR-06 §21 — JSON blob of resource metrics (peak memory,
    /// wall time, gpu vs cpu time). `None` if the engine did
    /// not record them.
    #[serde(default)]
    pub resource_metrics: Option<String>,
}

/// CR-06 §16 — three-way safety classification. Every AI operation
/// carries one of these and the UI must surface it (never hide the
/// classification per ADR-06.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiSafetyClassification {
    /// Class A — deterministic. Output is reproducible from input
    /// + parameters. Examples: classical denoise, segmentation,
    ///   conventional deconvolution, masking.
    #[default]
    Deterministic,
    /// Class B — learned / perceptual. Model-based reconstruction
    /// that may infer detail. Examples: super-resolution, learned
    /// sharpening, learned restoration.
    Perceptual,
    /// Class C — generative. May synthesize information not
    /// directly present in the source. Opt-in only per ADR-06.3.
    Generative,
}

impl AiSafetyClassification {
    /// Render the classification as a UI-friendly label.
    pub fn as_label(self) -> &'static str {
        match self {
            AiSafetyClassification::Deterministic => "Deterministic",
            AiSafetyClassification::Perceptual => "Perceptual",
            AiSafetyClassification::Generative => "Generative",
        }
    }

    /// Whether this classification requires the user-visible
    /// "perceptual enhancement" disclosure banner (CR-06 §17).
    pub fn requires_disclosure(self) -> bool {
        matches!(
            self,
            AiSafetyClassification::Perceptual | AiSafetyClassification::Generative
        )
    }
}

/// CR-06 §26 — structured observations produced by the image-
/// analysis engine. Rows are written once per analysis run; the
/// payload is a JSON blob conforming to the report shape in P2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAnalysis {
    pub analysis_id: String,
    pub project_id: String,
    pub image_version_id: String,
    /// CR-06 §5.1 — the `ImageIntelligenceProfile` JSON shape
    /// (observations, evidence, confidence). Stored as TEXT so
    /// the schema does not pin a single shape; P2 defines the
    /// deserialization contract.
    pub profile_json: String,
    pub created_at: String,
}

/// CR-06 §26 — semantic regions detected on an Image Version.
/// Used for region-aware enhancement (P5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageRegionKind {
    Stars,
    StarField,
    Nebula,
    Galaxy,
    GlobularCluster,
    OpenCluster,
    PlanetaryLunar,
    BrightCore,
    FaintStructures,
    DustRegions,
    EmissionRegions,
    Background,
    UserMask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRegion {
    pub region_id: String,
    pub image_version_id: String,
    pub kind: ImageRegionKind,
    /// Optional human-readable label (e.g. "M42 core").
    #[serde(default)]
    pub label: Option<String>,
    /// Source provenance: `auto` (segmentation), `parametric`,
    /// `user`. CR-06 §12.
    #[serde(default)]
    pub source: Option<String>,
    /// Pixel-space mask. JSON blob of mask points / shapes so
    /// the schema does not pin a single mask encoding; P5
    /// defines the encoding contract.
    #[serde(default)]
    pub mask_json: Option<String>,
    pub created_at: String,
}

/// CR-06 §26 — AI recommendation row. P3 defines the
/// `recommendation_json` shape; P1 only persists the row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRecommendation {
    pub recommendation_id: String,
    pub project_id: String,
    pub image_version_id: String,
    /// Operation the recommendation points at
    /// (`denoise`, `deconv`, `star_reduce`, `sr`, `inpaint`, …).
    pub operation: String,
    /// Free-form rationale (CR-06 §7 sequencing rationale).
    pub rationale: Option<String>,
    /// 0.0–1.0 confidence score.
    pub confidence: f32,
    /// Risk level surfaced in the UI.
    pub risk_level: String,
    /// JSON blob holding evidence + model candidates + resource
    /// estimates (per CR-06 §27). P3 defines the shape.
    pub payload_json: String,
    pub created_at: String,
}

/// CR-06 §26 — AI mask row. P5 writes these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMask {
    pub mask_id: String,
    pub project_id: String,
    pub image_version_id: String,
    /// Mask provenance: `auto` / `parametric` / `user` /
    /// `composite`. CR-06 §12.
    pub provenance: String,
    /// Parent mask ids when this is a composite (JSON array of
    /// strings). Empty for the leaf kinds.
    #[serde(default)]
    pub parents_json: Option<String>,
    /// Mask encoding (JSON blob; P5 defines the encoding).
    pub mask_json: String,
    pub created_at: String,
}

/// CR-06 §22 — ordered list of AI operations applied on top of
/// an Image Version. P4 writes these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementStack {
    pub stack_id: String,
    pub project_id: String,
    pub source_image_version_id: String,
    /// JSON array of operation ids in execution order. P4
    /// defines the operation-id shape.
    pub operation_ids_json: String,
    /// Optional branched-from version id when the stack is a
    /// branch (CR-06 §24).
    #[serde(default)]
    pub branched_from_version_id: Option<String>,
    pub created_at: String,
}

/// CR-06 §26 — temporary preview artifact reference. P4 writes
/// these. Rows are not Image Versions (they are not surfaced as
/// user-meaningful versions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementPreview {
    pub preview_id: String,
    pub project_id: String,
    pub source_image_version_id: String,
    pub operation_id: String,
    pub artifact_id: Option<String>,
    pub parameters_json: String,
    pub status: String,
    pub created_at: String,
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
            safety_classification: AiSafetyClassification::Deterministic,
            experimental: false,
            input_artifact_id: None,
            output_artifact_id: None,
            engine_version: None,
            tile_configuration: None,
            resource_metrics: None,
        };
        let json = serde_json::to_string(&op).expect("serialize");
        let back: AiOperation = serde_json::from_str(&json).expect("deserialize");
        assert!(back.deterministic);
        assert!(!back.experimental);
    }
}
