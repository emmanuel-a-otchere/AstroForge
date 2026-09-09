#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use astroforge_core::fits;
use astroforge_core::gallery::{GalleryItemUpdate, GalleryStore};
use astroforge_core::ingest::{self, FrameInfo};
use astroforge_core::mvp_pipeline::{self, PipelineConfig, PipelineResult, Verbosity};
use astroforge_core::domain_store::DomainStore;
use astroforge_core::project::ProjectManager;
use astroforge_core::recipe::Recipe;
use astroforge_core::recipe_store::{RecipeStore, RecipeSummary, RecipeVersion};
use astroforge_core::session::SessionStore;
use serde::Serialize;
use tauri::{Manager, State};

mod commands_pipeline_plan;
mod commands_preview;
mod commands_project;
mod commands_resource;

/// Tauri-managed state: holds the GalleryStore (rusqlite) behind a
/// mutex so the IPC handlers can borrow it immutably across awaits.
struct GalleryState(Mutex<GalleryStore>);

/// Tauri-managed state: holds the SessionStore (rusqlite) for crash-safe
/// autosave. Mirrors the GalleryState pattern.
struct SessionState(Mutex<SessionStore>);

/// Tauri-managed state: holds the RecipeStore (rusqlite) for pipeline
/// profile persistence. Same mutex pattern as GalleryState.
struct RecipeState(Mutex<RecipeStore>);

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

// ─ ─── Stage checkpoints (M7 T1: versioned artefact store) ─ ─ ─ ─ ─ ─ ─ ─
//
// Saves / reads per-stage pixel-data snapshots so undo / re-apply can
// restore an exact pre-operation state without re-executing the
// pipeline (Phase 1.5 PR-B1).
#[tauri::command]
fn session_save_checkpoint(
    state: State<'_, SessionState>,
    session_id: String,
    stage_id: String,
    artifact_path: String,
) -> Result<i64, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    store
        .save_checkpoint(&session_id, &stage_id, &artifact_path)
        .map_err(CommandError::from)
}

#[tauri::command]
fn session_get_latest_checkpoint(
    state: State<'_, SessionState>,
    session_id: String,
) -> Result<Option<astroforge_core::session::CheckpointInfo>, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    Ok(store.get_latest_checkpoint(&session_id))
}

#[tauri::command]
fn session_get_checkpoints(
    state: State<'_, SessionState>,
    session_id: String,
) -> Result<Vec<astroforge_core::session::CheckpointInfo>, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    Ok(store.list_checkpoints(&session_id))
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

// ─ ─── Recipe (pipeline profile) IPC ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//
// Phase 1.5 PR-A: backs the "telescope pipeline profile" feature
// (docs/PROFILE_PIPELINES_PLAN.md). The Svelte side calls these to list
// available profiles, retrieve a specific version, and save new
// versions of a profile. Reads use Recipe::from_json_migrated so older
// v1 payloads upgrade transparently.

#[tauri::command]
fn recipe_list(state: State<'_, RecipeState>) -> Result<Vec<RecipeSummary>, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    store.list().map_err(Into::into)
}

#[tauri::command]
fn recipe_list_versions(
    state: State<'_, RecipeState>,
    profile_id: String,
) -> Result<Vec<RecipeVersion>, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    store.list_versions(&profile_id).map_err(Into::into)
}

#[tauri::command]
fn recipe_get(
    state: State<'_, RecipeState>,
    profile_id: String,
    version: u32,
) -> Result<Recipe, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    store.get(&profile_id, version).map_err(Into::into)
}

#[tauri::command]
fn recipe_get_head(
    state: State<'_, RecipeState>,
    profile_id: String,
) -> Result<Recipe, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    store.get_head(&profile_id).map_err(Into::into)
}

