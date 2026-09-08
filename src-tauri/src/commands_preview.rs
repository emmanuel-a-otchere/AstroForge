//! CR-05 P4 slice 4 — Tauri command surface for preview-before-commit
//! (CR-05 §11 + §26). Slice 4 ships the IPC + lifecycle plumbing.
//! The actual stage-handler integration (running a handler at scale
//! and writing a real preview artifact) lands in slice 5.
//!
//! **Additive only.** Existing widgets continue to read from their
//! current stores. The commands below are dormant until a future
//! Svelte PR wires the UI to them.

use astroforge_core::artifact::ContentStore;
use astroforge_core::domain::{Artifact, ArtifactCategory, PreviewRun};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tauri::State;

use super::commands_pipeline_plan::PipelinePlanState;

/// CR-05 P4 slice 4 — frontend-facing DTO. Mirrors the
/// `PreviewRunDto` interface in `astroforge-api.ts`.
#[derive(Debug, Serialize)]
pub struct PreviewRunDto {
    pub preview_id: String,
    pub stage_execution_id: String,
    pub source_version_id: String,
    pub preview_artifact_id: Option<String>,
    pub parameters_json: String,
    pub parameters_hash: String,
    pub status: String,
    pub scale: f64,
    pub label: String,
    pub error_json: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

impl From<&PreviewRun> for PreviewRunDto {
    fn from(p: &PreviewRun) -> Self {
        Self {
            preview_id: p.preview_id.clone(),
            stage_execution_id: p.stage_execution_id.clone(),
            source_version_id: p.source_version_id.clone(),
            preview_artifact_id: p.preview_artifact_id.clone(),
            parameters_json: p.parameters_json.clone(),
            parameters_hash: p.parameters_hash.clone(),
            status: p.status.clone(),
            scale: p.scale,
            label: p.label.clone(),
            error_json: p.error_json.clone(),
            started_at: p.started_at.clone(),
            completed_at: p.completed_at.clone(),
            created_at: p.created_at.clone(),
        }
    }
}

/// CR-05 P4 slice 4 — input to `create_preview_run`. Mirrors the
/// `CreatePreviewRunRequest` TypeScript interface.
#[derive(Debug, Deserialize)]
pub struct CreatePreviewRunRequest {
    pub stage_execution_id: String,
    pub source_version_id: String,
    pub parameters_json: String,
    /// Downscale factor applied to the input (e.g. 0.25 for a
    /// quarter-size preview). Default 0.25 if omitted.
    pub scale: Option<f64>,
    /// User-facing label. Default "Preview — reduced resolution" if
    /// omitted.
    pub label: Option<String>,
}

/// CR-05 P4 slice 4 — create a preview row and (slice 4
/// placeholder behaviour) immediately complete it with a
/// synthesized artifact. Slice 5 swaps the synthesis for a real
/// stage-handler invocation that downsamples and persists a real
/// image artifact.
#[tauri::command]
pub fn create_preview_run(
    state: State<'_, PipelinePlanState>,
    request: CreatePreviewRunRequest,
) -> Result<PreviewRunDto, String> {
    let domain_store = state
        .domain_store
        .clone()
        .ok_or_else(|| "domain store not is unavailable".to_string())?;

    // CR-05 P4 slice 4 — placeholder behaviour: drive the preview
    // synchronously inside the IPC. Slice 5 moves the synthesis
    // into a background task that reads the actual handler
    // dispatch path.
    let now_ms = now_unix_ms_string();
    let parameters_hash = sha256_hex_of_str(&request.parameters_json);
    let scale = request.scale.unwrap_or(0.25);
    let label = request
        .label
        .clone()
        .unwrap_or_else(|| "Preview — reduced resolution".to_string());

    let preview = PreviewRun {
        preview_id: String::new(),
        stage_execution_id: request.stage_execution_id.clone(),
        source_version_id: request.source_version_id.clone(),
        preview_artifact_id: None,
        parameters_json: request.parameters_json.clone(),
        parameters_hash: parameters_hash.clone(),
        status: astroforge_core::domain::preview_status::PENDING.into(),
        scale,
        label: label.clone(),
        error_json: None,
        started_at: Some(now_ms.clone()),
        completed_at: None,
        created_at: String::new(),
    };
    let preview_id = domain_store
        .create_preview_run(&preview)
        .map_err(|e| format!("create_preview_run: {e}"))?;

    // Mark running — slice 5 keeps this lifecycle but defers the
    // synthesis work into a worker.
    domain_store
        .mark_preview_running(&preview_id, &now_ms)
        .map_err(|e| format!("mark_preview_running: {e}"))?;

    // Slice 4 placeholder: synthesize a tiny placeholder artifact
    // whose bytes are the JSON + scale + label. Slice 5 replaces
    // this with a downsampled image. The artifact is real (it
    // has a path, hash, and rows in the artifacts table) so the
    // rest of the pipeline (read-back, list, etc.) can be wired
    // in this PR.
    let artifact = synthesize_placeholder_artifact(
        &domain_store,
        &preview_id,
        &request.stage_execution_id,
        &label,
        scale,
        &request.parameters_json,
    )
    .map_err(|e| format!("synthesize placeholder: {e}"))?;

    domain_store
        .mark_preview_completed(&preview_id, &artifact.artifact_id, &now_ms)
        .map_err(|e| format!("mark_preview_completed: {e}"))?;

    let stored = domain_store
        .get_preview_run(&preview_id)
        .map_err(|e| format!("get_preview_run: {e}"))?;
    Ok(PreviewRunDto::from(&stored))
}

/// CR-05 P4 slice 4 — list every preview recorded against a given
/// stage execution, in creation order.
#[tauri::command]
pub fn list_preview_runs_for_stage_execution(
    state: State<'_, PipelinePlanState>,
    stage_execution_id: String,
) -> Result<Vec<PreviewRunDto>, String> {
    let domain_store = state
        .domain_store
        .clone()
        .ok_or_else(|| "domain store not is unavailable".to_string())?;
    let rows = domain_store
        .list_preview_runs_for_stage_execution(&stage_execution_id)
        .map_err(|e| format!("list_preview_runs: {e}"))?;
    Ok(rows.iter().map(PreviewRunDto::from).collect())
}

/// CR-05 P4 slice 4 — fetch a single preview by id.
#[tauri::command]
pub fn get_preview_run(
    state: State<'_, PipelinePlanState>,
    preview_id: String,
) -> Result<PreviewRunDto, String> {
    let domain_store = state
        .domain_store
        .clone()
        .ok_or_else(|| "domain store not is unavailable".to_string())?;
    let row = domain_store
        .get_preview_run(&preview_id)
        .map_err(|e| format!("get_preview_run: {e}"))?;
    Ok(PreviewRunDto::from(&row))
}

/// CR-05 P4 slice 4 — mark a preview failed (rare in the slice 4
/// placeholder flow; the real failure path lands with slice 5 when
/// the synthesis moves to a background task).
#[tauri::command]
pub fn mark_preview_failed(
    state: State<'_, PipelinePlanState>,
    preview_id: String,
    error_json: String,
) -> Result<(), String> {
    let domain_store = state
        .domain_store
        .clone()
        .ok_or_else(|| "domain store not is unavailable".to_string())?;
    domain_store
        .mark_preview_failed(&preview_id, &error_json, &now_unix_ms_string())
        .map_err(|e| format!("mark_preview_failed: {e}"))
}

/// CR-05 P4 slice 4 — delete a preview row. Slice 4 keeps the
/// companion artifact intact; slice 5 will add artifact cleanup.
#[tauri::command]
pub fn delete_preview_run(
    state: State<'_, PipelinePlanState>,
    preview_id: String,
) -> Result<(), String> {
    let domain_store = state
        .domain_store
        .clone()
        .ok_or_else(|| "domain store not is unavailable".to_string())?;
    domain_store
        .delete_preview_run(&preview_id)
        .map_err(|e| format!("delete_preview_run: {e}"))
}

// ─── helpers ────────────────────────────────────────────────────────────

fn now_unix_ms_string() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("unix_ms:{}", now.as_millis())
}

