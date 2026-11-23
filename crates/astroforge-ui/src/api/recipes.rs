//! Recipe (pipeline profile) management — port of the inline recipe
//! commands in `src-tauri/src/main.rs` (`recipe_*` +
//! `quality_profile_list`).
//!
//! Covers: listing / retrieval (`recipe_list`, `recipe_list_versions`,
//! `recipe_get`, `recipe_get_head`, `recipe_get_for_image_version`),
//! save + lifecycle events (`recipe_save`), duplicate / delete /
//! archive / system gating (`recipe_duplicate`, `recipe_delete`,
//! `recipe_archive` / `recipe_unarchive` / `recipe_is_archived`,
//! `recipe_mark_as_system`, `recipe_is_system`),
//! import/export (`recipe_import`, `recipe_export`),
//! apply / preview / applicability / adaptation
//! (`recipe_apply`, `recipe_preview`, `recipe_check_applicability`,
//! `recipe_adapt`, `recipe_accept_adaptation`),
//! provenance + reproducibility (`recipe_get_provenance`,
//! `recipe_get_reproducibility_report`),
//! diffs (`recipe_ai_diff_summary`, `recipe_parameter_diff`,
//! `recipe_compare_versions`, `recipe_pipeline_plan_hash`),
//! "save pipeline as recipe" (`recipe_save_from_pipeline_plan`,
//! `recipe_save_from_stage_runs`),
//! §20 security validation with the local `SPEC_CATALOG` copy
//! (`recipe_security_validate`), the CR-08 §23 Recipe event log read
//! (`recipe_list_events`), and the §22 quality-profile catalog
//! (`quality_profile_list`).
//!
//! Bodies are ported 1:1 from the Tauri commands; errors are `String`
//! with the same text the Tauri commands surfaced.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Serialize;

use crate::services::Services;

use astroforge_core::adaptive::{derive_adapted_recipe, ImageMetrics, RecipeAdaptationResponse};
use astroforge_core::recipe::{
    apply_recipe, check_recipe_applicability, preview_recipe, recipe_from_pipeline_plan,
    recipe_from_stage_runs, recipe_provenance, validate_compatibility, ApplicabilityMatrix,
    ApplicabilityOutcome, QualityProfile, Recipe, RecipeAiDiffSummary, RecipeComparison,
    RecipeParameterDiff, RecipePreviewResponse, RecipeProvenance,
};
use astroforge_core::recipe_events::{
    now_iso as recipe_events_now_iso, payload_pipeline_saved_as_recipe,
    payload_recipe_adaptation_accepted, payload_recipe_adaptation_proposed,
    payload_recipe_applicability_evaluated, payload_recipe_applied, payload_recipe_created,
    payload_recipe_exported, payload_recipe_import_completed, payload_recipe_import_failed,
    payload_recipe_import_started, payload_recipe_version_created,
    payload_reproducibility_record_created, RecipeEvent, RecipeEventFilter, RecipeEventKind,
};
use astroforge_core::recipe_store::{RecipeStore, RecipeSummary, RecipeVersion};
use astroforge_core::reproducibility::ReproducibilityRecord;
use astroforge_core::validation::{
    validate_recipe_security, ParamRange, SecurityValidationReport, StageSpec,
};

fn lock_err<E: std::fmt::Display>(e: E) -> String {
    format!("lock poisoned: {e}")
}

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
///
/// Verbatim copy of `main.rs::SPEC_CATALOG`.
static SPEC_CATALOG: LazyLock<HashMap<String, StageSpec>> = LazyLock::new(|| {
    let mut m: HashMap<String, StageSpec> = HashMap::new();

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
    color_calibration.required_stages.push("debayer".into());
    m.insert("color_calibration".into(), color_calibration);

    m.insert("debayer".into(), StageSpec::cheap("debayer", 30));
    m.insert("crop".into(), StageSpec::cheap("crop", 10));
    m.insert("cosmetic".into(), StageSpec::cheap("cosmetic", 20));
    m.insert("curves".into(), StageSpec::cheap("curves", 30));
    m.insert("stacking".into(), StageSpec::cheap("stacking", 100));
    m
});

/// Mirror of `main.rs::QualityProfileInfo` — the §22 catalog entry for
/// the frontend picker (same shape as the TS `QualityProfileInfo`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityProfileInfo {
    pub id: String,
    pub label: String,
    pub description: String,
}

/// Mirror of `main.rs::RecipeApplyResponse` — the Recipe identity
/// (version + branch) so the UI can scope follow-up apply calls +
/// provenance writes, plus the ordered list of `(stage_id, params)`
/// pairs to drive. The `profile_id` is echoed back from the request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeApplyResponse {
    pub profile_id: String,
    pub version: u32,
    pub branch: String,
    pub stages: Vec<(String, HashMap<String, serde_json::Value>)>,
}

/// `recipe_list`
pub fn recipe_list(services: &Services) -> Result<Vec<RecipeSummary>, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    store.list().map_err(|e| e.to_string())
}

/// `recipe_list_versions`
pub fn recipe_list_versions(
    services: &Services,
    profile_id: &str,
) -> Result<Vec<RecipeVersion>, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    store.list_versions(profile_id).map_err(|e| e.to_string())
}

/// `quality_profile_list` — CR-07 §22 catalog. Mirrors the Rust
/// enum's 4 variants with display labels and descriptions.
pub fn quality_profile_list() -> Vec<QualityProfileInfo> {
    QualityProfile::ALL
        .iter()
        .map(|p| QualityProfileInfo {
            id: format!("{:?}", p).to_lowercase(),
            label: p.label().to_string(),
            description: p.description().to_string(),
        })
        .collect()
}

