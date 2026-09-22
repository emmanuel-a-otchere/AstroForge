//! CR-08 §10.2: Recipe Guided tier tests.
//!
//! Pins the Guided tier half of CR-08 §10:
//! - `ProcessingObjective` enum (5 variants) with
//!   `label()` + `tag()` accessors + serde
//!   round-trip + snake-case wire tags.
//! - `QualityTargets` struct with optional fields
//!   that skip serialization when absent.
//! - `Recipe.processing_objectives`,
//!   `Recipe.quality_targets`,
//!   `Recipe.optional_operations` default to
//!   empty values for legacy Recipes.
//! - `RecipeStage.ai_enhancement_override` per-stage
//!   override semantics (None inherits
//!   recipe-level).
//! - `Recipe::effective_ai_enhancement_for_stage`
//!   resolution: per-stage override wins; unknown
//!   stage IDs return the recipe-level default;
//!   stage with no override inherits.
//! - `ProcessingObjective::ALL` is canonical and
//!   stable (5 elements, stable ordering).
//! - `Recipe::new()` initializes the new fields.

use astroforge_core::recipe::{
    AiEnhancementLevel, ProcessingObjective, QualityTargets, Recipe, RecipeStage,
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn processing_objective_label_is_human_readable() {
    assert_eq!(
        ProcessingObjective::PreserveStarColors.label(),
        "Preserve star colors"
    );
    assert_eq!(
        ProcessingObjective::MaximizeDetail.label(),
        "Maximize detail"
    );
    assert_eq!(
        ProcessingObjective::MaximizeSmoothness.label(),
        "Maximize smoothness"
    );
    assert_eq!(
        ProcessingObjective::MaximizeDynamicRange.label(),
        "Maximize dynamic range"
    );
    assert_eq!(
        ProcessingObjective::MaximizeReproducibility.label(),
        "Maximize reproducibility"
    );
}

#[test]
fn processing_objective_tag_is_snake_case() {
    assert_eq!(
        ProcessingObjective::PreserveStarColors.tag(),
        "preserve_star_colors"
    );
    assert_eq!(ProcessingObjective::MaximizeDetail.tag(), "maximize_detail");
    assert_eq!(
        ProcessingObjective::MaximizeSmoothness.tag(),
        "maximize_smoothness"
    );
    assert_eq!(
        ProcessingObjective::MaximizeDynamicRange.tag(),
        "maximize_dynamic_range"
    );
    assert_eq!(
        ProcessingObjective::MaximizeReproducibility.tag(),
        "maximize_reproducibility"
    );
}

#[test]
fn processing_objective_serde_round_trip() {
    for obj in ProcessingObjective::ALL {
        let json = serde_json::to_string(&obj).expect("serialize");
        // The wire format is the snake_case tag
        // (e.g. "preserve_star_colors"), not the
        // PascalCase variant name.
        assert!(
            json.contains('_') || json == "\"maximize\"",
            "expected snake-case tag in JSON, got: {}",
            json
        );
        let back: ProcessingObjective = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(obj, back);
    }
}

#[test]
fn processing_objective_all_is_canonical_and_stable() {
    // The list is intentional. If you change it,
    // update the §10 spec and the Guided tier UI.
    assert_eq!(ProcessingObjective::ALL.len(), 5);
    assert_eq!(
        ProcessingObjective::ALL[0],
        ProcessingObjective::PreserveStarColors
    );
    assert_eq!(
        ProcessingObjective::ALL[4],
        ProcessingObjective::MaximizeReproducibility
    );
}

#[test]
fn processing_objective_collection_serde_round_trip() {
    let r = Recipe::new("guided-test", "stretch");
    let objectives = vec![
        ProcessingObjective::PreserveStarColors,
        ProcessingObjective::MaximizeReproducibility,
    ];
    let json = serde_json::to_string(&objectives).expect("serialize");
    let back: Vec<ProcessingObjective> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(objectives, back);
    // Suppress unused-warning on r; it's a smoke
    // check that the type lives in the recipe
    // crate.
    let _ = r;
}

#[test]
fn quality_targets_default_is_empty() {
    let r = Recipe::new("qt-default", "stretch");
    assert_eq!(r.quality_targets, QualityTargets::default());
    assert!(r.quality_targets.target_snr_db.is_none());
    assert!(r.quality_targets.target_sharpness.is_none());
    assert!(r.quality_targets.target_background_smoothness.is_none());
}

#[test]
fn quality_targets_partial_construction() {
    let qt = QualityTargets {
        target_snr_db: Some(40.0),
        target_sharpness: None,
        target_background_smoothness: None,
    };
    assert_eq!(qt.target_snr_db, Some(40.0));
    assert!(qt.target_sharpness.is_none());
}

#[test]
fn quality_targets_full_construction() {
    let qt = QualityTargets {
        target_snr_db: Some(45.0),
        target_sharpness: Some(0.85),
        target_background_smoothness: Some(0.7),
    };
    let json = serde_json::to_string(&qt).expect("serialize");
    let back: QualityTargets = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(qt, back);
}

#[test]
fn quality_targets_absent_fields_skip_serialization() {
    // None fields skip serialization so the JSON
    // stays compact (no `"target_snr_db": null`
    // noise on Recipes that didn't set them).
    let qt = QualityTargets {
        target_snr_db: Some(40.0),
        target_sharpness: None,
        target_background_smoothness: None,
    };
    let json = serde_json::to_string(&qt).expect("serialize");
    assert!(
        !json.contains("target_sharpness"),
        "expected absent fields to skip serialization, got: {}",
        json
    );
    assert!(
        !json.contains("target_background_smoothness"),
        "expected absent fields to skip serialization, got: {}",
        json
    );
}

#[test]
fn recipe_new_initializes_guided_tier_fields_to_empty() {
    let r = Recipe::new("guided-init", "stretch");
    assert!(r.processing_objectives.is_empty());
    assert_eq!(r.quality_targets, QualityTargets::default());
    assert!(r.optional_operations.is_empty());
    // Each stage's per-stage AI override defaults
    // to None so the recipe-level default is
    // inherited.
    for s in &r.stages {
        assert!(s.ai_enhancement_override.is_none());
    }
}

#[test]
fn legacy_recipe_without_guided_fields_deserializes_via_default() {
    // Legacy Recipes from before §10.2 don't
    // carry the new fields. The serde defaults
    // must backfill them so the apply round
    // doesn't choke on missing keys.
    let legacy_json = r#"{
        "schema_version": "2",
        "name": "legacy",
        "description": "",
        "target_type": "stretch",
        "stages": [],
        "required_models": [],
        "integrity": {
            "perceptual_models_used": false,
            "deterministic_models_used": false,
            "seed_recorded": false,
            "models": []
        },
        "version": 1,
        "parent_version": null,
        "branch": "main",
        "is_system": false,
        "created_at": "",
        "flags": [],
        "quality_profile": "natural",
        "ai_enhancement_level": "recommended"
    }"#;
    let r: Recipe = serde_json::from_str(legacy_json).expect("deserialize");
    assert!(r.processing_objectives.is_empty());
    assert_eq!(r.quality_targets, QualityTargets::default());
    assert!(r.optional_operations.is_empty());
}

