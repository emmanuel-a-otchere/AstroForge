//! CR-08 §10 Beginner tier tests: pin the
//! `AiEnhancementLevel` enum's serde contract + the
//! Beginner-tier Recipe shape (name + target_type +
//! quality_profile + ai_enhancement_level).
//!
//! Coverage:
//! - All four enum variants round-trip through serde
//!   (lowercase tag).
//! - The default variant is `Recommended`.
//! - Legacy Recipes that predate §10's slice deserialize
//!   with `ai_enhancement_level = Recommended` (the
//!   serde default).
//! - `pipeline_plan_hash` is influenced by the field
//!   (changing the level changes the hash).
//! - The Beginner tier UI inputs (name + target_type +
//!   quality_profile + ai_enhancement_level) survive a
//!   recipe_save + reload round-trip via the
//!   `RecipeStore`.

use astroforge_core::recipe::{AiEnhancementLevel, IntegrityBadge, QualityProfile, Recipe};
use std::collections::HashMap;

fn minimal_recipe(name: &str, target: &str) -> Recipe {
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
fn ai_enhancement_default_is_recommended() {
    let r = Recipe::new("test", "stretch");
    assert_eq!(
        r.ai_enhancement_level,
        AiEnhancementLevel::Recommended,
        "new Recipe must default to Recommended"
    );
}

#[test]
fn ai_enhancement_all_returns_four_variants_in_display_order() {
    assert_eq!(
        AiEnhancementLevel::ALL.len(),
        4,
        "ALL must surface exactly 4 variants for the radio picker"
    );
    assert_eq!(AiEnhancementLevel::ALL[0], AiEnhancementLevel::Off);
    assert_eq!(AiEnhancementLevel::ALL[1], AiEnhancementLevel::Conservative);
    assert_eq!(AiEnhancementLevel::ALL[2], AiEnhancementLevel::Recommended);
    assert_eq!(AiEnhancementLevel::ALL[3], AiEnhancementLevel::Advanced);
}

#[test]
fn ai_enhancement_labels_are_nonempty() {
    for level in AiEnhancementLevel::ALL {
        assert!(
            !level.label().is_empty(),
            "{:?} label must be non-empty",
            level
        );
        assert!(
            !level.description().is_empty(),
            "{:?} description must be non-empty",
            level
        );
    }
}

#[test]
fn ai_enhancement_round_trips_through_serde() {
    for level in AiEnhancementLevel::ALL {
        let json = serde_json::to_string(&level).expect("serialize");
        let back: AiEnhancementLevel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(level, back, "round-trip must preserve {:?}", level);
    }
}

#[test]
fn ai_enhancement_lowercase_tag() {
    // The Beginner tier's radio picker relies on the
    // wire-format tag being lowercase so the JSON
    // payload mirrors the §22 quality_profile shape.
    for level in AiEnhancementLevel::ALL {
        let json = serde_json::to_string(&level).expect("serialize");
        // JSON for a serde(rename_all = "lowercase")
        // enum is a single quoted tag, e.g. "off".
        assert_eq!(
            json,
            format!("\"{}\"", level.label().to_lowercase()),
            "{:?} must serialize to lowercase tag",
            level
        );
    }
}

#[test]
fn legacy_recipe_without_ai_enhancement_field_deserializes_as_recommended() {
    // A Recipe JSON that pre-dates §10 (no
    // `ai_enhancement_level` field) must deserialize
    // without error and default the field to
    // Recommended.
    let legacy_json = r#"{
        "schema_version": "2.0",
        "name": "legacy-recipe",
        "description": "",
        "target_type": "stretch",
        "stages": [],
        "required_models": [],
        "integrity": {
            "perceptual_models_used": false,
            "deterministic_models_used": true,
            "seed_recorded": false,
            "models": []
        },
        "version": 1,
        "parent_version": null,
        "branch": "main",
        "created_at": "",
        "flags": [],
        "quality_profile": "natural"
    }"#;
    let r: Recipe = serde_json::from_str(legacy_json).expect("legacy Recipe must deserialize");
    assert_eq!(
        r.ai_enhancement_level,
        AiEnhancementLevel::Recommended,
        "legacy Recipes must default to Recommended"
    );
}

#[test]
fn beginner_tier_inputs_round_trip_through_recipe_save() {
    // The Beginner editor writes the four fields
    // (name, target_type, quality_profile,
    // ai_enhancement_level) via `recipe_save`. Verify
    // the in-memory Recipe round-trips through
    // `to_json` / `from_json_migrated` so the
    // RecipeStore's save path can persist the user's
    // Beginner picks without losing them on reload.
    let mut r = minimal_recipe("beginner-pick", "narrowband");
    r.description = "Beginner tier quickstart".into();
    r.quality_profile = QualityProfile::Clean;
    r.ai_enhancement_level = AiEnhancementLevel::Conservative;
    r.add_stage("stretch", HashMap::new());

    let json = r.to_json().expect("serialize");
    let back = Recipe::from_json_migrated(&json).expect("deserialize");
    assert_eq!(back.name, "beginner-pick");
    assert_eq!(back.target_type, "narrowband");
    assert_eq!(back.quality_profile, QualityProfile::Clean);
    assert_eq!(back.ai_enhancement_level, AiEnhancementLevel::Conservative);
    assert_eq!(back.stages.len(), 1);
}

#[test]
fn pipeline_plan_hash_includes_ai_enhancement_level() {
    // The hash is content-addressed; a Beginner-tier
    // change (Off vs Advanced) must flip the hash so
    // the §32.4 content-hash surface reflects the
    // editor's intent.
    let mut a = minimal_recipe("a", "stretch");
    let mut b = minimal_recipe("b", "stretch");
    a.ai_enhancement_level = AiEnhancementLevel::Off;
    b.ai_enhancement_level = AiEnhancementLevel::Advanced;
    assert_ne!(
        a.pipeline_plan_hash(),
        b.pipeline_plan_hash(),
        "ai_enhancement_level must influence the hash"
    );
}

#[test]
fn pipeline_plan_hash_distinguishes_quality_profile_change() {
    // Sanity check: the §22 quality_profile axis
    // (carried over from CR-07) must also influence
    // the hash. If this fails the test infra is stale
    // and the Beginner tier's hash test above is
    // meaningless.
    let mut a = minimal_recipe("a", "stretch");
    let mut b = minimal_recipe("b", "stretch");
    a.quality_profile = QualityProfile::Natural;
    b.quality_profile = QualityProfile::Publication;
    assert_ne!(
        a.pipeline_plan_hash(),
        b.pipeline_plan_hash(),
        "quality_profile must influence the hash"
    );
}
