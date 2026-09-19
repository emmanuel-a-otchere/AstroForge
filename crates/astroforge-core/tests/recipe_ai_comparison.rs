//! CR-07 §32.4: AI comparison integration tests.
//!
//! Integration-level coverage of `astroforge_core::recipe`'s
//! AI comparison surface from outside the lib. The unit tests
//! in `recipe.rs` cover the function directly; this file
//! covers the public API surface that the comparison UI will
//! consume, end-to-end via the public types:
//!
//! - `Recipe::pipeline_plan_hash()` produces a stable,
//!   content-addressed 64-char lowercase hex SHA-256 hash.
//! - `recipe_ai_diff_summary(a, b)` surfaces the audit's
//!   "model / version / hash / classification / parameters /
//   provenance" comparison fields.
//!
//! These tests deliberately avoid the in-memory DomainStore;
//! the surface under test is purely the Recipe data model +
//! its comparison functions.

use astroforge_core::recipe::{
    recipe_ai_diff_summary, ModelType, QualityProfile, Recipe, RecipeAiDiffSummary,
};

// ─── helpers ────────────────────────────────────────────────

fn natural_recipe() -> Recipe {
    let mut r = Recipe::new("m42-natural", "stretch");
    r.add_stage(
        "stretch",
        [("amount".to_string(), serde_json::json!(0.05))]
            .into_iter()
            .collect(),
    );
    r
}

fn ai_recipe() -> Recipe {
    let mut r = Recipe::new("m42-ai", "stretch");
    r.add_stage(
        "stretch",
        [("amount".to_string(), serde_json::json!(0.05))]
            .into_iter()
            .collect(),
    );
    r.add_model("neural-denoiser", ModelType::Perceptual);
    r
}

// ─── §32.4.1: pipeline_plan_hash public surface ─────────────

#[test]
fn pipeline_plan_hash_is_64_char_lowercase_hex_sha256() {
    let r = natural_recipe();
    let h = r.pipeline_plan_hash();
    assert_eq!(h.len(), 64, "SHA-256 hex must be 64 chars");
    for c in h.chars() {
        assert!(
            c.is_ascii_digit() || (c.is_ascii_alphabetic() && c.is_ascii_lowercase()),
            "hash char {:?} must be lowercase hex digit",
            c
        );
    }
}

#[test]
fn pipeline_plan_hash_is_deterministic() {
    let r = natural_recipe();
    let h1 = r.pipeline_plan_hash();
    let h2 = r.pipeline_plan_hash();
    let h3 = r.pipeline_plan_hash();
    assert_eq!(h1, h2);
    assert_eq!(h2, h3);
}

#[test]
fn pipeline_plan_hash_distinguishes_ai_from_natural_recipes() {
    let natural = natural_recipe();
    let ai = ai_recipe();
    assert_ne!(
        natural.pipeline_plan_hash(),
        ai.pipeline_plan_hash(),
        "natural vs AI Recipe must produce different hashes"
    );
}

#[test]
fn pipeline_plan_hash_distinguishes_different_param_values() {
    let mut a = natural_recipe();
    a.stages.clear();
    a.add_stage(
        "stretch",
        [("amount".to_string(), serde_json::json!(0.1))]
            .into_iter()
            .collect(),
    );
    let mut b = natural_recipe();
    b.stages.clear();
    b.add_stage(
        "stretch",
        [("amount".to_string(), serde_json::json!(0.9))]
            .into_iter()
            .collect(),
    );
    assert_ne!(a.pipeline_plan_hash(), b.pipeline_plan_hash());
}

#[test]
fn pipeline_plan_hash_ignores_presentational_fields() {
    let mut a = natural_recipe();
    a.description = "alpha".into();
    a.created_at = "2026-01-01T00:00:00Z".into();
    a.flags = vec!["flag1".into()];
    let mut b = natural_recipe();
    b.description = "beta (entirely different)".into();
    b.created_at = "2099-12-31T23:59:59Z".into();
    b.flags = vec!["flag1".into(), "flag2".into()];
    b.parent_version = Some(99);
    assert_eq!(
        a.pipeline_plan_hash(),
        b.pipeline_plan_hash(),
        "description / created_at / flags / parent_version must NOT influence the hash"
    );
}

#[test]
fn pipeline_plan_hash_serializes_for_json_round_trip() {
    // Confirm that two Recipes that serialise to the same
    // JSON produce the same hash. This pins the property
    // that the hash function agrees with `to_json`.
    let r1 = natural_recipe();
    let json1 = r1.to_json().unwrap();
    let r2: Recipe = serde_json::from_str(&json1).unwrap();
    let json2 = r2.to_json().unwrap();
    assert_eq!(json1, json2);
    assert_eq!(r1.pipeline_plan_hash(), r2.pipeline_plan_hash());
}

// ─── §32.4.2: recipe_ai_diff_summary public surface ─────────

#[test]
fn ai_diff_summary_natural_vs_ai_classification_differs() {
    let natural = natural_recipe();
    let ai = ai_recipe();
    let s = recipe_ai_diff_summary(&natural, &ai);
    assert!(!s.ai_used_a);
    assert!(s.ai_used_b);
    assert!(
        s.ai_classification_differs,
        "natural-vs-AI must surface as classification-differs"
    );
    assert_eq!(s.perceptual_models_b, vec!["neural-denoiser"]);
    assert!(s.perceptual_models_a.is_empty());
}

#[test]
fn ai_diff_summary_two_natural_recipes_no_classification_diff() {
    let a = natural_recipe();
    let b = natural_recipe();
    let s = recipe_ai_diff_summary(&a, &b);
    assert!(!s.ai_classification_differs);
    assert!(!s.ai_used_a);
    assert!(!s.ai_used_b);
    assert!(!s.hash_differs);
    assert_eq!(s.hash_a, s.hash_b);
}

