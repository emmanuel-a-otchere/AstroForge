//! Session autosave, stage checkpoints, ingest scanning, diff cache,
//! and multi-format export — port of the inline commands in
//! `src-tauri/src/main.rs` (`session_*`, `ingest_scan_directory`,
//! `diff_cache_*`, `export_multi_format`).
//!
//! Note: `session_continue` / `session_list_for_project` are NOT
//! ported here — they live in `src-tauri/src/commands_import.rs` and
//! belong to the `api::import` module's port. Bodies are ported 1:1
//! from the Tauri commands; errors are `String` with the same text
//! the Tauri commands surfaced.

use serde::{Deserialize, Serialize};

use crate::services::Services;

use astroforge_core::diff_cache::{DiffCacheKey, DiffCacheStats};
use astroforge_core::difference::DiffKind;
use astroforge_core::export::{ExportFormat, FrameStats, ProcessingReport};
use astroforge_core::import_scan::{self, FrameInfo};
use astroforge_core::session::{CheckpointInfo, StageRunInfo};

fn lock_err<E: std::fmt::Display>(e: E) -> String {
    format!("lock poisoned: {e}")
}

// ─ ─── Session autosave ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//
// These functions back the crash-safe autosave promised by the spec
// (§5 NFR) and tracked by issue #144. Each maps to a single
// SessionStore method; the UI calls them on every stage commit.

/// `session_create_project`
pub fn session_create_project(
    services: &Services,
    name: &str,
    target_type: Option<&str>,
) -> Result<String, String> {
    let store = services.sessions.lock().map_err(lock_err)?;
    store
        .create_project(name, target_type)
        .map_err(|e| e.to_string())
}

/// `session_create`
pub fn session_create(
    services: &Services,
    project_id: &str,
    source_dir: Option<&str>,
    verbosity: &str,
) -> Result<String, String> {
    let store = services.sessions.lock().map_err(lock_err)?;
    store
        .create_session(project_id, source_dir, verbosity)
        .map_err(|e| e.to_string())
}

/// `session_record_stage` — records a stage run and auto-completes it
/// when the caller already knows the outcome (saves an extra
/// round-trip for the common "commit then persist" path).
pub fn session_record_stage(
    services: &Services,
    session_id: &str,
    stage_id: &str,
    status: &str,
    params_json: Option<&str>,
    metrics_json: Option<&str>,
    error: Option<&str>,
) -> Result<i64, String> {
    let store = services.sessions.lock().map_err(lock_err)?;
    let run_id = store
        .record_stage_run(
            session_id,
            stage_id,
            status,
            params_json,
            metrics_json,
            error,
        )
        .map_err(|e| e.to_string())?;
    // Auto-complete if the caller already knows the outcome — saves an
    // extra round-trip for the common "commit then persist" path.
    if matches!(status, "completed" | "failed" | "skipped") {
        store
            .complete_stage_run(run_id, status)
            .map_err(|e| e.to_string())?;
    }
    Ok(run_id)
}

/// `session_find_interrupted`
pub fn session_find_interrupted(services: &Services) -> Result<Vec<String>, String> {
    let store = services.sessions.lock().map_err(lock_err)?;
    Ok(store.find_interrupted_sessions())
}

/// `session_get_receipts`
pub fn session_get_receipts(
    services: &Services,
    session_id: &str,
) -> Result<Vec<StageRunInfo>, String> {
    let store = services.sessions.lock().map_err(lock_err)?;
    Ok(store.list_stage_runs(session_id))
}

// ─ ─── Stage checkpoints (M7 T1: versioned artefact store) ─ ─ ─ ─ ─ ─ ─
//
// Saves / reads per-stage pixel-data snapshots so undo / re-apply can
// restore an exact pre-operation state without re-executing the
// pipeline (Phase 1.5 PR-B1).

/// `session_save_checkpoint`
pub fn session_save_checkpoint(
    services: &Services,
    session_id: &str,
    stage_id: &str,
    artifact_path: &str,
) -> Result<i64, String> {
    let store = services.sessions.lock().map_err(lock_err)?;
    store
        .save_checkpoint(session_id, stage_id, artifact_path)
        .map_err(|e| e.to_string())
}

