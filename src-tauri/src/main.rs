#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use astroforge_core::diff_cache::{DiffCache, DiffCacheKey, DiffCacheStats};
use astroforge_core::difference::DiffKind;
use astroforge_core::domain_store::DomainStore;
use astroforge_core::fits;
use astroforge_core::gallery::{GalleryItemUpdate, GalleryStore};
use astroforge_core::ingest::{self, FrameInfo};
use astroforge_core::mvp_pipeline::{self, PipelineConfig, PipelineResult, Verbosity};
use astroforge_core::project::ProjectManager;
use astroforge_core::recipe::{QualityProfile, Recipe, RecipeAiDiffSummary};
use astroforge_core::recipe_store::{RecipeStore, RecipeSummary, RecipeVersion};
use astroforge_core::session::SessionStore;
use serde::Serialize;
use tauri::{Manager, State};

mod commands_ai_enhancement;
mod commands_ai_models;
mod commands_comparison;
mod commands_image_versions;
mod commands_import;
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

/// CR-07 §29.2a: in-memory diff cache for `compute_diff`.
/// Single-threaded by construction (the cache is wrapped
/// in a Mutex<T> so multiple IPC calls are serialized).
/// The cache is global to the app session; it lives for
/// the lifetime of the Tauri runtime. Versions that
/// change their primary artifact (TIFF) should call
/// `diff_cache_invalidate_version` to drop stale
/// entries.
struct DiffCacheState(Mutex<DiffCache>);

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
fn gallery_list(
    state: State<'_, GalleryState>,
) -> Result<Vec<astroforge_core::gallery::GalleryItem>, CommandError> {
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

// CR-07 §22: QualityProfile catalog. Mirrors the
// Rust enum's 4 variants with display labels and
// descriptions for the frontend picker.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QualityProfileInfo {
    id: String,
    label: String,
    description: String,
}

#[tauri::command]
fn quality_profile_list() -> Vec<QualityProfileInfo> {
    QualityProfile::ALL
        .iter()
        .map(|p| QualityProfileInfo {
            id: format!("{:?}", p).to_lowercase(),
            label: p.label().to_string(),
            description: p.description().to_string(),
        })
        .collect()
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
    let next_version = store
        .next_version_for(&profile_id)
        .map_err(CommandError::from)?;
    let mut owned = recipe;
    if owned.version == 0 {
        owned.version = next_version;
    }
    if owned.parent_version.is_none() && next_version > 1 {
        owned.parent_version = Some(next_version - 1);
    }
    store.save(&owned).map_err(Into::into)
}

/// CR-07 B13c: look up the live `Recipe` that produced a
/// given Image Version. Walks two stores:
///
/// 1. The active project's `DomainStore.image_versions`
///    table (B13a) for the `recipe_id` foreign key.
/// 2. The global `RecipeStore` (`recipe_get_head`) for
///    the live `Recipe` row keyed by `profile_id`.
///
/// Returns `Ok(None)` when:
/// - the version does not exist (returns `None` rather
///   than an error: the caller treats "no such version"
///   the same as "no recipe" at the panel level), OR
/// - the version exists but its `recipe_id` is `None`
///   (legacy / AI-applied versions; the ProvenancePanel
///   surfaces "Profile not recorded" in this case).
///
/// Returns `Ok(Some(recipe))` when both lookups succeed.
/// Returns `Err(...)` only on the `recipe_get_head` failure
/// (e.g. a `recipe_id` in `image_versions` points at a
/// profile that no longer exists in `RecipeStore`).
#[tauri::command]
fn recipe_get_for_image_version(
    project_state: State<'_, commands_project::ProjectState>,
    recipe_state: State<'_, RecipeState>,
    version_id: String,
) -> Result<Option<Recipe>, CommandError> {
    use commands_ai_enhancement::image_version_get;
    use commands_project::store_err_to_string;

    // Step 1: read the version row from the active
    // project's store. `image_version_get` returns
    // `Value::Null` for "not found" (see
    // commands_ai_enhancement::image_version_get).
    let raw = image_version_get(version_id, project_state)
        .map_err(store_err_to_string)?;
    let version_value = match raw {
        serde_json::Value::Null => return Ok(None),
        v => v,
    };

    // Step 2: extract recipe_id. Serde-derived; the field
    // is on the IPC wire because B13a added it to the
    // `ImageVersion` Rust struct + serde, so it's already
    // present in the `image_version_get` JSON.
    let profile_id: Option<String> = serde_json::from_value(
        serde_json::json!({ "recipe_id": version_value.get("recipe_id") }),
    )
    .map_err(|e| {
        CommandError {
            message: format!("malformed ImageVersion payload: {e}"),
        }
    })?;
    let profile_id = match profile_id {
        Some(s) if !s.is_empty() => s,
        // No recipe recorded for this version. Honest
        // "not recorded" path; the UI renders an empty
        // state, not an error.
        _ => return Ok(None),
    };

    // Step 3: pull the live Recipe from the global
    // RecipeStore keyed by the profile_id. The
    // `recipe_get_head` helper returns the head version
    // of the named profile.
    let store = recipe_state.0.lock().expect("recipe store mutex poisoned");
    match store.get_head(&profile_id) {
        Ok(recipe) => Ok(Some(recipe)),
        Err(e) => Err(CommandError {
            message: format!(
                "recipe_id {profile_id:?} recorded on version but RecipeStore.get_head failed: {e}"
            ),
        }),
    }
}

