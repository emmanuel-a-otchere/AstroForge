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
use astroforge_core::recipe::{apply_recipe, QualityProfile, Recipe, RecipeAiDiffSummary};
use astroforge_core::recipe_store::{RecipeStore, RecipeSummary, RecipeVersion};
use astroforge_core::session::SessionStore;
use astroforge_core::validation::StageSpec;
use serde::Serialize;
use tauri::{Manager, State};

/// CR-08 §20: canonical stage spec catalog. The
/// stage IDs are drawn from the existing
/// `pipeline_plan_hash` ordering in
/// `astroforge-core::recipe` (the §32.4 content
/// hash sorts stages by `stage_id`); the spec
/// entries document the §20 ranges + dependencies
/// for the canonical stages that ship in §22
/// quality catalogs.
///
/// `LazyLock` keeps the spec table out of the
/// hot loop (it's built once at process start).
/// The table mirrors the §22 quality profile
/// catalog shape so the §20 UI panel can render
/// the same stage list verbatim.
static SPEC_CATALOG: std::sync::LazyLock<
    std::collections::HashMap<String, StageSpec>,
> = std::sync::LazyLock::new(|| {
    use astroforge_core::validation::ParamRange;
    let mut m: std::collections::HashMap<String, StageSpec> =
        std::collections::HashMap::new();

    // Each stage ships with:
    // - resource_units (rough cost estimate
    //   against MAX_RESOURCE_UNITS = 300)
    // - per-key numeric ranges for any param
    //   that benefits from clamping (e.g.
    //   denoise radius, stretch bias)
    // - dependency edges (e.g. color_calibration
    //   depends on debayer)
    //
    // The specs are intentionally conservative;
    // a follow-on slice that walks the §22
    // quality-catalog recipes will tighten
    // individual ranges per quality profile.
    let mut stretch = StageSpec::cheap("stretch", 80);
    stretch
        .params
        .insert("bias".into(), ParamRange::bounded(0.0, 1.0));
    m.insert("stretch".into(), stretch);

    let mut denoise = StageSpec::cheap("denoise", 60);
    denoise
        .params
        .insert("radius".into(), ParamRange::bounded(0.5, 5.0));
    m.insert("denoise".into(), denoise);

    let mut sharpen = StageSpec::cheap("sharpen", 40);
    sharpen
        .params
        .insert("amount".into(), ParamRange::bounded(0.0, 2.0));
    m.insert("sharpen".into(), sharpen);

    let mut color_calibration = StageSpec::cheap("color_calibration", 50);
    color_calibration
        .required_stages
        .push("debayer".into());
    m.insert("color_calibration".into(), color_calibration);

    m.insert("debayer".into(), StageSpec::cheap("debayer", 30));
    m.insert("crop".into(), StageSpec::cheap("crop", 10));
    m.insert("cosmetic".into(), StageSpec::cheap("cosmetic", 20));
    m.insert("curves".into(), StageSpec::cheap("curves", 30));
    m.insert("stacking".into(), StageSpec::cheap("stacking", 100));
    m
});

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

