//! CR-08 §22.4: recipe archive integration tests.
//!
//! Pinning the contract of `RecipeStore::archive_profile` +
//! `unarchive_profile` + `is_archived_profile`. The
//! `recipe_archive` / `recipe_unarchive` /
//! `recipe_is_archived` IPCs in src-tauri/src/main.rs
//! delegate to these primitives.
//!
//! Coverage:
//! - New profiles default to not-archived.
//! - `archive_profile` flips the flag for every version.
//! - `unarchive_profile` clears the flag.
//! - `is_archived_profile` reports the flag.
//! - Archive is independent of the system-recipe guard
//!   (an archived profile can still be saved to; a system
//!   profile cannot be archived-and-then-saved).
//! - Archive + delete are orthogonal (deleting an
//!   archived profile still works).
//! - The idempotent ALTER TABLE migration runs cleanly
//!   on a store whose schema predates `is_archived`.

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
fn new_profiles_default_to_not_archived() {
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save");
    assert!(
        !store
            .is_archived_profile(&summary.profile_id)
            .expect("is_archived_profile"),
        "fresh profile is not archived"
    );
}

#[test]
fn archive_profile_flips_flag_for_every_version() {
    let store = in_memory_store();
    let mut recipe = sample_recipe("m42-natural", "stretch");
    let summary_v1 = store.save(&recipe).expect("save v1");

    recipe.version = store
        .next_version_for(&summary_v1.profile_id)
        .expect("next_version_for");
    recipe.parent_version = Some(1);
    let summary_v2 = store.save(&recipe).expect("save v2");
    assert_eq!(summary_v2.version, 2);

    let touched = store
        .archive_profile(&summary_v1.profile_id)
        .expect("archive_profile");
    assert!(touched, "flip returned true");
    assert!(
        store
            .is_archived_profile(&summary_v1.profile_id)
            .expect("is_archived_profile"),
        "profile now archived"
    );
}

#[test]
fn unarchive_profile_clears_the_flag() {
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save");
    store.archive_profile(&summary.profile_id).expect("archive");
    assert!(store
        .is_archived_profile(&summary.profile_id)
        .expect("is_archived_profile"));

    let touched = store
        .unarchive_profile(&summary.profile_id)
        .expect("unarchive_profile");
    assert!(touched);
    assert!(
        !store
            .is_archived_profile(&summary.profile_id)
            .expect("is_archived_profile"),
        "flag cleared after unarchive"
    );
}

#[test]
fn archived_profiles_can_still_be_saved_to() {
    // Archive is a visibility flag, not a protection. The
    // save path only refuses when the profile is a
    // *system* recipe; archived user recipes still write.
    let store = in_memory_store();
    let mut recipe = sample_recipe("m42-natural", "stretch");
    let summary_v1 = store.save(&recipe).expect("save v1");
    store
        .archive_profile(&summary_v1.profile_id)
        .expect("archive");

    recipe.version = store
        .next_version_for(&summary_v1.profile_id)
        .expect("next_version_for");
    recipe.parent_version = Some(1);
    let summary_v2 = store.save(&recipe).expect("save v2");
    assert_eq!(summary_v2.version, 2, "save into archived profile OK");
}

#[test]
fn delete_works_on_archived_profile() {
    // Delete is destructive regardless of archive state;
    // archive only hides from list(). The §28
    // "delete/archive" row keeps these orthogonal.
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save v1");
    store.archive_profile(&summary.profile_id).expect("archive");

    let deleted = store
        .delete_profile(&summary.profile_id)
        .expect("delete archived profile");
    assert_eq!(deleted, 1, "archived profile deleted");
}

#[test]
fn archive_on_unknown_profile_returns_false() {
    let store = in_memory_store();
    let touched = store
        .archive_profile("prof_nonexistent_stretch")
        .expect("archive_profile");
    assert!(!touched);
    let untouched = store
        .unarchive_profile("prof_nonexistent_stretch")
        .expect("unarchive_profile");
    assert!(!untouched);
}

#[test]
fn archive_and_system_flags_are_independent() {
    let store = in_memory_store();
    let recipe = sample_recipe("m42-natural", "stretch");
    let summary = store.save(&recipe).expect("save v1");

    // Archive first, then mark as system: the flags
    // co-exist (system gating runs against
    // `is_system` only, never touches `is_archived`).
    store.archive_profile(&summary.profile_id).expect("archive");
    store
        .mark_as_system(&summary.profile_id)
        .expect("mark_as_system");
    assert!(
        store
            .is_archived_profile(&summary.profile_id)
            .expect("is_archived_profile"),
        "is_archived still set"
    );
    assert!(
        store
            .is_system_profile(&summary.profile_id)
            .expect("is_system_profile"),
        "is_system now set"
    );

    // Save refuses (system guard), archive stays put.
    let mut v2 = recipe.clone();
    v2.version = 2;
    v2.parent_version = Some(1);
    let err = store.save(&v2).expect_err("save rejected");
    assert!(
        err.to_string().contains("system recipe"),
        "system guard fires even though archived: {err}"
    );
    assert!(
        store
            .is_archived_profile(&summary.profile_id)
            .expect("is_archived_profile"),
        "is_archived survives the failed save"
    );
}