/// `recipe_get` — a specific (profile_id, version) pair.
pub fn recipe_get(services: &Services, profile_id: &str, version: u32) -> Result<Recipe, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    store.get(profile_id, version).map_err(|e| e.to_string())
}

/// `recipe_get_head` — the head version of a profile.
pub fn recipe_get_head(services: &Services, profile_id: &str) -> Result<Recipe, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    store.get_head(profile_id).map_err(|e| e.to_string())
}

/// `recipe_save` — computes the next version for the profile, pins
/// the lineage, recomputes the content hash, persists, and emits the
/// RecipeCreated / RecipeVersionCreated lifecycle event.
pub fn recipe_save(services: &Services, recipe: Recipe) -> Result<RecipeSummary, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    // Caller can either set recipe.version themselves or rely on us to
    // compute the next version based on (name, target_type). For PR-A
    // we always compute here so the UI side can be naive.
    let profile_id = RecipeStore::profile_id_for(&recipe.name, &recipe.target_type);
    let next_version = store
        .next_version_for(&profile_id)
        .map_err(|e| e.to_string())?;
    let owned_version = recipe.version;
    let mut owned = recipe;
    if owned.version == 0 {
        owned.version = next_version;
    }
    if owned.parent_version.is_none() && next_version > 1 {
        owned.parent_version = Some(next_version - 1);
    }
    // CR-08 §21 follow-on / Slice E + §28 / Slice H:
    // recompute the content hash on every save so the
    // stored hash always matches the saved content.
    // Recomputing (rather than reading the incoming
    // payload's hash) defends against a stale client
    // passing a hash that doesn't match the staged
    // content.
    owned.content_hash = owned.pipeline_plan_hash();
    let summary = store.save(&owned).map_err(|e| e.to_string())?;

    // CR-08 §23 / Slice F: emit the Recipe lifecycle
    // event. The event kind branches on whether the
    // save created a fresh profile (version 1) or a
    // new version of an existing profile.
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    let (kind, payload) = if owned_version == 0 && next_version == 1 {
        (
            RecipeEventKind::RecipeCreated,
            payload_recipe_created(&owned.name, &owned.target_type, owned.version),
        )
    } else {
        (
            RecipeEventKind::RecipeVersionCreated,
            payload_recipe_version_created(owned.parent_version),
        )
    };
    if let Err(e) = event_store.record_event(
        kind,
        Some(&profile_id),
        Some(owned.version),
        payload,
        &recipe_events_now_iso(),
    ) {
        eprintln!("recipe_save: failed to record event for {profile_id:?} v{owned_version}: {e}");
    }

    Ok(summary)
}

/// `recipe_get_for_image_version` — CR-07 B13c: look up the live
/// `Recipe` that produced a given Image Version. Walks two stores:
///
/// 1. The project `DomainStore.image_versions` table (B13a) for the
///    `recipe_id` foreign key (the port replicates the
///    `commands_ai_enhancement::image_version_get` read + JSON
///    round-trip inline).
/// 2. The global `RecipeStore` (`recipe_get_head`) for the live
///    `Recipe` row keyed by `profile_id`.
///
/// Returns `Ok(None)` when the version does not exist (returns `None`
/// rather than an error: the caller treats "no such version" the same
/// as "no recipe" at the panel level) OR when the version exists but
/// its `recipe_id` is `None` (legacy / AI-applied versions). Returns
/// `Err(...)` only on the `recipe_get_head` failure.
pub fn recipe_get_for_image_version(
    services: &Services,
    version_id: &str,
) -> Result<Option<Recipe>, String> {
    // Step 1: read the version row from the project's store.
    // `image_version_get` returns `Value::Null` for "not found".
    let raw: serde_json::Value = {
        let store = services.domain_store.lock().map_err(lock_err)?;
        match store.get_image_version(version_id) {
            Ok(Some(row)) => serde_json::to_value(&row).map_err(|e| e.to_string())?,
            Ok(None) => serde_json::Value::Null,
            Err(e) => return Err(e.to_string()),
        }
    };
    let version_value = match raw {
        serde_json::Value::Null => return Ok(None),
        v => v,
    };

    // Step 2: extract recipe_id. Serde-derived; the field is on the
    // IPC wire because B13a added it to the `ImageVersion` Rust struct
    // + serde, so it's already present in the JSON.
    let profile_id: Option<String> =
        serde_json::from_value(serde_json::json!({ "recipe_id": version_value.get("recipe_id") }))
            .map_err(|e| format!("malformed ImageVersion payload: {e}"))?;
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
    let store = services.recipes.lock().map_err(lock_err)?;
    match store.get_head(&profile_id) {
        Ok(recipe) => Ok(Some(recipe)),
        Err(e) => Err(format!(
            "recipe_id {profile_id:?} recorded on version but RecipeStore.get_head failed: {e}"
        )),
    }
}

/// `recipe_pipeline_plan_hash` — CR-07 §32.6: compute the pipeline
/// plan hash for a Recipe. Pure function: loads the Recipe, calls
/// `pipeline_plan_hash()`. Errors if the Recipe is not found.
pub fn recipe_pipeline_plan_hash(
    services: &Services,
    profile_id: &str,
    version: u32,
) -> Result<String, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let recipe = store.get(profile_id, version).map_err(|e| e.to_string())?;
    Ok(recipe.pipeline_plan_hash())
}