/// `session_get_latest_checkpoint`
pub fn session_get_latest_checkpoint(
    services: &Services,
    session_id: &str,
) -> Result<Option<CheckpointInfo>, String> {
    let store = services.sessions.lock().map_err(lock_err)?;
    Ok(store.get_latest_checkpoint(session_id))
}

/// `session_get_checkpoints`
pub fn session_get_checkpoints(
    services: &Services,
    session_id: &str,
) -> Result<Vec<CheckpointInfo>, String> {
    let store = services.sessions.lock().map_err(lock_err)?;
    Ok(store.list_checkpoints(session_id))
}

// ─ ─── Ingest scan ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─

/// Mirror of `main.rs::IngestFrameDto` — one classified frame with the
/// header information needed by the UI to render the manifest (counts
/// by type, exposure total, anomalies to flag).
#[derive(Debug, Clone, Serialize)]
pub struct IngestFrameDto {
    pub path: String,
    pub frame_type: String,
    pub exptime: Option<f64>,
    pub filter: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub binning: Option<i64>,
    pub anomalies: Vec<String>,
}

impl From<FrameInfo> for IngestFrameDto {
    fn from(f: FrameInfo) -> Self {
        IngestFrameDto {
            path: f.path.to_string_lossy().into_owned(),
            frame_type: f.frame_type.as_str().to_string(),
            exptime: f.exptime,
            filter: f.filter,
            width: f.width,
            height: f.height,
            binning: f.binning,
            anomalies: f.anomalies,
        }
    }
}

/// `ingest_scan_directory` — walk a directory for any SUPPORTED file
/// (per import_scan's extension table) and classify each one.
///
/// #465 W3 — previously this used the FITS-only walk, so every
/// TIFF/PNG/JPEG/DNG capture silently vanished at Step 1 despite the
/// UI's format claim. The scan now honors the full extension table;
/// the pipeline's decoder switch (dispatch.rs) remains the honest
/// gate for what can actually be processed.
pub fn ingest_scan_directory(dir_path: &str) -> Result<Vec<IngestFrameDto>, String> {
    let dir = std::path::PathBuf::from(dir_path);
    let paths = import_scan::scan_directory(&dir).map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(paths.len());
    for p in paths.into_iter().filter(|p| import_scan::is_supported(p)) {
        let bytes = match std::fs::read(&p) {
            Ok(b) => b,
            Err(e) => {
                // Surface a soft warning but keep going — one bad file
                // shouldn't abort the whole scan.
                eprintln!("ingest_scan_directory: skip {}: {e}", p.display());
                continue;
            }
        };
        let frame = import_scan::classify_frame(&p, &bytes);
        out.push(frame.into());
    }
    Ok(out)
}

// ─ ─── Diff cache (CR-07 §29.2a) ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//
// In-memory cache for `compute_diff`, global to the app session; it
// lives for the lifetime of the UI runtime. Versions that change
// their primary artifact (TIFF) should call
// `diff_cache_invalidate_version` to drop stale entries.

/// `diff_cache_get_or_compute` — cache lookup keyed by
/// (version_a_id, version_b_id, mode, gain). On miss, computes via
/// `compute_diff` + stores. The mode is a kebab-case string
/// ("absolute" / "signed" / "amplified" / "structural").
#[allow(clippy::too_many_arguments)] // flat args mirror the cache key
pub fn diff_cache_get_or_compute(
    services: &Services,
    version_a_id: &str,
    version_b_id: &str,
    mode: &str,
    gain: f32,
    width: u32,
    image_a: Vec<u8>,
    image_b: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let diff_kind = match mode {
        "absolute" => DiffKind::Absolute,
        "signed" => DiffKind::Signed,
        "amplified" => DiffKind::Amplified,
        "structural" => DiffKind::Structural,
        _ => {
            return Err(format!("unknown diff mode: {mode:?}"));
        }
    };
    let key = DiffCacheKey::new(version_a_id, version_b_id, diff_kind, gain);
    let mut cache = services.diff_cache.lock().map_err(lock_err)?;
    let result = cache.get_or_compute(key, &image_a, &image_b, gain, width);
    // The cache stores owned Vec<u8>; clone to return to the
    // caller without holding the lock.
    Ok(result.clone())
}