/// CR-08 §22.3: apply a Recipe (any profile, any version) and
/// return the stage-id + params pairs the caller should drive
/// the next apply round with. The IPC loads the Recipe via
/// the same `RecipeStore::get` path the duplicate + content-
/// hash IPCs use, then delegates to
/// `astroforge_core::recipe::apply_recipe`, which performs the
/// schema-version guard + the missing-models compatibility
/// check and returns the enabled stages in order.
///
/// `available_models` is the caller-provided inventory of
/// models currently registered in the running app (no
/// central registry exists yet; the orchestrator's model
/// list is the canonical source). When the list is `None`
/// (or empty), the compatibility check is skipped for
/// missing-models but the schema-version guard still
/// runs. Pass `Some(vec![])` for an explicit "no models"
/// session.
///
/// Per-stage parameters are returned in Recipe order. The
/// caller decides what to do with each stage (apply, skip,
/// preview). This keeps the IPC a pure function over the
/// Recipe store; the chained apply is the UI's job.
#[tauri::command]
fn recipe_apply(
    state: State<'_, RecipeState>,
    profile_id: String,
    version: u32,
    available_models: Option<Vec<String>>,
) -> Result<RecipeApplyResponse, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let recipe = store.get(&profile_id, version)?;
    let models_slice: &[String] = available_models
        .as_deref()
        .unwrap_or(&[]);
    let stages = apply_recipe(&recipe, models_slice).map_err(|e| {
        CommandError::Invalid(format!(
            "recipe {profile_id:?} v{version} is not applicable: {e}"
        ))
    })?;
    // CR-08 §15: stamp `last_used_at` so the Recently Used
    // tab surfaces this recipe at the top after the apply.
    // The stamp is best-effort: if the row vanished between
    // the `get` and now (e.g. concurrent delete), the apply
    // result is still returned; the UI just won't show it
    // in Recently Used.
    if let Err(e) = store.mark_last_used(&profile_id) {
        eprintln!(
            "recipe_apply: failed to stamp last_used_at for {profile_id:?}: {e}"
        );
    }
    Ok(RecipeApplyResponse {
        profile_id,
        version: recipe.version,
        branch: recipe.branch.clone(),
        stages,
    })
}

/// CR-08 §22.3 response payload: the Recipe identity
/// (version + branch) so the UI can scope follow-up
/// apply calls + provenance writes, plus the ordered
/// list of `(stage_id, params)` pairs to drive. The
/// `profile_id` is echoed back from the request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecipeApplyResponse {
    profile_id: String,
    version: u32,
    branch: String,
    stages: Vec<(String, std::collections::HashMap<String, serde_json::Value>)>,
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

/// CR-08 §19: export a Recipe (any profile, any version;
/// version=None exports the head) as its canonical JSON
/// payload. The payload is the same `payload_json` form the
/// store persists; `Recipe::to_json` round-trips through
/// `from_json_migrated` on the import side. The file-save
/// dialog + `.afrecipe` filename convention live in the UI
/// follow-on slice; this handler returns the content.
#[tauri::command]
fn recipe_export(
    state: State<'_, RecipeState>,
    profile_id: String,
    version: Option<u32>,
) -> Result<String, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let recipe = match version {
        Some(v) => store.get(&profile_id, v)?,
        None => store.get_head(&profile_id)?,
    };
    recipe.to_json().map_err(CommandError::from)
}

/// CR-08 §19: import a Recipe from its canonical JSON
/// payload. Runs `from_json_migrated` (v1 -> v2 migration,
/// hard error on unknown future schemas), then RE-LINEAGES
/// the recipe against the local store: the imported recipe
/// always lands as the next version of its (name,
/// target_type) profile, with `parent_version` pointing at
/// the local head (or None for a fresh profile). Importing
/// a recipe whose (name, target_type) matches an existing
/// profile appends to that profile rather than forking a
/// duplicate; importing under a fresh name starts a new
/// lineage at v1. The original `version` / `parent_version`
/// from the foreign chain are intentionally discarded:
/// version numbers are local-store lineage, and keeping
/// foreign numbers would create phantom ancestry.
#[tauri::command]
fn recipe_import(
    state: State<'_, RecipeState>,
    json: String,
) -> Result<RecipeSummary, CommandError> {
    let mut recipe =
        Recipe::from_json_migrated(&json).map_err(CommandError::from)?;
    if recipe.name.trim().is_empty() {
        return Err(CommandError::new(
            "validation",
            "imported recipe has an empty name",
        ));
    }
    if recipe.target_type.trim().is_empty() {
        return Err(CommandError::new(
            "validation",
            "imported recipe has an empty target_type",
        ));
    }
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let profile_id =
        astroforge_core::recipe_store::RecipeStore::profile_id_for(
            &recipe.name,
            &recipe.target_type,
        );
    let next_version = store.next_version_for(&profile_id)?;
    recipe.version = if next_version == 0 { 1 } else { next_version };
    recipe.parent_version = match store.get_head(&profile_id) {
        Ok(head) => Some(head.version),
        Err(_) => None,
    };
    let summary = store.save(&recipe).map_err(Into::into)?;
    // CR-08 §15: flag the just-saved head row as imported so
    // the Imported tab can classify it. The flag lives on
    // every row of the profile, so a subsequent `recipe_save`
    // (user-edits the imported recipe) keeps the imported
    // classification.
    if let Err(e) = store.mark_imported(&profile_id) {
        eprintln!(
            "recipe_import: failed to mark_imported for {profile_id:?}: {e}"
        );
    }
    Ok(summary)
}

