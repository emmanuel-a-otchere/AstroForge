//! CR-07 B4 — Tauri commands for Image Decisions, Comparison Sets,
//! and version-pair metric comparison.
//!
//! B3 shipped these shells against a private store at
//! `~/.astroforge/cr-07.sqlite`. B4 wires them into the managed
//! project store (`ProjectState.store`, the `projects.db`
//! `DomainStore` from `main.rs`) so decisions and comparison sets
//! share the lifecycle — and the durable home — of every other
//! project-scoped row the Compare workspace reads
//! (`image_version_list` reads the same store). The B3 command
//! signatures are unchanged.
//!
//! The metrics command (`compare_version_metrics`) is new in B4:
//! it decodes both versions' primary artifacts and produces the
//! §10 delta table + §11 summary consumed by `MetricsTable.svelte`.

use crate::commands_project::{lock_err, ProjectState};
use astroforge_core::comparison::{ComparisonSet, ImageDecision, ImageDecisionState};
use astroforge_core::comparison_metrics::VersionComparisonReport;
use astroforge_core::decision_store;
use astroforge_core::domain_store::DomainStore;
use astroforge_core::image::F32Image;
use astroforge_core::metric_registry::MetricDeltaRow;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Run `f` against the managed project store. Mirrors the locking
/// pattern in `commands_image_versions::image_version_list`.
fn with_store<R>(
    state: &State<'_, ProjectState>,
    f: impl FnOnce(&DomainStore) -> R,
) -> Result<R, String> {
    let guard = state.store.lock().map_err(lock_err)?;
    Ok(f(&guard))
}

// ─── Decision commands ─────────────────────────────────────────────────────

/// Persist an `ImageDecision` (B1 type) to the store. The decision's
/// full history is rewritten atomically.
#[tauri::command]
pub fn save_image_decision(
    state: State<'_, ProjectState>,
    decision: ImageDecision,
) -> Result<(), String> {
    with_store(&state, |s| {
        decision_store::save_decision(s, &decision).map_err(|e| e.to_string())
    })?
}

/// Load the current `ImageDecision` for a version. Returns
/// `Err("not found: ...")` if no decision row exists.
#[tauri::command]
pub fn load_image_decision(
    state: State<'_, ProjectState>,
    version_id: String,
) -> Result<ImageDecision, String> {
    with_store(&state, |s| {
        decision_store::load_decision(s, &version_id).map_err(|e| e.to_string())
    })?
}

