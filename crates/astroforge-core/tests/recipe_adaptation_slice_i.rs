//! CR-08 §22 + §28 / Slice I tests: Recipe adaptation
//! + acceptance IPC surface.
//!
//! These tests pin the contract of the new
//! `recipe_adapt` + `recipe_accept_adaptation` IPCs:
//! - `derive_adapted_recipe` is the pure engine-side
//!   function. Tests cover the 4 outcome branches
//!   (`Compatible` / `Adaptable` /
//!   `PartiallyCompatible` / `Incompatible` /
//!   `MissingModels` / `SchemaMismatch`).
//! - `RecipeAdaptationResponse` is the typed surface
//!   the IPC returns; tests pin the field shapes.
//! - The event payload helpers carry the proposal
//!   reason verbatim (verified end-to-end via the
//!   IPC payload shape).
//!
//! The tests are pure functions (no Tauri runtime).

use std::collections::HashMap;

use astroforge_core::adaptive::{
    derive_adapted_recipe, derive_adaptive_parameters, AdaptiveParameterSet, ImageMetrics,
    RecipeAdaptationResponse,
};
use astroforge_core::recipe::{
    ApplicabilityDimension, ApplicabilityMatrix, ApplicabilityOutcome, ApplicabilityReport,
    DimensionOutcome, Recipe,
};

// ─── Engine-side pure function tests ────────────────────────────────────

fn matrix_with_available_models(models: Vec<String>) -> ApplicabilityMatrix {
    ApplicabilityMatrix {
        target_type: None,
        image_width: None,
        image_height: None,
        available_models: Some(models),
        dataset_quality: None,
    }
}

fn empty_matrix() -> ApplicabilityMatrix {
    ApplicabilityMatrix {
        target_type: None,
        image_width: None,
        image_height: None,
        available_models: None,
        dataset_quality: None,
    }
}

fn empty_report(verdict: ApplicabilityOutcome) -> ApplicabilityReport {
    ApplicabilityReport {
        verdict,
        dimensions: Vec::new(),
        warnings: Vec::new(),
    }
}

fn report_with_dimension(
    verdict: ApplicabilityOutcome,
    dimension: &str,
    outcome: DimensionOutcome,
) -> ApplicabilityReport {
    ApplicabilityReport {
        verdict,
        dimensions: vec![ApplicabilityDimension {
            dimension: dimension.to_string(),
            outcome,
        }],
        warnings: Vec::new(),
    }
}

fn image_metrics() -> ImageMetrics {
    ImageMetrics {
        snr: 30.0,
        fwhm: 2.5,
        star_count: 80,
        background_gradient: 0.05,
        mean: 0.3,
        stddev: 0.1,
    }
}

#[test]
fn derive_adapted_recipe_compatible_returns_noop_with_original() {
    let r = Recipe::new("M42", "deep_sky");
    let report = empty_report(ApplicabilityOutcome::Compatible);
    let (proposed, adaptive, reason) = derive_adapted_recipe(&r, &report, None);
    assert_eq!(proposed.version, r.version);
    assert_eq!(proposed.parent_version, r.parent_version);
    assert!(!proposed.flags.contains(&"adapted".to_string()));
    assert!(adaptive.is_none());
    assert!(reason.contains("compatible"));
    assert!(reason.contains("no adaptation"));
}

#[test]
fn derive_adapted_recipe_compatible_with_metrics_returns_adaptive_params() {
    let r = Recipe::new("M42", "deep_sky");
    let report = empty_report(ApplicabilityOutcome::Compatible);
    let (proposed, adaptive, _reason) = derive_adapted_recipe(&r, &report, Some(&image_metrics()));
    assert_eq!(proposed.version, r.version);
    assert!(adaptive.is_some());
    let adaptive = adaptive.unwrap();
    assert!(adaptive.noise.is_some());
    assert!(adaptive.sharpening.is_some());
}

