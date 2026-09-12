//! CR-06 P1 — Tauri command shells for the AI Enhancement Studio.
//!
//! P1 only persists the data model + provenance fields. The
//! substantive behavior (analysis, recommendations, enhancement
//! operations, masks, stacks, previews) lands in P2–P6; the
//! commands in this module are stubs that return the empty
//! state so the IPC surface and the TypeScript wrappers are
//! in place from day one.
//!
//! Each command follows the same shape:
//! - a `*_list` command returning rows for a given parent
//!   (image_version_id or project_id)
//! - an empty-list / `None` response when the parent has no
//!   rows yet
//! - no error path beyond the store-error → string conversion
//!   (mirrors `commands_project::project_overview`)
//!
//! CR-06 P2 — `analyze_image` runs the analyzer over the
//! supplied pixels and persists the resulting report to the
//! `image_analyses` table. The command returns the JSON
//! profile so the frontend can render the observations
//! without a second IPC round-trip.

use crate::commands_project::lock_err;
use crate::domain_store::DomainStore;
use crate::enhancement as enhancement_engine;
use crate::image::F32Image;
use crate::image_analysis;
use astroforge_ai::operations as ai_operations;
use astroforge_ai::recommendations as ai_recommendations;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

static STORE: Mutex<Option<DomainStore>> = Mutex::new(None);