/// `recipe_apply` — CR-08 §22.3: apply a Recipe (any profile, any
/// version) and return the stage-id + params pairs the caller should
/// drive the next apply round with. Delegates to
/// `astroforge_core::recipe::apply_recipe`, which performs the
/// schema-version guard + the missing-models compatibility check and
/// returns the enabled stages in order. Stamps `last_used_at` (§15,
/// best-effort) and emits the RecipeApplied event (§23 Slice F).
pub fn recipe_apply(
    services: &Services,
    profile_id: &str,
    version: u32,
    available_models: Option<Vec<String>>,
) -> Result<RecipeApplyResponse, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let recipe = store.get(profile_id, version).map_err(|e| e.to_string())?;
    let models_slice: &[String] = available_models.as_deref().unwrap_or(&[]);
    let stages = apply_recipe(&recipe, models_slice)
        .map_err(|e| format!("recipe {profile_id:?} v{version} is not applicable: {e}"))?;
    // CR-08 §15: stamp `last_used_at` so the Recently Used
    // tab surfaces this recipe at the top after the apply.
    // The stamp is best-effort: if the row vanished between
    // the `get` and now (e.g. concurrent delete), the apply
    // result is still returned; the UI just won't show it
    // in Recently Used.
    if let Err(e) = store.mark_last_used(profile_id) {
        eprintln!("recipe_apply: failed to stamp last_used_at for {profile_id:?}: {e}");
    }

    // CR-08 §23 / Slice F: emit the RecipeApplied event.
    // The event carries the Recipe identity (profile_id +
    // version) + the `available_models` slice the apply
    // round used.
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    let model_refs: Vec<&str> = models_slice.iter().map(|s| s.as_str()).collect();
    if let Err(e) = event_store.record_event(
        RecipeEventKind::RecipeApplied,
        Some(profile_id),
        Some(version),
        payload_recipe_applied("", &model_refs),
        &recipe_events_now_iso(),
    ) {
        eprintln!("recipe_apply: failed to record event for {profile_id:?} v{version}: {e}");
    }

    Ok(RecipeApplyResponse {
        profile_id: profile_id.to_string(),
        version: recipe.version,
        branch: recipe.branch.clone(),
        stages,
    })
}

/// Helper for `recipe_duplicate`: does a profile with the
/// given name + target_type already exist? Returns true on
/// UNIQUE-collision risk. Pure read; holds the store lock
/// for the duration of the call (caller already holds it).
fn profile_exists(store: &RecipeStore, name: &str, target_type: &str) -> bool {
    let profile_id = RecipeStore::profile_id_for(name, target_type);
    match store.list() {
        Ok(summaries) => summaries.iter().any(|s| s.profile_id == profile_id),
        Err(_) => false,
    }
}

/// `recipe_duplicate` — CR-08 §22.1: duplicate an existing Recipe
/// (any profile, any version) into a fresh, independently-named
/// profile. The new `name` is suffixed `" (Copy)"` and disambiguated
/// if a profile with that suffix already exists (sweep up to 99
/// copies, then a millisecond-precision timestamp suffix). The new
/// profile has no `parent_version` (a fresh lineage, not a
/// save-of-new-version).
pub fn recipe_duplicate(
    services: &Services,
    profile_id: &str,
    version: u32,
) -> Result<RecipeSummary, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let source = store.get(profile_id, version).map_err(|e| e.to_string())?;
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
    let new_profile_id = RecipeStore::profile_id_for(&owned.name, &owned.target_type);
    let next_version = store
        .next_version_for(&new_profile_id)
        .map_err(|e| e.to_string())?;
    owned.version = if next_version == 0 { 1 } else { next_version };
    owned.parent_version = None;
    store.save(&owned).map_err(|e| e.to_string())
}

/// `recipe_export` — CR-08 §19: export a Recipe (any profile, any
/// version; version=None exports the head) as its canonical JSON
/// payload. Emits the RecipeExported event (§23 Slice F) with the
/// stand-in "tauri://recipe_export" destination (the file dialog
/// lives in the UI layer).
pub fn recipe_export(
    services: &Services,
    profile_id: &str,
    version: Option<u32>,
) -> Result<String, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let recipe = match version {
        Some(v) => store.get(profile_id, v).map_err(|e| e.to_string())?,
        None => store.get_head(profile_id).map_err(|e| e.to_string())?,
    };
    let json = recipe.to_json().map_err(|e| e.to_string())?;
    let bytes = json.len() as u64;

    // CR-08 §23 / Slice F: emit the RecipeExported
    // event. The destination is unknown to this call
    // (the file dialog lives in the UI), so the payload
    // carries "tauri://recipe_export" as a stand-in
    // destination. The bytes field is the canonical
    // payload size so consumers can track export volume.
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    if let Err(e) = event_store.record_event(
        RecipeEventKind::RecipeExported,
        Some(profile_id),
        Some(recipe.version),
        payload_recipe_exported("tauri://recipe_export", bytes),
        &recipe_events_now_iso(),
    ) {
        eprintln!("recipe_export: failed to record event for {profile_id:?}: {e}");
    }

    Ok(json)
}

