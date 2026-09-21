//! CR-08 §22.2: recipe delete integration tests.
//!
//! Coverage of `RecipeStore::delete_profile` (the store method
//! behind the `recipe_delete` IPC in src-tauri/src/main.rs).
//! Per the `astroforge-cr-slices` skill: `src-tauri` is a
//! binary-only crate outside the cargo workspace; the
//! behavioral tests live in `astroforge-core`.
//!
//! The IPC contract being pinned:
//!
//! - Delete removes EVERY version of the named profile
//!   (all branches, not just head).
//! - After delete, `list()` no longer contains the profile
//!   and `get()` / `get_head()` return NotFound.
//! - Deleting a profile that does not exist returns
//!   `Ok(0)` (idempotent, no pre-check needed).
//! - Sibling profiles (different name + target_type) are
//!   unaffected.

use astroforge_core::recipe::{IntegrityBadge, QualityProfile, Recipe};
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

fn in_memory_store() -> RecipeStore {
    let path = PathBuf::from(":memory:");
    RecipeStore::new(&path).expect("open in-memory recipe store")
}

#[test]
fn delete_removes_every_version_of_the_profile() {
    let store = in_memory_store();
    let source = sample_recipe("m42-natural", "stretch");
    let v1 = store.save(&source).expect("save v1");

    // Grow the profile to v2 (mirror of recipe_save's
    // next_version_for pattern).
    let mut v2 = store.get(&v1.profile_id, v1.version).expect("load v1");
    v2.version = 2;
    store.save(&v2).expect("save v2");
    assert_eq!(store.list().expect("list").len(), 1);

    let deleted = store
        .delete_profile(&v1.profile_id)
        .expect("delete profile");
    assert_eq!(deleted, 2, "both versions removed in one call");

    // The profile is gone from list + get + get_head.
    assert!(store.list().expect("list after delete").is_empty());
    assert!(store.get(&v1.profile_id, 1).is_err());
    assert!(store.get(&v1.profile_id, 2).is_err());
    assert!(store.get_head(&v1.profile_id).is_err());
}

#[test]
fn delete_is_idempotent_for_missing_profiles() {
    let store = in_memory_store();
    let ghost_id = RecipeStore::profile_id_for("ghost", "deep_sky");
    let deleted = store
        .delete_profile(&ghost_id)
        .expect("delete missing profile");
    assert_eq!(deleted, 0);
}

#[test]
fn delete_leaves_sibling_profiles_untouched() {
    let store = in_memory_store();
    let a = sample_recipe("m42-natural", "stretch");
    let b = sample_recipe("m51-wide", "mosaic");
    let sum_a = store.save(&a).expect("save a");
    let sum_b = store.save(&b).expect("save b");
    assert_ne!(sum_a.profile_id, sum_b.profile_id);

    let deleted = store.delete_profile(&sum_a.profile_id).expect("delete a");
    assert_eq!(deleted, 1);

    // B survives intact; the list has exactly one entry.
    let remaining = store.list().expect("list after delete");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].profile_id, sum_b.profile_id);
    let still_b = store.get(&sum_b.profile_id, sum_b.version).expect("load b");
    assert_eq!(still_b.name, "m51-wide");
}

#[test]
fn delete_then_recreate_starts_lineage_at_v1() {
    let store = in_memory_store();
    let source = sample_recipe("m42-natural", "stretch");
    let v1 = store.save(&source).expect("save v1");
    store
        .delete_profile(&v1.profile_id)
        .expect("delete profile");

    // Recreate the same profile name; the lineage restarts
    // at v1 because next_version_for sees no rows.
    let recreated = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recreated).expect("save recreated");
    assert_eq!(summary.version, 1);
    let next = store
        .next_version_for(&summary.profile_id)
        .expect("next_version_for recreated");
    assert_eq!(next, 2);
}