#[test]
fn ai_diff_summary_two_ai_recipes_no_classification_diff() {
    let a = ai_recipe();
    let b = ai_recipe();
    let s = recipe_ai_diff_summary(&a, &b);
    assert!(!s.ai_classification_differs);
    assert!(s.ai_used_a && s.ai_used_b);
    assert!(!s.hash_differs);
    assert_eq!(s.hash_a, s.hash_b);
    assert_eq!(s.perceptual_models_a, s.perceptual_models_b);
}

#[test]
fn ai_diff_summary_surfaces_version_numbers() {
    let mut a = natural_recipe();
    a.version = 7;
    let mut b = ai_recipe();
    b.version = 12;
    let s = recipe_ai_diff_summary(&a, &b);
    assert_eq!(s.version_a, 7);
    assert_eq!(s.version_b, 12);
}

#[test]
fn ai_diff_summary_surfaces_schema_versions() {
    let mut a = natural_recipe();
    a.schema_version = "2.0".into();
    let mut b = natural_recipe();
    b.schema_version = "1.5".into();
    let s = recipe_ai_diff_summary(&a, &b);
    assert_eq!(s.schema_version_a, "2.0");
    assert_eq!(s.schema_version_b, "1.5");
    assert!(
        s.hash_differs,
        "different schema versions must produce different hashes"
    );
}

#[test]
fn ai_diff_summary_surfaces_quality_profile() {
    let mut a = natural_recipe();
    a.quality_profile = QualityProfile::Natural;
    let mut b = natural_recipe();
    b.quality_profile = QualityProfile::Publication;
    let s = recipe_ai_diff_summary(&a, &b);
    assert_eq!(s.quality_profile_a, QualityProfile::Natural);
    assert_eq!(s.quality_profile_b, QualityProfile::Publication);
}

#[test]
fn ai_diff_summary_surfaces_required_models_difference() {
    let natural = natural_recipe();
    let mut ai = ai_recipe();
    ai.required_models.push("extra-required-model".into());
    let s = recipe_ai_diff_summary(&natural, &ai);
    assert!(s.required_models_differ);
}

#[test]
fn ai_diff_summary_provenance_line_is_human_readable() {
    let mut a = natural_recipe();
    a.version = 3;
    let mut b = ai_recipe();
    b.version = 5;
    let s = recipe_ai_diff_summary(&a, &b);
    assert!(
        s.provenance.contains("v3"),
        "provenance must mention version A: {}",
        s.provenance
    );
    assert!(
        s.provenance.contains("v5"),
        "provenance must mention version B: {}",
        s.provenance
    );
    assert!(
        s.provenance.contains("natural") || s.provenance.contains("ai"),
        "provenance must classify the recipes"
    );
}

#[test]
fn ai_diff_summary_is_pure_function_of_inputs() {
    // Pin purity: calling recipe_ai_diff_summary on the
    // same pair twice produces identical output.
    let a = natural_recipe();
    let b = ai_recipe();
    let s1 = recipe_ai_diff_summary(&a, &b);
    let s2 = recipe_ai_diff_summary(&a, &b);
    assert_eq!(s1, s2);
}

#[test]
fn ai_diff_summary_surfaces_hash_difference_for_param_change() {
    let mut a = natural_recipe();
    a.stages.clear();
    a.add_stage(
        "stretch",
        [("amount".to_string(), serde_json::json!(0.1))]
            .into_iter()
            .collect(),
    );
    let mut b = natural_recipe();
    b.stages.clear();
    b.add_stage(
        "stretch",
        [("amount".to_string(), serde_json::json!(0.9))]
            .into_iter()
            .collect(),
    );
    let s = recipe_ai_diff_summary(&a, &b);
    assert!(
        s.hash_differs,
        "different param values must produce different hashes"
    );
    assert_ne!(s.hash_a, s.hash_b);
}

// ─── §32.4.3: RecipeAiDiffSummary serialisation surface ────

#[test]
fn recipe_ai_diff_summary_serde_round_trip() {
    let a = natural_recipe();
    let b = ai_recipe();
    let s: RecipeAiDiffSummary = recipe_ai_diff_summary(&a, &b);
    let json = serde_json::to_string(&s).unwrap();
    let back: RecipeAiDiffSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(s, back);
}

#[test]
fn recipe_ai_diff_summary_serde_includes_all_audit_fields() {
    // Pins that the serialised JSON includes every field the
    // audit calls out: model / version / hash / classification /
    // parameters / provenance.
    let a = natural_recipe();
    let b = ai_recipe();
    let s = recipe_ai_diff_summary(&a, &b);
    let json = serde_json::to_string(&s).unwrap();
    // Model
    assert!(json.contains("perceptual_models_a"));
    assert!(json.contains("perceptual_models_b"));
    // Version
    assert!(json.contains("version_a"));
    assert!(json.contains("version_b"));
    assert!(json.contains("schema_version_a"));
    assert!(json.contains("schema_version_b"));
    // Hash
    assert!(json.contains("hash_a"));
    assert!(json.contains("hash_b"));
    assert!(json.contains("hash_differs"));
    // Classification
    assert!(json.contains("ai_used_a"));
    assert!(json.contains("ai_used_b"));
    assert!(json.contains("ai_classification_differs"));
    assert!(json.contains("quality_profile_a"));
    assert!(json.contains("quality_profile_b"));
    // Parameters (per-stage params feed into hash_a/b, which
    // is the proxy the surface uses for param surfacing).
    // Provenance
    assert!(json.contains("provenance"));
    assert!(json.contains("required_models_differ"));
}