/// `recipe_import` — CR-08 §19: import a Recipe from its canonical
/// JSON payload. Runs `from_json_migrated` (v1 -> v2 migration, hard
/// error on unknown future schemas), validates name/target_type +
/// the §28 strict import gates (security + compatibility), then
/// RE-LINEAGES the recipe against the local store: the imported
/// recipe always lands as the next version of its (name,
/// target_type) profile, with `parent_version` pointing at the local
/// head (or None for a fresh profile). Foreign `version` /
/// `parent_version` are intentionally discarded.
pub fn recipe_import(services: &Services, json: &str) -> Result<RecipeSummary, String> {
    // CR-08 §23 / Slice F: emit RecipeImportStarted
    // before parsing so consumers see the attempt
    // even if the parse fails.
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    if let Err(e) = event_store.record_event(
        RecipeEventKind::RecipeImportStarted,
        None,
        None,
        payload_recipe_import_started("tauri://recipe_import"),
        &recipe_events_now_iso(),
    ) {
        eprintln!("recipe_import: failed to record ImportStarted: {e}");
    }
    drop(event_store);

    let mut recipe = Recipe::from_json_migrated(json).map_err(|e| e.to_string())?;
    if recipe.name.trim().is_empty() {
        return Err(format!(
            "validation: {}",
            "imported recipe has an empty name"
        ));
    }
    if recipe.target_type.trim().is_empty() {
        return Err(format!(
            "validation: {}",
            "imported recipe has an empty target_type"
        ));
    }
    // CR-08 §28 / Slice H: strict import validation.
    // Reject the import (before persist) when the
    // Recipe fails either the §22 security validator
    // (any violation is fatal at import time) or the
    // §20 compatibility validator (the imported Recipe
    // must be applicable on a vanilla local machine,
    // not just on the source machine).
    let security_report = validate_recipe_security(&recipe, &SPEC_CATALOG);
    if !security_report.violations.is_empty() {
        let kinds: Vec<String> = security_report
            .violations
            .iter()
            .map(|v| v.kind.clone())
            .collect();
        // CR-08 §23 / Slice F: surface the rejection
        // through the RecipeImportFailed event so the
        // audit log captures the reason. The store
        // does not persist anything (we reject before
        // `store.save`); the rejection is captured
        // through the event log only.
        let event_store = services.recipe_events.lock().map_err(lock_err)?;
        let _ = event_store.record_event(
            RecipeEventKind::RecipeImportFailed,
            None,
            None,
            payload_recipe_import_failed(&format!(
                "imported recipe failed security validation: {}",
                kinds.join(", ")
            )),
            &recipe_events_now_iso(),
        );
        return Err(format!(
            "validation: imported recipe failed security validation: {}",
            kinds.join(", ")
        ));
    }
    // §22 round 1 compatibility validator: the
    // imported Recipe must be applicable against
    // an empty `available_models` list (i.e. the
    // imported Recipe must not require any models
    // the source machine had but a fresh local
    // machine might not). A Recipe that fails this
    // gate is rejected at import time.
    let compat_result = validate_compatibility(&recipe, &[]);
    if !compat_result.is_compatible() {
        // CR-08 §23 / Slice F: surface the rejection
        // through the RecipeImportFailed event.
        let event_store = services.recipe_events.lock().map_err(lock_err)?;
        let _ = event_store.record_event(
            RecipeEventKind::RecipeImportFailed,
            None,
            None,
            payload_recipe_import_failed("imported recipe failed compatibility validation"),
            &recipe_events_now_iso(),
        );
        return Err(format!(
            "validation: {}",
            "imported recipe failed compatibility validation"
        ));
    }
    let store = services.recipes.lock().map_err(lock_err)?;
    let profile_id = RecipeStore::profile_id_for(&recipe.name, &recipe.target_type);
    let next_version = store
        .next_version_for(&profile_id)
        .map_err(|e| e.to_string())?;
    recipe.version = if next_version == 0 { 1 } else { next_version };
    recipe.parent_version = match store.get_head(&profile_id) {
        Ok(head) => Some(head.version),
        Err(_) => None,
    };
    let summary = match store.save(&recipe) {
        Ok(s) => s,
        Err(e) => {
            // CR-08 §23 / Slice F: emit RecipeImportFailed
            // when the persist step fails. The error
            // message travels in the payload.
            let event_store = services.recipe_events.lock().map_err(lock_err)?;
            let _ = event_store.record_event(
                RecipeEventKind::RecipeImportFailed,
                Some(&profile_id),
                Some(recipe.version),
                payload_recipe_import_failed(&e.to_string()),
                &recipe_events_now_iso(),
            );
            return Err(e.to_string());
        }
    };
    // CR-08 §15: flag the just-saved head row as imported so
    // the Imported tab can classify it. The flag lives on
    // every row of the profile, so a subsequent `recipe_save`
    // (user-edits the imported recipe) keeps the imported
    // classification.
    if let Err(e) = store.mark_imported(&profile_id) {
        eprintln!("recipe_import: failed to mark_imported for {profile_id:?}: {e}");
    }

    // CR-08 §23 / Slice F: emit RecipeImportCompleted
    // after the persist + classification steps succeed.
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    if let Err(e) = event_store.record_event(
        RecipeEventKind::RecipeImportCompleted,
        Some(&profile_id),
        Some(recipe.version),
        payload_recipe_import_completed(&recipe.name, &recipe.target_type, recipe.version),
        &recipe_events_now_iso(),
    ) {
        eprintln!("recipe_import: failed to record ImportCompleted for {profile_id:?}: {e}");
    }

    Ok(summary)
}