#[test]
fn per_stage_ai_override_serde_round_trip() {
    let mut r = Recipe::new("override", "stretch");
    r.stages.push(RecipeStage {
        stage_id: "denoise".into(),
        enabled: true,
        params: HashMap::new(),
        ai_enhancement_override: Some(AiEnhancementLevel::Conservative),
    });
    let json = serde_json::to_string(&r).expect("serialize");
    let back: Recipe = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        back.stages[0].ai_enhancement_override,
        Some(AiEnhancementLevel::Conservative)
    );
}

#[test]
fn effective_ai_enhancement_inherits_when_no_override() {
    let r = Recipe::new("inherit", "stretch");
    // Recipe-level default is Recommended.
    assert_eq!(
        r.effective_ai_enhancement_for_stage("any-stage"),
        AiEnhancementLevel::Recommended
    );
}

#[test]
fn effective_ai_enhancement_per_stage_override_wins() {
    let mut r = Recipe::new("override-wins", "stretch");
    r.ai_enhancement_level = AiEnhancementLevel::Advanced;
    r.stages.push(RecipeStage {
        stage_id: "denoise".into(),
        enabled: true,
        params: HashMap::new(),
        ai_enhancement_override: Some(AiEnhancementLevel::Off),
    });
    // Override wins.
    assert_eq!(
        r.effective_ai_enhancement_for_stage("denoise"),
        AiEnhancementLevel::Off
    );
    // Unknown stage falls back to recipe-level.
    assert_eq!(
        r.effective_ai_enhancement_for_stage("nonexistent"),
        AiEnhancementLevel::Advanced
    );
}

#[test]
fn optional_operations_accepts_any_string() {
    // The optional_operations list is
    // stage-ID-typed; we don't validate against
    // the spec table here. Validation lives in
    // the §20 pipeline. This slice only adds
    // the data shape.
    let mut r = Recipe::new("opt-ops", "stretch");
    r.optional_operations = vec![
        "cosmetic".into(),
        "curves".into(),
        "user-defined-stage".into(),
    ];
    assert_eq!(r.optional_operations.len(), 3);
    let json = serde_json::to_string(&r).expect("serialize");
    let back: Recipe = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(r.optional_operations, back.optional_operations);
}

#[test]
fn processing_objectives_serde_round_trip_on_recipe() {
    let mut r = Recipe::new("obj-rountrip", "stretch");
    r.processing_objectives = vec![
        ProcessingObjective::MaximizeDetail,
        ProcessingObjective::MaximizeReproducibility,
    ];
    let json = serde_json::to_string(&r).expect("serialize");
    let back: Recipe = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(r.processing_objectives, back.processing_objectives);
}

#[test]
fn quality_targets_serde_round_trip_on_recipe() {
    let mut r = Recipe::new("qt-roundtrip", "stretch");
    r.quality_targets = QualityTargets {
        target_snr_db: Some(38.0),
        target_sharpness: Some(0.8),
        target_background_smoothness: None,
    };
    let json = serde_json::to_string(&r).expect("serialize");
    let back: Recipe = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(r.quality_targets, back.quality_targets);
}

#[test]
fn add_stage_default_ai_override_is_none() {
    // The add_stage convenience builder must
    // initialize the new field to None so
    // existing call sites (which pre-date §10.2)
    // continue to compile.
    let mut r = Recipe::new("add-stage", "stretch");
    r.add_stage("denoise", HashMap::from([("radius".into(), json!(2.5))]));
    assert_eq!(r.stages.len(), 1);
    assert!(r.stages[0].ai_enhancement_override.is_none());
}
