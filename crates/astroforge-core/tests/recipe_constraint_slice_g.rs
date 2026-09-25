//! CR-08 §21 / Slice G tests: Recipe constraint +
//! resource policy + provenance record types.
//!
//! These tests pin the contract of the three new
//! types added in Slice G:
//! - `RecipeConstraint` + `RecipeConstraintKind`:
//!   `param_range` + `stage_dependency` +
//!   `order_constraint` constructors; serde
//!   round-trip; `description` formatting.
//! - `RecipeResourcePolicy`: `unbounded()` +
//!   `is_unbounded()`; serde round-trip.
//! - `ProvenanceRecord` + `provenance_record()`
//!   pure constructor: empty chain, populated
//!   chain, Recipe resolution with / without
//!   Recipe (None case); serde round-trip.
//!
//! The tests are pure functions (no IO, no
//! DomainStore, no Tauri runtime). They live
//! alongside the §22 round 2 + Slice F tests as
//! the canonical contract pin.

use astroforge_core::domain::{AiOperation, AiSafetyClassification, ImageVersion, StageRunRecord};
use astroforge_core::recipe::{
    provenance_record, ProvenanceRecord, RecipeConstraint, RecipeConstraintKind, RecipeProvenance,
    RecipeResourcePolicy, ReproducibilityVerdictLite,
};
use astroforge_core::recipe_store::RecipeStore;

fn base_image_version() -> ImageVersion {
    ImageVersion {
        version_id: "ver_test".to_string(),
        project_id: "proj_test".to_string(),
        label: "Test".to_string(),
        sequence: 1,
        primary_artifact_id: "art_test".to_string(),
        source_version_id: None,
        created_at: "2026-09-25T00:00:00Z".to_string(),
        hidden: false,
        recipe_id: Some("prof_m42_deep_sky".to_string()),
        recipe_version: Some(1),
        recipe_hash: Some("abc123".to_string()),
    }
}

fn base_recipe() -> astroforge_core::recipe::Recipe {
    let mut r = astroforge_core::recipe::Recipe::new("M42", "deep_sky");
    r.version = 1;
    r.description = "M42 natural".to_string();
    r
}

fn base_stage_run(stage_id: &str, attempt: u32, status: &str) -> StageRunRecord {
    StageRunRecord {
        stage_run_id: format!("sr_{stage_id}_{attempt}"),
        run_id: "run_test".to_string(),
        stage_id: stage_id.to_string(),
        status: status.to_string(),
        attempt,
        params_json: None,
        metrics_json: None,
        error: None,
        started_at: Some("2026-09-25T00:00:00Z".to_string()),
        completed_at: Some("2026-09-25T00:01:00Z".to_string()),
    }
}

fn base_ai_op(op_id: &str, model_id: &str) -> AiOperation {
    AiOperation {
        operation_id: op_id.to_string(),
        stage_run_id: "sr_stretch_1".to_string(),
        model_id: model_id.to_string(),
        model_version: "v1".to_string(),
        model_hash: None,
        runtime: Some("native".to_string()),
        backend: Some("cpu".to_string()),
        precision: None,
        parameters_json: None,
        seed: None,
        deterministic: true,
        safety_classification: AiSafetyClassification::Deterministic,
        experimental: false,
        input_artifact_id: Some("art_in".to_string()),
        output_artifact_id: Some("art_out".to_string()),
        engine_version: Some("astroforge-ai-0.1.0".to_string()),
        tile_configuration: None,
        resource_metrics: None,
    }
}

// ─── RecipeConstraint tests ──────────────────────────────────────────────

#[test]
fn recipe_constraint_param_range_constructor() {
    let c = RecipeConstraint::param_range("stretch", "bias", 0.0, 1.0);
    match &c.constraint {
        RecipeConstraintKind::ParamRange {
            stage_id,
            param,
            min,
            max,
        } => {
            assert_eq!(stage_id, "stretch");
            assert_eq!(param, "bias");
            assert_eq!(*min, 0.0);
            assert_eq!(*max, 1.0);
        }
        _ => panic!("expected ParamRange variant"),
    }
    assert_eq!(c.description, "stretch.bias in [0, 1]");
    assert_eq!(c.source, "user");
}

#[test]
fn recipe_constraint_stage_dependency_constructor() {
    let c = RecipeConstraint::stage_dependency("color_calibration", "debayer");
    match &c.constraint {
        RecipeConstraintKind::StageDependency {
            stage_id,
            depends_on,
        } => {
            assert_eq!(stage_id, "color_calibration");
            assert_eq!(depends_on, "debayer");
        }
        _ => panic!("expected StageDependency variant"),
    }
    assert_eq!(c.description, "color_calibration requires debayer");
}

