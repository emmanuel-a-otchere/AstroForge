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
use crate::image::F32Image;
use crate::image_analysis;
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
