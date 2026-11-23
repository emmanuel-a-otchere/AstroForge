//! Image Decisions, Comparison Sets, and version-pair metric
//! comparison — port of `src-tauri/src/commands_comparison.rs`
//! (CR-07 B3/B4 + §23) to direct calls over `Services`.
//!
//! All store access goes through the shared `services.domain_store`
//! (`projects.db`), exactly like the Tauri commands read the managed
//! `ProjectState.store`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use astroforge_core::comparison::{ComparisonSet, ImageDecision, ImageDecisionState};
use astroforge_core::comparison_metrics::VersionComparisonReport;
use astroforge_core::comparison_metrics::{ClippingMasks, FwhmHistogram, NoiseMap};
use astroforge_core::decision_store;
use astroforge_core::domain_store::DomainStore;
use astroforge_core::image::F32Image;
use astroforge_core::metric_registry::MetricDeltaRow;

use crate::services::Services;

fn lock_err<E: std::fmt::Display>(e: E) -> String {
    format!("lock poisoned: {e}")
}

fn store_err_to_string(e: astroforge_core::domain_store::DomainStoreError) -> String {
    e.to_string()
}

/// Run `f` against the shared project store. Mirrors the locking
/// pattern of the Tauri `with_store` helper (which took the managed
/// `ProjectState`).
fn with_store<R>(services: &Services, f: impl FnOnce(&DomainStore) -> R) -> Result<R, String> {
    let guard = services.domain_store.lock().map_err(lock_err)?;
    Ok(f(&guard))
}

// ─── Decision commands ─────────────────────────────────────────────────────

/// `save_image_decision` — persist an `ImageDecision` (B1 type) to the
/// store. The decision's full history is rewritten atomically.
pub fn save_image_decision(services: &Services, decision: ImageDecision) -> Result<(), String> {
    with_store(services, |s| {
        decision_store::save_decision(s, &decision).map_err(|e| e.to_string())
    })?
}

/// `load_image_decision` — load the current `ImageDecision` for a
/// version. Returns `Err("not found: ...")` if no decision row exists.
pub fn load_image_decision(services: &Services, version_id: &str) -> Result<ImageDecision, String> {
    let version_id = version_id.to_string();
    with_store(services, |s| {
        decision_store::load_decision(s, &version_id).map_err(|e| e.to_string())
    })?
}

/// `list_image_decisions_for_project` — list all `ImageDecision` rows
/// for a project, newest first. Unknown project ids error with a clear
/// "project not found" path rather than an empty list (mirrors
/// `image_version_list`).
pub fn list_image_decisions_for_project(
    services: &Services,
    project_id: &str,
) -> Result<Vec<ImageDecision>, String> {
    let project_id = project_id.to_string();
    with_store(services, |s| {
        s.get_project(&project_id).map_err(store_err_to_string)?;
        decision_store::list_decisions_for_project(s, &project_id).map_err(|e| e.to_string())
    })?
}

/// Request payload of `apply_image_decision` (same shape the Tauri
/// command deserialized).
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

/// `apply_image_decision` — apply a state transition and persist
/// atomically. Returns the updated `ImageDecision` (with the new state
/// and appended history). When no decision row exists yet, a fresh
/// `Working` decision is created first (B3 store contract).
pub fn apply_image_decision(
    services: &Services,
    request: ApplyDecisionRequest,
) -> Result<ImageDecision, String> {
    with_store(services, |s| {
        decision_store::apply_and_save_decision_with_profile(
            s,
            &request.version_id,
            request.new_state,
            request.reason,
            request.quality_profile,
        )
        .map_err(|e| e.to_string())
    })?
}

// ─── Comparison set commands ───────────────────────────────────────────────

/// `save_comparison_set` — persist a `ComparisonSet`.
pub fn save_comparison_set(services: &Services, set: ComparisonSet) -> Result<(), String> {
    with_store(services, |s| {
        decision_store::save_comparison_set(s, &set).map_err(|e| e.to_string())
    })?
}

/// `load_comparison_set` — load a `ComparisonSet` by ID. Returns
/// `Err("not found: ...")` if the set doesn't exist.
pub fn load_comparison_set(services: &Services, set_id: &str) -> Result<ComparisonSet, String> {
    let set_id = set_id.to_string();
    with_store(services, |s| {
        decision_store::load_comparison_set(s, &set_id).map_err(|e| e.to_string())
    })?
}

/// `list_comparison_sets_for_project` — list all `ComparisonSet` rows
/// for a project, newest first. Unknown project ids error with a clear
/// "project not found" path.
pub fn list_comparison_sets_for_project(
    services: &Services,
    project_id: &str,
) -> Result<Vec<ComparisonSet>, String> {
    let project_id = project_id.to_string();
    with_store(services, |s| {
        s.get_project(&project_id).map_err(store_err_to_string)?;
        decision_store::list_comparison_sets_for_project(s, &project_id).map_err(|e| e.to_string())
    })?
}