/// `recipe_save_from_pipeline_plan` — CR-08 §14: "Save Pipeline as
/// Recipe" UX. Loads a `PipelinePlan`, builds a [`Recipe`] from its
/// stages, and persists it via `RecipeStore::save`. Emits the
/// PipelineSavedAsRecipe event with `source = "plan"`.
pub fn recipe_save_from_pipeline_plan(
    services: &Services,
    plan_id: &str,
    name: &str,
    target_type: Option<&str>,
) -> Result<RecipeSummary, String> {
    // Load the plan (clone-then-release pattern so we
    // don't hold both locks at once).
    let plan = services
        .pipeline_plans
        .load_plan(plan_id)
        .map_err(|e| format!("failed to load pipeline plan: {e}"))?;
    // Build the Recipe from the loaded plan (pure function).
    let mut recipe = recipe_from_pipeline_plan(&plan, name, target_type);
    // Assign the next version BEFORE save (mirrors the
    // recipe_save IPC pattern).
    let profile_id = RecipeStore::profile_id_for(&recipe.name, &recipe.target_type);
    let version = {
        let store = services.recipes.lock().map_err(lock_err)?;
        store
            .next_version_for(&profile_id)
            .map_err(|e| format!("failed to compute next version: {e}"))?
    };
    recipe.version = version;
    let store = services.recipes.lock().map_err(lock_err)?;
    let summary = store.save(&recipe).map_err(|e| e.to_string())?;

    // CR-08 §23 / Slice F: emit the PipelineSavedAsRecipe
    // event with `source = "plan"` so consumers can
    // distinguish plan-side saves from stage-run-side saves.
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    if let Err(e) = event_store.record_event(
        RecipeEventKind::PipelineSavedAsRecipe,
        Some(&profile_id),
        Some(version),
        payload_pipeline_saved_as_recipe("plan", plan_id, name),
        &recipe_events_now_iso(),
    ) {
        eprintln!("recipe_save_from_pipeline_plan: failed to record event for {profile_id:?}: {e}");
    }

    Ok(summary)
}

/// `recipe_save_from_stage_runs` — CR-08 §22 round 2 / Slice B: "Save
/// Processing as Recipe" UX. Loads the `StageRunRecord`s for a
/// pipeline run, builds a [`Recipe`] from the terminal attempt per
/// stage, and persists it via `RecipeStore::save`. Emits the
/// PipelineSavedAsRecipe event with `source = "stage_runs"`.
pub fn recipe_save_from_stage_runs(
    services: &Services,
    run_id: &str,
    name: &str,
    target_type: Option<&str>,
) -> Result<RecipeSummary, String> {
    // Load the stage runs (clone-then-release pattern so
    // we don't hold the project lock while we hold the
    // recipe lock).
    let stage_runs = {
        let store = services
            .domain_store
            .lock()
            .map_err(|_| "project store mutex poisoned".to_string())?;
        store
            .list_stage_runs(run_id)
            .map_err(|e| format!("failed to list stage runs for {run_id}: {e}"))?
    };
    // Build the Recipe from the loaded stage runs
    // (pure function).
    let mut recipe = recipe_from_stage_runs(&stage_runs, run_id, name, target_type);
    // Assign the next version BEFORE save (mirrors the
    // recipe_save IPC + §14 sibling pattern).
    let profile_id = RecipeStore::profile_id_for(&recipe.name, &recipe.target_type);
    let version = {
        let store = services.recipes.lock().map_err(lock_err)?;
        store
            .next_version_for(&profile_id)
            .map_err(|e| format!("failed to compute next version: {e}"))?
    };
    recipe.version = version;
    let store = services.recipes.lock().map_err(lock_err)?;
    let summary = store.save(&recipe).map_err(|e| e.to_string())?;

    // CR-08 §23 / Slice F: emit the PipelineSavedAsRecipe
    // event with `source = "stage_runs"` so consumers can
    // distinguish execution-history-side saves from plan-side
    // saves.
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    if let Err(e) = event_store.record_event(
        RecipeEventKind::PipelineSavedAsRecipe,
        Some(&profile_id),
        Some(version),
        payload_pipeline_saved_as_recipe("stage_runs", run_id, name),
        &recipe_events_now_iso(),
    ) {
        eprintln!("recipe_save_from_stage_runs: failed to record event for {profile_id:?}: {e}");
    }

    Ok(summary)
}

/// `recipe_mark_as_system` — CR-08 §3.1: mark a Recipe profile as a
/// system Recipe. All existing versions of the profile get
/// `is_system = 1` in the on-disk column, after which `recipe_save`
/// and `recipe_delete` refuse to mutate the profile. Returns the
/// `profile_id`'s flag result so the caller can chain a UI refresh.
pub fn recipe_mark_as_system(services: &Services, profile_id: &str) -> Result<bool, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    store.mark_as_system(profile_id).map_err(|e| e.to_string())
}

/// `recipe_is_system` — CR-08 §3.1: read whether any version of a
/// profile is currently marked as a system Recipe. The UI uses this
/// to render the "System" badge + gate destructive actions in the
/// RecipesScreen toolbar.
pub fn recipe_is_system(services: &Services, profile_id: &str) -> Result<bool, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    store
        .is_system_profile(profile_id)
        .map_err(|e| e.to_string())
}

/// `recipe_archive` — CR-08 §22.4: archive a Recipe profile. Archived
/// profiles remain on disk but are hidden from the default
/// `recipe_list` results. Returns true when at least one row was
/// updated, false when the profile does not exist yet.
pub fn recipe_archive(services: &Services, profile_id: &str) -> Result<bool, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    store.archive_profile(profile_id).map_err(|e| e.to_string())
}

/// `recipe_unarchive` — CR-08 §22.4: unarchive a Recipe profile.
/// Returns true when at least one row was updated.
pub fn recipe_unarchive(services: &Services, profile_id: &str) -> Result<bool, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    store
        .unarchive_profile(profile_id)
        .map_err(|e| e.to_string())
}