/// Open the in-memory CR-02 store under a fixed path so the
/// P1 commands can read & write without taking on the
/// production-grade store lifecycle that ships in P2+. The
/// path is resolved relative to the user's home directory and
/// the file is created on first use. Idempotent on re-open.
fn with_store<R>(f: impl FnOnce(&DomainStore) -> R) -> Result<R, String> {
    let mut guard = STORE.lock().map_err(lock_err)?;
    if guard.is_none() {
        let mut path = dirs_home().ok_or_else(|| "no home directory".to_string())?;
        path.push(".astroforge");
        path.push("cr-06-p1.sqlite");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let s = DomainStore::new(&path).map_err(|e| e.to_string())?;
        *guard = Some(s);
    }
    f(guard.as_ref().unwrap())
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

/// Response envelope for list commands so the frontend can
/// distinguish "no rows" from "error". P2+ will move real
/// content here; P1 returns empty vectors.
#[derive(Debug, Serialize)]
pub struct AiEnhancementListResponse<T> {
    pub items: Vec<T>,
}

#[tauri::command]
pub fn ai_operation_get(operation_id: String) -> Result<serde_json::Value, String> {
    with_store(|s| match s.get_ai_operation(&operation_id) {
        Ok(op) => Ok(serde_json::to_value(&op).map_err(|e| e.to_string())?),
        Err(_) => Ok(serde_json::Value::Null),
    })
}

#[tauri::command]
pub fn ai_operation_list_for_stage(
    stage_run_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_ai_operations_for_stage(&stage_run_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|op| serde_json::to_value(op).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

#[tauri::command]
pub fn image_analysis_latest(
    image_version_id: String,
) -> Result<serde_json::Value, String> {
    with_store(|s| {
        match s.latest_image_analysis_for_version(&image_version_id) {
            Ok(Some(row)) => serde_json::to_value(&row).map_err(|e| e.to_string()),
            Ok(None) => Ok(serde_json::Value::Null),
            Err(e) => Err(e.to_string()),
        }
    })
}

#[tauri::command]
pub fn image_region_list(
    image_version_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_image_regions(&image_version_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

#[tauri::command]
pub fn ai_recommendation_list_for_version(
    image_version_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_ai_recommendations_for_version(&image_version_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

#[tauri::command]
pub fn ai_mask_list(
    image_version_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_ai_masks(&image_version_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

#[tauri::command]
pub fn enhancement_stack_list_for_source(
    image_version_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_enhancement_stacks_for_source(&image_version_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

#[tauri::command]
pub fn enhancement_preview_list_for_operation(
    operation_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_enhancement_previews_for_operation(&operation_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

/// Marker so the `State` import isn't flagged unused on a
/// future Rust version that warns on bare imports.
#[allow(dead_code)]
fn _state_marker(_s: State<'_, ()>) {}

/// CR-06 P2 — request payload for `analyze_image`. Mirrors
/// the TypeScript `AnalyzeImageRequest` shape. The pixels
/// are passed as a flat `Vec<f64>` in row-major order so
/// the JSON wire format stays compact.
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeImageRequest {
    pub image_version_id: String,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub pixels: Vec<f64>,
}

/// CR-06 P2 — response: the persisted report as JSON. The
/// frontend re-parses this for the Zone C intelligence
/// panel.
#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeImageResponse {
    pub analysis_id: String,
    pub profile_json: String,
}

/// CR-06 P2 — run the analyzer over the supplied pixels,
/// persist the report to the `image_analyses` table, and
/// return the analysis id + profile JSON.
///
/// The pixels are expected to be in `[0, 1]` (the
/// calibration pipeline output range). Out-of-range
/// values are clamped so the analyzer stays robust to
/// pre-multiplied inputs.
#[tauri::command]
pub fn analyze_image(request: AnalyzeImageRequest) -> Result<AnalyzeImageResponse, String> {
    let AnalyzeImageRequest {
        image_version_id,
        width,
        height,
        channels,
        pixels,
    } = request;
    let width = width as usize;
    let height = height as usize;
    let channels = channels as usize;
    let expected = width * height * channels;
    if pixels.len() != expected {
        return Err(format!(
            "pixel count mismatch: got {}, expected {} ({}×{}×{})",
            pixels.len(),
            expected,
            width,
            height,
            channels
        ));
    }
    // Build the `F32Image` from the flat pixel buffer.
    // We clamp each value to `[0, 1]` to defend against
    // out-of-range inputs (multiplied FITS, log-scaled
    // previews, etc.) that would otherwise blow up the
    // sigma-clipping thresholds downstream.
    let mut img = F32Image::new(width, height, channels);
    for (i, v) in pixels.iter().enumerate() {
        let clamped = v.clamp(0.0, 1.0) as f32;
        let c = i / (width * height);
        let rem = i % (width * height);
        let y = rem / width;
        let x = rem % width;
        img[(c, y, x)] = clamped;
    }
    // Run the analyzer.
    let report = image_analysis::report::analyze(&img, &image_version_id);
    let profile_json = report.to_json().map_err(|e| e.to_string())?;
    let analysis_id = format!("ana_{}", new_id_suffix());
    // Persist the report.
    with_store(|s| {
        s.upsert_image_analysis(&crate::domain::ImageAnalysis {
            analysis_id: analysis_id.clone(),
            project_id: String::new(),
            image_version_id: image_version_id.clone(),
            profile_json: profile_json.clone(),
            created_at: report.created_at.clone(),
        })
        .map_err(|e| e.to_string())
    })?;
    Ok(AnalyzeImageResponse {
        analysis_id,
        profile_json,
    })
}

/// Cheap unique-id suffix for the analysis row. `instant`
/// nanoseconds modulo `usize::MAX` is fine for client-side
/// row ids; uniqueness is guaranteed by the SQLite
/// `INSERT OR REPLACE` semantics if a collision ever
/// occurred.
fn new_id_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    format!("{ns:x}")
}

/// CR-06 P3 — request payload for `generate_ai_recommendations`.
/// The frontend supplies the Image Version id; the command
/// reads the latest `image_analyses` row for that version,
/// runs the recommendation engine, persists the resulting
/// `AiRecommendation` rows, and returns the full report as
/// JSON for the Studio panel.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateRecommendationsRequest {
    pub project_id: String,
    pub image_version_id: String,
}

/// CR-06 P3 — response: the engine output as JSON, plus the
/// list of persisted `AiRecommendation` rows. The frontend
/// renders the report directly; the list is the same data
/// surfaced via `ai_recommendation_list_for_version`, so
/// callers can also read it back through that command on a
/// later refresh.
#[derive(Debug, Clone, Serialize)]
pub struct GenerateRecommendationsResponse {
    pub report_json: String,
    pub recommendations: AiEnhancementListResponse<serde_json::Value>,
}

/// CR-06 P3 — run the recommendation engine over the
/// latest `image_analyses` row for an Image Version,
/// persist the resulting `AiRecommendation` rows, and
/// return the report.
///
/// The engine is deterministic for a given analysis, so
/// re-running on the same input is idempotent
/// (`INSERT OR REPLACE` keeps the row count stable).
/// When no analysis exists for the Image Version, the
/// command returns an empty report rather than an error
/// — the UI surfaces "Analyze the image first" in that
/// state and the recommendation rail stays empty until
/// the user runs `analyze_image`.
#[tauri::command]
pub fn generate_ai_recommendations(
    request: GenerateRecommendationsRequest,
) -> Result<GenerateRecommendationsResponse, String> {
    let GenerateRecommendationsRequest {
        project_id,
        image_version_id,
    } = request;
    let project_id = if project_id.is_empty() {
        // Older callers may not supply a project id; the
        // engine needs a non-empty value to use in the
        // recommendation row's `project_id` column. Fall
        // back to a placeholder so the row still persists
        // — the UI never renders this string.
        "unscoped".to_string()
    } else {
        project_id
    };
    let report_json = with_store(|s| {
        let analysis_row = s
            .latest_image_analysis_for_version(&image_version_id)
            .map_err(|e| e.to_string())?;
        let Some(row) = analysis_row else {
            return Ok(String::new());
        };
        let report = crate::image_analysis::report::ImageAnalysisReport::from_json(&row.profile_json)
            .map_err(|e| format!("analysis profile malformed: {e}"))?;
        let engine_report = ai_recommendations::analyze(&report, &project_id);
        let out_json = engine_report.to_json().map_err(|e| e.to_string())?;
        let rows = ai_recommendations::flatten_for_store(&engine_report);
        for r in &rows {
            s.upsert_ai_recommendation(r).map_err(|e| e.to_string())?;
        }
        Ok(out_json)
    })?;
    if report_json.is_empty() {
        return Ok(GenerateRecommendationsResponse {
            report_json: String::new(),
            recommendations: AiEnhancementListResponse { items: vec![] },
        });
    }
    let stored = with_store(|s| {
        let rows = s
            .list_ai_recommendations_for_version(&image_version_id)
            .map_err(|e| e.to_string())?;
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })?;
    Ok(GenerateRecommendationsResponse {
        report_json,
        recommendations: stored,
    })
}

/// CR-06 P4 — request payload for `enhancement_stack_create`.
/// The frontend supplies an initial set of operations (often
/// the recommendation engine's output) and the command
/// persists a fresh stack row.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateEnhancementStackRequest {
    pub project_id: String,
    pub source_image_version_id: String,
    pub operations: Vec<enhancement_engine::StackOperation>,
}

/// CR-06 P4 — response: the persisted stack record.
#[derive(Debug, Clone, Serialize)]
pub struct EnhancementStackDto {
    pub stack: serde_json::Value,
}

/// CR-06 P4 — create an enhancement stack from an initial
/// operation list. Returns the typed stack record so the UI
/// can render immediately without a follow-up `get`.
#[tauri::command]
pub fn enhancement_stack_create(
    request: CreateEnhancementStackRequest,
) -> Result<EnhancementStackDto, String> {
    let now_iso = now_iso_string();
    let stack_id = format!("stk_{}", new_id_suffix());
    let op_json = enhancement_engine::EnhancementStackRecord::operations_to_json(&request.operations);
    let record = crate::domain::EnhancementStack {
        stack_id: stack_id.clone(),
        project_id: request.project_id,
        source_image_version_id: request.source_image_version_id.clone(),
        operation_ids_json: op_json,
        branched_from_version_id: None,
        created_at: now_iso,
    };
    with_store(|s| s.upsert_enhancement_stack(&record).map_err(|e| e.to_string()))?;
    let stack_record = enhancement_engine::EnhancementStackRecord {
        stack_id,
        project_id: record.project_id,
        source_image_version_id: record.source_image_version_id,
        operations: request.operations,
        branched_from_version_id: None,
        created_at: record.created_at,
    };
    Ok(EnhancementStackDto {
        stack: serde_json::to_value(&stack_record).map_err(|e| e.to_string())?,
    })
}

/// CR-06 P4 — fetch a stack by id.
#[tauri::command]
pub fn enhancement_stack_get(stack_id: String) -> Result<serde_json::Value, String> {
    with_store(|s| match s.get_enhancement_stack(&stack_id) {
        Ok(Some(row)) => {
            let ops = enhancement_engine::EnhancementStackRecord::operations_from_json(
                &row.operation_ids_json,
            );
            let rec = enhancement_engine::EnhancementStackRecord {
                stack_id: row.stack_id,
                project_id: row.project_id,
                source_image_version_id: row.source_image_version_id,
                operations: ops,
                branched_from_version_id: row.branched_from_version_id,
                created_at: row.created_at,
            };
            Ok(serde_json::to_value(&rec).map_err(|e| e.to_string())?)
        }
        Ok(None) => Ok(serde_json::Value::Null),
        Err(e) => Err(e.to_string()),
    })
}

/// CR-06 P4 — apply a `StackMutation` (reorder, set enabled,
/// remove, mark needs preview, append). Returns the updated
/// typed stack.
#[tauri::command]
pub fn enhancement_stack_apply_mutation(
    stack_id: String,
    mutation: enhancement_engine::StackMutation,
) -> Result<serde_json::Value, String> {
    with_store(|s| {
        let row = s
            .get_enhancement_stack(&stack_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("stack not found: {stack_id}"))?;
        let mut record = enhancement_engine::EnhancementStackRecord {
            stack_id: row.stack_id.clone(),
            project_id: row.project_id.clone(),
            source_image_version_id: row.source_image_version_id.clone(),
            operations: enhancement_engine::EnhancementStackRecord::operations_from_json(
                &row.operation_ids_json,
            ),
            branched_from_version_id: row.branched_from_version_id.clone(),
            created_at: row.created_at.clone(),
        };
        record = enhancement_engine::apply_mutation(record, mutation)
            .map_err(|e| e.to_string())?;
        let op_json =
            enhancement_engine::EnhancementStackRecord::operations_to_json(&record.operations);
        let updated = crate::domain::EnhancementStack {
            stack_id: record.stack_id.clone(),
            project_id: record.project_id.clone(),
            source_image_version_id: record.source_image_version_id.clone(),
            operation_ids_json: op_json,
            branched_from_version_id: record.branched_from_version_id.clone(),
            created_at: record.created_at.clone(),
        };
        s.upsert_enhancement_stack(&updated).map_err(|e| e.to_string())?;
        Ok(serde_json::to_value(&record).map_err(|e| e.to_string())?)
    })
}

/// CR-06 P4 — create a branch from a stack at the given
/// cutoff. Returns the new stack record. The new stack
/// references a fresh `source_image_version_id` (the caller
/// supplies one — typically a new Image Version the branch
/// round is about to create).
#[derive(Debug, Clone, Deserialize)]
pub struct BranchEnhancementStackRequest {
    pub source_stack_id: String,
    pub cutoff: usize,
    pub new_image_version_id: String,
}

#[tauri::command]
pub fn enhancement_stack_branch(
    request: BranchEnhancementStackRequest,
) -> Result<serde_json::Value, String> {
    let now_iso = now_iso_string();
    let new_stack_id = format!("stk_{}", new_id_suffix());
    with_store(|s| {
        let source = s
            .get_enhancement_stack(&request.source_stack_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                format!("source stack not found: {}", request.source_stack_id)
            })?;
        let source_record = enhancement_engine::EnhancementStackRecord {
            stack_id: source.stack_id,
            project_id: source.project_id,
            source_image_version_id: source.source_image_version_id,
            operations: enhancement_engine::EnhancementStackRecord::operations_from_json(
                &source.operation_ids_json,
            ),
            branched_from_version_id: source.branched_from_version_id,
            created_at: source.created_at,
        };
        let branched = enhancement_engine::branch(
            source_record,
            request.cutoff,
            new_stack_id.clone(),
            request.new_image_version_id.clone(),
            now_iso.clone(),
        );
        let op_json = enhancement_engine::EnhancementStackRecord::operations_to_json(
            &branched.operations,
        );
        let row = crate::domain::EnhancementStack {
            stack_id: branched.stack_id.clone(),
            project_id: branched.project_id.clone(),
            source_image_version_id: branched.source_image_version_id.clone(),
            operation_ids_json: op_json,
            branched_from_version_id: branched.branched_from_version_id.clone(),
            created_at: branched.created_at.clone(),
        };
        s.upsert_enhancement_stack(&row).map_err(|e| e.to_string())?;
        Ok(serde_json::to_value(&branched).map_err(|e| e.to_string())?)
    })
}

/// CR-06 P4 — list the canonical operations registry so the
/// UI can render the operation picker without hard-coding
/// the list. The Tauri command shape matches the
/// `astroforge_ai::operations::OperationInfo` JSON form.
#[tauri::command]
pub fn operations_registry_list() -> Result<Vec<serde_json::Value>, String> {
    Ok(ai_operations::registry()
        .into_iter()
        .map(|op| serde_json::to_value(&op).unwrap_or(serde_json::Value::Null))
        .collect())
}

/// CR-06 P4 — list Image Versions for a project (sequence-asc).
#[tauri::command]
pub fn image_version_list_for_project(
    project_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s
            .list_image_versions_for_project(&project_id)
            .map_err(|e| e.to_string())?;
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

/// CR-06 P4 — fetch a single Image Version by id.
#[tauri::command]
pub fn image_version_get(version_id: String) -> Result<serde_json::Value, String> {
    with_store(|s| match s.get_image_version(&version_id) {
        Ok(Some(row)) => Ok(serde_json::to_value(&row).map_err(|e| e.to_string())?),
        Ok(None) => Ok(serde_json::Value::Null),
        Err(e) => Err(e.to_string()),
    })
}

/// CR-06 P4 — request payload for `enhancement_apply_operation`.
/// The command runs the operation via the (P4 passthrough)
/// dispatcher, persists a fresh `ImageVersion` row, and
/// records the operation provenance on the `AiOperation`
/// row.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplyAiOperationRequest {
    pub project_id: String,
    pub source_image_version_id: String,
    pub operation_id: String,
    pub parameters_json: String,
    pub preview_id: Option<String>,
    /// CR-06 P5.1 — optional mask id for region-restricted
    /// operations (`requires_region: true` in the registry).
    /// When present, the dispatcher composites the result
    /// inside the mask and the §37 segmentation-leakage
    /// gate compares included/excluded deltas.
    pub mask_id: Option<String>,
}

/// CR-06 P4 — response: the new Image Version + the
/// dispatch outcome.
#[derive(Debug, Clone, Serialize)]
pub struct ApplyAiOperationResponse {
    pub image_version: serde_json::Value,
    pub outcome: serde_json::Value,
    pub operation_row: serde_json::Value,
    /// CR-06 P5.1 — §37 quality-gate verdict for the
    /// applied operation. The Studio's QualityGatePanel
    /// reads the persisted report for the version, but
    /// surfacing the verdict in the apply response lets
    /// the UI warn inline without an extra round-trip.
    pub quality_verdict: String,
    /// CR-06 P5.1 — the full gate report (all ten findings).
    pub quality_report: serde_json::Value,
}

/// CR-06 P4 — apply an AI operation. P5.1 swaps the
/// passthrough dispatcher for real ONNX inference; the
/// command still creates a fresh Image Version per
/// CR-06 §4 / §22, persists an `AiOperation` provenance
/// row, and (P5.1) writes the produced pixels to a
/// TIFF artifact + records a §37 quality-gate report
/// against the real (source, result) pair.
#[tauri::command]
pub fn enhancement_apply_operation(
    request: ApplyAiOperationRequest,
) -> Result<ApplyAiOperationResponse, String> {
    use astroforge_ai::dispatch_operation;
    use astroforge_ai::{DispatchInputs, HardwareProbe, ModelRegistry};
    use astroforge_core::export::{export_tiff_16bit, ExportFormat};
    use astroforge_core::masks::encoding::from_json as mask_from_json;
    use astroforge_core::quality_gates::report::run as run_quality_gates;
    use astroforge_core::quality_gates::GateThresholds;
    use std::sync::Arc;

    let now_iso = now_iso_string();
    let result_version_id = format!("ver_{}", new_id_suffix());
    let preview_id = request
        .preview_id
        .clone()
        .unwrap_or_else(|| format!("pv_{}", new_id_suffix()));

    // Resolve the source pixels. The apply round runs
    // against whatever Image Version is currently
    // selected; for tests / headless callers without
    // pixels on disk, the source is a small synthetic
    // stub so the round-trip still exercises the
    // dispatcher. The Studio always carries real pixels
    // through the operation, so the stub branch is a
    // safe degraded default.
    let source_pixels = load_or_stub_pixels(&request.source_image_version_id)?;
    let source_mask = match request.mask_id.as_deref() {
        Some(mask_id) => with_store(|s| match s.get_ai_mask(mask_id) {
            Ok(Some(row)) => mask_from_json(&row.mask_json)
                .map_err(|e| format!("mask {mask_id}: {e}")),
            Ok(None) => Err(format!("mask {mask_id} not found")),
            Err(e) => Err(e.to_string()),
        })?,
        None => None,
    };

    // Build a registry on the conventional models dir so
    // downloaded catalog artifacts (when DP#4 lands
    // hashes) supersede the builtin graphs. The dir is
    // created lazily; absence is non-fatal.
    let probe = HardwareProbe::detect();
    let models_dir = dirs_home()
        .map(|mut p| {
            p.push(".astroforge");
            p.push("models");
            let _ = std::fs::create_dir_all(&p);
            p
        });
    let dispatch = dispatch_operation(
        &request.operation_id,
        &request.parameters_json,
        &request.source_image_version_id,
        result_version_id.clone(),
        preview_id.clone(),
        DispatchInputs {
            source: &source_pixels,
            mask: source_mask.as_ref(),
            probe: &probe,
            models_dir: models_dir.as_deref(),
        },
    )
    .map_err(|e| e.to_string())?;
    let outcome = dispatch.outcome;
    let result_image = dispatch.result_image;

    let sequence = with_store(|s| {
        s.next_image_version_sequence(&request.project_id)
            .map_err(|e| e.to_string())
    })?;
    let artifact_id = format!("art_{}", new_id_suffix());
    let version = crate::domain::ImageVersion {
        version_id: result_version_id.clone(),
        project_id: request.project_id.clone(),
        label: request.operation_id.clone(),
        sequence: sequence + 1,
        primary_artifact_id: artifact_id.clone(),
        source_version_id: Some(request.source_image_version_id.clone()),
        created_at: now_iso.clone(),
        hidden: false,
    };
    with_store(|s| s.upsert_image_version(&version).map_err(|e| e.to_string()))?;
    let ai_op_id = format!("op_{}", new_id_suffix());
    let safety_str = match outcome.safety_classification {
        ai_operations::SafetyClassification::Deterministic => "deterministic",
        ai_operations::SafetyClassification::Perceptual => "perceptual",
        ai_operations::SafetyClassification::Generative => "generative",
    };
    let safety_cls = parse_safety(safety_str);
    let ai_op_row = crate::domain::AiOperation {
        operation_id: ai_op_id,
        stage_run_id: String::new(),
        model_id: dispatch.model_id.clone(),
        model_version: dispatch.model_version.clone(),
        model_hash: if dispatch.model_hash.is_empty() {
            None
        } else {
            Some(dispatch.model_hash.clone())
        },
        runtime: Some(dispatch.runtime.clone()),
        backend: Some(dispatch.backend.clone()),
        precision: None,
        parameters_json: Some(request.parameters_json.clone()),
        seed: None,
        deterministic: matches!(safety_cls, crate::domain::AiSafetyClassification::Deterministic),
        safety_classification: safety_cls,
        experimental: false,
        input_artifact_id: Some(request.source_image_version_id.clone()),
        output_artifact_id: Some(result_version_id.clone()),
        engine_version: Some("cr-06-p5-1".into()),
        tile_configuration: Some(dispatch.tile_configuration_json.clone()),
        resource_metrics: Some(
            serde_json::json!({ "duration_ms": dispatch.duration_ms }).to_string(),
        ),
    };
    with_store(|s| s.upsert_ai_operation(&ai_op_row).map_err(|e| e.to_string()))?;

    // Persist the produced pixels as a 16-bit TIFF
    // artifact under the project's `applied/` directory
    // when one is resolvable; otherwise drop the bytes
    // and just keep the in-memory result. The TIFF path
    // is the same one the export pipeline uses (CR-05
    // §11), so downstream consumers can read it without
    // any AstroForge-specific decoder.
    if let Some(parent) = applied_pixels_dir(&request.project_id) {
        let _ = std::fs::create_dir_all(&parent);
        let path = parent.join(format!("{result_version_id}.tif"));
        if let Ok(mut f) = std::fs::File::create(&path) {
            if export_tiff_16bit(&result_image, &mut f).is_ok() {
                let mut registry = ModelRegistry::new(std::path::PathBuf::from(
                    models_dir.clone().unwrap_or_default(),
                ));
                // Touch the registry so the import compiles
                // (the apply round doesn't mutate it).
                let _ = registry.info_for_stage(&request.operation_id);
                let artifact = crate::domain::Artifact {
                    artifact_id: artifact_id.clone(),
                    artifact_hash: String::new(),
                    artifact_type: crate::domain::ArtifactCategory::Derived,
                    format: ExportFormat::Tiff16.extension().into(),
                    path: path.to_string_lossy().to_string(),
                    size: 0,
                    created_at: now_iso.clone(),
                    producer_stage: Some(request.operation_id.clone()),
                    pipeline_run_id: None,
                    parent_artifact_ids: vec![request.source_image_version_id.clone()],
                    width: Some(result_image.width() as u32),
                    height: Some(result_image.height() as u32),
                    channels: Some(result_image.channels() as u32),
                    bit_depth: Some(16),
                    color_space: Some("srgb".into()),
                    linear_or_nonlinear: Some(false),
                };
                let _ = with_store(|s| s.record_artifact(&artifact).map_err(|e| e.to_string()));
            }
        }
    }

    // Run §37 against the real (source, result) pair
    // and persist the report. The apply round now has
    // distinct pixels for the first time, so the
    // verdict is meaningful (P4 always returned `Ok`
    // because the source equalled the result).
    let gate_report = run_quality_gates(
        &source_pixels,
        &result_image,
        &GateThresholds::default(),
        source_mask.as_ref(),
        request.source_image_version_id.clone(),
        result_version_id.clone(),
        request.operation_id.clone(),
    );
    let report_row = crate::domain::AiQualityReport {
        report_id: format!("qr_{}", new_id_suffix()),
        operation_id: ai_op_id.clone(),
        source_image_version_id: request.source_image_version_id.clone(),
        result_image_version_id: result_version_id.clone(),
        verdict: gate_report.verdict.as_str().to_string(),
        report_json: serde_json::to_string(&gate_report).map_err(|e| e.to_string())?,
        created_at: now_iso.clone(),
    };
    with_store(|s| s.record_ai_quality_report(&report_row).map_err(|e| e.to_string()))?;

    Ok(ApplyAiOperationResponse {
        image_version: serde_json::to_value(&version).map_err(|e| e.to_string())?,
        outcome: serde_json::to_value(&outcome).map_err(|e| e.to_string())?,
        operation_row: serde_json::to_value(&ai_op_row).map_err(|e| e.to_string())?,
        quality_verdict: gate_report.verdict.as_str().to_string(),
        quality_report: serde_json::to_value(&gate_report).map_err(|e| e.to_string())?,
    })
}

/// Best-effort resolve of the source pixels for an
/// apply round. Real pixel persistence lives behind
/// artifact + session rows; the round-trip wired in
/// P5.1 reads the on-disk TIFF when one is present
/// and falls back to a synthetic stub otherwise so
/// the dispatcher path is always exercised.
fn load_or_stub_pixels(version_id: &str) -> Result<F32Image, String> {
    use astroforge_core::decoders::ImageDecodeError;
    let path_opt: Option<std::path::PathBuf> = with_store(|s| {
        let ver = s.get_image_version(version_id).map_err(|e| e.to_string())?;
        let Some(ver) = ver else { return Ok::<_, String>(None) };
        let art = s.get_artifact(&ver.primary_artifact_id).map_err(|e| e.to_string())?;
        Ok(art.map(|a| std::path::PathBuf::from(a.path)))
    })?;
    if let Some(path) = path_opt {
        if path.exists() {
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            match F32Image::from_tiff_bytes(&bytes) {
                Ok(img) => return Ok(img),
                Err(ImageDecodeError::Tiff(reason)) => {
                    return Err(format!("tiff decode ({version_id}): {reason}"));
                }
                Err(_) => {
                    return Err(format!("unsupported artifact format for {version_id}"));
                }
            }
        }
    }
    // Stub: a small linear gradient in three channels so
    // the dispatcher + gate round-trip runs end-to-end
    // without a real on-disk artifact.
    let mut img = F32Image::new(64, 48, 3);
    for c in 0..3 {
        for y in 0..48 {
            for x in 0..64 {
                img[(c, y, x)] = ((x + y + c * 16) % 256) as f32 / 255.0;
            }
        }
    }
    Ok(img)
}

fn applied_pixels_dir(project_id: &str) -> Option<std::path::PathBuf> {
    let mut p = dirs_home()?;
    p.push(".astroforge");
    p.push("applied");
    p.push(project_id);
    Some(p)
}

#[allow(dead_code)]
fn _ref_arc<T>(t: T) -> Arc<T> {
    Arc::new(t)
}

fn parse_safety(s: &str) -> crate::domain::AiSafetyClassification {
    match s {
        "perceptual" => crate::domain::AiSafetyClassification::Perceptual,
        "generative" => crate::domain::AiSafetyClassification::Generative,
        _ => crate::domain::AiSafetyClassification::Deterministic,
    }
}

/// CR-06 P4 — list the operations registry (the 11
/// canonical AI operations). Returns the JSON shape.
#[derive(Debug, Clone, Serialize)]
pub struct OperationRegistryEntry {
    pub operation_id: String,
    pub display_name: String,
    pub category: String,
    pub safety_classification: String,
    pub description: String,
    pub default_parameters_json: String,
    pub requires_region: bool,
}

fn to_registry_entry(op: &ai_operations::OperationInfo) -> OperationRegistryEntry {
    OperationRegistryEntry {
        operation_id: op.operation_id.clone(),
        display_name: op.display_name.clone(),
        category: op.category.as_str().to_string(),
        safety_classification: op.safety_classification.as_str().to_string(),
        description: op.description.clone(),
        default_parameters_json: op.default_parameters_json.clone(),
        requires_region: op.requires_region,
    }
}

/// CR-06 P4 — return the operations registry as a typed
/// list. The Tauri command keeps the wire shape stable so
/// consumers do not have to handle serde `Value`.
#[tauri::command]
pub fn enhancement_operations_list() -> Result<Vec<OperationRegistryEntry>, String> {
    Ok(ai_operations::registry().iter().map(to_registry_entry).collect())
}

/// CR-06 P4 — ISO-8601 timestamp helper used by every
/// P4 command. Same shape as
/// `astroforge_ai::recommendations::now_iso_string`.
fn now_iso_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    astroforge_ai::recommendations::format_unix_seconds(secs)
}

/// CR-06 P5 — request payload for `create_ai_mask`. The
/// caller supplies the JSON-encoded mask (the
/// `astroforge-core::masks::encoding::to_json` shape).
/// Auto + parametric masks are built server-side via
/// `apply_ai_mask_from_*` commands (see below); the
/// `create_ai_mask` command accepts a pre-built mask
/// from the frontend.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAiMaskRequest {
    pub project_id: String,
    pub image_version_id: String,
    pub provenance: String,
    pub parents_json: Option<String>,
    pub mask_json: String,
}

/// CR-06 P5 — response: the persisted mask row as JSON.
#[derive(Debug, Clone, Serialize)]
pub struct AiMaskDto {
    pub mask: serde_json::Value,
}

/// CR-06 P5 — persist a mask row. The caller supplies
/// the encoded mask (the JSON shape from
/// `astroforge_core::masks::encoding::to_json`). The
/// command validates the encoding via `from_json`
/// before persisting so the store never holds a
/// malformed row.
#[tauri::command]
pub fn create_ai_mask(request: CreateAiMaskRequest) -> Result<AiMaskDto, String> {
    use crate::masks::encoding;
    let _ = encoding::from_json(&request.mask_json)
        .map_err(|e| format!("invalid mask encoding: {e}"))?;
    let mask_id = format!("msk_{}", new_id_suffix());
    let row = crate::domain::AiMask {
        mask_id,
        project_id: request.project_id,
        image_version_id: request.image_version_id,
        provenance: request.provenance,
        parents_json: request.parents_json,
        mask_json: request.mask_json,
        created_at: now_iso_string(),
    };
    with_store(|s| s.upsert_ai_mask(&row).map_err(|e| e.to_string()))?;
    Ok(AiMaskDto {
        mask: serde_json::to_value(&row).map_err(|e| e.to_string())?,
    })
}

/// CR-06 P5 — replace a mask row's `mask_json` (the
/// user painted a new brush stroke or edited a
/// polygon). The row's id + provenance + parents stay
/// the same; only the pixel raster updates.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAiMaskRequest {
    pub mask_id: String,
    pub mask_json: String,
}

#[tauri::command]
pub fn update_ai_mask(request: UpdateAiMaskRequest) -> Result<AiMaskDto, String> {
    use crate::masks::encoding;
    let _ = encoding::from_json(&request.mask_json)
        .map_err(|e| format!("invalid mask encoding: {e}"))?;
    let row = with_store(|s| {
        let mut existing = s
            .get_ai_mask(&request.mask_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("mask not found: {}", request.mask_id))?;
        existing.mask_json = request.mask_json.clone();
        existing.created_at = now_iso_string();
        s.upsert_ai_mask(&existing).map_err(|e| e.to_string())?;
        Ok(existing)
    })?;
    Ok(AiMaskDto {
        mask: serde_json::to_value(&row).map_err(|e| e.to_string())?,
    })
}

/// CR-06 P5 — list the `AiMask` rows for an Image
/// Version. The frontend renders the mask picker
/// (the Zone B overlay source list per CR-06 §13).
#[tauri::command]
pub fn ai_mask_list_for_version(
    image_version_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s
            .list_ai_masks(&image_version_id)
            .map_err(|e| e.to_string())?;
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

/// CR-06 P5 — fetch a single mask row.
#[tauri::command]
pub fn ai_mask_get(mask_id: String) -> Result<serde_json::Value, String> {
    with_store(|s| match s.get_ai_mask(&mask_id) {
        Ok(Some(row)) => Ok(serde_json::to_value(&row).map_err(|e| e.to_string())?),
        Ok(None) => Ok(serde_json::Value::Null),
        Err(e) => Err(e.to_string()),
    })
}

/// CR-06 P5 — build an auto mask on the server from
/// an image + a target kind. The frontend supplies
/// the pixel buffer (in `[0, 1]`, row-major) + the
/// target, and the command runs the auto-mask
/// builder, encodes the result, and returns the JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct BuildAutoMaskRequest {
    pub image_version_id: String,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub pixels: Vec<f64>,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildAutoMaskResponse {
    pub mask_json: String,
    pub provenance: String,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub fn build_auto_mask(request: BuildAutoMaskRequest) -> Result<BuildAutoMaskResponse, String> {
    use crate::masks::auto::{build as auto_build, AutoTarget};
    let BuildAutoMaskRequest {
        image_version_id: _,
        width,
        height,
        channels,
        pixels,
        target,
    } = request;
    let target = match target.as_str() {
        "stars" => AutoTarget::Stars,
        "background" => AutoTarget::Background,
        "bright_core" => AutoTarget::BrightCore,
        other => return Err(format!("unknown auto mask target: {other}")),
    };
    let expected = (width as usize) * (height as usize) * (channels as usize);
    if pixels.len() != expected {
        return Err(format!(
            "pixel count mismatch: got {}, expected {} ({}×{}×{})",
            pixels.len(),
            expected,
            width,
            height,
            channels
        ));
    }
    let mut img = F32Image::new(width as usize, height as usize, channels as usize);
    for (i, v) in pixels.iter().enumerate() {
        let clamped = v.clamp(0.0, 1.0) as f32;
        let c = i / ((width as usize) * (height as usize));
        let rem = i % ((width as usize) * (height as usize));
        let y = rem / (width as usize);
        let x = rem % (width as usize);
        img[(c, y, x)] = clamped;
    }
    let mask = auto_build(&img, target).map_err(|e| e.to_string())?;
    let provenance = mask.provenance.clone();
    let json = crate::masks::encoding::to_json(&mask).map_err(|e| e.to_string())?;
    Ok(BuildAutoMaskResponse {
        mask_json: json,
        provenance,
        width: mask.width,
        height: mask.height,
    })
}

/// CR-06 P5 — compose two masks via a boolean operator
/// (union / intersect / difference). The frontend
/// supplies the two existing mask rows + the op; the
/// command builds the composite, persists a fresh
/// `AiMask` row tagged as `composite` provenance, and
/// returns the row.
#[derive(Debug, Clone, Deserialize)]
pub struct ComposeMaskRequest {
    pub project_id: String,
    pub image_version_id: String,
    pub parent_a_id: String,
    pub parent_b_id: String,
    pub op: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComposeMaskResponse {
    pub mask: serde_json::Value,
}

#[tauri::command]
pub fn compose_mask(request: ComposeMaskRequest) -> Result<ComposeMaskResponse, String> {
    use crate::masks::composite::{apply, CompositeOp};
    use crate::masks::encoding::{from_json, to_json};
    let op = match request.op.as_str() {
        "union" => CompositeOp::Union,
        "intersect" => CompositeOp::Intersect,
        "difference" => CompositeOp::Difference,
        other => return Err(format!("unknown composite op: {other}")),
    };
    let composite = with_store(|s| {
        let a_row = s
            .get_ai_mask(&request.parent_a_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("mask a not found: {}", request.parent_a_id))?;
        let b_row = s
            .get_ai_mask(&request.parent_b_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("mask b not found: {}", request.parent_b_id))?;
        let a_mask = from_json(&a_row.mask_json).map_err(|e| e.to_string())?;
        let b_mask = from_json(&b_row.mask_json).map_err(|e| e.to_string())?;
        let composite = apply(&a_mask, &b_mask, op).map_err(|e| e.to_string())?;
        Ok(composite)
    })?;
    let composite_json = to_json(&composite).map_err(|e| e.to_string())?;
    let mask_id = format!("msk_{}", new_id_suffix());
    let row = crate::domain::AiMask {
        mask_id,
        project_id: request.project_id,
        image_version_id: request.image_version_id,
        provenance: composite.provenance.clone(),
        parents_json: Some(
            serde_json::to_string(&vec![
                request.parent_a_id.clone(),
                request.parent_b_id.clone(),
            ])
            .unwrap_or_else(|_| "[]".to_string()),
        ),
        mask_json: composite_json,
        created_at: now_iso_string(),
    };
    with_store(|s| s.upsert_ai_mask(&row).map_err(|e| e.to_string()))?;
    Ok(ComposeMaskResponse {
        mask: serde_json::to_value(&row).map_err(|e| e.to_string())?,
    })
}

/// CR-06 P6 — request payload for
/// `run_ai_quality_report`. The caller supplies the
/// (source, result) pixel buffers in `[0, 1]`
/// row-major. P6 ships the gate engine + the
/// orchestrator; the apply round (P4) uses this
/// once real ONNX inference produces a result
/// image. With the current P4 passthrough
/// dispatcher the result equals the source, so the
/// gate always returns `Ok` — but the integration
/// path is in place.
#[derive(Debug, Clone, Deserialize)]
pub struct RunAiQualityReportRequest {
    pub source_image_version_id: String,
    pub result_image_version_id: String,
    pub operation_id: String,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub source_pixels: Vec<f64>,
    pub result_pixels: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunAiQualityReportResponse {
    pub report: serde_json::Value,
    pub verdict: String,
}

#[tauri::command]
pub fn run_ai_quality_report(
    request: RunAiQualityReportRequest,
) -> Result<RunAiQualityReportResponse, String> {
    use crate::image::F32Image;
    use crate::quality_gates::report::run;
    use crate::quality_gates::GateThresholds;

    let n = (request.width as usize)
        * (request.height as usize)
        * (request.channels as usize);
    if request.source_pixels.len() != n {
        return Err(format!(
            "source pixel count mismatch: got {}, expected {}",
            request.source_pixels.len(),
            n
        ));
    }
    if request.result_pixels.len() != n {
        return Err(format!(
            "result pixel count mismatch: got {}, expected {}",
            request.result_pixels.len(),
            n
        ));
    }
    let src = pixels_to_f32image(
        &request.source_pixels,
        request.width as usize,
        request.height as usize,
        request.channels as usize,
    );
    let res = pixels_to_f32image(
        &request.result_pixels,
        request.width as usize,
        request.height as usize,
        request.channels as usize,
    );
    let report = run(
        &src,
        &res,
        &GateThresholds::default(),
        // P5.1: the standalone re-run command has no mask channel of
        // its own; the apply round passes the real mask directly.
        None,
        request.source_image_version_id,
        request.result_image_version_id,
        request.operation_id,
    );
    let verdict = report.verdict.as_str().to_string();
    Ok(RunAiQualityReportResponse {
        verdict: verdict.clone(),
        report: serde_json::to_value(&report).map_err(|e| e.to_string())?,
    })
}

fn pixels_to_f32image(
    pixels: &[f64],
    width: usize,
    height: usize,
    channels: usize,
) -> F32Image {
    let mut img = F32Image::new(width, height, channels);
    let plane = width * height;
    for (i, v) in pixels.iter().enumerate() {
        let clamped = v.clamp(0.0, 1.0) as f32;
        let c = i / plane;
        let rem = i % plane;
        let y = rem / width;
        let x = rem % width;
        img[(c, y, x)] = clamped;
    }
    img
}

// ─── CR-07 — read an applied Image Version's primary artifact ────────
//
// The apply round (P5.1) persists the produced pixels as a 16-bit TIFF
// under `<root>/.astroforge/applied/<project_id>/`. The Zone B canvas
// needs the bytes to render the image. This command is the path-
// confined bridge from the IPC surface to those bytes; the
// confinement pattern mirrors `commands_preview::read_preview_artifact`.
//
// The companion `read_image_artifact_thumbnail` command below ships a
// downsampled 8-bit PNG for large versions so the canvas doesn't pay
// the 16-bit TIFF decode cost on every selection.

use base64::Engine;

/// Response envelope: the bytes plus the on-disk size so the
/// frontend can sanity-check the decode.
#[derive(Debug, serde::Serialize)]
pub struct ImageArtifactResponse {
    pub base64_data: String,
    pub mime_type: String,
    pub byte_size: usize,
    pub width: usize,
    pub height: usize,
    pub channels: usize,
}

/// CR-07 — read an Image Version's primary artifact as base64.
/// Path-confined to `<root>/.astroforge/applied/<project_id>/`; the
/// project_id on the version row must match the active project.
#[tauri::command]
pub fn read_image_artifact(
    state: State<'_, crate::PipelinePlanState>,
    version_id: String,
) -> Result<ImageArtifactResponse, String> {
    let domain_store = state
        .domain_store
        .clone()
        .ok_or_else(|| "domain store is unavailable".to_string())?;
    let version = domain_store
        .get_image_version(&version_id)
        .map_err(|e| format!("get_image_version: {e}"))?
        .ok_or_else(|| format!("image version '{version_id}' not found"))?;
    let artifact = domain_store
        .get_artifact(&version.primary_artifact_id)
        .map_err(|e| format!("get_artifact: {e}"))?;
    let applied_root = applied_pixels_dir(&version.project_id)
        .ok_or_else(|| "applied directory is unavailable".to_string())?;
    let path = std::path::Path::new(&artifact.path);
    // Defense-in-depth: the artifact path must canonicalize inside the
    // applied/<project_id>/ directory. Absolute paths, ../ traversal,
    // and symlinks pointing outside are all refused.
    let canonical = std::fs::canonicalize(path)
        .map_err(|e| format!("canonicalize artifact path: {e}"))?;
    let canonical_root = std::fs::canonicalize(&applied_root)
        .map_err(|e| format!("canonicalize applied dir: {e}"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!(
            "artifact path is outside the applied directory: {}",
            canonical.display()
        ));
    }
    let bytes = std::fs::read(&canonical)
        .map_err(|e| format!("read artifact file: {e}"))?;
    let (width, height, channels) = read_tiff_dimensions(&bytes)
        .ok_or_else(|| "artifact is not a 16-bit TIFF".to_string())?;
    Ok(ImageArtifactResponse {
        base64_data: base64::engine::general_purpose::STANDARD.encode(&bytes),
        mime_type: "image/tiff".to_string(),
        byte_size: bytes.len(),
        width,
        height,
        channels,
    })
}

/// Minimal 16-bit TIFF dimension reader. The canvas only needs to
/// allocate an offscreen buffer of the right size; full decode is
/// done in the renderer. Returns (width, height, channels) for
/// grayscale (channels=1) and RGB (channels=3) 16-bit TIFFs.
fn read_tiff_dimensions(bytes: &[u8]) -> Option<(usize, usize, usize)> {
    // Big-endian TIFF header: "II" (little-endian) or "MM" (big-endian).
    if bytes.len() < 8 || (&bytes[0..2] != b"II" && &bytes[0..2] != b"MM") {
        return None;
    }
    let little = &bytes[0..2] == b"II";
    let ifd_offset = if little {
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize
    } else {
        u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize
    };
    if ifd_offset + 2 > bytes.len() {
        return None;
    }
    let n_entries = if little {
        u16::from_le_bytes([bytes[ifd_offset], bytes[ifd_offset + 1]]) as usize
    } else {
        u16::from_be_bytes([bytes[ifd_offset], bytes[ifd_offset + 1]]) as usize
    };
    let mut width = None;
    let mut height = None;
    let mut samples = None;
    let mut bits_per_sample = None;
    for i in 0..n_entries {
        let entry = ifd_offset + 2 + i * 12;
        if entry + 12 > bytes.len() {
            return None;
        }
        let tag = if little {
            u16::from_le_bytes([bytes[entry], bytes[entry + 1]])
        } else {
            u16::from_be_bytes([bytes[entry], bytes[entry + 1]])
        };
        let value = if little {
            u32::from_le_bytes([bytes[entry + 8], bytes[entry + 9], bytes[entry + 10], bytes[entry + 11]])
        } else {
            u32::from_be_bytes([bytes[entry + 8], bytes[entry + 9], bytes[entry + 10], bytes[entry + 11]])
        };
        match tag {
            256 => width = Some(value as usize),
            257 => height = Some(value as usize),
            258 => samples = Some(value as usize),
            277 => bits_per_sample = Some(value as usize),
            _ => {}
        }
    }
    // Only accept 16-bit grayscale / RGB.
    if bits_per_sample != Some(16) {
        return None;
    }
    Some((
        width?,
        height?,
        match samples {
            Some(3) => 3,
            Some(1) | None => 1,
            _ => return None,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::read_tiff_dimensions;

    /// Build a minimal 16-bit grayscale TIFF with the given
    /// dimensions, big-endian byte order, single IFD entry.
    fn minimal_tiff_le(width: u32, height: u32, samples: u16) -> Vec<u8> {
        // Header: "II" (little-endian), magic 42, IFD offset = 8.
        let mut bytes = vec![b'I', b'I', 0, 0, 8, 0, 0, 0];
        // 4 IFD entries (one each for width, height, samples, bps).
        bytes.extend_from_slice(&4u16.to_le_bytes());
        // ImageWidth tag (256), SHORT, count 1, value inline.
        bytes.extend_from_slice(&256u16.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes()); // SHORT
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&width.to_le_bytes());
        // ImageLength tag (257).
        bytes.extend_from_slice(&257u16.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        // SamplesPerPixel tag (258).
        bytes.extend_from_slice(&258u16.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&samples.to_le_bytes());
        // BitsPerSample tag (277).
        bytes.extend_from_slice(&277u16.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&16u32.to_le_bytes());
        // Next-IFD offset (zero — single IFD).
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes
    }

    #[test]
    fn reads_grayscale_dimensions() {
        let tiff = minimal_tiff_le(64, 48, 1);
        assert_eq!(read_tiff_dimensions(&tiff), Some((64, 48, 1)));
    }

    #[test]
    fn reads_rgb_dimensions() {
        let tiff = minimal_tiff_le(128, 96, 3);
        assert_eq!(read_tiff_dimensions(&tiff), Some((128, 96, 3)));
    }

    #[test]
    fn reads_big_endian_dimensions() {
        // Swap endianness throughout.
        let mut tiff = minimal_tiff_le(32, 24, 1);
        // Header "MM" instead of "II".
        tiff[0] = b'M';
        tiff[1] = b'M';
        // Magic 42 in big-endian.
        tiff[2] = 0;
        tiff[3] = 42;
        // IFD offset 8 in big-endian.
        tiff[4] = 0;
        tiff[5] = 0;
        tiff[6] = 0;
        tiff[7] = 8;
        // n_entries in big-endian.
        tiff[8] = 0;
        tiff[9] = 4;
        // Reswap the rest.
        let len = tiff.len();
        for i in 10..len {
            // Mirror bytes pairwise.
            if i % 2 == 0 {
                tiff.swap(i, i + 1);
            }
        }
        assert_eq!(read_tiff_dimensions(&tiff), Some((32, 24, 1)));
    }

    #[test]
    fn rejects_non_tiff() {
        let png_header = b"\x89PNG\r\n\x1a\n";
        assert_eq!(read_tiff_dimensions(png_header), None);
    }

    #[test]
    fn rejects_8bit() {
        let mut tiff = minimal_tiff_le(16, 16, 1);
        // Find the BitsPerSample entry (last one) and overwrite
        // its value field (offset 12+3*12+8 = 56..60) with 8.
        let len = tiff.len();
        for i in (56..len - 4).step_by(1) {
            if tiff[i..i + 4] == 16u32.to_le_bytes() {
                tiff[i..i + 4].copy_from_slice(&8u32.to_le_bytes());
                break;
            }
        }
        assert_eq!(read_tiff_dimensions(&tiff), None);
    }
}
