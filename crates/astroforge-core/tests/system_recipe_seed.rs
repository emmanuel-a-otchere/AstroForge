//! CR-08 §3.2: system-recipe seed flow tests.
//!
//! Pinning the contract of `RecipeStore::seed_if_empty`:
//! first launch inserts both built-ins (DwarfII v1 +
//! M42-Natural-v1) and flips them to `is_system = 1`.
//! Re-running on a pre-existing DB is a no-op apart from
//! the system-flag sync (a pre-system-flag schema
//! migration does not leave the profile un-protected).

use astroforge_core::recipe_store::RecipeStore;
use std::path::PathBuf;

fn in_memory_store() -> RecipeStore {
    RecipeStore::new(&PathBuf::from(":memory:")).expect("open in-memory recipe store")
}

#[test]
fn first_launch_seeds_dwarfii_as_system() {
    let store = in_memory_store();
    store.seed_if_empty().expect("seed_if_empty first launch");

    let dwarf_id = RecipeStore::profile_id_for(
        astroforge_core::seed::DWARF2_V1_NAME,
        astroforge_core::seed::DWARF2_V1_TARGET_TYPE,
    );
    assert!(
        store
            .is_system_profile(&dwarf_id)
            .expect("is_system_profile dwarf"),
        "DwarfII v1 flagged as system Recipe"
    );
}

#[test]
fn first_launch_seeds_m42_as_system() {
    let store = in_memory_store();
    store.seed_if_empty().expect("seed_if_empty first launch");

    let m42_id = RecipeStore::profile_id_for(
        astroforge_core::seed::M42_NATURAL_V1_NAME,
        astroforge_core::seed::M42_NATURAL_V1_TARGET_TYPE,
    );
    assert!(
        store
            .is_system_profile(&m42_id)
            .expect("is_system_profile m42"),
        "M42-Natural-v1 flagged as system Recipe"
    );
}

#[test]
fn first_launch_seeds_both_profiles_with_one_version() {
    let store = in_memory_store();
    store.seed_if_empty().expect("seed_if_empty first launch");

    let dwarf_id = RecipeStore::profile_id_for(
        astroforge_core::seed::DWARF2_V1_NAME,
        astroforge_core::seed::DWARF2_V1_TARGET_TYPE,
    );
    let m42_id = RecipeStore::profile_id_for(
        astroforge_core::seed::M42_NATURAL_V1_NAME,
        astroforge_core::seed::M42_NATURAL_V1_TARGET_TYPE,
    );

    let dwarf_v1 = store.get(&dwarf_id, 1).expect("DwarfII v1 should exist");
    assert_eq!(dwarf_v1.version, 1);
    assert_eq!(dwarf_v1.branch, "main");
    assert!(dwarf_v1.is_system);

    let m42_v1 = store.get(&m42_id, 1).expect("M42 v1 should exist");
    assert_eq!(m42_v1.version, 1);
    assert_eq!(m42_v1.branch, "main");
    assert!(m42_v1.is_system);
}

#[test]
fn seed_if_empty_is_idempotent_on_repeat() {
    let store = in_memory_store();
    store.seed_if_empty().expect("seed_if_empty first launch");
    let dwarf_id = RecipeStore::profile_id_for(
        astroforge_core::seed::DWARF2_V1_NAME,
        astroforge_core::seed::DWARF2_V1_TARGET_TYPE,
    );
    let dwarf_v1 = store
        .get(&dwarf_id, 1)
        .expect("DwarfII v1 after first launch");

    // Second call must not insert a duplicate or change the
    // existing v1's content.
    store.seed_if_empty().expect("seed_if_empty second launch");
    let dwarf_v1_again = store
        .get(&dwarf_id, 1)
        .expect("DwarfII v1 after second launch");
    assert_eq!(dwarf_v1.version, dwarf_v1_again.version);
    assert_eq!(dwarf_v1.stages.len(), dwarf_v1_again.stages.len());
    assert!(store.is_system_profile(&dwarf_id).unwrap());
}

#[test]
fn seeded_builtins_refuse_user_save() {
    // End-to-end: a user trying to save a follow-up v2 into
    // a built-in profile (DwarfII) must hit the §3.1
    // SystemRecipeProtected error. This is the wire that
    // the §28 "System Recipes are protected from
    // modification" row promises.
    let store = in_memory_store();
    store.seed_if_empty().expect("seed_if_empty first launch");
    let dwarf_id = RecipeStore::profile_id_for(
        astroforge_core::seed::DWARF2_V1_NAME,
        astroforge_core::seed::DWARF2_V1_TARGET_TYPE,
    );

    let mut v2 = store.get(&dwarf_id, 1).expect("DwarfII v1");
    v2.version = 2;
    v2.parent_version = Some(1);
    let err = store.save(&v2).expect_err("save v2 into dwarf system");
    assert!(
        err.to_string().contains("system recipe"),
        "system guard fires on seed-built-in: {err}"
    );
}

#[test]
fn seeded_builtins_refuse_user_delete() {
    let store = in_memory_store();
    store.seed_if_empty().expect("seed_if_empty first launch");
    let m42_id = RecipeStore::profile_id_for(
        astroforge_core::seed::M42_NATURAL_V1_NAME,
        astroforge_core::seed::M42_NATURAL_V1_TARGET_TYPE,
    );
    let err = store
        .delete_profile(&m42_id)
        .expect_err("delete on m42 system");
    assert!(
        err.to_string().contains("system recipe"),
        "delete guard fires on seed-built-in: {err}"
    );
}

#[test]
fn seed_upgrade_path_flips_pre_existing_builtin_to_system() {
    // Simulate a pre-system-flag DB: save DwarfII v1
    // directly (without is_system) then run seed_if_empty.
    // The seed path must detect the pre-existing profile
    // and flip it to is_system = 1.
    let store = in_memory_store();
    let dwarf_recipe = astroforge_core::seed::dwarf2_v1();
    store.save(&dwarf_recipe).expect("save dwarf v1 raw");

    let dwarf_id = RecipeStore::profile_id_for(
        astroforge_core::seed::DWARF2_V1_NAME,
        astroforge_core::seed::DWARF2_V1_TARGET_TYPE,
    );
    assert!(
        !store.is_system_profile(&dwarf_id).unwrap(),
        "pre-seed: not a system Recipe"
    );

    store.seed_if_empty().expect("seed_if_empty upgrade");
    assert!(
        store.is_system_profile(&dwarf_id).unwrap(),
        "post-seed: system flag flipped"
    );
}
