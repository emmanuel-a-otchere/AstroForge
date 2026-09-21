//! CR-08 §14: "Save Pipeline as Recipe" integration tests.
//!
//! Pinning the contract of `recipe_from_pipeline_plan`:
//! the pure-function conversion the
//! `recipe_save_from_pipeline_plan` IPC delegates to.
//!
//! Coverage:
//! - Empty plan -> empty Recipe (with default fields).
//! - Plan with N stages -> Recipe with N stages.
//! - `parameters_json` (JSON object) is parsed into the
//!   Recipe stage's `params` HashMap.
//! - Plan `enabled = false` survives: the Recipe's
//!   stage.enabled mirrors the plan's flag (not always
//!   true).
//! - `target_type` is derived from the plan when the
//!   caller passes None; the IPC path passes through a
//!   caller-supplied override when provided.
//! - `quality_profile` is `Natural` (matches the §14
//!   spec's "balanced default" decision).
//! - `is_system` is false (a user-saved Recipe is
//!   never auto-flagged as a system Recipe).
//! - `description` carries the plan_id + session_id for
//!   provenance.

use astroforge_core::domain::{ObjectType, PipelinePlan, PipelinePlanStatus, PipelineStage};
use astroforge_core::recipe::{recipe_from_pipeline_plan, QualityProfile};
use std::collections::HashMap;

fn empty_plan() -> PipelinePlan {
    PipelinePlan {
        plan_id: "plan-test-1".into(),
        project_id: "proj-test".into(),
        session_id: "sess-test".into(),
        recipe_id: None,
        mode: "auto".into(),
        target_type: ObjectType::DeepSky,
        status: PipelinePlanStatus::Completed,
        created_at: "2026-09-21T00:00:00Z".into(),
        schema_version: 1,
        stages: vec![],
    }
}

fn make_stage(
    stage_id: &str,
    enabled: bool,
    params: Option<HashMap<String, serde_json::Value>>,
) -> PipelineStage {
    PipelineStage {
        stage_id: stage_id.into(),
        plan_id: "plan-test-1".into(),
        stage_type: "default".into(),
        sequence: 0,
        label: stage_id.into(),
        required: false,
        enabled,
        parameters_json: params.map(|p| serde_json::to_string(&p).unwrap()),
        produces_image_version: false,
        undo_supported: false,
    }
}

#[test]
fn empty_plan_yields_empty_recipe_with_default_fields() {
    let plan = empty_plan();
    let recipe = recipe_from_pipeline_plan(&plan, "M42 Final", None);

    assert_eq!(recipe.name, "M42 Final");
    // ObjectType::DeepSky serializes as "deep_sky" via serde_json
    // (rename_all = "snake_case"). The conversion falls back
    // to the plan's serialized form when target_type is None.
    assert_eq!(recipe.target_type, "deep_sky");
    assert_eq!(recipe.quality_profile, QualityProfile::Natural);
    assert_eq!(recipe.version, 1);
    assert_eq!(recipe.parent_version, None);
    assert_eq!(recipe.branch, "main");
    assert!(!recipe.is_system, "user Recipe not auto-flagged");
    assert!(recipe.required_models.is_empty());
    assert!(recipe.flags.is_empty());
    assert!(recipe.stages.is_empty());
    // description carries provenance
    assert!(recipe.description.contains("plan-test-1"));
    assert!(recipe.description.contains("sess-test"));
}

#[test]
fn plan_with_stages_preserves_order_and_count() {
    let mut plan = empty_plan();
    let mut p1 = HashMap::new();
    p1.insert("blackPoint".into(), serde_json::json!(0.02));
    plan.stages
        .push(make_stage("background_extraction", true, Some(p1.clone())));
    let mut p2 = HashMap::new();
    p2.insert("midtone".into(), serde_json::json!(0.45));
    plan.stages
        .push(make_stage("stretch", true, Some(p2.clone())));
    let mut p3 = HashMap::new();
    p3.insert("haOnly".into(), serde_json::json!(true));
    plan.stages
        .push(make_stage("creative_polish", true, Some(p3.clone())));

    let recipe = recipe_from_pipeline_plan(&plan, "M42 Final", None);

    assert_eq!(recipe.stages.len(), 3);
    let ids: Vec<&str> = recipe.stages.iter().map(|s| s.stage_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["background_extraction", "stretch", "creative_polish"]
    );

    // params survive the round-trip.
    assert_eq!(
        recipe.stages[0].params.get("blackPoint"),
        Some(&serde_json::json!(0.02))
    );
    assert_eq!(
        recipe.stages[1].params.get("midtone"),
        Some(&serde_json::json!(0.45))
    );
    assert_eq!(
        recipe.stages[2].params.get("haOnly"),
        Some(&serde_json::json!(true))
    );
}

#[test]
fn disabled_plan_stage_stays_disabled_in_recipe() {
    // CR-08 §14: a disabled PipelineStage should NOT be
    // silently flipped to enabled=true by the conversion.
    // The earlier `add_stage` always-true path is
    // patched by post-mutating `last.enabled = *enabled`.
    let mut plan = empty_plan();
    plan.stages
        .push(make_stage("creative_polish", false, Some(HashMap::new())));

    let recipe = recipe_from_pipeline_plan(&plan, "M42 Quiet", None);
    assert_eq!(recipe.stages.len(), 1);
    assert!(
        !recipe.stages[0].enabled,
        "disabled plan -> disabled Recipe"
    );
}

#[test]
fn missing_or_unparseable_parameters_json_falls_back_to_empty() {
    let mut plan = empty_plan();
    plan.stages.push(make_stage("ingest", true, None));
    plan.stages
        .push(make_stage("denoise", true, Some(HashMap::new())));
    plan.stages
        .push(make_stage("color_wb", true, Some(HashMap::new())));

    let recipe = recipe_from_pipeline_plan(&plan, "M42 Tolerant", None);
    assert_eq!(recipe.stages.len(), 3);
    // All three params HashMaps should be empty (None,
    // empty object, and a fresh empty object).
    for stage in &recipe.stages {
        assert!(
            stage.params.is_empty(),
            "missing/unparseable parameters_json must not panic; got {:?}",
            stage.params
        );
    }
}

#[test]
fn target_type_override_replaces_plan_default() {
    let plan = empty_plan(); // ObjectType::DeepSky -> "deep_sky"
    let recipe = recipe_from_pipeline_plan(&plan, "DwarfII Final", Some("smart_telescope_osc"));
    assert_eq!(recipe.target_type, "smart_telescope_osc");
}

#[test]
fn empty_target_type_override_falls_back_to_plan() {
    let plan = empty_plan();
    // Empty string is treated as "not provided".
    let recipe = recipe_from_pipeline_plan(&plan, "M42 Final", Some(""));
    assert_eq!(recipe.target_type, "deep_sky");
}

#[test]
fn recipe_is_not_auto_marked_as_system() {
    let mut plan = empty_plan();
    plan.stages
        .push(make_stage("stretch", true, Some(HashMap::new())));
    let recipe = recipe_from_pipeline_plan(&plan, "M42 Final", None);
    assert!(
        !recipe.is_system,
        "user Recipe must not auto-flip is_system; that is the mark_as_system IPC's job"
    );
}