/// `delete_comparison_set` — delete a `ComparisonSet` by ID. Returns
/// `true` if a row was deleted, `false` if the set didn't exist. The
/// underlying Image Versions are not affected (per CR-07 ADR-07.4).
pub fn delete_comparison_set(services: &Services, set_id: &str) -> Result<bool, String> {
    let set_id = set_id.to_string();
    with_store(services, |s| {
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

/// Load a version's applied pixels via the core loader. The direct
/// call's only jobs: resolve the path-confined applied root for the
/// version's project and map the typed error to a string.
fn load_pixels(store: &DomainStore, version_id: &str) -> Result<F32Image, String> {
    let project_id = store
        .get_image_version(version_id)
        .map_err(|e| format!("get_image_version: {e}"))?
        .ok_or_else(|| format!("image version '{version_id}' not found"))?
        .project_id;
    // The Tauri command routed this through
    // `commands_ai_enhancement::applied_pixels_dir`, which is exactly
    // this `DomainStore` associated function.
    let applied_root = DomainStore::applied_pixels_dir(&project_id)
        .ok_or_else(|| "applied directory is unavailable".to_string())?;
    astroforge_core::comparison_metrics::load_version_pixels(store, version_id, &applied_root)
        .map_err(|e| e.to_string())
}

/// `compare_version_metrics` — CR-07 B4: compute the §10 metric delta
/// table + §11 summary for a version pair over their real pixels.
///
/// Only metrics with shipped detectors produce values (see
/// `comparison_metrics::metric_snapshot`); every other registry
/// metric surfaces as an inconclusive "—" row so the table is
/// honest about what is and is not measured.
pub fn compare_version_metrics(
    services: &Services,
    version_id_a: &str,
    version_id_b: &str,
) -> Result<VersionMetricsComparisonDto, String> {
    let (img_a, img_b) = with_store(services, |s| {
        let a = load_pixels(s, version_id_a)?;
        let b = load_pixels(s, version_id_b)?;
        Ok::<(F32Image, F32Image), String>((a, b))
    })??;
    let VersionComparisonReport { rows, summary } =
        astroforge_core::comparison_metrics::compare_version_images(
            &img_a,
            &img_b,
            version_id_a,
            version_id_b,
        );
    Ok(VersionMetricsComparisonDto {
        version_id_a: version_id_a.to_string(),
        version_id_b: version_id_b.to_string(),
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

/// `get_version_metric_snapshot` — CR-07 §23.1: load a version's
/// applied pixels, compute the full metric snapshot (detectors +
/// per-channel statistics), and return the sorted entries plus the
/// image dimensions.
///
/// Used by the `ExpertChannelStats.svelte` component to render
/// the per-channel stats table. The endpoint is a thin wrapper
/// over `comparison_metrics::metric_snapshot_full` and the
/// existing path-confined `load_pixels` loader; the only new
/// logic is the BTreeMap → Vec projection.
pub fn get_version_metric_snapshot(
    services: &Services,
    version_id: &str,
) -> Result<VersionMetricSnapshotDto, String> {
    let (snapshot, width, height, channels) = with_store(services, |s| {
        let img = load_pixels(s, version_id)?;
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
        version_id: version_id.to_string(),
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
#[derive(Debug, Clone, Serialize)]
pub struct FwhmDistributionDto {
    pub version_id: String,
    pub histogram: FwhmHistogram,
    pub width: u32,
    pub height: u32,
}

/// `get_version_fwhm_distribution` — like the Tauri command (which
/// pushed the work into `spawn_blocking`), this opens a *fresh*
/// `projects.db` connection instead of locking the shared store, so a
/// long pixel decode never blocks Compare browsing. In the direct-call
/// port the work runs synchronously on the caller's thread.
pub fn get_version_fwhm_distribution(
    services: &Services,
    version_id: &str,
) -> Result<FwhmDistributionDto, String> {
    let store = DomainStore::new(&services.projects_root.join("projects.db"))
        .map_err(store_err_to_string)?;
    let img = load_pixels(&store, version_id)?;
    let width = img.width() as u32;
    let height = img.height() as u32;
    let histogram = astroforge_core::comparison_metrics::fwhm_histogram(&img);
    Ok(FwhmDistributionDto {
        version_id: version_id.to_string(),
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
#[derive(Debug, Clone, Serialize)]
pub struct NoiseMapDto {
    pub version_id: String,
    pub map: NoiseMap,
    pub width: u32,
    pub height: u32,
}

/// `get_version_noise_map` — fresh-connection variant of the Tauri
/// `spawn_blocking` command (see `get_version_fwhm_distribution`).
pub fn get_version_noise_map(services: &Services, version_id: &str) -> Result<NoiseMapDto, String> {
    let store = DomainStore::new(&services.projects_root.join("projects.db"))
        .map_err(store_err_to_string)?;
    let img = load_pixels(&store, version_id)?;
    let width = img.width() as u32;
    let height = img.height() as u32;
    let map = astroforge_core::comparison_metrics::noise_map(&img);
    Ok(NoiseMapDto {
        version_id: version_id.to_string(),
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
#[derive(Debug, Clone, Serialize)]
pub struct ClippingMasksDto {
    pub version_id: String,
    pub masks: ClippingMasks,
    pub width: u32,
    pub height: u32,
}

/// `get_version_clipping_masks` — fresh-connection variant of the
/// Tauri `spawn_blocking` command (see `get_version_fwhm_distribution`).
pub fn get_version_clipping_masks(
    services: &Services,
    version_id: &str,
) -> Result<ClippingMasksDto, String> {
    let store = DomainStore::new(&services.projects_root.join("projects.db"))
        .map_err(store_err_to_string)?;
    let img = load_pixels(&store, version_id)?;
    let width = img.width() as u32;
    let height = img.height() as u32;
    let masks = astroforge_core::comparison_metrics::clipping_masks(&img);
    Ok(ClippingMasksDto {
        version_id: version_id.to_string(),
        masks,
        width,
        height,
    })
}

/// UI helper (P3e): load a version's applied pixels as a linear
/// `F32Image` for the render pipeline. Same loading path the metric
/// commands use (`DomainStore::applied_pixels_dir`), exposed for the
/// canvas.
pub fn version_pixels(services: &Services, version_id: &str) -> Result<F32Image, String> {
    let store = services
        .domain_store
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    load_pixels(&store, version_id)
}
