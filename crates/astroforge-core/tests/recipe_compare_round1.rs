//! CR-08 §22 round 1 tests: `recipe_compare_versions` +
//! `recipe_provenance`. Pure-function tests; no I/O.
//!
//! Both functions wrap existing primitives
//! (`recipe_ai_diff_summary` + `recipe_parameter_diff`)
//! so most of the heavy lifting is already covered by
//! `recipe_compare.rs` + `recipe_diff.rs`. These tests
//! pin the round-1 contract: identical Recipes fold
//! to `identical: true`; non-identical Recipes fold
//! to `identical: false` even when parameter_diff
//! reports `identical`; lineage_summary equals the
//! `ai.provenance` line; `recipe_provenance` walks
//! the parent_version chain + surfaces system-recipe
//! markers deterministically.

use astroforge_core::recipe::{
    recipe_compare_versions, recipe_provenance, AiEnhancementLevel, IntegrityBadge, ModelType,
    ProcessingObjective, QualityProfile, QualityTargets, Recipe, RecipeProvenance, RecipeStage,
};

fn empty_recipe(
    name: &str,
    target: &str,
    quality_profile: QualityProfile,
    perceptual_models_used: bool,
) -> Recipe {
    Recipe {
        schema_version: "2.0".to_string(),
        name: name.to_string(),
        description: String::new(),
        target_type: target.to_string(),
        stages: Vec::new(),
        required_models: Vec::new(),
        integrity: IntegrityBadge {
            perceptual_models_used,
            deterministic_models_used: !perceptual_models_used,
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
fn identical_recipes_fold_to_identical_true() {
    let a = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail, false);
    let b = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail, false);

    let comparison = recipe_compare_versions(&a, &b);
    assert!(comparison.parameter_diff.identical);
    assert!(!comparison.ai.hash_differs);
    assert!(!comparison.ai.ai_classification_differs);
    assert!(!comparison.ai.required_models_differ);
    assert!(
        comparison.identical,
        "identical Recipes should fold to identical: true"
    );
    assert!(!comparison.lineage_summary.is_empty());
}

#[test]
fn quality_profile_divergence_folds_to_identical_false() {
    let a = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail, false);
    let mut b = a.clone();
    b.quality_profile = QualityProfile::Publication;

    let comparison = recipe_compare_versions(&a, &b);
    assert!(comparison.parameter_diff.identical);
    assert!(!comparison.identical);
    assert!(comparison.lineage_summary.contains("detail"));
    assert!(comparison.lineage_summary.contains("publication"));
}

#[test]
fn stage_diff_folds_to_identical_false() {
    let mut a = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail, false);
    a.stages = vec![stage("calibrate-stretch", true)];
    let mut b = a.clone();
    b.stages = vec![stage("calibrate-stretch", false)];

    let comparison = recipe_compare_versions(&a, &b);
    assert!(!comparison.parameter_diff.identical);
    assert!(!comparison.identical);
}

#[test]
fn lineage_summary_equals_ai_provenance() {
    let a = empty_recipe("recipe-a", "Nebula", QualityProfile::Clean, false);
    let b = empty_recipe("recipe-b", "Galaxy", QualityProfile::Publication, true);

    let comparison = recipe_compare_versions(&a, &b);
    assert_eq!(comparison.lineage_summary, comparison.ai.provenance);
    assert!(comparison.lineage_summary.contains("Recipe A v1"));
    assert!(comparison.lineage_summary.contains("Recipe B v1"));
    assert!(comparison.lineage_summary.contains("clean"));
    assert!(comparison.lineage_summary.contains("publication"));
}

#[test]
fn provenance_basic_fields_round_trip() {
    let mut recipe = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail, true);
    recipe.integrity.models = vec![astroforge_core::recipe::ModelUsage {
        model_name: "m-2".to_string(),
        model_type: ModelType::Perceptual,
    }];
    recipe.required_models = vec!["m-2".to_string()];

    let prov: RecipeProvenance = recipe_provenance("recipe-a-id", &recipe);
    assert_eq!(prov.profile_id, "recipe-a-id");
    assert_eq!(prov.version, 1);
    assert_eq!(prov.parent_version, None);
    assert_eq!(prov.schema_version, "2.0");
    assert_eq!(prov.name, "recipe-a");
    assert_eq!(prov.target_type, "Nebula");
    assert_eq!(prov.quality_profile, QualityProfile::Detail);
    assert!(!prov.pipeline_plan_hash.is_empty());
    assert!(prov.ai_used);
    assert_eq!(prov.perceptual_models, vec!["m-2".to_string()]);
    assert_eq!(prov.required_models, vec!["m-2".to_string()]);
    assert!(!prov.is_system);
    assert_eq!(prov.lineage_steps.len(), 1);
    assert!(prov.lineage_steps[0].contains("Created as v1"));
}

#[test]
fn provenance_walks_parent_version_chain() {
    let mut recipe = empty_recipe("recipe-a", "Galaxy", QualityProfile::Natural, false);
    recipe.parent_version = Some(3);

    let prov = recipe_provenance("recipe-a-id", &recipe);
    assert_eq!(prov.parent_version, Some(3));
    assert_eq!(prov.lineage_steps.len(), 2);
    assert!(prov.lineage_steps[0].contains("Created as v1"));
    assert!(prov.lineage_steps[1].contains("Adapted from v3"));
}

#[test]
fn system_recipe_marks_lineage_as_system() {
    let mut recipe = empty_recipe(
        "system-recipe-x",
        "Nebula",
        QualityProfile::Publication,
        false,
    );
    recipe.is_system = true;

    let prov = recipe_provenance("system-recipe-x-id", &recipe);
    assert!(prov.is_system);
    assert_eq!(prov.lineage_steps.len(), 1);
    assert_eq!(prov.lineage_steps[0], "System recipe");
}

#[test]
fn perceptual_models_sorted_for_stable_output() {
    let mut recipe = empty_recipe("recipe-a", "Nebula", QualityProfile::Detail, true);
    recipe.integrity.models = vec![
        astroforge_core::recipe::ModelUsage {
            model_name: "zeta-enhancer".to_string(),
            model_type: ModelType::Perceptual,
        },
        astroforge_core::recipe::ModelUsage {
            model_name: "alpha-denoiser".to_string(),
            model_type: ModelType::Perceptual,
        },
    ];

    let prov = recipe_provenance("recipe-a-id", &recipe);
    assert_eq!(
        prov.perceptual_models,
        vec!["alpha-denoiser".to_string(), "zeta-enhancer".to_string()]
    );
}