#[test]
fn recipe_constraint_order_constraint_constructor() {
    let c = RecipeConstraint::order_constraint("stretch", "denoise");
    match &c.constraint {
        RecipeConstraintKind::OrderConstraint { before, after } => {
            assert_eq!(before, "stretch");
            assert_eq!(after, "denoise");
        }
        _ => panic!("expected OrderConstraint variant"),
    }
    assert_eq!(c.description, "stretch before denoise");
}

#[test]
fn recipe_constraint_serde_round_trip() {
    let original = vec![
        RecipeConstraint::param_range("stretch", "bias", 0.0, 1.0),
        RecipeConstraint::stage_dependency("color_calibration", "debayer"),
        RecipeConstraint::order_constraint("stretch", "denoise"),
    ];
    let json = serde_json::to_string(&original).unwrap();
    let back: Vec<RecipeConstraint> = serde_json::from_str(&json).unwrap();
    assert_eq!(original, back);
}

#[test]
fn recipe_field_constraints_round_trip() {
    let mut recipe = base_recipe();
    recipe.constraints = vec![
        RecipeConstraint::param_range("stretch", "bias", 0.0, 1.0),
        RecipeConstraint::stage_dependency("color_calibration", "debayer"),
    ];
    let json = serde_json::to_string(&recipe).unwrap();
    let back: astroforge_core::recipe::Recipe = serde_json::from_str(&json).unwrap();
    assert_eq!(recipe.constraints, back.constraints);
}

// ─── RecipeResourcePolicy tests ──────────────────────────────────────────

#[test]
fn recipe_resource_policy_unbounded() {
    let p = RecipeResourcePolicy::unbounded();
    assert!(p.is_unbounded());
    assert_eq!(p.max_cpu_units, None);
    assert_eq!(p.max_memory_mb, None);
    assert_eq!(p.max_disk_mb, None);
    assert_eq!(p.max_wall_clock_secs, None);
    assert_eq!(p.notes, "");
}

#[test]
fn recipe_resource_policy_with_caps_is_not_unbounded() {
    let mut p = RecipeResourcePolicy::unbounded();
    p.max_cpu_units = Some(300.0);
    p.max_memory_mb = Some(8192.0);
    assert!(!p.is_unbounded());
    assert_eq!(p.max_cpu_units, Some(300.0));
    assert_eq!(p.max_memory_mb, Some(8192.0));
}

#[test]
fn recipe_resource_policy_partial_caps_is_not_unbounded() {
    // Even a single cap flips the policy out of
    // "unbounded" mode (the apply round should
    // gate at least one dimension).
    let mut p = RecipeResourcePolicy::unbounded();
    p.max_wall_clock_secs = Some(60.0);
    assert!(!p.is_unbounded());
}