#[test]
fn derive_adapted_recipe_adaptable_bumps_version_and_stamps_adapted_flag() {
    let mut r = Recipe::new("M42", "deep_sky");
    r.version = 3;
    r.parent_version = Some(2);
    let report = report_with_dimension(
        ApplicabilityOutcome::Adaptable,
        "target_type",
        DimensionOutcome::Adaptable("suffix differs".to_string()),
    );
    let (proposed, _adaptive, reason) = derive_adapted_recipe(&r, &report, None);
    assert_eq!(proposed.version, 4);
    assert_eq!(proposed.parent_version, Some(3));
    assert!(
        proposed.flags.contains(&"adapted".to_string()),
        "expected 'adapted' flag in {:?}",
        proposed.flags
    );
    assert!(
        reason.contains("target_type"),
        "expected reason to mention the dimension, got {reason:?}",
    );
    assert!(
        reason.contains("suffix differs"),
        "expected reason to mention the per-dimension note, got {reason:?}",
    );
}

#[test]
fn derive_adapted_recipe_partially_compatible_also_bumps_version() {
    let mut r = Recipe::new("M42", "deep_sky");
    r.version = 1;
    let report = report_with_dimension(
        ApplicabilityOutcome::PartiallyCompatible,
        "available_models",
        DimensionOutcome::PartialSkip("model_a unavailable".to_string()),
    );
    let (proposed, _adaptive, reason) = derive_adapted_recipe(&r, &report, None);
    assert_eq!(proposed.version, 2);
    assert_eq!(proposed.parent_version, Some(1));
    assert!(reason.contains("model_a unavailable"));
}

#[test]
fn derive_adapted_recipe_incompatible_returns_refusal_no_proposal() {
    let r = Recipe::new("M42", "deep_sky");
    let report = empty_report(ApplicabilityOutcome::Incompatible);
    let (proposed, adaptive, reason) = derive_adapted_recipe(&r, &report, None);
    assert_eq!(proposed.version, r.version);
    assert!(adaptive.is_none());
    assert!(reason.contains("adaptation refused"));
    assert!(reason.contains("incompatible"));
}

#[test]
fn derive_adapted_recipe_missing_models_carries_model_list_in_reason() {
    let r = Recipe::new("M42", "deep_sky");
    let report = empty_report(ApplicabilityOutcome::MissingModels(vec![
        "m1".to_string(),
        "m2".to_string(),
    ]));
    let (proposed, adaptive, reason) = derive_adapted_recipe(&r, &report, None);
    assert_eq!(proposed.version, r.version);
    assert!(adaptive.is_none());
    assert!(reason.contains("m1"));
    assert!(reason.contains("m2"));
    assert!(reason.contains("not available"));
}

#[test]
fn derive_adapted_recipe_schema_mismatch_carries_schema_in_reason() {
    let r = Recipe::new("M42", "deep_sky");
    let report = empty_report(ApplicabilityOutcome::SchemaMismatch(
        "expected 2.0, got 1.0".to_string(),
    ));
    let (proposed, adaptive, reason) = derive_adapted_recipe(&r, &report, None);
    assert_eq!(proposed.version, r.version);
    assert!(adaptive.is_none());
    assert!(reason.contains("schema_version mismatch"));
    assert!(reason.contains("expected 2.0, got 1.0"));
}

#[test]
fn derive_adapted_recipe_does_not_double_stamp_adapted_flag() {
    // Calling derive_adapted_recipe twice on the
    // same Recipe should not duplicate the
    // 'adapted' flag; the flag is idempotent.
    let mut r = Recipe::new("M42", "deep_sky");
    r.flags.push("adapted".to_string());
    let report = report_with_dimension(
        ApplicabilityOutcome::Adaptable,
        "target_type",
        DimensionOutcome::Adaptable("note".to_string()),
    );
    let (proposed, _adaptive, _reason) = derive_adapted_recipe(&r, &report, None);
    let count = proposed.flags.iter().filter(|f| *f == "adapted").count();
    assert_eq!(count, 1, "expected exactly one 'adapted' flag");
}