/// `diff_cache_invalidate_version` — invalidate every cache entry
/// referencing the given version id. Used when a version's primary
/// artifact (TIFF) changes. Returns the number of entries dropped.
pub fn diff_cache_invalidate_version(
    services: &Services,
    version_id: &str,
) -> Result<usize, String> {
    let mut cache = services.diff_cache.lock().map_err(lock_err)?;
    Ok(cache.invalidate_version(version_id))
}

/// `diff_cache_clear` — drop every cache entry. Stats counters are
/// preserved.
pub fn diff_cache_clear(services: &Services) -> Result<(), String> {
    let mut cache = services.diff_cache.lock().map_err(lock_err)?;
    cache.clear();
    Ok(())
}

/// `diff_cache_stats` — snapshot of the cache stats (hits + misses
/// counters).
pub fn diff_cache_stats(services: &Services) -> Result<DiffCacheStats, String> {
    let cache = services.diff_cache.lock().map_err(lock_err)?;
    Ok(cache.stats().clone())
}

/// `diff_cache_len` — current cache size (number of entries).
pub fn diff_cache_len(services: &Services) -> Result<usize, String> {
    let cache = services.diff_cache.lock().map_err(lock_err)?;
    Ok(cache.len())
}

// ─ ─── Multi-format export (M7 T3) ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//
// Writes the supplied image to each requested format. The original
// Tauri command takes a raw f32 buffer (channel-major row-major)
// because there is no F32Image at the boundary; the UI side loads the
// image and ships the bytes, and the Rust side reshapes the bytes
// into the F32Image for export.

/// Mirror of `main.rs::MultiExportArgs`.
#[derive(Deserialize)]
pub struct MultiExportArgs {
    /// Raw float32 pixel data, channel-major (R, G, B, ...) then
    /// row-major within each channel.
    pub pixels: Vec<f32>,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    /// Base path without extension; the multi_export dispatcher adds
    /// the per-format extension.
    pub base_path: String,
    /// Formats to emit (Tiff16, Png8, Jpeg8{quality}, Fits32,
    /// Xisf{history_json}, SidecarJson{recipe_json}).
    pub formats: Vec<ExportFormat>,
    /// Sidecar context: required when SidecarJson is in the format
    /// list. Ignored for other formats.
    pub report: Option<ProcessingReport>,
}

/// `export_multi_format`
pub fn export_multi_format(args: MultiExportArgs) -> Result<Vec<String>, String> {
    use astroforge_core::export::multi_export;
    // Reshape the flat pixel buffer into an F32Image (channels,
    // height, width). If the buffer is the wrong length, the reshape
    // fails with a clear error string rather than a panic. (The
    // original reshaped via `ndarray::Array3::from_shape_vec`; this
    // port pre-checks the length and uses
    // `F32Image::from_channels`, which shares the same planar
    // (channels, height, width) layout.)
    let expected = args.channels as usize * args.height as usize * args.width as usize;
    if args.pixels.len() != expected {
        return Err(format!(
            "pixel reshape failed: expected {expected} pixel values, got {}",
            args.pixels.len()
        ));
    }
    let img = astroforge_core::image::F32Image::from_channels(
        args.width as usize,
        args.height as usize,
        args.channels as usize,
        &args.pixels,
    );
    let report = args.report.unwrap_or_else(|| ProcessingReport {
        session_id: "unknown".into(),
        frame_stats: FrameStats {
            total_frames: 0,
            lights: 0,
            darks: 0,
            flats: 0,
            biases: 0,
            total_exposure: 0.0,
        },
        rejected_frames: vec![],
        stage_parameters: vec![],
        export_path: None,
    });
    let base = std::path::PathBuf::from(&args.base_path);
    let written = multi_export(&img, &report, &base, &args.formats).map_err(|e| e.to_string())?;
    Ok(written
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect())
}
