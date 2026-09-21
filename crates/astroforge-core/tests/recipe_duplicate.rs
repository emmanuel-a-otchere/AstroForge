//! CR-08 §22.1: recipe duplicate integration tests.
//!
//! Pure-data-model coverage of the duplicate algorithm that
//! `recipe_duplicate` IPC (src-tauri/src/main.rs) executes.
//! The IPC layer is a thin pass-through; the data path is:
//!
//! - Read source `Recipe` via `RecipeStore::get(profile_id, version)`.
//! - Build a new name: "<name> (Copy)"; if that already
//!   exists, sweep "(Copy 2)" .. "(Copy 99)"; if all collide,
//!   fall back to a timestamp suffix.
//! - Set `version = 0` + `parent_version = None` so the
//!   subsequent `recipe_save` auto-assigns version = 1.
//! - Save via the existing store path.
//!
//! These tests cover the helper algorithm in isolation by
//! exercising the public `RecipeStore` surface against an
//! in-memory SQLite path (`:memory:`). Per the
//! `astroforge-cr-slices` skill: `src-tauri` is a binary-only
//! crate outside the cargo workspace; the behavioral tests
//! live in `astroforge-core`.

use astroforge_core::recipe::{IntegrityBadge, ModelType, QualityProfile, Recipe, RecipeStage};
use astroforge_core::recipe_store::RecipeStore;
use std::path::PathBuf;

fn sample_recipe(name: &str, target: &str) -> Recipe {
    let mut r = Recipe::new(name, target);
    r.add_stage(
        "stretch",
        [("amount".to_string(), serde_json::json!(0.05))]
            .into_iter()
            .collect(),
    );
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

/// Open a fresh in-memory RecipeStore. `:memory:` paths work
/// via `RecipeStore::new` because the SQLite connection
/// ignores the parent-dir creation when the path is bare.
fn in_memory_store() -> RecipeStore {
    let path = PathBuf::from(":memory:");
    RecipeStore::new(&path).expect("open in-memory recipe store")
}

#[test]
fn duplicate_creates_independent_profile_at_version_one() {
    let store = in_memory_store();
    let source = sample_recipe("m42-natural", "stretch");
    let summary_v1 = store.save(&source).expect("save source v1");
    assert_eq!(summary_v1.version, 1);

    // To produce a v2 of the source, the IPC layer calls
    // `store.save` after mutating `version = next_version_for`
    // (mirroring the recipe_save handler). Mirror that here
    // so the source reaches v2 before we duplicate.
    let mut source_v2 = store
        .get(&summary_v1.profile_id, summary_v1.version)
        .expect("load v1");
    source_v2.version = 2;
    let summary = store.save(&source_v2).expect("save source v2");
    assert_eq!(summary.version, 2);

    // Simulate the IPC handler: load v2, append " (Copy)",
    // compute next_version_for the new profile_id, save.
    let loaded = store
        .get(&summary.profile_id, summary.version)
        .expect("load v2");
    let mut cloned = loaded;
    cloned.name = format!("{} (Copy)", cloned.name);
    let new_profile_id = RecipeStore::profile_id_for(&cloned.name, &cloned.target_type);
    cloned.version = store
        .next_version_for(&new_profile_id)
        .expect("next_version_for new profile");
    cloned.parent_version = None;
    let copy_summary = store.save(&cloned).expect("save copy");

    // New profile, distinct profile_id, fresh lineage.
    assert_ne!(copy_summary.profile_id, summary.profile_id);
    assert_eq!(copy_summary.version, 1);
    assert_eq!(copy_summary.parent_version, None);
    assert_eq!(copy_summary.name, "m42-natural (Copy)");

    // The original is untouched at v2.
    let still_source = store
        .get(&summary.profile_id, summary.version)
        .expect("reload source");
    assert_eq!(still_source.version, 2);
}

#[test]
fn duplicate_appends_copy_suffix_on_collision() {
    let store = in_memory_store();
    let source = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&source).expect("save source");

    // First duplicate.
    let mut first = store
        .get(&summary.profile_id, summary.version)
        .expect("load");
    first.name = format!("{} (Copy)", first.name);
    first.version = 0;
    first.parent_version = None;
    let first_copy = store.save(&first).expect("save first copy");
    assert_eq!(first_copy.name, "m42-natural (Copy)");

    // Second duplicate; the "(Copy)" name now collides. The
    // algorithm sweeps "(Copy 2)". We test the full
    // candidate generation by directly invoking the helper
    // logic (mirror of the IPC handler).
    let source_again = store
        .get(&summary.profile_id, summary.version)
        .expect("reload source");
    let mut second = source_again;
    let base = second.name.clone();
    // Algorithm: try "(Copy)" first; if collides, "(Copy 2..99)".
    let candidate = if !profile_exists(&store, &format!("{base} (Copy)"), &second.target_type) {
        format!("{base} (Copy)")
    } else {
        (2..=99)
            .map(|n| format!("{base} (Copy {n})"))
            .find(|n| !profile_exists(&store, n, &second.target_type))
            .expect("find a free suffix in 2..=99")
    };
    second.name = candidate.clone();
    second.version = 0;
    second.parent_version = None;
    let second_copy = store.save(&second).expect("save second copy");
    assert_eq!(second_copy.name, "m42-natural (Copy 2)");
}

#[test]
fn duplicate_preserves_stages_and_integrity() {
    let store = in_memory_store();
    let mut source = sample_recipe("m42-natural", "stretch");
    source.integrity = IntegrityBadge {
        perceptual_models_used: true,
        deterministic_models_used: false,
        seed_recorded: true,
        models: vec![astroforge_core::recipe::ModelUsage {
            model_name: "m42-denoise-v1".into(),
            model_type: ModelType::Perceptual,
        }],
    };
    let summary = store.save(&source).expect("save source");
    let loaded = store
        .get(&summary.profile_id, summary.version)
        .expect("load");
    let mut cloned = loaded.clone();
    cloned.name = format!("{} (Copy)", cloned.name);
    cloned.version = 0;
    cloned.parent_version = None;
    let copy = store.save(&cloned).expect("save copy");
    let copy_recipe = store
        .get(&copy.profile_id, copy.version)
        .expect("load copy");

    // Stage list, params, integrity, required_models are
    // preserved verbatim.
    assert_eq!(copy_recipe.stages.len(), 1);
    assert!(matches!(
        copy_recipe.stages[0],
        RecipeStage { ref stage_id, enabled: true, .. } if stage_id == "stretch"
    ));
    assert_eq!(
        copy_recipe.stages[0].params.get("amount"),
        Some(&serde_json::json!(0.05))
    );
    assert_eq!(copy_recipe.required_models, vec!["m42-blur-v2"]);
    assert!(copy_recipe.integrity.perceptual_models_used);
    assert!(copy_recipe.integrity.seed_recorded);
    assert_eq!(copy_recipe.integrity.models.len(), 1);
    assert_eq!(copy_recipe.integrity.models[0].model_name, "m42-denoise-v1");
    assert_eq!(
        copy_recipe.integrity.models[0].model_type,
        ModelType::Perceptual
    );
}

/// Mirror of the helper in `src-tauri/src/main.rs::profile_exists`.
/// Holds the store mutex internally (the actual implementation
/// takes the lock once and reuses it; here we acquire and
/// release per call to mirror the test invariant).
fn profile_exists(store: &RecipeStore, name: &str, target_type: &str) -> bool {
    let profile_id = RecipeStore::profile_id_for(name, target_type);
    match store.list() {
        Ok(summaries) => summaries.iter().any(|s| s.profile_id == profile_id),
        Err(_) => false,
    }
}