/// CR-07 §32.6: compute the pipeline plan hash for a Recipe.
/// Pure function: loads the Recipe, calls `pipeline_plan_hash()`.
/// Returns CommandError if the Recipe is not found.
#[tauri::command]
fn recipe_pipeline_plan_hash(
    state: State<'_, RecipeState>,
    profile_id: String,
    version: u32,
) -> Result<String, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let recipe = store.get(&profile_id, version)?;
    Ok(recipe.pipeline_plan_hash())
}

/// CR-08 §22.1: duplicate an existing Recipe (any profile, any
/// version) into a fresh, independently-named profile at
/// version 1. The duplicated Recipe carries the same
/// `stages[]`, `required_models`, `integrity` payload,
/// `quality_profile`, and `description` as the source; the
/// new `name` is suffixed `" (Copy)"` and disambiguated if
/// a profile with that suffix already exists (sweep up to
/// 99 copies). The new profile has no `parent_version`
/// (it's a fresh lineage, not a save-of-new-version), so
/// `recipe_save` auto-assigns `version = 1`.
#[tauri::command]
fn recipe_duplicate(
    state: State<'_, RecipeState>,
    profile_id: String,
    version: u32,
) -> Result<RecipeSummary, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let source = store.get(&profile_id, version)?;
    // Build the new name. Start with "<name> (Copy)"; if
    // that name + target_type already exists as a
    // different profile, try "(Copy 2)" .. "(Copy 99)"; if
    // all 99 collide, fall back to a millisecond-precision
    // timestamp suffix so the duplicate is always creatable
    // (the path through `recipe_save` will then own the
    // error surface).
    let base_name = source.name.clone();
    let mut candidate = format!("{base_name} (Copy)");
    if !profile_exists(&store, &candidate, &source.target_type) {
        // First try succeeded.
    } else {
        let mut found = None;
        for n in 2..=99 {
            let try_name = format!("{base_name} (Copy {n})");
            if !profile_exists(&store, &try_name, &source.target_type) {
                candidate = try_name;
                found = Some(());
                break;
            }
        }
        if found.is_none() {
            candidate = format!(
                "{base_name} (Copy {})",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            );
        }
    }
    let mut owned = source;
    owned.name = candidate;
    // Compute the next version for this freshly-named
    // profile BEFORE save, mirroring the recipe_save IPC's
    // behavior: next_version_for returns 1 for a brand-new
    // profile_id, so the duplicate lands at v1 with no
    // parent_version (it's a fresh lineage, not a
    // save-of-new-version).
    let new_profile_id =
        astroforge_core::recipe_store::RecipeStore::profile_id_for(
            &owned.name,
            &owned.target_type,
        );
    let next_version = store
        .next_version_for(&new_profile_id)
        .map_err(CommandError::from)?;
    owned.version = if next_version == 0 { 1 } else { next_version };
    owned.parent_version = None;
    store.save(&owned).map_err(Into::into)
}

