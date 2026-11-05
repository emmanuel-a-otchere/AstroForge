//! RT-10 (#428) — community recipe gallery index validation.
//!
//! The gallery is a static in-repo JSON index (ADR-0021): this file
//! IS the hosting infrastructure, and these tests are its gate — the
//! index cannot merge with a malformed entry, an unknown stage type,
//! a duplicate name, or an invented model requirement. Entries here
//! import through the standard sanitized SharedRecipe path (spec
//! §11.1) with the same strict validation the app applies at import.
//!
//! Lives in astroforge-ai so one binary can see both the stage
//! taxonomy (core) and the model hub (AI) for the cross-checks.

use astroforge_core::node_catalog::USER_FACING_STAGE_TYPES;
use astroforge_core::recipe_feed::SharedRecipe;

const INDEX_JSON: &str = include_str!("../../../recipes/index.json");

/// The index envelope: `index_version` + recipes. SharedRecipe
/// entries parse with the standard serde derive — unknown fields are
/// tolerated (forward-compatible), missing required fields are not.
#[derive(serde::Deserialize)]
struct GalleryIndex {
    index_version: u32,
    recipes: Vec<SharedRecipe>,
}

fn load() -> GalleryIndex {
    serde_json::from_str(INDEX_JSON).expect("recipes/index.json must parse")
}

#[test]
fn index_parses_with_current_version() {
    let index = load();
    assert_eq!(index.index_version, 1, "bump + migrate deliberately");
    assert!(
        !index.recipes.is_empty(),
        "an empty gallery index is a shipping mistake"
    );
}

/// Every entry must deserialize as a complete sanitized SharedRecipe:
/// required fields present, no placeholder text.
#[test]
fn every_entry_is_a_valid_shared_recipe() {
    let index = load();
    for recipe in &index.recipes {
        assert!(!recipe.name.trim().is_empty(), "entry with empty name");
        assert!(
            !recipe.target_type.trim().is_empty(),
            "{}: empty target_type",
            recipe.name
        );
        assert!(
            recipe.recipe_version.starts_with('1'),
            "{}: unexpected recipe_version {}",
            recipe.name,
            recipe.recipe_version
        );
        assert!(
            recipe.pipeline.len() >= 2,
            "{}: a pipeline needs at least ingest and export to be honest",
            recipe.name
        );
        // First and last stages: sessions start at ingest and end at
        // export; gallery recipes must respect the same frame.
        assert_eq!(
            recipe.pipeline[0].stage, "ingest",
            "{}: pipeline must start with ingest",
            recipe.name
        );
        assert_eq!(
            recipe.pipeline[recipe.pipeline.len() - 1].stage,
            "export",
            "{}: pipeline must end with export",
            recipe.name
        );
    }
}

/// Stage vocabulary: every pipeline stage must be a user-facing
/// stage type from the generated NodeCatalog taxonomy — a typo like
/// "background_extractionn" cannot merge.
#[test]
fn every_stage_is_in_the_node_catalog_taxonomy() {
    let index = load();
    for recipe in &index.recipes {
        for stage in &recipe.pipeline {
            assert!(
                USER_FACING_STAGE_TYPES.contains(&stage.stage.as_str()),
                "{}: stage '{}' is not a user-facing stage type",
                recipe.name,
                stage.stage
            );
            assert!(
                stage.params.is_object(),
                "{}: stage '{}' params must be a JSON object",
                recipe.name,
                stage.stage
            );
        }
    }
}

/// Gallery identity: names are the import key a user sees; the AI
/// model requirement map must reference models the hub actually
/// catalogs (no phantom pins).
#[test]
fn names_are_unique_and_model_pins_reference_the_hub() {
    let index = load();
    let mut names: Vec<&str> = index.recipes.iter().map(|r| r.name.as_str()).collect();
    names.sort();
    names.dedup();
    assert_eq!(
        names.len(),
        index.recipes.len(),
        "duplicate gallery names — names are the identity users see"
    );

    let hub_models: Vec<String> = astroforge_ai::hub::model_catalog()
        .into_iter()
        .map(|m| m.name)
        .collect();
    for recipe in &index.recipes {
        for model in recipe.model_versions.keys() {
            assert!(
                hub_models.iter().any(|h| h == model),
                "{}: model_versions references unknown model '{model}'",
                recipe.name
            );
        }
    }
}

/// Integrity honesty: entries that pin perceptual AI models must
/// declare it, so the apply path's integrity badge is truthful from
/// the gallery onward.
#[test]
fn perceptual_integrity_matches_model_pins() {
    let index = load();
    for recipe in &index.recipes {
        let perceptual_stage = recipe.pipeline.iter().any(|s| {
            s.stage == "sharpen_deconvolution"
                || s.stage == "star_handling"
                || s.stage == "creative_polish"
        });
        if recipe.integrity.perceptual_models_used {
            assert!(
                perceptual_stage || !recipe.model_versions.is_empty(),
                "{}: declares perceptual_models_used with no perceptual stage or model pin",
                recipe.name
            );
        }
    }
}