#[tauri::command]
fn recipe_save(
    state: State<'_, RecipeState>,
    recipe: Recipe,
) -> Result<RecipeSummary, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    // Caller can either set recipe.version themselves or rely on us to
    // compute the next version based on (name, target_type). For PR-A
    // we always compute here so the Svelte side can be naive.
    let profile_id = RecipeStore::profile_id_for(&recipe.name, &recipe.target_type);
    let next_version = store.next_version_for(&profile_id).map_err(CommandError::from)?;
    let mut owned = recipe;
    if owned.version == 0 {
        owned.version = next_version;
    }
    if owned.parent_version.is_none() && next_version > 1 {
        owned.parent_version = Some(next_version - 1);
    }
    store.save(&owned).map_err(Into::into)
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

fn recipe_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Resolves to e.g. <app_data_dir>/recipes.sqlite. Separate file
    // from the gallery and session DBs so each store can move
    // independently.
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(dir.join("recipes.sqlite"))
}

/// CR-05 P1 (Decision D-CR05-2) — separate sqlite for PipelinePlan rows.
/// Same per-store pattern as gallery / session / recipe.
fn pipeline_plans_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(dir.join("pipeline_plans.sqlite"))
}

/// CR-02.6 — durable project root. Each child of this directory is one
/// AstroForge Project (§18 self-contained layout).
fn projects_root_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(dir.join("projects"))
}

// ─ ─── Multi-format export (M7 T3) ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//
// Writes the supplied FITS file to each requested format in parallel.
// The Tauri command takes a raw f32 buffer (channel-major row-major)
// because we don't have an F32Image at the boundary; the Svelte side
// loads the image and ships the bytes. The Rust side reshapes the
// bytes into the F32Image for export.
#[derive(serde::Deserialize)]
struct MultiExportArgs {
    /// Raw float32 pixel data, channel-major (R, G, B, ...) then
    /// row-major within each channel.
    pixels: Vec<f32>,
    width: u32,
    height: u32,
    channels: u32,
    /// Base path without extension; the multi_export dispatcher adds
    /// the per-format extension.
    base_path: String,
    /// Formats to emit (Tiff16, Png8, Jpeg8{quality}, Fits32,
    /// Xisf{history_json}, SidecarJson{recipe_json}).
    formats: Vec<astroforge_core::export::ExportFormat>,
    /// Sidecar context: required when SidecarJson is in the format
    /// list. Ignored for other formats.
    report: Option<astroforge_core::export::ProcessingReport>,
}