/// Helper for `recipe_duplicate`: does a profile with the
/// given name + target_type already exist? Returns true on
/// UNIQUE-collision risk. Pure read; holds the store lock
/// for the duration of the call (caller already holds it).
fn profile_exists(
    store: &std::sync::MutexGuard<'_, astroforge_core::recipe_store::RecipeStore>,
    name: &str,
    target_type: &str,
) -> bool {
    let profile_id =
        astroforge_core::recipe_store::RecipeStore::profile_id_for(name, target_type);
    match store.list() {
        Ok(summaries) => summaries.iter().any(|s| s.profile_id == profile_id),
        Err(_) => false,
    }
}

/// CR-07 §32.6: compute the AI-diff summary for two Recipes.
/// Pure function: loads both Recipes, calls `recipe_ai_diff_summary`.
/// Returns CommandError if either Recipe is not found.
#[tauri::command]
fn recipe_ai_diff_summary(
    state: State<'_, RecipeState>,
    profile_id_a: String,
    version_a: u32,
    profile_id_b: String,
    version_b: u32,
) -> Result<RecipeAiDiffSummary, CommandError> {
    use astroforge_core::recipe::recipe_ai_diff_summary as ai_diff_fn;
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let recipe_a = store.get(&profile_id_a, version_a)?;
    let recipe_b = store.get(&profile_id_b, version_b)?;
    Ok(ai_diff_fn(&recipe_a, &recipe_b))
}

/// CR-07 §29.2a: get-or-compute a diff image.
/// Cache lookup keyed by (version_a_id, version_b_id, mode, gain).
/// On miss, computes via `compute_diff` + stores. The mode
/// is passed as a kebab-case string ("absolute" /
/// "signed" / "amplified" / "structural"); the
/// `version_a_id` + `version_b_id` are opaque strings
/// (typically the ImageVersion row id from the IPC layer).
#[tauri::command]
fn diff_cache_get_or_compute(
    state: State<'_, DiffCacheState>,
    version_a_id: String,
    version_b_id: String,
    mode: String,
    gain: f32,
    width: u32,
    image_a: Vec<u8>,
    image_b: Vec<u8>,
) -> Result<Vec<u8>, CommandError> {
    let diff_kind = match mode.as_str() {
        "absolute" => DiffKind::Absolute,
        "signed" => DiffKind::Signed,
        "amplified" => DiffKind::Amplified,
        "structural" => DiffKind::Structural,
        _ => {
            return Err(CommandError {
                message: format!("unknown diff mode: {mode:?}"),
            });
        }
    };
    let key = DiffCacheKey::new(version_a_id, version_b_id, diff_kind, gain);
    let mut cache = state.0.lock().expect("diff cache mutex poisoned");
    let result = cache.get_or_compute(key, &image_a, &image_b, gain, width);
    // The cache stores owned Vec<u8>; clone to return to the
    // caller without holding the lock.
    Ok(result.clone())
}

/// CR-07 §29.2a: invalidate every cache entry referencing
/// the given version id. Used when a version's primary
/// artifact (TIFF) changes. Returns the number of entries
/// dropped.
#[tauri::command]
fn diff_cache_invalidate_version(
    state: State<'_, DiffCacheState>,
    version_id: String,
) -> Result<usize, CommandError> {
    let mut cache = state.0.lock().expect("diff cache mutex poisoned");
    Ok(cache.invalidate_version(&version_id))
}

/// CR-07 §29.2a: drop every cache entry. Stats counters
/// are preserved.
#[tauri::command]
fn diff_cache_clear(state: State<'_, DiffCacheState>) -> Result<(), CommandError> {
    let mut cache = state.0.lock().expect("diff cache mutex poisoned");
    cache.clear();
    Ok(())
}

/// CR-07 §29.2a: get a snapshot of the cache stats
/// (hits + misses counters).
#[tauri::command]
fn diff_cache_stats(state: State<'_, DiffCacheState>) -> Result<DiffCacheStats, CommandError> {
    let cache = state.0.lock().expect("diff cache mutex poisoned");
    Ok(cache.stats().clone())
}

