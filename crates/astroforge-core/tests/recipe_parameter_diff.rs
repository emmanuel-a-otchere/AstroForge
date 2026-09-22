//! CR-08 §13: Recipe parameter-diff integration tests.
//!
//! Pinning the contract of `recipe_parameter_diff` (the
//! mechanical side-by-side per-stage parameter diff).
//!
//! Coverage:
//! - Identical Recipes produce `identical = true` and
//!   three empty buckets.
//! - Pure Added stage (in B only) classifies its params as
//!   `Added` with `a = None`.
//! - Pure Removed stage (in A only) classifies its params as
//!   `Removed` with `b = None`.
//! - Modified stage: per-param Added / Removed / Changed /
//!   Unchanged classification against the union of keys.
//! - `enabled_differs` flag captures the side-channel that
//!   does not fit into the per-param diff.
//! - `identical` is `false` when only the enabled flag
//!   changes (params unchanged, but the stage itself
//!   differs semantically).
//! - Pure-stage-reordering is NOT detected (the diff is
//!   keyed by `stage_id`, not by position); two Recipes
//!   with the same stages in different order are
//!   `identical = true`. This is intentional: stage
//!   identity is the `stage_id`, not the list index.
//! - The serde round-trip preserves all fields, including
//!   the `ParamChange` enum tag.

use astroforge_core::recipe::{
    recipe_parameter_diff, IntegrityBadge, ParamChange, QualityProfile, Recipe,
};
use serde_json::json;
use std::collections::HashMap;

fn sample_recipe(name: &str, target: &str) -> Recipe {
    let mut r = Recipe::new(name, target);
    r.integrity = IntegrityBadge {
        perceptual_models_used: false,
        deterministic_models_used: true,
        seed_recorded: false,
        models: vec![],
    };
    r.quality_profile = QualityProfile::Natural;
    r
}

#[test]
fn identical_recipes_produce_identical_true_and_empty_buckets() {
    let mut a = sample_recipe("a", "stretch");
    let mut b = sample_recipe("b", "stretch");
    let mut p = HashMap::new();
    p.insert("gain".to_string(), json!(1.0));
    a.add_stage("stretch", p.clone());
    b.add_stage("stretch", p);

    let diff = recipe_parameter_diff(&a, &b);
    assert!(diff.identical, "expected identical");
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert_eq!(diff.modified.len(), 1);
    let m = &diff.modified[0];
    assert_eq!(m.stage_id, "stretch");
    assert!(!m.enabled_differs);
    assert!(
        m.params.iter().all(|p| p.change == ParamChange::Unchanged),
        "expected all params Unchanged, got {:?}",
        m.params
    );
}

#[test]
fn pure_added_stage_classifies_params_as_added() {
    let a = sample_recipe("a", "stretch");
    let mut b = sample_recipe("b", "stretch");
    let mut p = HashMap::new();
    p.insert("strength".to_string(), json!(0.8));
    b.add_stage("denoise", p);

    let diff = recipe_parameter_diff(&a, &b);
    assert!(!diff.identical);
    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.added[0].stage_id, "denoise");
    assert!(diff.added[0].enabled_a.is_none());
    assert_eq!(diff.added[0].enabled_b, Some(true));
    assert!(diff.added[0].enabled_differs);
    let entry = &diff.added[0].params[0];
    assert_eq!(entry.key, "strength");
    assert!(entry.a.is_none());
    assert_eq!(entry.b, Some(json!(0.8)));
    assert_eq!(entry.change, ParamChange::Added);
    assert!(diff.removed.is_empty());
    assert!(diff.modified.is_empty());
}

#[test]
fn pure_removed_stage_classifies_params_as_removed() {
    let mut a = sample_recipe("a", "stretch");
    let b = sample_recipe("b", "stretch");
    let mut p = HashMap::new();
    p.insert("strength".to_string(), json!(0.5));
    a.add_stage("denoise", p);

    let diff = recipe_parameter_diff(&a, &b);
    assert!(!diff.identical);
    assert_eq!(diff.removed.len(), 1);
    assert_eq!(diff.removed[0].stage_id, "denoise");
    assert_eq!(diff.removed[0].enabled_a, Some(true));
    assert!(diff.removed[0].enabled_b.is_none());
    assert!(diff.removed[0].enabled_differs);
    let entry = &diff.removed[0].params[0];
    assert_eq!(entry.key, "strength");
    assert_eq!(entry.a, Some(json!(0.5)));
    assert!(entry.b.is_none());
    assert_eq!(entry.change, ParamChange::Removed);
    assert!(diff.added.is_empty());
    assert!(diff.modified.is_empty());
}

