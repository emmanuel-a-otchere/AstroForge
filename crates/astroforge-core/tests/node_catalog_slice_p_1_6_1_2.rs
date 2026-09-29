//! CR-10 P1.6.1.2 / Slice P1.6.1.2 tests:
//! NodeCatalog snapshot + cross-source consistency.
//!
//! These tests pin:
//! 1. The checked-in `node_catalog.json` matches
//!    the in-Rust `node_catalog()` output (snapshot
//!    test).
//! 2. The catalog version field is current.
//! 3. Every entry in the manifest has the fields
//!    the TS side will consume
//!    (`stage_type`, `label`, `description`,
//!    `ai_uses_ai`, `ai_model_id`, `destructive`,
//!    `produces_image_version`, `undo_supported`,
//!    `default_params`, `supported_models`).
//! 4. The 12-variant user-facing stage-type list
//!    in the manifest matches the
//!    `USER_FACING_STAGE_TYPES` constant.
//! 5. The catalog order matches the user-facing
//!    stage-type order (TS-pipeline source order).
//! 6. The manifest is idempotent (re-emit produces
//!    identical bytes; the diff is clean).
//! 7. The catalog is loadable from disk via
//!    `load_catalog_from_path` (the production
//!    reader path the TS bridge will call once
//!    P1.6.1.3 lands).
//! 8. The 7.7.9 mapping table reconciles every
//!    user-facing type to a canonical type that
//!    `AiBoundaryLabel::for_stage_type` knows
//!    about.

use astroforge_core::node_catalog::{
    emit_catalog_json, load_catalog_from_str, node_catalog, CANONICAL_STAGE_TYPES,
    USER_FACING_STAGE_TYPES,
};
use std::path::Path;

fn manifest_path() -> std::path::PathBuf {
    // The manifest is checked in next to the
    // node_catalog.rs module. The integration
    // test runs in `crates/astroforge-core/`.
    let crate_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set during cargo test");
    Path::new(&crate_dir).join("src/node_catalog.json")
}

#[test]
fn checked_in_manifest_matches_in_rust_source() {
    let from_disk_raw = std::fs::read_to_string(manifest_path())
        .expect("checked-in manifest exists at crates/astroforge-core/src/node_catalog.json");
    let from_source_raw = emit_catalog_json();
    // Strip trailing whitespace so a final-newline
    // difference (the `cargo run > file` redirect
    // adds one) does not produce false drift.
    let from_disk = from_disk_raw.trim_end();
    let from_source = from_source_raw.trim_end();
    if from_disk != from_source {
        panic!(
            "checked-in node_catalog.json has drifted from the Rust source.\n\
             disk_len={}, src_len={}\n\
             Regenerate via: cargo run -p astroforge-core --example emit_node_catalog > src/node_catalog.json",
            from_disk.len(),
            from_source.len()
        );
    }
}

#[test]
fn manifest_has_current_version() {
    let raw = std::fs::read_to_string(manifest_path()).expect("manifest exists");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("manifest parses as JSON");
    let version = parsed
        .get("version")
        .and_then(|v| v.as_u64())
        .expect("manifest has numeric version");
    use astroforge_core::node_catalog::CATALOG_VERSION;
    assert_eq!(
        version as u32, CATALOG_VERSION,
        "manifest version must equal CATALOG_VERSION"
    );
}

#[test]
fn every_manifest_entry_has_required_fields() {
    let raw = std::fs::read_to_string(manifest_path()).expect("manifest exists");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("manifest parses as JSON");
    let entries = parsed
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("manifest has entries array");
    for entry in entries {
        let stage_type = entry
            .get("stage_type")
            .and_then(|v| v.as_str())
            .unwrap_or("<missing>");
        for field in [
            "stage_type",
            "label",
            "description",
            "ai_uses_ai",
            "ai_model_id",
            "destructive",
            "produces_image_version",
            "undo_supported",
            "default_params",
            "supported_models",
        ] {
            assert!(
                entry.get(field).is_some(),
                "entry {stage_type} missing required field `{field}`"
            );
        }
        let label = entry.get("label").and_then(|v| v.as_str()).unwrap_or("");
        let description = entry
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(!label.is_empty(), "{stage_type} label is empty");
        assert!(!description.is_empty(), "{stage_type} description is empty");
    }
}