/// CR-07 §29.2a: get the current cache size (number of
/// entries).
#[tauri::command]
fn diff_cache_len(state: State<'_, DiffCacheState>) -> Result<usize, CommandError> {
    let cache = state.0.lock().expect("diff cache mutex poisoned");
    Ok(cache.len())
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
        (
            args.channels as usize,
            args.height as usize,
            args.width as usize,
        ),
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

            // CR-07 §29.2a: in-memory diff cache. Global to
            // the app session, lives for the lifetime of
            // the Tauri runtime. No DB backing.
            app.manage(DiffCacheState(Mutex::new(DiffCache::new())));

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
            let pipeline_plans_store =
                astroforge_core::pipeline_plans_store::PipelinePlanStore::new(&pipeline_plans_db)
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
                        std::sync::Arc::new(
                            astroforge_core::pipeline_plan::dispatch::CalibrateHandler,
                        ),
                    );
                    reg.insert(
                        "debayer",
                        std::sync::Arc::new(
                            astroforge_core::pipeline_plan::dispatch::DebayerHandler,
                        ),
                    );
                    reg.insert(
                        "register",
                        std::sync::Arc::new(
                            astroforge_core::pipeline_plan::dispatch::RegisterHandler,
                        ),
                    );
                    reg.insert(
                        "stack",
                        std::sync::Arc::new(astroforge_core::pipeline_plan::dispatch::StackHandler),
                    );
                    reg.insert(
                        "background",
                        std::sync::Arc::new(
                            astroforge_core::pipeline_plan::dispatch::BackgroundHandler,
                        ),
                    );
                    reg.insert(
                        "stretch",
                        std::sync::Arc::new(
                            astroforge_core::pipeline_plan::dispatch::StretchHandler,
                        ),
                    );
                    reg.insert(
                        "denoise",
                        std::sync::Arc::new(
                            astroforge_core::pipeline_plan::dispatch::DenoiseHandler,
                        ),
                    );
                    reg.insert(
                        "export",
                        std::sync::Arc::new(
                            astroforge_core::pipeline_plan::dispatch::ExportHandler,
                        ),
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
            quality_profile_list,
            recipe_get,
            recipe_get_head,
            recipe_get_for_image_version,
            recipe_save,
            recipe_duplicate,
            recipe_pipeline_plan_hash,
            recipe_ai_diff_summary,
            diff_cache_get_or_compute,
            diff_cache_invalidate_version,
            diff_cache_clear,
            diff_cache_stats,
            diff_cache_len,
            // CR-05 R1 — read-only AI model catalog (Recipes/AI
            // Models/Settings/Help application-level surfaces).
            commands_ai_models::ai_model_list,
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
            // CR-05 R2 — truthful project-level checklist state.
            commands_project::project_overview,
            // CR-05 R3 — image-version timeline for the Compare
            // workspace. Derived from the durable event log.
            commands_image_versions::image_version_list,
            // CR-06 P1 — AI Enhancement Studio data model +
            // provenance + safety classification shells.
            // Substantive behavior lands in P2–P6; P1 only
            // exposes the IPC surface so the TS wrapper
            // types and the project-lifecycle reset hook
            // can land alongside the schema migration.
            commands_ai_enhancement::ai_operation_get,
            commands_ai_enhancement::ai_operation_list_for_stage,
            commands_ai_enhancement::image_analysis_latest,
            commands_ai_enhancement::image_region_list,
            commands_ai_enhancement::ai_recommendation_list_for_version,
            commands_ai_enhancement::ai_mask_list,
            commands_ai_enhancement::enhancement_stack_list_for_source,
            commands_ai_enhancement::enhancement_preview_list_for_operation,
            // CR-06 P2 — image-analysis engine. Runs the
            // analyzer over the supplied pixel buffer and
            // persists the report to `image_analyses`.
            commands_ai_enhancement::analyze_image,
            // CR-06 P3 — recommendation engine. Reads the
            // latest analysis, runs the rules, persists
            // `ai_recommendations`, and returns the report.
            commands_ai_enhancement::generate_ai_recommendations,
            // CR-06 P4 — enhancement stack + apply round +
            // image-version tracking. The stack engine is
            // pure logic in `astroforge-core::enhancement`;
            // the apply round creates a fresh Image
            // Version per CR-06 §4 / §22.
            commands_ai_enhancement::enhancement_stack_create,
            commands_ai_enhancement::enhancement_stack_get,
            commands_ai_enhancement::enhancement_stack_apply_mutation,
            commands_ai_enhancement::enhancement_stack_branch,
            commands_ai_enhancement::enhancement_apply_operation,
            commands_ai_enhancement::enhancement_operations_list,
            commands_ai_enhancement::operations_registry_list,
            commands_ai_enhancement::image_version_list_for_project,
            commands_ai_enhancement::image_version_get,
            // CR-07 — read the applied Image Version's primary
            // artifact bytes for the Zone B canvas. Path-confined
            // to <root>/.astroforge/applied/<project_id>/.
            commands_ai_enhancement::read_image_artifact,
            // CR-06 P5 — region-aware masks. The mask
            // engine is pure logic in
            // `astroforge-core::masks`; the create /
            // update / compose commands persist the
            // resulting rows. `build_auto_mask` runs the
            // auto-segmentation on the supplied pixels
            // and returns the encoded raster.
            commands_ai_enhancement::create_ai_mask,
            commands_ai_enhancement::update_ai_mask,
            commands_ai_enhancement::ai_mask_get,
            commands_ai_enhancement::ai_mask_list_for_version,
            commands_ai_enhancement::build_auto_mask,
            commands_ai_enhancement::compose_mask,
            // CR-06 P6 — quality gate orchestrator.
            // The engine in
            // `astroforge-core::quality_gates` runs
            // the ten §37 checks; the command
            // accepts (source, result) pixels and
            // returns the verdict + per-gate findings.
            // Real ONNX inference will trigger this
            // from `enhancement_apply_operation` once
            // the dispatcher is swapped (P5.1).
            commands_ai_enhancement::run_ai_quality_report,
            // CR-04 P8 — Import Understanding IPC surface.
            // Wires P3..P7 into the import wizard; the UI
            // (P9) reads these commands to render the
            // Understanding panel + ambiguity dialog.
            commands_import::import_analyse_session,
            commands_import::import_get_understanding,
            commands_import::import_confirm,
            commands_import::import_override_classification,
            commands_import::import_set_materialised,
            // CR-04 P10 — AI provenance IPC. Surfaces the
            // source of authority for the target
            // classification (deterministic / ai-stub /
            // user-override) to the Understanding panel.
            commands_import::import_get_target_provenance,
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
            // CR-05 P6.1b (§9) — per-stage retry + skip commands
            // that close the ErrorRecoveryPanel button wiring
            // deferred from P6.1.
            commands_pipeline_plan::retry_stage,
            commands_pipeline_plan::skip_stage,
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
            // CR-07 B3 — image decisions + comparison sets.
            commands_comparison::save_image_decision,
            commands_comparison::load_image_decision,
            commands_comparison::list_image_decisions_for_project,
            commands_comparison::apply_image_decision,
            commands_comparison::save_comparison_set,
            commands_comparison::load_comparison_set,
            commands_comparison::list_comparison_sets_for_project,
            commands_comparison::delete_comparison_set,
            // CR-07 B4 — version-pair metric comparison (§10 + §11).
            commands_comparison::compare_version_metrics,
            // CR-07 §23.1: per-version full metric snapshot for
            // the expert channel-stats panel.
            commands_comparison::get_version_metric_snapshot,
            // CR-07 §23.2: per-version FWHM distribution
            // (per-star FWHM values + pre-binned histogram
            // + seven-number summary) for the expert FWHM
            // distribution panel.
            commands_comparison::get_version_fwhm_distribution,
            // CR-07 §23.3: per-version 2D noise map
            // (per-pixel local sigma field + three-number
            // summary) for the expert noise map panel.
            commands_comparison::get_version_noise_map,
            // CR-07 §23.4: per-version highlight + shadow
            // clipping masks for the expert clipping-masks
            // panel.
            commands_comparison::get_version_clipping_masks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AstroForge");
}