/// `recipe_is_archived` — CR-08 §22.4: read whether any version of a
/// profile is currently archived. The UI uses this to render the
/// "Archived" badge + toggle in RecipesScreen.
pub fn recipe_is_archived(services: &Services, profile_id: &str) -> Result<bool, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    store
        .is_archived_profile(profile_id)
        .map_err(|e| e.to_string())
}

/// `recipe_delete` — CR-08 §22.2: delete a Recipe profile (every
/// version, every branch). Returns the number of rows deleted.
/// Deleting a profile that does not exist returns 0 (idempotent so
/// the UI can fire delete without a pre-check).
pub fn recipe_delete(services: &Services, profile_id: &str) -> Result<u32, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let deleted = store
        .delete_profile(profile_id)
        .map_err(|e| e.to_string())?;
    Ok(deleted as u32)
}

/// `recipe_ai_diff_summary` — CR-07 §32.6: compute the AI-diff
/// summary for two Recipes. Pure function: loads both Recipes, calls
/// `recipe_ai_diff_summary`. Errors if either Recipe is not found.
pub fn recipe_ai_diff_summary(
    services: &Services,
    profile_id_a: &str,
    version_a: u32,
    profile_id_b: &str,
    version_b: u32,
) -> Result<RecipeAiDiffSummary, String> {
    use astroforge_core::recipe::recipe_ai_diff_summary as ai_diff_fn;
    let store = services.recipes.lock().map_err(lock_err)?;
    let recipe_a = store
        .get(profile_id_a, version_a)
        .map_err(|e| e.to_string())?;
    let recipe_b = store
        .get(profile_id_b, version_b)
        .map_err(|e| e.to_string())?;
    Ok(ai_diff_fn(&recipe_a, &recipe_b))
}

/// `recipe_parameter_diff` — CR-08 §13: compute the mechanical
/// parameter diff for two Recipes. Pure function: loads both Recipes,
/// calls `recipe_parameter_diff`. Returns the per-stage Added /
/// Removed / Modified breakdown. Errors if either Recipe is not
/// found.
pub fn recipe_parameter_diff(
    services: &Services,
    profile_id_a: &str,
    version_a: u32,
    profile_id_b: &str,
    version_b: u32,
) -> Result<RecipeParameterDiff, String> {
    use astroforge_core::recipe::recipe_parameter_diff as param_diff_fn;
    let store = services.recipes.lock().map_err(lock_err)?;
    let recipe_a = store
        .get(profile_id_a, version_a)
        .map_err(|e| e.to_string())?;
    let recipe_b = store
        .get(profile_id_b, version_b)
        .map_err(|e| e.to_string())?;
    Ok(param_diff_fn(&recipe_a, &recipe_b))
}

/// `recipe_compare_versions` — CR-08 §22 round 1: combined comparison
/// of two Recipes. Thin wrapper that loads both Recipes and
/// delegates to `recipe_compare_versions` (which folds
/// `recipe_ai_diff_summary` + `recipe_parameter_diff` into a single
/// `RecipeComparison` response). Errors if either Recipe is not
/// found.
pub fn recipe_compare_versions(
    services: &Services,
    profile_id_a: &str,
    version_a: u32,
    profile_id_b: &str,
    version_b: u32,
) -> Result<RecipeComparison, String> {
    use astroforge_core::recipe::recipe_compare_versions as compare_fn;
    let store = services.recipes.lock().map_err(lock_err)?;
    let recipe_a = store
        .get(profile_id_a, version_a)
        .map_err(|e| e.to_string())?;
    let recipe_b = store
        .get(profile_id_b, version_b)
        .map_err(|e| e.to_string())?;
    Ok(compare_fn(&recipe_a, &recipe_b))
}

/// `recipe_get_provenance` — CR-08 §22 round 1: Recipe provenance.
/// Returns a `RecipeProvenance` describing the Recipe itself
/// (identity, lineage_steps, perceptual_models, required_models,
/// etc). Computes `profile_id` is already supplied; forwards to
/// `recipe_provenance` so the core function stays pure. Errors if
/// the Recipe is not found.
pub fn recipe_get_provenance(
    services: &Services,
    profile_id: &str,
    version: u32,
) -> Result<RecipeProvenance, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let recipe = store.get(profile_id, version).map_err(|e| e.to_string())?;
    Ok(recipe_provenance(profile_id, &recipe))
}

/// `recipe_preview` — CR-08 §22 round 2 / Slice A: read-only preview
/// of a Recipe. The read-only sibling of `recipe_apply` (which
/// mutates `last_used_at` and filters to enabled stages);
/// `preview_recipe` returns the full preview surface (all stages +
/// metadata + provenance + applicability + warnings) even when the
/// Recipe is not applicable so the UI can show WHY. Errors if the
/// Recipe is not found.
pub fn recipe_preview(
    services: &Services,
    profile_id: &str,
    version: u32,
    available_models: Option<Vec<String>>,
) -> Result<RecipePreviewResponse, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let recipe = store.get(profile_id, version).map_err(|e| e.to_string())?;
    let models_slice: &[String] = available_models.as_deref().unwrap_or(&[]);
    Ok(preview_recipe(profile_id, &recipe, models_slice))
}

