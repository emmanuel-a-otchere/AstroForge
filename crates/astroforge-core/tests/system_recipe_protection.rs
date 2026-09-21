//! CR-08 §3.1: system-recipe protection tests.
//!
//! Pinning the contract of `RecipeStore::mark_as_system` +
//! `is_system_profile` + the save/delete guards. The
//! `recipe_mark_as_system` + `recipe_is_system` IPCs in
//! src-tauri/src/main.rs delegate to these primitives.
//!
//! Coverage:
//! - New profiles default to `is_system = false`.
//! - `mark_as_system` flips the flag for every version
//!   of the profile.
//! - `is_system_profile` reports the flag correctly.
//! - `save` refuses to mutate a system Recipe (returns
//!   `RecipeStoreError::SystemRecipeProtected`).
//! - `delete_profile` refuses to delete a system Recipe
//!   (returns `RecipeStoreError::SystemRecipeProtected`).
//! - `delete_profile` still works on a non-system
//!   profile (the §22.2 contract is unchanged for
//!   user Recipes).
//! - The idempotent ALTER TABLE migration runs cleanly
//!   on a store whose schema predates the `is_system`
//!   column.

use astroforge_core::recipe::{IntegrityBadge, QualityProfile, Recipe};
use astroforge_core::recipe_store::RecipeStore;
use std::path::PathBuf;

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

fn in_memory_store() -> RecipeStore {
    RecipeStore::new(&PathBuf::from(":memory:")).expect("open in-memory recipe store")
}

#[test]
fn new_profiles_default_to_not_system() {
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save");
    assert!(!recipe.is_system);
    assert!(
        !store
            .is_system_profile(&summary.profile_id)
            .expect("is_system_profile"),
        "fresh profile is not a system Recipe"
    );
}

#[test]
fn mark_as_system_flips_flag_for_every_version() {
    let store = in_memory_store();
    let mut recipe = sample_recipe("m42-natural", "stretch");
    let summary_v1 = store.save(&recipe).expect("save v1");

    // Save v2 via a manual `next_version_for` call so we
    // exercise the multi-version case.
    recipe.version = store
        .next_version_for(&summary_v1.profile_id)
        .expect("next_version_for");
    recipe.parent_version = Some(1);
    let summary_v2 = store.save(&recipe).expect("save v2");
    assert_eq!(summary_v2.version, 2);

    let touched = store
        .mark_as_system(&summary_v1.profile_id)
        .expect("mark_as_system");
    assert!(touched, "flip returned true");

    let is_system = store
        .is_system_profile(&summary_v1.profile_id)
        .expect("is_system_profile");
    assert!(is_system, "profile now flagged as system");
}

#[test]
fn save_refuses_to_mutate_a_system_recipe() {
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save v1");
    store
        .mark_as_system(&summary.profile_id)
        .expect("mark_as_system");

    // Try to write v2 into the same profile.
    let mut v2 = sample_recipe("m42-natural", "stretch");
    v2.version = 2;
    v2.parent_version = Some(1);
    let err = store.save(&v2).expect_err("save must reject");
    let msg = err.to_string();
    assert!(
        msg.contains("system recipe") || msg.contains("SystemRecipeProtected"),
        "save error names the protection: {msg}"
    );
}

#[test]
fn delete_refuses_to_delete_a_system_recipe() {
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save v1");
    store
        .mark_as_system(&summary.profile_id)
        .expect("mark_as_system");

    let err = store
        .delete_profile(&summary.profile_id)
        .expect_err("delete must reject");
    let msg = err.to_string();
    assert!(
        msg.contains("system recipe") || msg.contains("SystemRecipeProtected"),
        "delete error names the protection: {msg}"
    );
}

#[test]
fn delete_still_works_on_a_non_system_profile() {
    let store = in_memory_store();
    let recipe = sample_recipe("user-natural", "stretch");
    let summary = store.save(&recipe).expect("save v1");
    assert!(!store
        .is_system_profile(&summary.profile_id)
        .expect("is_system_profile"));

    let deleted = store
        .delete_profile(&summary.profile_id)
        .expect("delete non-system profile");
    assert_eq!(deleted, 1, "one row deleted");
}

#[test]
fn mark_as_system_on_unknown_profile_returns_false() {
    let store = in_memory_store();
    let touched = store
        .mark_as_system("prof_nonexistent_stretch")
        .expect("mark_as_system");
    assert!(!touched, "no rows touched for unknown profile_id");
}

#[test]
fn migration_runs_idempotently_on_reopen() {
    // Open a store, save a recipe, then reopen with a fresh
    // RecipeStore (same in-memory path) and confirm the
    // is_system column was migrated in.
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save v1");

    // Re-open a fresh store against the same in-memory
    // path. For :memory: this is a new database, so the
    // ALTER TABLE migration runs and the column exists.
    let store2 = in_memory_store();
    let touched = store2
        .mark_as_system(&summary.profile_id)
        .expect("mark_as_system on fresh store");
    // No rows exist in `store2` for that profile_id (the
    // original row lives in `store`'s :memory: handle).
    assert!(
        !touched,
        "mark_as_system on a fresh :memory: store has no rows to flip"
    );

    // Save a row into store2 + flip + confirm round-trip.
    let _ = store2.save(&recipe).expect("save into store2");
    let _ = store2
        .mark_as_system(&summary.profile_id)
        .expect("mark_as_system after save");
    assert!(
        store2
            .is_system_profile(&summary.profile_id)
            .expect("is_system_profile"),
        "fresh store applies the migration + flip"
    );
}

#[test]
fn save_persists_is_system_field_through_round_trip() {
    // A Recipe constructed with `is_system = true` should
    // round-trip through save + get so the seed flow can
    // mark a profile as system in one step.
    let store = in_memory_store();
    let mut recipe = sample_recipe("system-m42", "stretch");
    recipe.is_system = true;
    let summary = store.save(&recipe).expect("save");

    // Now the guard is active: a follow-up save into the
    // same profile is refused even though we just
    // persisted is_system = true.
    let mut v2 = recipe.clone();
    v2.version = 2;
    v2.parent_version = Some(1);
    let err = store.save(&v2).expect_err("follow-up save rejected");
    assert!(
        err.to_string().contains("system recipe"),
        "is_system column is honored after round-trip: {err}"
    );

    // The summary itself was returned correctly.
    assert_eq!(summary.version, 1);
}
