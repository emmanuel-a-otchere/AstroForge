//! CR-10 P1.6.4.1 / P1.6.4.2 — the Graph ↔ `.astroforge-recipe`
//! round-trip contract, over the real builtin recipe → plan flow:
//!
//! 1. plan → recipe JSON (sanitized per §11.2)
//! 2. recipe JSON → materialized plan (catalog-resolved stages)
//! 3. unknown-stage diagnostic carries the pipeline entry index
//! 4. AI stages set `perceptual_models_used`
//! 5. re-materializing the same recipe yields the same plan id

use astroforge_core::node_graph::recipe_io::{plan_to_recipe_json, recipe_to_plan, RecipeIoError};
use astroforge_core::pipeline_plan::builtin::deep_sky_osc_balanced;
use astroforge_core::pipeline_plan::{
    derive_understanding_from_session, generate_plan, now_unix_ms, GenerationContext,
};

fn sample_plan() -> astroforge_core::domain::PipelinePlan {
    let understanding = derive_understanding_from_session(None, &[]);
    let (stages, target) = deep_sky_osc_balanced();
    generate_plan(
        &GenerationContext {
            project_id: "proj_rt".into(),
            session_id: "sess_rt".into(),
            session_understanding: understanding,
            recipe_id: Some("recipe_deep_sky_osc_balanced".into()),
            mode: "auto".into(),
            generated_at_unix_ms: now_unix_ms(),
        },
        &stages,
        target,
    )
    .expect("plan")
}

/// P1.6.4.1 — a real plan serializes to the §11.1 shape: versioned,
/// target-typed, with a `pipeline[]` of `{stage, params}` entries in
/// sequence order.
#[test]
fn plan_serializes_to_recipe_shape() {
    let plan = sample_plan();
    let json = plan_to_recipe_json(&plan, "0.1.0").expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(v["recipe_version"], "1.0");
    assert_eq!(v["app_version"], "0.1.0");
    assert_eq!(v["target_type"], "deep_sky");
    let pipeline = v["pipeline"].as_array().expect("pipeline array");
    assert!(!pipeline.is_empty(), "builtin plan has stages");
    for entry in pipeline {
        assert!(
            entry["stage"].is_string(),
            "every entry names its stage: {entry}"
        );
        assert!(
            entry["params"].is_object(),
            "params always an object: {entry}"
        );
    }
    // Sequence order preserved: first entry is the first stage.
    let first = plan
        .stages
        .iter()
        .min_by_key(|s| s.sequence)
        .expect("stage");
    assert_eq!(pipeline[0]["stage"], first.stage_type);
}

/// §11.2 — params keys touching paths / timestamps / machine info
/// are stripped before sharing (recursively, case-insensitive).
#[test]
fn sanitize_strips_paths_timestamps_and_machine_keys() {
    let mut plan = sample_plan();
    if let Some(stage) = plan.stages.first_mut() {
        stage.parameters_json = Some(
            r#"{"strength":0.7,"source_path":"/Users/sky/secret.fits","GPUModel":"M3 Max",
               "DateTimeObs":"2026-11-13","nested":{"home_dir":"/Users/sky","keep":1}}"#
                .into(),
        );
    }
    let json = plan_to_recipe_json(&plan, "0.1.0").expect("serialize");
    assert!(
        !json.contains("secret.fits"),
        "paths must be stripped: {json}"
    );
    assert!(!json.contains("M3 Max"), "machine info must be stripped");
    assert!(!json.contains("2026-11-13"), "timestamps must be stripped");
    assert!(json.contains("\"strength\""), "real params survive");
    assert!(
        json.contains("\"keep\""),
        "nested non-sensitive params survive"
    );
}