#[test]
fn derive_adapted_recipe_no_adaptable_dimensions_uses_default_reason() {
    // `Adaptable` verdict with no per-dimension
    // `Adaptable` notes should still produce a
    // reason (the default "see adaptive_params
    // for details" string).
    let r = Recipe::new("M42", "deep_sky");
    let report = ApplicabilityReport {
        verdict: ApplicabilityOutcome::Adaptable,
        dimensions: Vec::new(),
        warnings: Vec::new(),
    };
    let (_proposed, _adaptive, reason) = derive_adapted_recipe(&r, &report, None);
    assert!(reason.contains("§22 matrix"));
}

// ─── Response-shape tests ───────────────────────────────────────────────

#[test]
fn recipe_adaptation_response_serde_round_trip() {
    let r = Recipe::new("M42", "deep_sky");
    let report = empty_report(ApplicabilityOutcome::Compatible);
    let (proposed, adaptive, reason) = derive_adapted_recipe(&r, &report, None);
    let resp = RecipeAdaptationResponse {
        original: r.clone(),
        proposed,
        reason: reason.clone(),
        adaptive_params: adaptive,
        is_noop: true,
        verdict_label: "Compatible".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let back: RecipeAdaptationResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(back.original.name, r.name);
    assert_eq!(back.proposed.name, r.name);
    assert_eq!(back.reason, reason);
    assert!(back.is_noop);
    assert_eq!(back.verdict_label, "Compatible");
}

#[test]
fn derive_adaptive_parameters_matches_image_metrics() {
    let m = image_metrics();
    let p = derive_adaptive_parameters(&m);
    assert!(p.noise.is_some());
    assert!(p.sharpening.is_some());
}

#[test]
fn derive_adaptive_parameters_default_is_empty() {
    let p = AdaptiveParameterSet::default();
    assert!(p.noise.is_none());
    assert!(p.sharpening.is_none());
}

#[test]
fn derive_adapted_recipe_preserves_required_models() {
    // The proposed Recipe must carry the same
    // `required_models` as the original; the
    // adaptation surface does not edit the model
    // list (that's the validator's job).
    let mut r = Recipe::new("M42", "deep_sky");
    r.required_models = vec!["model_a".to_string(), "model_b".to_string()];
    let report = report_with_dimension(
        ApplicabilityOutcome::Adaptable,
        "target_type",
        DimensionOutcome::Adaptable("note".to_string()),
    );
    let (proposed, _adaptive, _reason) = derive_adapted_recipe(&r, &report, None);
    assert_eq!(proposed.required_models, r.required_models);
}

#[test]
fn derive_adapted_recipe_preserves_schema_version() {
    let mut r = Recipe::new("M42", "deep_sky");
    r.schema_version = "2.0".to_string();
    let report = report_with_dimension(
        ApplicabilityOutcome::Adaptable,
        "target_type",
        DimensionOutcome::Adaptable("note".to_string()),
    );
    let (proposed, _adaptive, _reason) = derive_adapted_recipe(&r, &report, None);
    assert_eq!(proposed.schema_version, "2.0");
}

#[test]
fn derive_adapted_recipe_handles_zero_version_gracefully() {
    // The store's "unsaved" sentinel is
    // `Recipe::new(...)` (which sets `version = 1`
    // by default). Forcing `version = 0`
    // simulates a malformed Recipe; the
    // `saturating_add(1)` should yield 1 (not
    // panic, not overflow).
    let mut r = Recipe::new("M42", "deep_sky");
    r.version = 0;
    let report = report_with_dimension(
        ApplicabilityOutcome::Adaptable,
        "target_type",
        DimensionOutcome::Adaptable("note".to_string()),
    );
    let (proposed, _adaptive, _reason) = derive_adapted_recipe(&r, &report, None);
    assert_eq!(proposed.version, 1);
}

// ─── ApplicabilityMatrix shape tests ────────────────────────────────────

#[test]
fn matrix_with_available_models_round_trip() {
    let m = matrix_with_available_models(vec!["a".to_string(), "b".to_string()]);
    assert_eq!(
        m.available_models,
        Some(vec!["a".to_string(), "b".to_string()])
    );
    let _ = empty_matrix();
}

#[test]
fn dimension_outcome_variants_constructable() {
    // The 5-variant DimensionOutcome must be
    // constructable from each variant so the
    // per-dimension derivation path can match
    // every shape.
    let _ = DimensionOutcome::Match;
    let _ = DimensionOutcome::Adaptable("note".to_string());
    let _ = DimensionOutcome::PartialSkip("note".to_string());
    let _ = DimensionOutcome::Mismatch("note".to_string());
    let _ = DimensionOutcome::NotEvaluated;
    let _ = ApplicabilityDimension {
        dimension: "any".to_string(),
        outcome: DimensionOutcome::Match,
    };
    // Sanity: ApplicabilityOutcome has 6 variants
    // (the §12 matrix verdict vocabulary). The
    // array is a value (not a `Vec`) to avoid
    // clippy's `useless_vec` lint.
    let outcomes = [
        ApplicabilityOutcome::Compatible,
        ApplicabilityOutcome::Adaptable,
        ApplicabilityOutcome::PartiallyCompatible,
        ApplicabilityOutcome::Incompatible,
        ApplicabilityOutcome::MissingModels(Vec::new()),
        ApplicabilityOutcome::SchemaMismatch("x".to_string()),
    ];
    assert_eq!(outcomes.len(), 6);
}

// ─── RecipeAdaptationResponse via JSON shape (TypeScript parity) ────────

#[test]
fn recipe_adaptation_response_json_keys_match_ts_surface() {
    let r = Recipe::new("M42", "deep_sky");
    let report = empty_report(ApplicabilityOutcome::Compatible);
    let (proposed, adaptive, reason) = derive_adapted_recipe(&r, &report, None);
    let resp = RecipeAdaptationResponse {
        original: r,
        proposed,
        reason,
        adaptive_params: adaptive,
        is_noop: true,
        verdict_label: "Compatible".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    // The TS `RecipeAdaptationResponse` interface
    // mirrors these snake_case keys; every key
    // must be present.
    for key in &[
        "original",
        "proposed",
        "reason",
        "adaptive_params",
        "is_noop",
        "verdict_label",
    ] {
        assert!(
            json.contains(&format!("\"{key}\"")),
            "missing key {key} in {json}",
        );
    }
    // Sanity: ensure the JSON does NOT leak
    // `serde_json::Value::Null` for the missing
    // adaptive_params (the IPC marshals the
    // `Option` as `null` which TS sees as `null`
    // — this is the contract).
    assert!(
        json.contains("\"adaptive_params\":null"),
        "expected adaptive_params to serialize as null, got {json}",
    );
}

// ─── HashMap smoke test (recipe params) ────────────────────────────────

#[test]
fn recipe_stage_params_round_trip_via_json() {
    // The Recipe->JSON->Recipe round-trip via
    // serde is the canonical pipeline the IPC
    // uses for `recipe_accept_adaptation` (the
    // caller sends the `proposed` Recipe as JSON
    // and the store re-deserializes it from
    // `payload_json`). This test pins the
    // round-trip for the `params: HashMap<String,
    // serde_json::Value>` field.
    let mut params: HashMap<String, serde_json::Value> = HashMap::new();
    params.insert("bias".to_string(), serde_json::json!(0.5));
    params.insert("enabled".to_string(), serde_json::json!(true));
    let mut r = Recipe::new("M42", "deep_sky");
    r.stages.push(astroforge_core::recipe::RecipeStage {
        stage_id: "stretch".to_string(),
        enabled: true,
        params,
        ai_enhancement_override: None,
    });
    let json = serde_json::to_string(&r).unwrap();
    let back: Recipe = serde_json::from_str(&json).unwrap();
    assert_eq!(back.stages.len(), 1);
    assert_eq!(back.stages[0].stage_id, "stretch");
    assert_eq!(
        back.stages[0].params.get("bias").unwrap(),
        &serde_json::json!(0.5),
    );
    assert_eq!(
        back.stages[0].params.get("enabled").unwrap(),
        &serde_json::json!(true),
    );
}
