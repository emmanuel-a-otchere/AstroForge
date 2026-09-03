#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use astroforge_core::fits;
use astroforge_core::gallery::{GalleryItemUpdate, GalleryStore};
use astroforge_core::ingest::{self, FrameInfo};
use astroforge_core::mvp_pipeline::{self, PipelineConfig, PipelineResult, Verbosity};
use astroforge_core::session::SessionStore;
use serde::Serialize;
use tauri::{Manager, State};

/// Tauri-managed state: holds the GalleryStore (rusqlite) behind a
/// mutex so the IPC handlers can borrow it immutably across awaits.
struct GalleryState(Mutex<GalleryStore>);

/// Tauri-managed state: holds the SessionStore (rusqlite) for crash-safe
/// autosave. Mirrors the GalleryState pattern.
struct SessionState(Mutex<SessionStore>);

#[derive(Serialize)]
struct CommandError {
    message: String,
}

impl<E: std::fmt::Display> From<E> for CommandError {
    fn from(e: E) -> Self {
        CommandError {
            message: e.to_string(),
        }
    }
}

// AstroForge errors are also Display-able; keep them going through the
// blanket impl above so we don't have to reimplement per-error-type.

#[tauri::command]
fn gallery_list(state: State<'_, GalleryState>) -> Result<Vec<astroforge_core::gallery::GalleryItem>, CommandError> {
    let store = state.0.lock().expect("gallery store mutex poisoned");
    store.list().map_err(Into::into)
}

#[tauri::command]
fn gallery_upsert(
    state: State<'_, GalleryState>,
    update: GalleryItemUpdate,
) -> Result<astroforge_core::gallery::GalleryItem, CommandError> {
    let store = state.0.lock().expect("gallery store mutex poisoned");
    store.upsert(update).map_err(Into::into)
}

#[tauri::command]
fn gallery_delete(state: State<'_, GalleryState>, id: String) -> Result<(), CommandError> {
    let store = state.0.lock().expect("gallery store mutex poisoned");
    store.delete(&id).map_err(Into::into)
}

// ─ ─── Session autosave IPC ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//
// These commands back the crash-safe autosave promised by the spec (§5
// NFR) and tracked by issue #144. Each maps to a single SessionStore
// method; the Svelte side calls them on every stage commit.

#[tauri::command]
fn session_create_project(
    state: State<'_, SessionState>,
    name: String,
    target_type: Option<String>,
) -> Result<String, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    store
        .create_project(&name, target_type.as_deref())
        .map_err(Into::into)
}

#[tauri::command]
fn session_create(
    state: State<'_, SessionState>,
    project_id: String,
    source_dir: Option<String>,
    verbosity: String,
) -> Result<String, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    store
        .create_session(&project_id, source_dir.as_deref(), &verbosity)
        .map_err(Into::into)
}

#[tauri::command]
fn session_record_stage(
    state: State<'_, SessionState>,
    session_id: String,
    stage_id: String,
    status: String,
    params_json: Option<String>,
    metrics_json: Option<String>,
    error: Option<String>,
) -> Result<i64, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    let run_id = store.record_stage_run(
        &session_id,
        &stage_id,
        &status,
        params_json.as_deref(),
        metrics_json.as_deref(),
        error.as_deref(),
    )?;
    // Auto-complete if the caller already knows the outcome — saves an
    // extra round-trip for the common "commit then persist" path.
    if matches!(status.as_str(), "completed" | "failed" | "skipped") {
        store.complete_stage_run(run_id, &status)?;
    }
    Ok(run_id)
}

#[tauri::command]
fn session_find_interrupted(state: State<'_, SessionState>) -> Result<Vec<String>, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    Ok(store.find_interrupted_sessions())
}

#[tauri::command]
fn session_get_receipts(
    state: State<'_, SessionState>,
    session_id: String,
) -> Result<Vec<astroforge_core::session::StageRunInfo>, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    Ok(store.list_stage_runs(&session_id))
}

// ─ ─── Pipeline IPC ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//
// Phase 9 (M2 tranche): wires the Rust ingest + mvp_pipeline modules
// through Tauri so the Svelte side can run end-to-end: pick a folder
// of FITS lights → classify → read → run MVP pipeline → return
// PipelineResult. The Svelte side then displays the stretched output
// via the PreviewCanvas.