/// CR-08 §14: "Save Pipeline as Recipe" UX. Loads a
/// `PipelinePlan`, builds a [`Recipe`] from its stages,
/// and persists it via `RecipeStore::save`. Mirrors the
/// `recipe_apply` IPC's pattern: the new IPC is the
/// production-side companion that turns a session's
/// terminal pipeline plan into a portable Recipe.
///
/// - `plan_id`: the `PipelinePlan.plan_id` to read.
/// - `name`: the new Recipe's name.
/// - `target_type`: optional override; when `None`, the
///   plan's `target_type` (serialized snake_case) is
///   used.
///
/// Returns the `RecipeSummary` of the saved v1 Recipe
/// (a freshly-saved user Recipe is never flagged as a
/// system Recipe -- callers use `recipe_mark_as_system`
/// to flip that if desired).
#[tauri::command]
fn recipe_save_from_pipeline_plan(
    recipe_state: State<'_, RecipeState>,
    pipeline_state: State<'_, commands_pipeline_plan::PipelinePlanState>,
    plan_id: String,
    name: String,
    target_type: Option<String>,
) -> Result<RecipeSummary, CommandError> {
    // Load the plan (clone-then-release pattern so we
    // don't hold both locks at once).
    let plan = {
        let store = pipeline_state
            .store
            .lock()
            .map_err(|_| "pipeline plan store mutex poisoned".to_string())?;
        store
            .load_plan(&plan_id)
            .map_err(|e| format!("failed to load pipeline plan: {e}"))?
    };
    // Build the Recipe from the loaded plan (pure function).
    let mut recipe = astroforge_core::recipe::recipe_from_pipeline_plan(
        &plan,
        &name,
        target_type.as_deref(),
    );
    // Assign the next version BEFORE save (mirrors the
    // recipe_save IPC pattern).
    let profile_id = astroforge_core::recipe_store::RecipeStore::profile_id_for(
        &recipe.name,
        &recipe.target_type,
    );
    let version = {
        let store = recipe_state
            .0
            .lock()
            .expect("recipe store mutex poisoned");
        store
            .next_version_for(&profile_id)
            .map_err(|e| format!("failed to compute next version: {e}"))?
    };
    recipe.version = version;
    let store = recipe_state.0.lock().expect("recipe store mutex poisoned");
    store.save(&recipe).map_err(Into::into)
}

/// CR-08 §3.1: mark a Recipe profile as a system Recipe.
/// All existing versions of the profile get `is_system = 1`
/// in the on-disk column, after which `recipe_save` and
/// `recipe_delete` refuse to mutate the profile. Returns
/// the `profile_id` so the caller can chain a UI refresh.
#[tauri::command]
fn recipe_mark_as_system(
    state: State<'_, RecipeState>,
    profile_id: String,
) -> Result<bool, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    store.mark_as_system(&profile_id).map_err(Into::into)
}

/// CR-08 §3.1: read whether any version of a profile is
/// currently marked as a system Recipe. The UI uses this
/// to render the "System" badge + gate destructive
/// actions in the RecipesScreen toolbar.
#[tauri::command]
fn recipe_is_system(
    state: State<'_, RecipeState>,
    profile_id: String,
) -> Result<bool, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    store.is_system_profile(&profile_id).map_err(Into::into)
}

