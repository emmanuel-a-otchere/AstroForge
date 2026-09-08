//! CR-05 P4 slice 4+5 — Tauri command surface for preview-before-commit
//! (CR-05 §11 + §26).
//!
//! Slice 4 shipped the IPC + lifecycle plumbing with a placeholder
//! artifact. Slice 5 replaces the placeholder with the real pipeline:
//! resolve the stage execution, run the stage handler against
//! downsampled input frames ([`pipeline_plan::preview::run_preview`]),
//! export the result as a PNG into the project's previews directory,
//! and record a real artifact row.
//!
//! ## Failure honesty
//!
//! The frame-loading helper (`dispatch::load_frames_for_session`)
//! is now real (slice 5.1): it decodes TIFF/FITS source assets
//! from the session's `original_path` and returns them in order.
//! Files that fail to decode are logged and skipped, so one corrupt
//! asset doesn't kill the whole run. If the session has no source
//! assets at all, the preview still fails with
//! `PreviewError::NoSourceFrames` — the UI shows the error banner
//! rather than a silent failure.
//!
//! **Additive only.** Existing widgets continue to read from their
//! current stores.

use astroforge_core::domain::{Artifact, ArtifactCategory, PreviewRun};
use astroforge_core::domain_store::DomainStoreError;
use astroforge_core::export::export_png_8bit;
use astroforge_core::pipeline_plan::preview::{self, PreviewRequest};
use base64::Engine;
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

/// CR-05 P4 slice 5 — run a real preview: resolve the stage from the
/// plan, dispatch the handler against downsampled frames, export the
/// output as PNG, and record the artifact.
///
/// On success the row transitions pending → running → completed with
/// `preview_artifact_id` set. On failure the row is marked `failed`
/// with the real error message in `error_json` and the **DTO is still
/// returned** (the IPC succeeds) so the UI can render the failure
/// banner without special-casing invoke errors.
#[tauri::command]
pub fn create_preview_run(
    state: State<'_, PipelinePlanState>,
    request: CreatePreviewRunRequest,
) -> Result<PreviewRunDto, String> {
    let domain_store = state
        .domain_store
        .clone()
        .ok_or_else(|| "domain store is unavailable".to_string())?;

    let now_ms = now_unix_ms_string();
    let parameters_hash = sha256_hex_of_str(&request.parameters_json);
    let scale = request.scale.unwrap_or(0.25);
    let label = request
        .label
        .clone()
        .unwrap_or_else(|| "Preview — reduced resolution".to_string());

    // 1. Resolve the stage execution → plan → stage so the preview
    //    runs the *actual* stage type with the *actual* parameters.
    let (stage, session_id) = {
        let plan_store = state
            .store
            .lock()
            .map_err(|_| "plan store lock poisoned".to_string())?;
        let exec = plan_store
            .get_stage_execution(&request.stage_execution_id)
            .map_err(|e| format!("resolve stage execution: {e}"))?;
        let plan = plan_store
            .load_plan(&exec.plan_id)
            .map_err(|e| format!("load plan: {e}"))?;
        let stage = plan
            .stages
            .iter()
            .find(|s| s.stage_id == exec.stage_id)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "stage '{}' not found in plan '{}'",
                    exec.stage_id, exec.plan_id
                )
            })?;
        (stage, plan.session_id.clone())
    };

    // 2. Insert the pending row so the UI can show progress even for
    //    fast previews.
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
        started_at: None,
        completed_at: None,
        created_at: String::new(),
    };
    let preview_id = domain_store
        .create_preview_run(&preview)
        .map_err(|e| format!("create_preview_run: {e}"))?;

    domain_store
        .mark_preview_running(&preview_id, &now_ms)
        .map_err(|e| format!("mark_preview_running: {e}"))?;

    // 3. Run the real preview driver.
    let outcome = preview::run_preview(&PreviewRequest {
        stage,
        session_id,
        run_id: preview_id.clone(),
        scale,
        domain_store: domain_store.clone(),
        handler_registry: state.handler_registry.clone(),
        preloaded_frames: None,
    });

    // 4. Persist success or failure honestly.
    match outcome {
        Ok(output) => {
            let artifact = persist_preview_png(
                &domain_store,
                &state.previews_dir,
                &preview_id,
                &request.stage_execution_id,
                &output.image,
                output.metadata_json.as_deref(),
            )
            .map_err(|e| format!("persist preview artifact: {e}"))?;
            domain_store
                .mark_preview_completed(&preview_id, &artifact.artifact_id, &now_unix_ms_string())
                .map_err(|e| format!("mark_preview_completed: {e}"))?;
        }
        Err(err) => {
            let error_json = serde_json::json!({
                "error": err.to_string(),
                "kind": format!("{:?}", err),
            })
            .to_string();
            domain_store
                .mark_preview_failed(&preview_id, &error_json, &now_unix_ms_string())
                .map_err(|e| format!("mark_preview_failed: {e}"))?;
        }
    }

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
        .ok_or_else(|| "domain store is unavailable".to_string())?;
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
        .ok_or_else(|| "domain store is unavailable".to_string())?;
    let row = domain_store
        .get_preview_run(&preview_id)
        .map_err(|e| format!("get_preview_run: {e}"))?;
    Ok(PreviewRunDto::from(&row))
}

