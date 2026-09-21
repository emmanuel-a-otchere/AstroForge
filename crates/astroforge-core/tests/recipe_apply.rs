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