/// List all `ImageDecision` rows for a project, newest first.
/// Unknown project ids error with a clear "project not found" path
/// rather than an empty list (mirrors `image_version_list`).
#[tauri::command]
pub fn list_image_decisions_for_project(
    state: State<'_, ProjectState>,
    project_id: String,
) -> Result<Vec<ImageDecision>, String> {
    with_store(&state, |s| {
        s.get_project(&project_id)
            .map_err(crate::commands_project::store_err_to_string)?;
        decision_store::list_decisions_for_project(s, &project_id).map_err(|e| e.to_string())
    })?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyDecisionRequest {
    pub version_id: String,
    pub new_state: ImageDecisionState,
    pub reason: Option<String>,
}

/// Apply a state transition and persist atomically. Returns the
/// updated `ImageDecision` (with the new state + appended history).
/// When no decision row exists yet, a fresh `Working` decision is
/// created first (B3 store contract).
#[tauri::command]
pub fn apply_image_decision(
    state: State<'_, ProjectState>,
    request: ApplyDecisionRequest,
) -> Result<ImageDecision, String> {
    with_store(&state, |s| {
        decision_store::apply_and_save_decision(
            s,
            &request.version_id,
            request.new_state,
            request.reason,
        )
        .map_err(|e| e.to_string())
    })?
}

// ─── Comparison set commands ───────────────────────────────────────────────

/// Persist a `ComparisonSet`.
#[tauri::command]
pub fn save_comparison_set(
    state: State<'_, ProjectState>,
    set: ComparisonSet,
) -> Result<(), String> {
    with_store(&state, |s| {
        decision_store::save_comparison_set(s, &set).map_err(|e| e.to_string())
    })?
}

/// Load a `ComparisonSet` by ID. Returns `Err("not found: ...")` if
/// the set doesn't exist.
#[tauri::command]
pub fn load_comparison_set(
    state: State<'_, ProjectState>,
    set_id: String,
) -> Result<ComparisonSet, String> {
    with_store(&state, |s| {
        decision_store::load_comparison_set(s, &set_id).map_err(|e| e.to_string())
    })?
}

/// List all `ComparisonSet` rows for a project, newest first.
/// Unknown project ids error with a clear "project not found" path.
#[tauri::command]
pub fn list_comparison_sets_for_project(
    state: State<'_, ProjectState>,
    project_id: String,
) -> Result<Vec<ComparisonSet>, String> {
    with_store(&state, |s| {
        s.get_project(&project_id)
            .map_err(crate::commands_project::store_err_to_string)?;
        decision_store::list_comparison_sets_for_project(s, &project_id)
            .map_err(|e| e.to_string())
    })?
}

/// Delete a `ComparisonSet` by ID. Returns `true` if a row was
/// deleted, `false` if the set didn't exist. The underlying Image
/// Versions are not affected (per CR-07 ADR-07.4).
#[tauri::command]
pub fn delete_comparison_set(
    state: State<'_, ProjectState>,
    set_id: String,
) -> Result<bool, String> {
    with_store(&state, |s| {
        decision_store::delete_comparison_set(s, &set_id).map_err(|e| e.to_string())
    })?
}

// ─── CR-07 B4 — version-pair metric comparison ─────────────────────────────

/// Response envelope for `compare_version_metrics`: the §10 delta
/// table plus the §11 natural-language summary.
#[derive(Debug, Clone, Serialize)]
pub struct VersionMetricsComparisonDto {
    pub version_id_a: String,
    pub version_id_b: String,
    pub rows: Vec<MetricDeltaRow>,
    pub summary: String,
}

/// Load a version's applied pixels via the core loader. The
/// command's only jobs: resolve the path-confined applied root for
/// the version's project and map the typed error to a string.
fn load_pixels(store: &DomainStore, version_id: &str) -> Result<F32Image, String> {
    let project_id = store
        .get_image_version(version_id)
        .map_err(|e| format!("get_image_version: {e}"))?
        .ok_or_else(|| format!("image version '{version_id}' not found"))?
        .project_id;
    let applied_root = crate::commands_ai_enhancement::applied_pixels_dir(&project_id)
        .ok_or_else(|| "applied directory is unavailable".to_string())?;
    astroforge_core::comparison_metrics::load_version_pixels(store, version_id, &applied_root)
        .map_err(|e| e.to_string())
}

/// CR-07 B4 — compute the §10 metric delta table + §11 summary for
/// a version pair over their real pixels.
///
/// Only metrics with shipped detectors produce values (see
/// `comparison_metrics::metric_snapshot`); every other registry
/// metric surfaces as an inconclusive "—" row so the table is
/// honest about what is and is not measured.
#[tauri::command]
pub fn compare_version_metrics(
    state: State<'_, ProjectState>,
    version_id_a: String,
    version_id_b: String,
) -> Result<VersionMetricsComparisonDto, String> {
    let (img_a, img_b) = with_store(&state, |s| {
        let a = load_pixels(s, &version_id_a)?;
        let b = load_pixels(s, &version_id_b)?;
        Ok::<(F32Image, F32Image), String>((a, b))
    })??;
    let VersionComparisonReport { rows, summary } =
        astroforge_core::comparison_metrics::compare_version_images(
            &img_a,
            &img_b,
            &version_id_a,
            &version_id_b,
        );
    Ok(VersionMetricsComparisonDto {
        version_id_a,
        version_id_b,
        rows,
        summary,
    })
}