/// `recipe_check_applicability` — CR-08 §22 round 2 / Slice C: full
/// §12 6-variant applicability matrix. Reads a Recipe via the
/// existing `RecipeStore::get`, evaluates it against the caller's
/// `ApplicabilityMatrix`, and returns the complete
/// `ApplicabilityReport`. Emits the RecipeApplicabilityEvaluated
/// event (§23 Slice F).
pub fn recipe_check_applicability(
    services: &Services,
    profile_id: &str,
    version: u32,
    matrix: ApplicabilityMatrix,
) -> Result<astroforge_core::recipe::ApplicabilityReport, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let recipe = store.get(profile_id, version).map_err(|e| e.to_string())?;
    let report = check_recipe_applicability(&recipe, &matrix);

    // CR-08 §23 / Slice F: emit the RecipeApplicabilityEvaluated
    // event. The payload carries the verdict label (one of
    // the 6 §12 outcome labels) + the warnings list so the
    // consumer can reconstruct the per-dimension outcome.
    //
    // CR-08 §28 / Slice I: pre-existing bug fix. The
    // field is `report.verdict` (the struct's
    // `ApplicabilityReport.verdict` is the
    // top-level 6-variant `ApplicabilityOutcome`),
    // not `report.outcome` (which is on
    // `ApplicabilityDimension` and carries the
    // per-dimension `DimensionOutcome`).
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    let verdict_label = match &report.verdict {
        ApplicabilityOutcome::Compatible => "compatible",
        ApplicabilityOutcome::Adaptable => "adaptable",
        ApplicabilityOutcome::PartiallyCompatible => "partially_compatible",
        ApplicabilityOutcome::Incompatible => "incompatible",
        ApplicabilityOutcome::MissingModels(_) => "missing_models",
        ApplicabilityOutcome::SchemaMismatch(_) => "schema_mismatch",
    };
    let warn_refs: Vec<&str> = report.warnings.iter().map(|s| s.as_str()).collect();
    if let Err(e) = event_store.record_event(
        RecipeEventKind::RecipeApplicabilityEvaluated,
        Some(profile_id),
        Some(version),
        payload_recipe_applicability_evaluated(verdict_label, &warn_refs),
        &recipe_events_now_iso(),
    ) {
        eprintln!("recipe_check_applicability: failed to record event for {profile_id:?}: {e}");
    }

    Ok(report)
}

/// `recipe_adapt` — CR-08 §22 + §28 / Slice I: derive an adapted
/// Recipe for the user to accept or reject. Loads the Recipe by
/// `(profile_id, version)`, runs the §22 applicability matrix against
/// the caller's session criteria, and returns the proposed adapted
/// Recipe + the adaptive parameters + the reason. Emits
/// `RecipeAdaptationProposed` once the proposed Recipe is computed.
/// For the hard-blocker verdicts (`Incompatible` / `MissingModels` /
/// `SchemaMismatch`) the response surfaces `is_noop = true` +
/// `proposed = original` + a "adaptation refused: <reason>" reason.
pub fn recipe_adapt(
    services: &Services,
    profile_id: &str,
    version: u32,
    matrix: ApplicabilityMatrix,
    image_metrics: Option<ImageMetrics>,
) -> Result<RecipeAdaptationResponse, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let original = store.get(profile_id, version).map_err(|e| e.to_string())?;
    let report = check_recipe_applicability(&original, &matrix);
    let (proposed, adaptive_params, reason) =
        derive_adapted_recipe(&original, &report, image_metrics.as_ref());

    // CR-08 §23 / Slice F: emit
    // `RecipeAdaptationProposed` once the proposed
    // Recipe is computed. The payload carries the
    // proposed Recipe's `version` (as a JSON
    // number in the `params` slot per Slice F's
    // `payload_recipe_adaptation_proposed` shape)
    // + the reason in `stage_id` so the consumer
    // can render the proposal banner verbatim.
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    if let Err(e) = event_store.record_event(
        RecipeEventKind::RecipeAdaptationProposed,
        Some(profile_id),
        Some(proposed.version),
        payload_recipe_adaptation_proposed(&reason, serde_json::json!(proposed.version)),
        &recipe_events_now_iso(),
    ) {
        eprintln!("recipe_adapt: failed to record event for {profile_id:?}: {e}");
    }

    let is_noop = matches!(
        report.verdict,
        ApplicabilityOutcome::Compatible
            | ApplicabilityOutcome::Incompatible
            | ApplicabilityOutcome::MissingModels(_)
            | ApplicabilityOutcome::SchemaMismatch(_)
    );
    Ok(RecipeAdaptationResponse {
        verdict_label: report.verdict.label().to_string(),
        is_noop,
        original,
        proposed,
        reason,
        adaptive_params,
    })
}

/// `recipe_accept_adaptation` — CR-08 §22 + §28 / Slice I: accept an
/// adapted Recipe proposal. The caller sends the `proposed` Recipe
/// from the `recipe_adapt` response; this saves it as the next
/// version of the profile and emits `RecipeAdaptationAccepted` after
/// the persist step succeeds. Rejection is implicit: the caller
/// simply does not invoke this function.
pub fn recipe_accept_adaptation(
    services: &Services,
    proposed: Recipe,
) -> Result<RecipeSummary, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let summary = store.save(&proposed).map_err(|e| e.to_string())?;

    // CR-08 §23 / Slice F: emit
    // `RecipeAdaptationAccepted` once the persist
    // step succeeds. The payload carries the new
    // `version` (as a JSON number in the `params`
    // slot per Slice F's
    // `payload_recipe_adaptation_accepted` shape)
    // + the profile_id in `stage_id` so the
    // consumer can correlate the event with the
    // proposal.
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    if let Err(e) = event_store.record_event(
        RecipeEventKind::RecipeAdaptationAccepted,
        Some(&summary.profile_id),
        Some(summary.version),
        payload_recipe_adaptation_accepted(&summary.profile_id, serde_json::json!(summary.version)),
        &recipe_events_now_iso(),
    ) {
        eprintln!(
            "recipe_accept_adaptation: failed to record event for {:?}: {e}",
            summary.profile_id
        );
    }

    Ok(summary)
}