#[tauri::command]
fn export_multi_format(args: MultiExportArgs) -> Result<Vec<String>, CommandError> {
    use astroforge_core::export::{multi_export, ProcessingReport};
    use ndarray::Array3;
    // Reshape the flat pixel buffer into Array3 (channels, height, width).
    // If the buffer is the wrong length, the reshape fails with a clear
    // error string rather than a panic.
    let arr = Array3::from_shape_vec(
        (args.channels as usize, args.height as usize, args.width as usize),
        args.pixels,
    )
    .map_err(|e| CommandError {
        message: format!("pixel reshape failed: {e}"),
    })?;
    let img = astroforge_core::image::F32Image::from(arr);
    let report = args.report.unwrap_or_else(|| ProcessingReport {
        session_id: "unknown".into(),
        frame_stats: astroforge_core::export::FrameStats {
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
    let written = multi_export(&img, &report, &base, &args.formats)?;
    Ok(written
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let gallery_path = gallery_db_path(&app.handle())?;
            let gallery = GalleryStore::new(&gallery_path)
                .map_err(|e| format!("failed to open gallery store: {e}"))?;
            app.manage(GalleryState(Mutex::new(gallery)));

            let session_path = session_db_path(&app.handle())?;
            let sessions = SessionStore::new(&session_path)
                .map_err(|e| format!("failed to open session store: {e}"))?;
            app.manage(SessionState(Mutex::new(sessions)));

            let recipe_path = recipe_db_path(&app.handle())?;
            let recipes = RecipeStore::new(&recipe_path)
                .map_err(|e| format!("failed to open recipe store: {e}"))?;
            // Seed DwarfII v1 on first launch so the user sees a profile
            // they can load into a session.
            recipes
                .seed_if_empty()
                .map_err(|e| format!("failed to seed recipe store: {e}"))?;
            app.manage(RecipeState(Mutex::new(recipes)));

            // CR-02.6 — durable project state (additive; no existing
            // reader is touched). One global DomainStore for cross-project
            // queries, plus a ProjectManager over the projects_root.
            let projects_root = projects_root_path(&app.handle())?;
            std::fs::create_dir_all(&projects_root)
                .map_err(|e| format!("failed to create projects root: {e}"))?;
            let project_db = projects_root.join("projects.db");
            let project_store = DomainStore::new(&project_db)
                .map_err(|e| format!("failed to open project store: {e}"))?;
            app.manage(commands_project::ProjectState {
                manager: Mutex::new(ProjectManager::new(projects_root.clone())),
                store: Mutex::new(project_store),
                // CR-05 P4 slice 5 — clone so the PipelinePlanState
                // below can derive the previews dir from the same root.
                projects_root: projects_root.clone(),
            });

            // CR-05 P1 — PipelinePlan store (additive; P1 only).
            let pipeline_plans_db = pipeline_plans_db_path(&app.handle())?;
            let pipeline_plans_store = astroforge_core::pipeline_plans_store::PipelinePlanStore::new(&pipeline_plans_db)
                .map_err(|e| format!("failed to open pipeline plans store: {e}"))?;

            // CR-05 P2.6 — the Stack handler needs the DomainStore so
            // it can list source assets for the session. We open a
            // second connection to the same projects.db file. SQLite
            // serializes writes via file locking; the project
            // commands are short-lived so contention is minimal.
            let project_db_for_handler = projects_root.join("projects.db");
            let domain_store = DomainStore::new(&project_db_for_handler)
                .map_err(|e| format!("failed to open domain store for handler: {e}"))?;

            app.manage(commands_pipeline_plan::PipelinePlanState {
                store: std::sync::Arc::new(Mutex::new(pipeline_plans_store)),
                cancel_handles: Mutex::new(std::collections::HashMap::new()),
                // CR-05 P2.5 — per-plan pause handles (independent of
                // cancel handles; cancel wins if both flip).
                pause_handles: Mutex::new(std::collections::HashMap::new()),
                // CR-05 P2.6 — handler registry with the Stack handler
                // + the shared DomainStore. P2.7+ add more handlers.
                handler_registry: std::sync::Arc::new({
                    let mut reg = astroforge_core::pipeline_plan::dispatch::HandlerRegistry::new();
                    reg.insert(
                        "calibrate",
                        std::sync::Arc::new(astroforge_core::pipeline_plan::dispatch::CalibrateHandler),
                    );
                    reg.insert(
                        "debayer",
                        std::sync::Arc::new(astroforge_core::pipeline_plan::dispatch::DebayerHandler),
                    );
                    reg.insert(
                        "register",
                        std::sync::Arc::new(astroforge_core::pipeline_plan::dispatch::RegisterHandler),
                    );
                    reg.insert(
                        "stack",
                        std::sync::Arc::new(astroforge_core::pipeline_plan::dispatch::StackHandler),
                    );
                    reg.insert(
                        "background",
                        std::sync::Arc::new(astroforge_core::pipeline_plan::dispatch::BackgroundHandler),
                    );
                    reg.insert(
                        "stretch",
                        std::sync::Arc::new(astroforge_core::pipeline_plan::dispatch::StretchHandler),
                    );
                    reg.insert(
                        "denoise",
                        std::sync::Arc::new(astroforge_core::pipeline_plan::dispatch::DenoiseHandler),
                    );
                    reg.insert(
                        "export",
                        std::sync::Arc::new(astroforge_core::pipeline_plan::dispatch::ExportHandler),
                    );
                    reg
                }),
                domain_store: Some(std::sync::Arc::new(domain_store)),
                // CR-05 P3 slice 2 — recommendation engine wired in
                // with the default rule set (stretch / denoise /
                // background). Future slices register more rules
                // before app boot.
                recommendation_engine: Some(std::sync::Arc::new(
                    astroforge_core::recommendation::RecommendationEngine::with_defaults(),
                )),
                // CR-05 P4 slice 5 — preview PNGs live under the
                // projects root. Created lazily on first preview.
                previews_dir: projects_root.join("previews"),
            });

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
            session_save_checkpoint,
            session_get_latest_checkpoint,
            session_get_checkpoints,
            ingest_scan_directory,
            pipeline_run_session,
            recipe_list,
            recipe_list_versions,
            recipe_get,
            recipe_get_head,
            recipe_save,
            export_multi_format,
            // CR-02.6 — project / pipeline-run commands (additive).
            commands_project::project_list,
            commands_project::project_get,
            commands_project::project_create,
            commands_project::project_open,
            commands_project::project_rename,
            commands_project::project_archive,
            commands_project::project_delete,
            commands_project::project_recover,
            commands_project::pipeline_run_list,
            commands_project::pipeline_run_get,
            commands_project::pipeline_run_list_stages,
            commands_project::pipeline_run_find_interrupted,
            // CR-05 P1 — pipeline plan commands (additive).
            commands_pipeline_plan::create_pipeline_plan,
            commands_pipeline_plan::pipeline_plan_list_for_project,
            commands_pipeline_plan::pipeline_plan_get,
            // CR-05 P2 slice 1 — start + cancel commands (additive).
            commands_pipeline_plan::start_pipeline_run,
            commands_pipeline_plan::cancel_pipeline_run,
            commands_pipeline_plan::pipeline_plan_list_stage_executions,
            // CR-05 P2.5 — pause + resume + recovery commands (additive).
            commands_pipeline_plan::pause_pipeline_run,
            commands_pipeline_plan::resume_pipeline_run,
            commands_pipeline_plan::pipeline_plan_list_resumable_for_project,
            // CR-05 P3 slice 2 — recommendation engine commands
            // (additive; IntelligencePanel will consume them).
            commands_pipeline_plan::get_recommendations_for_stage_execution,
            commands_pipeline_plan::get_recommendations_for_plan,
            // CR-05 P3 slice 2.5 — user-decision lifecycle on
            // recommendations (apply / dismiss / reset).
            commands_pipeline_plan::apply_recommendation,
            commands_pipeline_plan::dismiss_recommendation,
            commands_pipeline_plan::reset_recommendation,
            // CR-05 P5 slice 4 (§24 + §27) — aggregate metrics for the
            // Expert DAG view and the recommendation banner.
            commands_pipeline_plan::get_processing_metrics,
            // CR-05 P6 slice 3 (§25) — processing timeline per stage.
            commands_pipeline_plan::get_processing_timeline,
            // CR-05 P4 slice 4+5 — preview-before-commit IPC. Slice 5
            // replaced `mark_preview_failed` (placeholder-only path)
            // with `read_preview_artifact` (real PNG read-back).
            commands_preview::create_preview_run,
            commands_preview::list_preview_runs_for_stage_execution,
            commands_preview::get_preview_run,
            commands_preview::read_preview_artifact,
            commands_preview::delete_preview_run,
            // CR-05 P4 slice 7 — §21 resource snapshot.
            commands_resource::get_resource_snapshot,
            // CR-05 P5 slice 1 — backend enumeration + §22 budget.
            commands_resource::list_backend_capabilities,
            commands_resource::derive_execution_budget,
            // CR-05 P5 slice 2 — stage-parameter pre-flight budget.
            commands_resource::stage_execution_budget,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AstroForge");
}