/// P1.6.4.2 — the serialized plan materializes back with the same
/// stage types in the same order, sequences compacted from the
/// pipeline order, and sanitized params preserved.
#[test]
fn round_trip_preserves_stage_types_and_order() {
    let plan = sample_plan();
    let json = plan_to_recipe_json(&plan, "0.1.0").expect("serialize");
    let rematerialized = recipe_to_plan(&json, "proj_rt2", "sess_rt2").expect("parse");

    let mut original: Vec<&astroforge_core::domain::PipelineStage> = plan.stages.iter().collect();
    original.sort_by_key(|s| s.sequence);
    assert_eq!(
        original.len(),
        rematerialized.stages.len(),
        "round trip keeps every stage"
    );
    for (a, b) in original.iter().zip(rematerialized.stages.iter()) {
        assert_eq!(a.stage_type, b.stage_type, "type preserved");
        assert_eq!(a.sequence, b.sequence, "sequences preserved");
    }
    // Sequences are 0..n in pipeline order.
    for (i, s) in rematerialized.stages.iter().enumerate() {
        assert_eq!(s.sequence, i as u32);
    }
    // The materialized plan targets the NEW project/session, is
    // Ready, and is NOT persisted anywhere (status + ids assert).
    assert_eq!(rematerialized.project_id, "proj_rt2");
    assert_eq!(rematerialized.session_id, "sess_rt2");
    assert!(matches!(
        rematerialized.status,
        astroforge_core::domain::PipelinePlanStatus::Ready
    ));
}

/// The user-facing vocabulary is accepted too: a recipe written in
/// user-facing types (`crop_rotate`, `star_handling`) materializes
/// into the canonical stages (`crop`, `star_handling`).
#[test]
fn user_facing_stage_names_resolve_to_canonical() {
    let recipe = r#"{
        "recipe_version": "1.0",
        "app_version": "0.1.0",
        "name": "User-facing graph",
        "target_type": "deep_sky",
        "pipeline": [
            { "stage": "crop_rotate", "params": { "width": 512 } },
            { "stage": "star_handling", "params": { "mode": "sharpen" } }
        ],
        "integrity": { "perceptual_models_used": false }
    }"#;
    let plan = recipe_to_plan(recipe, "proj_u", "sess_u").expect("parse");
    assert_eq!(plan.stages[0].stage_type, "crop");
    assert_eq!(plan.stages[1].stage_type, "star_handling");
    assert_eq!(plan.stages[0].label, "Crop");
}

/// The diagnostic carries the offending entry's index (P1.6.4.2).
#[test]
fn unknown_stage_reports_index_and_name() {
    let recipe = r#"{
        "recipe_version": "1.0",
        "app_version": "0.1.0",
        "name": "Bad",
        "target_type": "deep_sky",
        "pipeline": [
            { "stage": "stretch", "params": {} },
            { "stage": "teleport", "params": {} }
        ],
        "integrity": { "perceptual_models_used": false }
    }"#;
    let err = recipe_to_plan(recipe, "p", "s").unwrap_err();
    let msg = err.to_string();
    match err {
        RecipeIoError::UnknownStage { index, stage } => {
            assert_eq!(index, 1);
            assert_eq!(stage, "teleport");
        }
        other => panic!("expected UnknownStage, got {other:?}"),
    }
    // The message surfaces both, per the IPC error contract.
    assert!(msg.contains("teleport") && msg.contains("1"), "{msg}");
}

/// AI-carrying stages flip `perceptual_models_used` so gallery
/// consumers can gate integrity without re-deriving the taxonomy.
#[test]
fn ai_stage_sets_perceptual_flag() {
    let recipe = r#"{
        "recipe_version": "1.0",
        "app_version": "0.1.0",
        "name": "AI graph",
        "target_type": "deep_sky",
        "pipeline": [
            { "stage": "stretch", "params": {} },
            { "stage": "denoise", "params": {} }
        ],
        "integrity": { "perceptual_models_used": false }
    }"#;
    let json = plan_to_recipe_json(&recipe_to_plan(recipe, "p", "s").expect("parse"), "0.1.0")
        .expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(v["integrity"]["perceptual_models_used"], true);
}

