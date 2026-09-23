//! CR-08 §22.3: recipe apply integration tests.
//!
//! Pinned: the public surface the `recipe_apply` IPC uses
//! in src-tauri/src/main.rs (the same primitives the
//! `recipe_pipeline_plan_hash` + `recipe_duplicate` IPCs
//! delegate to). The IPC itself lives in the binary-only
//! src-tauri crate, so these tests cover the contract the
//! IPC is built on:
//!
//! - Apply returns enabled stages in Recipe order, with
//!   the (stage_id, params) shape the IPC serializes.
//! - Disabled stages are skipped.
//! - Missing-models rejection surfaces as a hard error.
//! - Schema-version rejection surfaces as a hard error.
//! - Apply of a Recipe with no required models succeeds
//!   even when the available-models list is empty.
//! - Round-trip from save -> get -> apply -> per-stage
//!   params round-trip preserves the Recipe's pipeline
//!   shape.

use astroforge_core::recipe::{apply_recipe, IntegrityBadge, QualityProfile, Recipe};
use astroforge_core::recipe_store::RecipeStore;
use std::collections::HashMap;
use std::path::PathBuf;

fn sample_recipe(name: &str, target: &str) -> Recipe {
    let mut r = Recipe::new(name, target);
    let mut stretch_params = HashMap::new();
    stretch_params.insert("amount".to_string(), serde_json::json!(0.05));
    r.add_stage("stretch", stretch_params);
    let mut crop_params = HashMap::new();
    crop_params.insert("x".to_string(), serde_json::json!(0));
    crop_params.insert("y".to_string(), serde_json::json!(0));
    crop_params.insert("width".to_string(), serde_json::json!(512));
    crop_params.insert("height".to_string(), serde_json::json!(512));
    r.add_stage("crop", crop_params);
    r.required_models = vec!["m42-blur-v2".to_string()];
    r.integrity = IntegrityBadge {
        perceptual_models_used: false,
        deterministic_models_used: true,
        seed_recorded: false,
        models: vec![],
    };
    r.quality_profile = QualityProfile::Natural;
    r
}

fn in_memory_store() -> RecipeStore {
    RecipeStore::new(&PathBuf::from(":memory:")).expect("open in-memory recipe store")
}

#[test]
fn apply_returns_enabled_stages_in_recipe_order() {
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save");
    let loaded = store.get(&summary.profile_id, 1).expect("load");

    // Pass the full inventory; the recipe requires
    // `m42-blur-v2`, so we include it.
    let stages = apply_recipe(&loaded, &["m42-blur-v2".to_string()]).expect("apply succeeds");

    assert_eq!(stages.len(), 2, "both stages enabled");
    assert_eq!(stages[0].0, "stretch");
    assert_eq!(
        stages[0].1.get("amount").and_then(|v| v.as_f64()),
        Some(0.05),
        "stretch params round-trip"
    );
    assert_eq!(stages[1].0, "crop");
    assert_eq!(
        stages[1].1.get("width").and_then(|v| v.as_u64()),
        Some(512),
        "crop params round-trip"
    );
}

#[test]
fn apply_skips_disabled_stages() {
    let store = in_memory_store();
    let mut recipe = sample_recipe("m42-natural", "stretch");
    recipe.required_models.clear();
    recipe.stages[1].enabled = false;
    let summary = store.save(&recipe).expect("save");
    let loaded = store.get(&summary.profile_id, 1).expect("load");

    let stages = apply_recipe(&loaded, &[]).expect("apply succeeds");
    assert_eq!(stages.len(), 1, "disabled stage is skipped");
    assert_eq!(stages[0].0, "stretch");
}

#[test]
fn apply_rejects_missing_models() {
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save");
    let loaded = store.get(&summary.profile_id, 1).expect("load");

    let err = apply_recipe(&loaded, &[]).expect_err("missing models must surface as error");
    let msg = err.to_string();
    assert!(
        msg.contains("m42-blur-v2"),
        "missing-model error names the missing model: {msg}"
    );
}

#[test]
fn apply_accepts_empty_inventory_when_no_models_required() {
    let store = in_memory_store();
    let mut recipe = sample_recipe("m42-natural", "stretch");
    recipe.required_models.clear();
    let summary = store.save(&recipe).expect("save");
    let loaded = store.get(&summary.profile_id, 1).expect("load");

    let stages = apply_recipe(&loaded, &[]).expect("no required models -> OK");
    assert_eq!(stages.len(), 2);
}

#[test]
fn apply_rejects_unknown_future_schema() {
    let store = in_memory_store();
    let mut recipe = sample_recipe("m42-natural", "stretch");
    // Drop the required_models so the missing-models check
    // does not fire first; the schema-version guard is what
    // we want to pin here.
    recipe.required_models.clear();
    let summary = store.save(&recipe).expect("save");
    let mut loaded = store.get(&summary.profile_id, 1).expect("load");

    // Force the in-memory Recipe into a future schema. The
    // store rejects this at save time, so we mutate the
    // loaded value directly to pin the apply-side guard.
    loaded.schema_version = "99.0".to_string();

    let err = apply_recipe(&loaded, &[]).expect_err("future schema must surface as error");
    let msg = err.to_string();
    assert!(
        msg.contains("99.0") || msg.contains("Incompatible"),
        "schema-version error names the version: {msg}"
    );
}