/// CR-08 §22.4: archive a Recipe profile. Archived
/// profiles remain on disk but are hidden from the
/// default `recipe_list` results (the §28
/// "delete/archive" row's archive half). Returns true
/// when at least one row was updated, false when the
/// profile does not exist yet.
#[tauri::command]
fn recipe_archive(
    state: State<'_, RecipeState>,
    profile_id: String,
) -> Result<bool, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    store.archive_profile(&profile_id).map_err(Into::into)
}

/// CR-08 §22.4: unarchive a Recipe profile. Returns
/// true when at least one row was updated.
#[tauri::command]
fn recipe_unarchive(
    state: State<'_, RecipeState>,
    profile_id: String,
) -> Result<bool, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    store.unarchive_profile(&profile_id).map_err(Into::into)
}

/// CR-08 §22.4: read whether any version of a profile
/// is currently archived. The UI uses this to render
/// the "Archived" badge + toggle in RecipesScreen.
#[tauri::command]
fn recipe_is_archived(
    state: State<'_, RecipeState>,
    profile_id: String,
) -> Result<bool, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    store.is_archived_profile(&profile_id).map_err(Into::into)
}

/// CR-08 §22.2: delete a Recipe profile (every version,
/// every branch). Returns the number of rows deleted.
/// Deleting a profile that does not exist returns 0 (the
/// call is idempotent so the UI can fire delete without a
/// pre-check). The system-recipe protection the CR-08 §3
/// spec calls for is not yet implemented; when §3 lands,
/// this handler is the gate point for the "protected"
/// check.
#[tauri::command]
fn recipe_delete(
    state: State<'_, RecipeState>,
    profile_id: String,
) -> Result<u32, CommandError> {
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let deleted = store.delete_profile(&profile_id)?;
    Ok(deleted as u32)
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

/// CR-08 §13: compute the mechanical parameter diff for
/// two Recipes. Pure function: loads both Recipes, calls
/// `recipe_parameter_diff`. Returns the per-stage
/// Added / Removed / Modified breakdown that the
/// `RecipeDiffPanel.svelte` consumer renders side-by-side
/// in CompareWorkspace. Returns CommandError if either
/// Recipe is not found.
#[tauri::command]
fn recipe_parameter_diff(
    state: State<'_, RecipeState>,
    profile_id_a: String,
    version_a: u32,
    profile_id_b: String,
    version_b: u32,
) -> Result<
    astroforge_core::recipe::RecipeParameterDiff,
    CommandError,
> {
    use astroforge_core::recipe::recipe_parameter_diff as param_diff_fn;
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let recipe_a = store.get(&profile_id_a, version_a)?;
    let recipe_b = store.get(&profile_id_b, version_b)?;
    Ok(param_diff_fn(&recipe_a, &recipe_b))
}

/// CR-08 §22 round 1: combined comparison of two
/// Recipes. Thin wrapper that loads both Recipes and
/// delegates to `recipe_compare_versions` (which folds
/// `recipe_ai_diff_summary` + `recipe_parameter_diff`
/// into a single `RecipeComparison` response so the
/// `RecipeDiffPanel.svelte` consumer can fetch both
/// halves in one IPC round-trip). Pure function on the
/// core side; the wrapper's only job is to load the two
/// Recipes from the store + convert `RecipeStoreError`
/// into `CommandError`. Returns CommandError if either
/// Recipe is not found.
#[tauri::command]
fn recipe_compare_versions(
    state: State<'_, RecipeState>,
    profile_id_a: String,
    version_a: u32,
    profile_id_b: String,
    version_b: u32,
) -> Result<
    astroforge_core::recipe::RecipeComparison,
    CommandError,
> {
    use astroforge_core::recipe::recipe_compare_versions as compare_fn;
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let recipe_a = store.get(&profile_id_a, version_a)?;
    let recipe_b = store.get(&profile_id_b, version_b)?;
    Ok(compare_fn(&recipe_a, &recipe_b))
}

/// CR-08 §22 round 1: Recipe provenance. Returns a
/// `RecipeProvenance` describing the Recipe itself
/// (identity, lineage_steps, perceptual_models,
/// required_models, etc). Distinct from the image-
/// version provenance rendered by `ProvenancePanel.svelte`
/// which walks the image's `recipe_id` chain; this
/// surface is Recipe-only. The IPC computes `profile_id`
/// from `name + target_type` via `RecipeStore::profile_id_for`
/// and forwards it to `recipe_provenance` so the core
/// function stays pure. Returns CommandError if the
/// Recipe is not found.
#[tauri::command]
fn recipe_get_provenance(
    state: State<'_, RecipeState>,
    profile_id: String,
    version: u32,
) -> Result<
    astroforge_core::recipe::RecipeProvenance,
    CommandError,
> {
    use astroforge_core::recipe::recipe_provenance as provenance_fn;
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let recipe = store.get(&profile_id, version)?;
    Ok(provenance_fn(&profile_id, &recipe))
}

/// CR-08 §22 round 2 / Slice A: read-only preview of
/// a Recipe. The read-only sibling of `recipe_apply`
/// (which mutates `last_used_at` and filters to
/// enabled stages); `preview_recipe` returns the full
/// preview surface (all stages + metadata +
/// provenance + applicability + warnings) even when
/// the Recipe is not applicable so the UI can show
/// WHY without forcing a fix-or-abort loop. The
/// `available_models` argument is optional; when
/// empty (the common preview-against-fleet case), the
/// function still returns the Recipe but the
/// `applicability` field may flag `MissingModels`.
/// Returns CommandError if the Recipe is not found.
#[tauri::command]
fn recipe_preview(
    state: State<'_, RecipeState>,
    profile_id: String,
    version: u32,
    available_models: Option<Vec<String>>,
) -> Result<
    astroforge_core::recipe::RecipePreviewResponse,
    CommandError,
> {
    use astroforge_core::recipe::preview_recipe as preview_fn;
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let recipe = store.get(&profile_id, version)?;
    let models_slice: &[String] = available_models
        .as_deref()
        .unwrap_or(&[]);
    Ok(preview_fn(&profile_id, &recipe, models_slice))
}

/// CR-08 §20: validate a Recipe against the
/// §20 stage-spec table for the five risk classes
/// (range, dependency, filesystem, executable,
/// resource). Pure function on top of the
/// `validation::validate_recipe_security` core +
/// the canonical `SPEC_CATALOG` constant. Returns
/// the full `SecurityValidationReport` so the
/// §20 UI panel can render every violation in
/// a single render. Returns CommandError if the
/// Recipe is not found.
#[tauri::command]
fn recipe_security_validate(
    state: State<'_, RecipeState>,
    profile_id: String,
    version: u32,
) -> Result<
    astroforge_core::validation::SecurityValidationReport,
    CommandError,
> {
    use astroforge_core::validation::validate_recipe_security;
    let store = state.0.lock().expect("recipe store mutex poisoned");
    let recipe = store.get(&profile_id, version)?;
    Ok(validate_recipe_security(&recipe, &SPEC_CATALOG))
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
        .plugin(tauri_plugin_fs::init())
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
            // CR-08 §3.2: seed DwarfII v1 + M42-Natural-v1 on
            // first launch. Both are flipped to is_system = 1
            // so the §3.1 guard refuses to mutate them.
            // Idempotent: pre-existing profiles are
            // left untouched (apart from the system-flag
            // upgrade path which keeps the flag in sync).
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
            recipe_delete,
            recipe_archive,
            recipe_unarchive,
            recipe_is_archived,
            recipe_mark_as_system,
            recipe_is_system,
            recipe_export,
            recipe_import,
            recipe_pipeline_plan_hash,
            recipe_apply,
            recipe_save_from_pipeline_plan,
            recipe_ai_diff_summary,
            recipe_parameter_diff,
            recipe_compare_versions,
            recipe_get_provenance,
            recipe_preview,
            recipe_security_validate,
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