#[test]
fn recipe_resource_policy_serde_round_trip() {
    let mut p = RecipeResourcePolicy::unbounded();
    p.max_cpu_units = Some(300.0);
    p.max_memory_mb = Some(8192.0);
    p.max_disk_mb = Some(1024.0);
    p.max_wall_clock_secs = Some(120.0);
    p.notes = "M42 budget".to_string();

    let json = serde_json::to_string(&p).unwrap();
    let back: RecipeResourcePolicy = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn recipe_field_resource_policy_round_trip() {
    let mut recipe = base_recipe();
    let mut p = RecipeResourcePolicy::unbounded();
    p.max_cpu_units = Some(300.0);
    p.notes = "test".to_string();
    recipe.resource_policy = Some(p.clone());
    let json = serde_json::to_string(&recipe).unwrap();
    let back: astroforge_core::recipe::Recipe = serde_json::from_str(&json).unwrap();
    assert_eq!(recipe.resource_policy, back.resource_policy);
    assert_eq!(back.resource_policy.as_ref().unwrap().notes, "test");
}

// ─── ProvenanceRecord tests ──────────────────────────────────────────────

#[test]
fn provenance_record_empty_chain_yields_zero_counts() {
    let iv = base_image_version();
    let recipe = base_recipe();
    let stage_runs: Vec<StageRunRecord> = vec![];
    let ai_ops: Vec<AiOperation> = vec![];
    let record = provenance_record(
        &iv.version_id,
        &iv,
        Some(&recipe),
        &stage_runs,
        &ai_ops,
        ReproducibilityVerdictLite::Exact,
    );
    assert_eq!(record.version_id, "ver_test");
    assert_eq!(record.stage_runs, 0);
    assert_eq!(record.ai_operations, 0);
    assert!(record.summary_lines.is_empty());
    assert!(record.is_exact);
}

#[test]
fn provenance_record_populated_chain_counts_and_summarizes() {
    let iv = base_image_version();
    let recipe = base_recipe();
    let stage_runs = vec![
        base_stage_run("stretch", 1, "completed"),
        base_stage_run("denoise", 1, "completed"),
        base_stage_run("sharpen", 1, "failed"),
    ];
    let ai_ops = vec![
        base_ai_op("op_stretch_1", "model_a"),
        base_ai_op("op_denoise_1", "model_b"),
    ];
    let record = provenance_record(
        &iv.version_id,
        &iv,
        Some(&recipe),
        &stage_runs,
        &ai_ops,
        ReproducibilityVerdictLite::NotExact,
    );
    assert_eq!(record.stage_runs, 3);
    assert_eq!(record.ai_operations, 2);
    assert_eq!(record.summary_lines.len(), 5);
    assert!(record.summary_lines[0].contains("stretch"));
    assert!(record.summary_lines[1].contains("denoise"));
    assert!(record.summary_lines[2].contains("sharpen"));
    assert!(record.summary_lines[3].contains("ai_op"));
    assert!(record.summary_lines[4].contains("ai_op"));
    assert!(!record.is_exact);
}

#[test]
fn provenance_record_recipe_id_resolves_from_recipe() {
    let iv = base_image_version();
    let recipe = base_recipe();
    let record = provenance_record(
        &iv.version_id,
        &iv,
        Some(&recipe),
        &[],
        &[],
        ReproducibilityVerdictLite::Exact,
    );
    // The expected profile_id is computed from
    // (name, target_type) via `RecipeStore::profile_id_for`.
    let expected_pid = RecipeStore::profile_id_for(&recipe.name, &recipe.target_type);
    assert_eq!(record.recipe_id.as_deref(), Some(expected_pid.as_str()));
    assert_eq!(record.recipe_version, Some(1));
    // recipe_hash is the pipeline plan hash, which
    // is a 64-char lowercase SHA-256 for any
    // non-empty Recipe.
    assert!(record.recipe_hash.is_some());
    assert_eq!(record.recipe_hash.as_ref().unwrap().len(), 64);
}

#[test]
fn provenance_record_without_recipe_yields_none_identity() {
    let iv = base_image_version();
    let record = provenance_record(
        &iv.version_id,
        &iv,
        None,
        &[],
        &[],
        ReproducibilityVerdictLite::Exact,
    );
    assert_eq!(record.recipe_id, None);
    assert_eq!(record.recipe_version, None);
    assert_eq!(record.recipe_hash, None);
}

#[test]
fn provenance_record_serde_round_trip() {
    let iv = base_image_version();
    let recipe = base_recipe();
    let stage_runs = vec![base_stage_run("stretch", 1, "completed")];
    let ai_ops = vec![base_ai_op("op_stretch_1", "model_a")];
    let original = provenance_record(
        &iv.version_id,
        &iv,
        Some(&recipe),
        &stage_runs,
        &ai_ops,
        ReproducibilityVerdictLite::Exact,
    );
    let json = serde_json::to_string(&original).unwrap();
    let back: ProvenanceRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(original, back);
}

#[test]
fn provenance_record_ai_op_without_backend_renders_none() {
    let iv = base_image_version();
    let mut op = base_ai_op("op_stretch_1", "model_a");
    op.backend = None;
    let record = provenance_record(
        &iv.version_id,
        &iv,
        Some(&base_recipe()),
        &[],
        &[op],
        ReproducibilityVerdictLite::Exact,
    );
    assert!(record.summary_lines[0].contains("backend none"));
}

// ─── Recipe constraint integration with existing surfaces ────────────────

#[test]
fn recipe_provenance_does_not_crash_with_constraints() {
    // The existing `recipe_provenance()` surface
    // should still build cleanly when the Recipe
    // carries Slice G's `constraints` field (the
    // field is `#[serde(default)]` so legacy
    // callers that do not touch it still work).
    let mut recipe = base_recipe();
    recipe.constraints = vec![RecipeConstraint::param_range("stretch", "bias", 0.0, 1.0)];
    let profile_id = RecipeStore::profile_id_for(&recipe.name, &recipe.target_type);
    let prov: RecipeProvenance = astroforge_core::recipe::recipe_provenance(&profile_id, &recipe);
    assert_eq!(prov.profile_id, profile_id);
    assert_eq!(prov.version, 1);
}