/// Re-materializing the same recipe yields the same deterministic
/// plan id (a re-import cannot duplicate graphs silently), while a
/// different session targets a different id.
#[test]
fn plan_id_is_deterministic_per_project_and_session() {
    let recipe = r#"{
        "recipe_version": "1.0",
        "app_version": "0.1.0",
        "name": "Same",
        "target_type": "deep_sky",
        "pipeline": [ { "stage": "stretch", "params": {} } ],
        "integrity": { "perceptual_models_used": false }
    }"#;
    let a = recipe_to_plan(recipe, "proj_d", "sess_d").expect("a");
    let b = recipe_to_plan(recipe, "proj_d", "sess_d").expect("b");
    assert_eq!(a.plan_id, b.plan_id, "same inputs, same id");
    let c = recipe_to_plan(recipe, "proj_d", "sess_other").expect("c");
    assert_ne!(a.plan_id, c.plan_id, "different session, different id");
}

/// P1.6.4.3 — the session-override semantics: pin a plan, read it
/// back, reset materializes the project default for the session,
/// and the reset is idempotent on an unchanged default.
#[test]
fn session_overrides_pin_read_and_reset() {
    use astroforge_core::node_graph::recipe_io::reset_session_to_project_default;
    use astroforge_core::pipeline_plans_store::PipelinePlanStore;

    let store = PipelinePlanStore::in_memory().expect("store");
    // Two plans in the project; the NEWEST is the default.
    let mut older = sample_plan();
    older.plan_id = "plan_older".into();
    older.created_at = "2026-11-13T00:00:00Z".into();
    let mut newer = sample_plan();
    newer.plan_id = "plan_newer".into();
    newer.project_id = older.project_id.clone();
    newer.created_at = "2026-11-14T00:00:00Z".into();
    store.insert_plan(&older).expect("insert older");
    store.insert_plan(&newer).expect("insert newer");

    // No override yet: the session follows the default.
    assert_eq!(
        store
            .get_graph_override(&older.project_id, "sess_ovr")
            .expect("read"),
        None
    );

    // Pin the older plan as the session's override.
    store
        .set_graph_override(&older.project_id, "sess_ovr", "plan_older")
        .expect("pin");
    assert_eq!(
        store
            .get_graph_override(&older.project_id, "sess_ovr")
            .expect("read"),
        Some("plan_older".into())
    );
    // Re-pinning replaces the row.
    store
        .set_graph_override(&older.project_id, "sess_ovr", "plan_newer")
        .expect("re-pin");
    assert_eq!(
        store
            .get_graph_override(&older.project_id, "sess_ovr")
            .expect("read"),
        Some("plan_newer".into())
    );

    // Reset materializes the DEFAULT (newest = plan_newer's graph)
    // for the session and clears the override.
    let reset =
        reset_session_to_project_default(&store, &older.project_id, "sess_ovr").expect("reset");
    assert_eq!(reset.project_id, older.project_id);
    assert_eq!(reset.session_id, "sess_ovr");
    assert!(
        reset
            .stages
            .iter()
            .zip(newer.stages.iter())
            .all(|(a, b)| a.stage_type == b.stage_type),
        "reset carries the default graph's stage types"
    );
    assert_eq!(
        store
            .get_graph_override(&older.project_id, "sess_ovr")
            .expect("read"),
        None,
        "reset clears the override row"
    );

    // Idempotent: resetting again rewrites the same plan rows.
    let again = reset_session_to_project_default(&store, &older.project_id, "sess_ovr")
        .expect("reset again");
    assert_eq!(reset.plan_id, again.plan_id, "deterministic plan id");
}

/// Reset without any plan in the project surfaces the honest error.
#[test]
fn reset_without_plans_fails_with_no_default() {
    use astroforge_core::node_graph::recipe_io::{reset_session_to_project_default, RecipeIoError};
    use astroforge_core::pipeline_plans_store::PipelinePlanStore;

    let store = PipelinePlanStore::in_memory().expect("store");
    let err = reset_session_to_project_default(&store, "proj_empty", "sess_e").unwrap_err();
    match err {
        RecipeIoError::NoDefaultGraph(id) => assert_eq!(id, "proj_empty"),
        other => panic!("expected NoDefaultGraph, got {other:?}"),
    }
}