fn sha256_hex_of_str(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    format!("{:x}", h.finalize())
}

/// CR-05 P4 slice 4 — synthesize a placeholder artifact whose
/// payload encodes the preview parameters + scale + label. Slice 5
/// replaces this with a real downsampled image produced by the
/// stage handler. Returns the recorded artifact.
fn synthesize_placeholder_artifact(
    domain_store: &Arc<astroforge_core::domain_store::DomainStore>,
    preview_id: &str,
    stage_execution_id: &str,
    label: &str,
    scale: f64,
    parameters_json: &str,
) -> Result<Artifact, astroforge_core::domain_store::DomainStoreError> {
    // Payload: a deterministic, non-image byte blob. The §11 spec
    // promises a "Preview — reduced resolution" experience, but
    // slice 4 ships lifecycle wiring only; a real PNG requires
    // either a tiny stub image library or deferring to slice 5.
    // The placeholder is a JSON document so the artifact
    // round-trip + reader API can be exercised end-to-end.
    let payload = format!(
        r#"{{"preview_id":"{preview_id}","stage_execution_id":"{stage_execution_id}","label":"{label}","scale":{scale},"parameters_json":{parameters_json}}}"#
    );
    let hash = ContentStore::sha256_hex(payload.as_bytes());
    let path = format!("preview://{preview_id}");
    let artifact = Artifact {
        artifact_id: String::new(),
        artifact_hash: hash,
        artifact_type: ArtifactCategory::Preview,
        format: "json".into(),
        // Slice 5 will write the real preview image to
        // ContentStore and patch the path. Slice 4 leaves the
        // path as a stable marker that points at the preview row.
        path: path.clone(),
        size: payload.len() as u64,
        created_at: String::new(),
        producer_stage: Some(stage_execution_id.to_string()),
        pipeline_run_id: None,
        parent_artifact_ids: vec![],
        width: Some(1),
        height: Some(1),
        channels: Some(1),
        bit_depth: Some(8),
        color_space: Some("placeholder".into()),
        linear_or_nonlinear: Some(false),
    };
    let stored_id = domain_store.record_artifact(&artifact)?;
    let mut stored = domain_store.get_artifact(&stored_id)?;
    // Re-apply the marker path so the read-back matches the
    // input (record_artifact doesn't preserve it otherwise).
    stored.path = path;
    Ok(stored)
}