#[test]
fn modified_stage_classifies_params_against_union_of_keys() {
    let mut a = sample_recipe("a", "stretch");
    let mut b = sample_recipe("b", "stretch");
    // A: stretch { gain: 1.0, algo: lanczos }
    let mut pa = HashMap::new();
    pa.insert("gain".to_string(), json!(1.0));
    pa.insert("algo".to_string(), json!("lanczos"));
    a.add_stage("stretch", pa);
    // B: stretch { gain: 1.5, radius: 3 }
    let mut pb = HashMap::new();
    pb.insert("gain".to_string(), json!(1.5));
    pb.insert("radius".to_string(), json!(3));
    b.add_stage("stretch", pb);

    let diff = recipe_parameter_diff(&a, &b);
    assert!(!diff.identical);
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert_eq!(diff.modified.len(), 1);
    let m = &diff.modified[0];
    assert_eq!(m.stage_id, "stretch");
    assert!(!m.enabled_differs);

    // gain: Changed
    let gain = m
        .params
        .iter()
        .find(|p| p.key == "gain")
        .expect("gain entry");
    assert_eq!(gain.change, ParamChange::Changed);
    assert_eq!(gain.a, Some(json!(1.0)));
    assert_eq!(gain.b, Some(json!(1.5)));

    // algo: Removed (A only)
    let algo = m
        .params
        .iter()
        .find(|p| p.key == "algo")
        .expect("algo entry");
    assert_eq!(algo.change, ParamChange::Removed);
    assert_eq!(algo.a, Some(json!("lanczos")));
    assert!(algo.b.is_none());

    // radius: Added (B only)
    let radius = m
        .params
        .iter()
        .find(|p| p.key == "radius")
        .expect("radius entry");
    assert_eq!(radius.change, ParamChange::Added);
    assert!(radius.a.is_none());
    assert_eq!(radius.b, Some(json!(3)));
}

#[test]
fn enabled_flag_change_marks_modified_even_with_identical_params() {
    let mut a = sample_recipe("a", "stretch");
    let mut b = sample_recipe("b", "stretch");
    let mut p = HashMap::new();
    p.insert("gain".to_string(), json!(1.0));
    let mut stage_a = astroforge_core::recipe::RecipeStage {
        stage_id: "stretch".to_string(),
        enabled: true,
        params: p.clone(),
    };
    let mut stage_b = astroforge_core::recipe::RecipeStage {
        stage_id: "stretch".to_string(),
        enabled: false,
        params: p,
    };
    a.stages.push(stage_a.clone());
    b.stages.push(stage_b.clone());
    // Touch vars to avoid unused-mut warnings if anything
    // changes above (intentionally build via push to keep
    // the enabled flag in scope).
    stage_a.enabled = true;
    stage_b.enabled = false;

    let diff = recipe_parameter_diff(&a, &b);
    assert!(
        !diff.identical,
        "enabled flag change must mark Recipes as different"
    );
    assert_eq!(diff.modified.len(), 1);
    assert!(diff.modified[0].enabled_differs);
    assert_eq!(diff.modified[0].enabled_a, Some(true));
    assert_eq!(diff.modified[0].enabled_b, Some(false));
}

#[test]
fn stage_reorder_is_treated_as_identical() {
    // §13 diff is keyed by stage_id, not list position.
    // Two Recipes with the same stages in different order
    // are semantically identical and the diff must not
    // flag them as removed + added.
    let mut a = sample_recipe("a", "stretch");
    let mut b = sample_recipe("b", "stretch");
    let mut p1 = HashMap::new();
    p1.insert("k".to_string(), json!(1));
    let mut p2 = HashMap::new();
    p2.insert("k".to_string(), json!(2));
    a.add_stage("first", p1.clone());
    a.add_stage("second", p2.clone());
    b.add_stage("second", p2);
    b.add_stage("first", p1);

    let diff = recipe_parameter_diff(&a, &b);
    assert!(diff.identical, "stage reorder should not flip identical");
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert_eq!(diff.modified.len(), 2);
}

#[test]
fn serde_round_trip_preserves_all_fields() {
    let mut a = sample_recipe("a", "stretch");
    let mut b = sample_recipe("b", "stretch");
    let mut pa = HashMap::new();
    pa.insert("gain".to_string(), json!(1.0));
    a.add_stage("stretch", pa);
    let mut pb = HashMap::new();
    pb.insert("gain".to_string(), json!(1.5));
    pb.insert("radius".to_string(), json!(3));
    b.add_stage("stretch", pb);
    b.add_stage("denoise", HashMap::new());

    let diff = recipe_parameter_diff(&a, &b);
    let json = serde_json::to_string(&diff).expect("serialize");
    let back: astroforge_core::recipe::RecipeParameterDiff =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        diff, back,
        "round-trip should preserve every field, including the enum tag"
    );

    // Wire shape: the snake_case keys match the TS
    // `RecipeParameterDiffFromRust` mirror.
    assert!(json.contains("\"added\""));
    assert!(json.contains("\"removed\""));
    assert!(json.contains("\"modified\""));
    assert!(json.contains("\"identical\""));
    assert!(json.contains("\"enabled_a\""));
    assert!(json.contains("\"enabled_b\""));
    assert!(json.contains("\"enabled_differs\""));
}
