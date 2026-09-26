//! CR-08 §22 round 2 / Slice A tests: `preview_recipe`.
//! Pure-function tests; no I/O.
//!
//! The function is the read-only sibling of `apply_recipe`.
//! It surfaces the full preview surface (all stages
//! plus metadata, provenance, applicability, warnings)
//! even when the Recipe is not applicable, so the UI
//! can show WHY without forcing a fix-or-abort loop.
//! Tests pin the round-2 contract:
//! - Compatible Recipes return Compatibility + no warnings.
//! - MissingModels produces a MissingModels applicability
//!   plus a human-readable advisory warning.
//! - Schema mismatch produces IncompatibleVersion plus
//!   a re-save advisory.
//! - Disabled stages surface an informational warning
//!   plus the stage appears in `resolved_stages` with
//!   `enabled: false` (apply filters them out; preview
//!   surfaces them).
//! - `resolved_ai_enhancement_level` echoes
//!   `effective_ai_enhancement_for_stage` per stage.
//! - Provenance re-uses the round-1 surface unchanged.

use astroforge_core::recipe::{
    preview_recipe, AiEnhancementLevel, IntegrityBadge, ProcessingObjective, QualityProfile,
    QualityTargets, Recipe, RecipeProvenance, RecipeStage, ValidationResult,
};

fn empty_recipe(name: &str, target: &str, quality_profile: QualityProfile) -> Recipe {
    Recipe {
        schema_version: "2.0".to_string(),
        name: name.to_string(),
        description: "test recipe".to_string(),
        target_type: target.to_string(),
        stages: Vec::new(),
        required_models: Vec::new(),
        integrity: IntegrityBadge {
            perceptual_models_used: false,
            deterministic_models_used: true,
            seed_recorded: false,
            models: Vec::new(),
        },
        version: 1,
        parent_version: None,
        branch: "main".to_string(),
        is_system: false,
        created_at: String::new(),
        flags: Vec::new(),
        quality_profile,
        ai_enhancement_level: AiEnhancementLevel::Recommended,
        processing_objectives: Vec::<ProcessingObjective>::new(),
        quality_targets: QualityTargets::default(),
        optional_operations: Vec::<String>::new(),
        // CR-08 §21 follow-on / Slice E + §28 / Slice H:
        // empty content hash for legacy tests.
        content_hash: String::new(),
        // CR-08 §21 / Slice G: empty constraints +
        // no resource policy for legacy tests.
        constraints: Vec::new(),
        resource_policy: None,
    }
}

fn stage(stage_id: &str, enabled: bool) -> RecipeStage {
    RecipeStage {
        stage_id: stage_id.to_string(),
        enabled,
        params: Default::default(),
        ai_enhancement_override: None,
    }
}

#[test]
fn preview_compatible_recipe_returns_compatible_no_warnings() {
    let recipe = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail);
    let preview = preview_recipe("recipe-a-id", &recipe, &[]);

    assert_eq!(preview.profile_id, "recipe-a-id");
    assert_eq!(preview.version, 1);
    assert_eq!(preview.branch, "main");
    assert_eq!(preview.metadata.name, "recipe-a");
    assert_eq!(preview.metadata.target_type, "Nebula");
    assert_eq!(preview.metadata.quality_profile, QualityProfile::Detail);
    assert!(!preview.metadata.is_system);
    assert_eq!(preview.applicability, ValidationResult::Compatible);
    assert!(preview.resolved_stages.is_empty());
    assert!(
        preview.warnings.is_empty(),
        "Compatible Recipe should produce no warnings; got {:?}",
        preview.warnings
    );
    // Provenance re-uses the round-1 surface unchanged.
    let _: RecipeProvenance = preview.provenance.clone();
}