#[test]
fn round_trip_save_get_apply_preserves_pipeline_shape() {
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save");
    let loaded = store.get(&summary.profile_id, 1).expect("load");
    let stages = apply_recipe(&loaded, &["m42-blur-v2".to_string()]).expect("apply");

    // The shape returned by apply (stage_id, params) must
    // equal the source's enabled stages in source order.
    // This pins the contract the IPC serializes:
    // `Vec<(String, HashMap<String, Value>)>` in
    // Recipe-order, only enabled stages.
    let expected: Vec<&str> = loaded
        .stages
        .iter()
        .filter(|s| s.enabled)
        .map(|s| s.stage_id.as_str())
        .collect();
    assert_eq!(
        stages.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
        expected,
        "apply returns enabled stages in Recipe order"
    );
    for ((_, got_params), source_stage) in stages
        .iter()
        .zip(loaded.stages.iter().filter(|s| s.enabled))
    {
        for k in source_stage.params.keys() {
            assert!(
                got_params.contains_key(k),
                "apply params include source key {k:?}"
            );
        }
    }
}

// ─── CR-08 §10.4 apply round integration tests ───
//
// The apply round now consults
// `effective_ai_enhancement_for_stage(stage_id)` and
// stamps the resolved AI Enhancement Level into the
// returned params hash under the `_ai_enhancement_level`
// key. The IPC serializes this key alongside the
// user-set params so the apply round can drive per-stage
// AI posture without a second round-trip.
//
// The contract: every enabled stage's returned params
// hash contains the resolved level as a string label
// ("Off" / "Conservative" / "Recommended" / "Advanced").
// The resolve order is per-stage override > recipe-level
// default; unknown stage IDs fall back to recipe-level.

use astroforge_core::recipe::AiEnhancementLevel;

fn recipe_with_ai_level(name: &str, target: &str, level: AiEnhancementLevel) -> Recipe {
    let mut r = sample_recipe(name, target);
    r.ai_enhancement_level = level;
    r
}

#[test]
fn apply_returns_recipe_level_ai_enhancement_for_each_enabled_stage() {
    // Recipe-level ai_enhancement_level = Recommended
    // (the serde default). Every enabled stage's
    // returned params hash carries
    // _ai_enhancement_level = "Recommended" because
    // there are no per-stage overrides.
    let recipe = recipe_with_ai_level("m42-natural", "stretch", AiEnhancementLevel::Recommended);
    let stages = apply_recipe(&recipe, &["m42-blur-v2".to_string()]).expect("apply");

    assert!(!stages.is_empty());
    for (stage_id, params) in &stages {
        let v = params
            .get("_ai_enhancement_level")
            .unwrap_or_else(|| panic!("stage {stage_id} must carry resolved AI level"));
        let s = v
            .as_str()
            .unwrap_or_else(|| panic!("AI level must serialize as string"));
        assert_eq!(
            s, "Recommended",
            "recipe-level default must surface as Recommended for stage {stage_id}"
        );
    }
}

#[test]
fn apply_returns_per_stage_override_when_set() {
    // Per-stage override wins over recipe-level.
    // Recipe level = Conservative; stretch override
    // = Advanced; crop has no override (falls back to
    // Conservative).
    let mut recipe = recipe_with_ai_level("m42-mix", "stretch", AiEnhancementLevel::Conservative);
    for stage in &mut recipe.stages {
        if stage.stage_id == "stretch" {
            stage.ai_enhancement_override = Some(AiEnhancementLevel::Advanced);
        }
    }
    let stages = apply_recipe(&recipe, &["m42-blur-v2".to_string()]).expect("apply");

    let by_id: HashMap<&str, &str> = stages
        .iter()
        .map(|(id, params)| {
            (
                id.as_str(),
                params
                    .get("_ai_enhancement_level")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            )
        })
        .collect();
    assert_eq!(by_id.get("stretch").copied(), Some("Advanced"));
    assert_eq!(by_id.get("crop").copied(), Some("Conservative"));
}

#[test]
fn apply_off_ai_level_serializes_as_off_label() {
    let recipe = recipe_with_ai_level("deterministic", "stretch", AiEnhancementLevel::Off);
    let stages = apply_recipe(&recipe, &["m42-blur-v2".to_string()]).expect("apply");
    for (stage_id, params) in &stages {
        assert_eq!(
            params.get("_ai_enhancement_level").and_then(|v| v.as_str()),
            Some("Off"),
            "stage {stage_id} must surface as Off"
        );
    }
}

#[test]
fn apply_preserves_user_set_params_alongside_resolved_level() {
    // Source stage params stay intact (the
    // _ai_enhancement_level key is additive, not
    // destructive). The round-trip contract still
    // holds: every source key is present in the
    // returned hash.
    let recipe = recipe_with_ai_level("m42-natural", "stretch", AiEnhancementLevel::Advanced);
    let stages = apply_recipe(&recipe, &["m42-blur-v2".to_string()]).expect("apply");

    for (stage_id, got_params) in &stages {
        let source_stage = recipe
            .stages
            .iter()
            .find(|s| &s.stage_id == stage_id)
            .expect("source stage present");
        for k in source_stage.params.keys() {
            assert!(
                got_params.contains_key(k),
                "apply preserves source key {k:?} for stage {stage_id}"
            );
        }
        assert!(
            got_params.contains_key("_ai_enhancement_level"),
            "apply adds resolved AI level for stage {stage_id}"
        );
    }
}
