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
use astroforge_core::comparison_metrics::ClippingMasks;
use astroforge_core::comparison_metrics::FwhmHistogram;
use astroforge_core::comparison_metrics::NoiseMap;
use astroforge_core::comparison_metrics::VersionComparisonReport;
use astroforge_core::decision_store;
use astroforge_core::domain_store::DomainStore;
use astroforge_core::image::F32Image;
use astroforge_core::metric_registry::MetricDeltaRow;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// CR-07 C-A3.5 — the Quality Profile the user had
    /// selected on the picker at the moment of this
    /// transition. None for callers that don't surface
    /// the picker (today: legacy callers). When set, the
    /// value is persisted on the image_decisions row so
    /// future slices can join "what was decided" with
    /// "what profile the user picked".
    #[serde(default)]
    pub quality_profile: Option<String>,
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
        decision_store::apply_and_save_decision_with_profile(
            s,
            &request.version_id,
            request.new_state,
            request.reason,
            request.quality_profile,
        )
    })
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

// ─── CR-07 §23.1: version per-metric snapshot (expert panel) ─────

/// Response envelope for `get_version_metric_snapshot`: the
/// full per-version snapshot (5 detector-backed keys from
/// `metric_snapshot` + 5 per-channel stats from `channel_stats`).
///
/// Returned as a `Vec<(String, f64)>` rather than a `BTreeMap`
/// because Tauri's IPC serialises a typed struct more cleanly
/// than a map-of-strings and the consumer (`ExpertChannelStats.svelte`)
/// wants a flat array it can iterate. The Rust side keeps the
/// `BTreeMap` so the keys are deterministically ordered; the
/// `(String, f64)` array is the IPC-friendly projection.
#[derive(Debug, Clone, Serialize)]
pub struct VersionMetricSnapshotDto {
    pub version_id: String,
    pub metrics: Vec<MetricEntryDto>,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricEntryDto {
    pub key: String,
    pub value: f64,
}

/// CR-07 §23.1: load a version's applied pixels, compute the
/// full metric snapshot (detectors + per-channel statistics),
/// and return the sorted entries plus the image dimensions.
///
/// Used by the `ExpertChannelStats.svelte` component to render
/// the per-channel stats table. The endpoint is a thin wrapper
/// over `comparison_metrics::metric_snapshot_full` and the
/// existing path-confined `load_pixels` loader; the only new
/// logic is the BTreeMap → Vec projection.
#[tauri::command]
pub fn get_version_metric_snapshot(
    state: State<'_, ProjectState>,
    version_id: String,
) -> Result<VersionMetricSnapshotDto, String> {
    let (snapshot, width, height, channels) = with_store(&state, |s| {
        let img = load_pixels(s, &version_id)?;
        let w = img.width() as u32;
        let h = img.height() as u32;
        let c = img.channels() as u32;
        let snap = astroforge_core::comparison_metrics::metric_snapshot_full(&img);
        Ok::<(BTreeMap<String, f64>, u32, u32, u32), String>((snap, w, h, c))
    })??;
    let metrics = snapshot
        .into_iter()
        .map(|(key, value)| MetricEntryDto { key, value })
        .collect();
    Ok(VersionMetricSnapshotDto {
        version_id,
        metrics,
        width,
        height,
        channels,
    })
}

// CR-07 §23.2: per-version FWHM distribution (expert panel).

/// Response envelope for `get_version_fwhm_distribution`:
/// the per-star FWHM values, the seven-number summary, and
/// the pre-binned histogram (Sturges' rule, capped to
/// [1, 50] bins). The histogram is ready-to-render in the
/// Svelte component; no client-side bucketing required.
#[derive(Serialize)]
pub struct FwhmDistributionDto {
    pub version_id: String,
    pub histogram: FwhmHistogram,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub async fn get_version_fwhm_distribution(
    version_id: String,
    state: tauri::State<'_, ProjectState>,
) -> Result<FwhmDistributionDto, String> {
    let project_root = lock_err(&state)?;
    let project_root_for_block = project_root.clone();
    let version_id_for_block = version_id.clone();
    let (histogram, width, height) = tokio::task::spawn_blocking(move || {
        let store = DomainStore::open(&project_root_for_block)?;
        let img = load_pixels(&store, &version_id_for_block)?;
        let w = img.width() as u32;
        let h = img.height() as u32;
        let hist = astroforge_core::comparison_metrics::fwhm_histogram(&img);
        Ok::<(FwhmHistogram, u32, u32), String>((hist, w, h))
    })
    .await
    .map_err(|e| format!("fwhm histogram task failed: {e}"))??;
    Ok(FwhmDistributionDto {
        version_id,
        histogram,
        width,
        height,
    })
}

// CR-07 §23.3: per-version noise map (2D sigma field).

/// Response envelope for `get_version_noise_map`: the
/// per-pixel local sigma field plus the three summary
/// stats (min, mean, max). The sigma field is a flat
/// row-major array of `width * height` f64 values.
#[derive(Serialize)]
pub struct NoiseMapDto {
    pub version_id: String,
    pub map: NoiseMap,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub async fn get_version_noise_map(
    version_id: String,
    state: tauri::State<'_, ProjectState>,
) -> Result<NoiseMapDto, String> {
    let project_root = lock_err(&state)?;
    let project_root_for_block = project_root.clone();
    let version_id_for_block = version_id.clone();
    let (map, width, height) = tokio::task::spawn_blocking(move || {
        let store = DomainStore::open(&project_root_for_block)?;
        let img = load_pixels(&store, &version_id_for_block)?;
        let w = img.width() as u32;
        let h = img.height() as u32;
        let m = astroforge_core::comparison_metrics::noise_map(&img);
        Ok::<(NoiseMap, u32, u32), String>((m, w, h))
    })
    .await
    .map_err(|e| format!("noise map task failed: {e}"))??;
    Ok(NoiseMapDto {
        version_id,
        map,
        width,
        height,
    })
}

// CR-07 §23.4: per-version clipping masks (highlight + shadow).

/// Response envelope for `get_version_clipping_masks`:
/// the per-pixel highlight + shadow clipping masks plus
/// their summary stats. The masks are flat row-major
/// u8 arrays of `width * height` pixels, indexed as
/// `mask[y * width + x]`. Each byte is `1` if the pixel
/// is clipped, `0` otherwise.
#[derive(Serialize)]
pub struct ClippingMasksDto {
    pub version_id: String,
    pub masks: ClippingMasks,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub async fn get_version_clipping_masks(
    version_id: String,
    state: tauri::State<'_, ProjectState>,
) -> Result<ClippingMasksDto, String> {
    let project_root = lock_err(&state)?;
    let project_root_for_block = project_root.clone();
    let version_id_for_block = version_id.clone();
    let (masks, width, height) = tokio::task::spawn_blocking(move || {
        let store = DomainStore::open(&project_root_for_block)?;
        let img = load_pixels(&store, &version_id_for_block)?;
        let w = img.width() as u32;
        let h = img.height() as u32;
        let m = astroforge_core::comparison_metrics::clipping_masks(&img);
        Ok::<(ClippingMasks, u32, u32), String>((m, w, h))
    })
    .await
    .map_err(|e| format!("clipping masks task failed: {e}"))??;
    Ok(ClippingMasksDto {
        version_id,
        masks,
        width,
        height,
    })
}