#[test]
fn manifest_stage_types_match_user_facing_constant() {
    let raw = std::fs::read_to_string(manifest_path()).expect("manifest exists");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("manifest parses as JSON");
    let entries = parsed
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("manifest has entries array");
    let manifest_types: Vec<&str> = entries
        .iter()
        .map(|e| {
            e.get("stage_type")
                .and_then(|v| v.as_str())
                .expect("stage_type is a string")
        })
        .collect();
    let constant_types: Vec<&str> = USER_FACING_STAGE_TYPES.to_vec();
    assert_eq!(
        manifest_types, constant_types,
        "manifest stage-type order must match USER_FACING_STAGE_TYPES"
    );
}

#[test]
fn manifest_is_idempotent_under_re_emit() {
    let raw = std::fs::read_to_string(manifest_path()).expect("manifest exists");
    let parsed = load_catalog_from_str(&raw).expect("manifest loads");
    let re_emitted = serde_json::to_string_pretty(&parsed).expect("re-emit");
    let raw_trimmed = raw.trim_end();
    let re_trimmed = re_emitted.trim_end();
    assert_eq!(raw_trimmed, re_trimmed, "manifest is not idempotent");
}

#[test]
fn manifest_is_loadable_from_disk() {
    let raw = std::fs::read_to_string(manifest_path()).expect("manifest exists");
    let parsed = load_catalog_from_str(&raw).expect("manifest loads");
    assert!(!parsed.entries.is_empty());
    assert!(parsed.entries.iter().all(|e| !e.stage_type.is_empty()));
}

#[test]
fn catalog_in_rust_matches_manifest() {
    let raw = std::fs::read_to_string(manifest_path()).expect("manifest exists");
    let from_disk = load_catalog_from_str(&raw).expect("manifest loads");
    let from_source = node_catalog();
    assert_eq!(
        from_disk.entries.len(),
        from_source.entries.len(),
        "manifest entry count must equal in-Rust count"
    );
    for (a, b) in from_disk.entries.iter().zip(from_source.entries.iter()) {
        assert_eq!(a, b, "entry drift in {}", a.stage_type);
    }
}

#[test]
fn every_user_facing_type_maps_to_known_canonical_type() {
    for ty in USER_FACING_STAGE_TYPES {
        let canonical = astroforge_core::node_catalog::user_facing_to_canonical(ty);
        assert!(
            CANONICAL_STAGE_TYPES.contains(&canonical),
            "user-facing {ty} maps to unknown canonical {canonical}"
        );
    }
}

#[test]
fn no_duplicate_stage_types_in_manifest() {
    let raw = std::fs::read_to_string(manifest_path()).expect("manifest exists");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("manifest parses as JSON");
    let entries = parsed
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("manifest has entries array");
    let mut seen = std::collections::HashSet::new();
    for entry in entries {
        let ty = entry
            .get("stage_type")
            .and_then(|v| v.as_str())
            .expect("stage_type is a string");
        assert!(seen.insert(ty), "duplicate stage_type in manifest: {ty}");
    }
}

#[test]
fn supported_models_only_populated_for_ai_stages() {
    let raw = std::fs::read_to_string(manifest_path()).expect("manifest exists");
    let parsed = load_catalog_from_str(&raw).expect("manifest loads");
    for entry in &parsed.entries {
        if entry.ai_uses_ai {
            assert!(
                !entry.supported_models.is_empty(),
                "AI stage {} has empty supported_models",
                entry.stage_type
            );
        } else {
            assert!(
                entry.supported_models.is_empty(),
                "classical stage {} has populated supported_models",
                entry.stage_type
            );
        }
    }
}