/// `recipe_get_reproducibility_report` — CR-08 §22 round 2 / Slice D:
/// full §6 reproducibility report for an ImageVersion. Walks the
/// ImageVersion -> Artifact -> PipelineRun -> StageRunRecord ->
/// AiOperation chain via the DomainStore + `ResourceSnapshot::detect()`
/// for hardware, and returns the complete `ReproducibilityRecord`.
/// Returns the `Indeterminate` verdict when the ImageVersion cannot
/// be found OR when the §6 conditions cannot be evaluated.
pub fn recipe_get_reproducibility_report(
    services: &Services,
    version_id: &str,
) -> Result<ReproducibilityRecord, String> {
    use astroforge_core::reproducibility::{summarize_reproducibility, HardwareSummary};
    use astroforge_core::resource::ResourceSnapshot;

    // Scope the domain lock so we don't hold it
    // while the aggregator runs (the aggregator
    // does not need the lock; only the IO lookups
    // do). The Recipe lookup is intentionally
    // best-effort: the ImageVersion carries a
    // `recipe_id` (profile_id), but the
    // RecipeStore.get requires (profile_id,
    // version) and the ImageVersion does not
    // carry the Recipe version. For Slice D the
    // aggregator surfaces None Recipe as Unknown
    // in the §6 recipe dimension: the honest
    // surface. A follow-on slice can extend the
    // Recipe resolution by persisting the Recipe
    // version on the ImageVersion.
    let (image_version, pipeline_run, ai_ops) = {
        let store = services.domain_store.lock().map_err(lock_err)?;
        let image_version = store
            .get_image_version(version_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("ImageVersion '{version_id}' not found"))?;

        // Walk the primary artifact to the
        // PipelineRun.
        let pipeline_run = match store.get_artifact(&image_version.primary_artifact_id) {
            Ok(artifact) => match artifact.pipeline_run_id {
                Some(run_id) => store.get_pipeline_run(&run_id).ok(),
                None => None,
            },
            Err(_) => None,
        };

        // Walk the PipelineRun -> StageRunRecord
        // -> AiOperation chain. Empty list when
        // the PipelineRun is unknown or has no
        // recorded stage runs.
        let mut ai_ops: Vec<astroforge_core::domain::AiOperation> = Vec::new();
        if let Some(run) = &pipeline_run {
            if let Ok(stage_runs) = store.list_stage_runs(&run.run_id) {
                for sr in stage_runs {
                    if let Ok(ops) = store.list_ai_operations_for_stage(&sr.stage_run_id) {
                        ai_ops.extend(ops);
                    }
                }
            }
        }

        (image_version, pipeline_run, ai_ops)
    };

    // Hardware summary from ResourceSnapshot::detect().
    let snap = ResourceSnapshot::detect();
    let hw = HardwareSummary::from_snapshot(&snap);

    let record = summarize_reproducibility(
        &image_version,
        pipeline_run.as_ref(),
        None,
        &ai_ops,
        Some(&hw),
    );

    // CR-08 §23 / Slice F: emit the ReproducibilityRecordCreated
    // event. The payload carries the verdict label (one of
    // the 3 §6 ReproducibilityVerdict labels) + the version_id.
    let event_store = services.recipe_events.lock().map_err(lock_err)?;
    let verdict_label = match record.verdict {
        astroforge_core::reproducibility::ReproducibilityVerdict::Exact => "exact",
        astroforge_core::reproducibility::ReproducibilityVerdict::Material => "material",
        astroforge_core::reproducibility::ReproducibilityVerdict::Indeterminate => "indeterminate",
    };
    if let Err(e) = event_store.record_event(
        RecipeEventKind::ReproducibilityRecordCreated,
        None,
        None,
        payload_reproducibility_record_created(version_id, verdict_label),
        &recipe_events_now_iso(),
    ) {
        eprintln!(
            "recipe_get_reproducibility_report: failed to record event for {version_id:?}: {e}"
        );
    }

    Ok(record)
}

/// `recipe_list_events` — CR-08 §23 / Slice F: list Recipe lifecycle
/// events. The list reads the append-only `recipe_events` table with
/// optional filters (profile_id, kind, since timestamp, limit).
/// Returns rows newest-first (highest event_id first). Read-only:
/// emitting events is the job of the write-side functions.
pub fn recipe_list_events(
    services: &Services,
    filter: RecipeEventFilter,
) -> Result<Vec<RecipeEvent>, String> {
    let store = services.recipe_events.lock().map_err(lock_err)?;
    store.list_events(&filter).map_err(|e| e.to_string())
}

/// `recipe_security_validate` — CR-08 §20: validate a Recipe against
/// the §20 stage-spec table for the five risk classes
/// (range, dependency, filesystem, executable, resource). Pure
/// function on top of `validation::validate_recipe_security` + the
/// canonical `SPEC_CATALOG` constant. Errors if the Recipe is not
/// found.
pub fn recipe_security_validate(
    services: &Services,
    profile_id: &str,
    version: u32,
) -> Result<SecurityValidationReport, String> {
    let store = services.recipes.lock().map_err(lock_err)?;
    let recipe = store.get(profile_id, version).map_err(|e| e.to_string())?;
    Ok(validate_recipe_security(&recipe, &SPEC_CATALOG))
}