#[derive(Serialize)]
struct IngestFrameDto {
    path: String,
    frame_type: String,
    exptime: Option<f64>,
    filter: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    binning: Option<i64>,
    anomalies: Vec<String>,
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

/// Walk a directory for `.fits` / `.fit` files and classify each one.
/// Returns one DTO per frame with the header information needed by
/// the UI to render the manifest (counts by type, exposure total,
/// anomalies to flag).
#[tauri::command]
fn ingest_scan_directory(dir_path: String) -> Result<Vec<IngestFrameDto>, CommandError> {
    let dir = PathBuf::from(&dir_path);
    let paths = ingest::scan_directory(&dir).map_err(CommandError::from)?;
    let mut out = Vec::with_capacity(paths.len());
    for p in paths {
        let bytes = match std::fs::read(&p) {
            Ok(b) => b,
            Err(e) => {
                // Surface a soft warning but keep going — one bad file
                // shouldn't abort the whole scan.
                eprintln!("ingest_scan_directory: skip {}: {e}", p.display());
                continue;
            }
        };
        let frame = ingest::classify_frame(&p, &bytes);
        out.push(frame.into());
    }
    Ok(out)
}

/// Run the MVP pipeline against a directory of light frames. Reads
/// every FITS file the ingest layer classified as a Light, calibrates
/// with `lights_only` (no master dark/flat yet — that's a later
/// stage), then runs the registration + stacking + stretching chain.
///
/// The returned `PipelineResult` carries a `ProcessingReport` the UI
/// can render (stage parameters, frame stats, optional export path).
#[tauri::command]
fn pipeline_run_session(
    session_id: String,
    dir_path: String,
    verbosity: String,
) -> Result<PipelineResult, CommandError> {
    let dir = PathBuf::from(&dir_path);

    let paths = ingest::scan_directory(&dir).map_err(CommandError::from)?;
    let mut frames: Vec<FrameInfo> = Vec::with_capacity(paths.len());
    for p in paths {
        let bytes = match std::fs::read(&p) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("pipeline_run_session: skip {}: {e}", p.display());
                continue;
            }
        };
        frames.push(ingest::classify_frame(&p, &bytes));
    }

    // Read every light frame into a normalised F32Image. Calibration
    // happens inside the MVP pipeline via `lights_only` for now; the
    // master dark/flat pipeline lands with the calibration milestone.
    let light_paths: Vec<PathBuf> = frames
        .iter()
        .filter(|f| matches!(f.frame_type, ingest::FrameType::Light))
        .map(|f| f.path.clone())
        .collect();

    let mut calibrated: Vec<astroforge_core::image::F32Image> = Vec::new();
    for p in &light_paths {
        let bytes = std::fs::read(p).map_err(CommandError::from)?;
        let header = fits::parse_header(&bytes)?;
        let img = fits::read_f32_image(&bytes, &header)?;
        calibrated.push(img);
    }

    let manifest = ingest::build_manifest(&session_id, &dir_path, frames);

    let verbosity = match verbosity.as_str() {
        "intermediate" => Verbosity::Intermediate,
        "expert" => Verbosity::Expert,
        _ => Verbosity::Beginner,
    };
    let config = PipelineConfig {
        verbosity,
        lights_only: true,
        ..PipelineConfig::default()
    };

    Ok(mvp_pipeline::run_pipeline(&manifest, calibrated, &config))
}

fn gallery_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Resolves to e.g. <app_data_dir>/gallery.sqlite. Falls back to
    // cwd if the app data dir isn't available (shouldn't happen in
    // Tauri runtime but keeps tests from panicking).
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(dir.join("gallery.sqlite"))
}

fn session_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Resolves to e.g. <app_data_dir>/sessions.sqlite. Separate file
    // from the gallery so the two stores can move independently.
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(dir.join("sessions.sqlite"))
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let gallery_path = gallery_db_path(&app.handle())?;
            let gallery = GalleryStore::new(&gallery_path)
                .map_err(|e| format!("failed to open gallery store: {e}"))?;
            app.manage(GalleryState(Mutex::new(gallery)));

            let session_path = session_db_path(&app.handle())?;
            let sessions = SessionStore::new(&session_path)
                .map_err(|e| format!("failed to open session store: {e}"))?;
            app.manage(SessionState(Mutex::new(sessions)));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            gallery_list,
            gallery_upsert,
            gallery_delete,
            session_create_project,
            session_create,
            session_record_stage,
            session_find_interrupted,
            session_get_receipts,
            ingest_scan_directory,
            pipeline_run_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AstroForge");
}