/// CR-05 P4 slice 5 — read the preview's PNG artifact back as a
/// base64 data payload for the webview. The asset protocol stays
/// disabled; only bytes belonging to an existing preview row cross
/// the boundary.
///
/// Returns the raw base64 (no `data:` prefix) so the caller can pick
/// the MIME; the artifact's `format` column is authoritative.
#[tauri::command]
pub fn read_preview_artifact(
    state: State<'_, PipelinePlanState>,
    preview_id: String,
) -> Result<String, String> {
    let domain_store = state
        .domain_store
        .clone()
        .ok_or_else(|| "domain store is unavailable".to_string())?;
    let preview = domain_store
        .get_preview_run(&preview_id)
        .map_err(|e| format!("get_preview_run: {e}"))?;
    let artifact_id = preview
        .preview_artifact_id
        .ok_or_else(|| format!("preview '{preview_id}' has no artifact"))?;
    let artifact = domain_store
        .get_artifact(&artifact_id)
        .map_err(|e| format!("get_artifact: {e}"))?;
    // Defense-in-depth: only serve files inside the previews dir.
    let path = std::path::Path::new(&artifact.path);
    if !path.starts_with(&state.previews_dir) {
        return Err(format!(
            "artifact path '{}' is outside the previews directory",
            artifact.path
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("read artifact file: {e}"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// CR-05 P4 slice 4 — delete a preview row. The companion artifact
/// row and PNG file are left in place (slice 5): artifact GC is a
/// §26 follow-up once retention policy is decided.
#[tauri::command]
pub fn delete_preview_run(
    state: State<'_, PipelinePlanState>,
    preview_id: String,
) -> Result<(), String> {
    let domain_store = state
        .domain_store
        .clone()
        .ok_or_else(|| "domain store is unavailable".to_string())?;
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

/// CR-05 P4 slice 5 — export the preview image as an 8-bit PNG into
/// `<previews_dir>/<preview_id>.png` and record the artifact row with
/// the real dimensions + hash of the file bytes.
fn persist_preview_png(
    domain_store: &Arc<astroforge_core::domain_store::DomainStore>,
    previews_dir: &std::path::Path,
    preview_id: &str,
    stage_execution_id: &str,
    image: &astroforge_core::image::F32Image,
    metadata_json: Option<&str>,
) -> Result<Artifact, DomainStoreError> {
    std::fs::create_dir_all(previews_dir).map_err(|e| DomainStoreError::Io(e.to_string()))?;
    let path = previews_dir.join(format!("{preview_id}.png"));

    let mut png_bytes: Vec<u8> = Vec::new();
    export_png_8bit(image, &mut png_bytes)
        .map_err(|e| DomainStoreError::Io(format!("png export: {e}")))?;
    std::fs::write(&path, &png_bytes).map_err(|e| DomainStoreError::Io(e.to_string()))?;

    let hash = astroforge_core::artifact::ContentStore::sha256_hex(&png_bytes);
    let artifact = Artifact {
        artifact_id: String::new(),
        artifact_hash: hash,
        artifact_type: ArtifactCategory::Preview,
        format: "png".into(),
        path: path.to_string_lossy().to_string(),
        size: png_bytes.len() as u64,
        created_at: String::new(),
        producer_stage: Some(stage_execution_id.to_string()),
        pipeline_run_id: Some(preview_id.to_string()),
        parent_artifact_ids: vec![],
        width: Some(image.width() as i64),
        height: Some(image.height() as i64),
        channels: Some(image.channels() as i64),
        bit_depth: Some(8),
        color_space: Some("srgb".into()),
        linear_or_nonlinear: Some(false),
    };
    let stored_id = domain_store.record_artifact(&artifact)?;
    let mut stored = domain_store.get_artifact(&stored_id)?;
    // Stash the handler metadata in the artifact path namespace
    // would be lossy; the metadata lives on the PreviewRun row's
    // parameters lineage instead. Nothing else to patch here.
    let _ = metadata_json;
    stored.size = png_bytes.len() as u64;
    Ok(stored)
}