#[test]
fn preview_missing_models_advisory_surfaces_in_warnings() {
    let mut recipe = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail);
    recipe.required_models = vec!["m-1".to_string(), "m-2".to_string()];
    let preview = preview_recipe("recipe-a-id", &recipe, &["m-1".to_string()]);

    assert!(matches!(
        preview.applicability,
        ValidationResult::MissingModels(ref m) if m == &vec!["m-2".to_string()]
    ));
    assert_eq!(preview.warnings.len(), 1);
    assert!(preview.warnings[0].contains("Missing required models"));
    assert!(preview.warnings[0].contains("m-2"));
}

#[test]
fn preview_schema_mismatch_surfaces_re_save_advisory() {
    let mut recipe = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail);
    recipe.schema_version = "9.9".to_string();
    let preview = preview_recipe("recipe-a-id", &recipe, &[]);

    assert!(matches!(
        preview.applicability,
        ValidationResult::IncompatibleVersion(ref v) if v == "9.9"
    ));
    assert_eq!(preview.warnings.len(), 1);
    assert!(preview.warnings[0].contains("Schema version mismatch"));
    assert!(preview.warnings[0].contains("9.9"));
    assert!(preview.warnings[0].contains("Re-save"));
}

#[test]
fn preview_disabled_stage_surfaces_in_resolved_stages_with_warning() {
    // Apply filters disabled stages out; preview
    // surfaces them in resolved_stages with a warning
    // so the UI can render a "skipped" pill.
    let mut recipe = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail);
    recipe.stages = vec![stage("calibrate-stretch", false)];

    let preview = preview_recipe("recipe-a-id", &recipe, &[]);
    assert_eq!(preview.resolved_stages.len(), 1);
    assert!(!preview.resolved_stages[0].enabled);
    assert_eq!(preview.resolved_stages[0].stage_id, "calibrate-stretch");
    assert!(preview
        .warnings
        .iter()
        .any(|w| w.contains("calibrate-stretch") && w.contains("disabled")));
}

#[test]
fn preview_resolved_ai_level_reflects_recipe_level_default() {
    // No per-stage override; recipe-level default
    // surfaces in `resolved_ai_enhancement_level`.
    let mut recipe = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail);
    recipe.ai_enhancement_level = AiEnhancementLevel::Conservative;
    recipe.stages = vec![stage("calibrate-stretch", true)];

    let preview = preview_recipe("recipe-a-id", &recipe, &[]);
    assert_eq!(preview.resolved_stages.len(), 1);
    assert_eq!(
        preview.resolved_stages[0].resolved_ai_enhancement_level,
        "Conservative"
    );
    // No "Off" warning should fire (Conservative is
    // not Off).
    assert!(!preview.warnings.iter().any(|w| w.contains("Off")));
}

#[test]
fn preview_off_level_surfaces_zero_ai_advisory() {
    // AI Enhancement Level = Off surfaces a
    // per-stage advisory so the UI can flag that
    // AI models will not be used even though the
    // stage is enabled.
    let mut recipe = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail);
    recipe.ai_enhancement_level = AiEnhancementLevel::Off;
    recipe.stages = vec![stage("calibrate-stretch", true)];

    let preview = preview_recipe("recipe-a-id", &recipe, &[]);
    assert_eq!(preview.resolved_stages.len(), 1);
    assert_eq!(
        preview.resolved_stages[0].resolved_ai_enhancement_level,
        "Off"
    );
    assert!(preview
        .warnings
        .iter()
        .any(|w| w.contains("calibrate-stretch") && w.contains("Off")));
}

#[test]
fn preview_does_not_stamp_last_used_at() {
    // The function is pure: no IO, no
    // last_used_at side effect. This test
    // documents that intent: even after running
    // preview_recipe, the recipe's version /
    // branch / etc are unchanged.
    let recipe = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail);
    let snapshot_version = recipe.version;
    let snapshot_branch = recipe.branch.clone();
    let preview = preview_recipe("recipe-a-id", &recipe, &[]);
    assert_eq!(recipe.version, snapshot_version);
    assert_eq!(recipe.branch, snapshot_branch);
    // And the preview mirrors those values.
    assert_eq!(preview.version, snapshot_version);
    assert_eq!(preview.branch, snapshot_branch);
}
