//! CR-10 P1.6.2.4 / Slice P1.6.2.4: node graph mutation integration tests.
//!
//! Exercises `node_graph::mutation::insert_stage` end-to-end:
//! store + mutation primitive + reload.
//!
//! The tests live in `crates/astroforge-core/tests/` (NOT
//! `src-tauri/`) per the `astroforge-cr-slices` skill rule that
//! `src-tauri` is CI-dead and tests must run in the workspace's
//! `astroforge-core` crate.

use astroforge_core::domain::{ObjectType, PipelinePlan, PipelinePlanStatus, PipelineStage};
use astroforge_core::node_catalog::node_catalog;
use astroforge_core::node_graph::mutation::{insert_stage, NodeGraphMutationError};
use astroforge_core::pipeline_plans_store::PipelinePlanStore;

// ─── Fixtures ───────────────────────────────────────────────────────────

fn synthetic_plan(plan_id: &str, n_stages: usize) -> PipelinePlan {
    // Builds a minimal hand-rolled plan with `n_stages` stages,
    // each with a unique sequence 0..n_stages. Stage types are
    // taken from the canonical NodeCatalog so the
    // `insert_stage` path can find them.
    let catalog = node_catalog();
    let user_facing: Vec<String> = catalog
        .entries
        .iter()
        .map(|e| e.stage_type.clone())
        .collect();
    let stages = (0..n_stages)
        .map(|i| PipelineStage {
            stage_id: format!("stage_seed_{}", i),
            plan_id: plan_id.into(),
            stage_type: user_facing[i % user_facing.len()].clone(),
            sequence: i as u32,
            label: format!("Seed Stage {}", i),
            required: false,
            enabled: true,
            parameters_json: Some("{}".into()),
            produces_image_version: true,
            undo_supported: true,
        })
        .collect();
    PipelinePlan {
        plan_id: plan_id.into(),
        project_id: "proj_x".into(),
        session_id: "sess_x".into(),
        recipe_id: None,
        mode: "guided".into(),
        target_type: ObjectType::DeepSky,
        status: PipelinePlanStatus::Draft,
        created_at: "2026-09-30T00:00:00Z".into(),
        schema_version: 1,
        stages,
    }
}

fn fresh_store() -> PipelinePlanStore {
    PipelinePlanStore::in_memory().expect("in-memory store")
}

// ─── insert_stage: happy path ───────────────────────────────────────────

#[test]
fn insert_appends_to_end_with_correct_sequence() {
    let store = fresh_store();
    let plan = synthetic_plan("plan_p1_6_2_4_a", 3);
    store.insert_plan(&plan).unwrap();

    let updated = insert_stage(&store, "plan_p1_6_2_4_a", "color_calibration").unwrap();

    assert_eq!(updated.stages.len(), 4, "appended one stage");
    let new = updated.stages.last().unwrap();
    assert_eq!(new.sequence, 3, "sequence is one past the prior max");
    assert_eq!(new.stage_type, "color_calibration");
    assert!(new.enabled);
    assert!(new.parameters_json.is_some(), "default_params serialised");
    assert!(
        !new.required,
        "'color_calibration' is optional in deep-sky pipelines"
    );
}

#[test]
fn insert_persists_through_store() {
    // Reload the plan and confirm the new stage landed in the
    // database, not just the in-memory copy.
    let store = fresh_store();
    let plan = synthetic_plan("plan_p1_6_2_4_b", 2);
    store.insert_plan(&plan).unwrap();
    insert_stage(&store, "plan_p1_6_2_4_b", "stretch").unwrap();
    let reloaded = store.load_plan("plan_p1_6_2_4_b").unwrap();
    assert_eq!(reloaded.stages.len(), 3);
    assert_eq!(reloaded.stages[2].stage_type, "stretch");
}

// ─── insert_stage: default_params shape ─────────────────────────────────

#[test]
fn insert_uses_catalog_default_params_not_empty() {
    let store = fresh_store();
    let plan = synthetic_plan("plan_p1_6_2_4_c", 0);
    store.insert_plan(&plan).unwrap();

    let updated = insert_stage(&store, "plan_p1_6_2_4_c", "stretch").unwrap();
    let new = updated.stages.last().unwrap();
    let params_str = new.parameters_json.as_ref().unwrap();
    // `stretch` has non-empty defaults in the catalog. Confirm
    // we did NOT just serialise `{}` (a regression marker).
    assert_ne!(
        params_str, "{}",
        "stretch defaults must not collapse to empty object"
    );
    // And it must round-trip as JSON.
    let parsed: serde_json::Value = serde_json::from_str(params_str).unwrap();
    assert!(parsed.is_object(), "parameters_json is a JSON object");
}

#[test]
fn insert_label_matches_catalog_label() {
    let store = fresh_store();
    let plan = synthetic_plan("plan_p1_6_2_4_d", 1);
    store.insert_plan(&plan).unwrap();

    let updated = insert_stage(&store, "plan_p1_6_2_4_d", "export").unwrap();
    let new = updated.stages.last().unwrap();
    // The catalog entry for `export` has a specific label; we
    // assert against the catalog directly to avoid hard-coding
    // a string that drifts across slices.
    let catalog = node_catalog();
    let catalog_export = catalog
        .entries
        .iter()
        .find(|e| e.stage_type == "export")
        .expect("export is in the canonical catalog");
    assert_eq!(new.label, catalog_export.label);
    assert!(
        new.required,
        "export is mandatory in deep-sky pipelines (mandatory_for lock)"
    );
}

// ─── insert_stage: error paths ──────────────────────────────────────────

#[test]
fn insert_with_unknown_stage_type_returns_invalid() {
    let store = fresh_store();
    let plan = synthetic_plan("plan_p1_6_2_4_e", 1);
    store.insert_plan(&plan).unwrap();

    let err = insert_stage(&store, "plan_p1_6_2_4_e", "totally_made_up_stage").unwrap_err();
    assert!(
        matches!(err, NodeGraphMutationError::InvalidStageType(ref s) if s == "totally_made_up_stage")
    );

    // Plan must be unchanged in error (no partial write).
    let reloaded = store.load_plan("plan_p1_6_2_4_e").unwrap();
    assert_eq!(reloaded.stages.len(), 1, "no partial write on error");
}

#[test]
fn insert_into_unknown_plan_returns_store_error() {
    let store = fresh_store();
    let err = insert_stage(&store, "no_such_plan", "color_calibration").unwrap_err();
    assert!(matches!(err, NodeGraphMutationError::Store(_)));
}

// ─── insert_stage: re-insert of same stage_type ─────────────────────────

#[test]
fn reinsert_same_stage_type_gets_fresh_id_and_sequence() {
    let store = fresh_store();
    let plan = synthetic_plan("plan_p1_6_2_4_f", 0);
    store.insert_plan(&plan).unwrap();

    let u1 = insert_stage(&store, "plan_p1_6_2_4_f", "color_calibration").unwrap();
    let id1 = u1.stages.last().unwrap().stage_id.clone();
    let seq1 = u1.stages.last().unwrap().sequence;

    let u2 = insert_stage(&store, "plan_p1_6_2_4_f", "color_calibration").unwrap();
    let id2 = u2.stages.last().unwrap().stage_id.clone();
    let seq2 = u2.stages.last().unwrap().sequence;

    assert_ne!(id1, id2, "distinct ids on re-insert");
    assert_eq!(seq1, 0);
    assert_eq!(seq2, 1, "sequence advances monotonically");
}

// ─── insert_stage: appends even when sequences are non-contiguous ───────

#[test]
fn insert_uses_max_plus_one_when_sequences_have_gaps() {
    // Belt-and-braces: confirm the sequence calc is robust to
    // non-contiguous prior sequences (a future slice that
    // deletes a stage would leave a gap).
    let store = fresh_store();
    let mut plan = synthetic_plan("plan_p1_6_2_4_g", 0);
    plan.stages.push(PipelineStage {
        stage_id: "stage_gap_0".into(),
        plan_id: "plan_p1_6_2_4_g".into(),
        stage_type: "calibrate".into(),
        sequence: 0,
        label: "Calibrate".into(),
        required: true,
        enabled: true,
        parameters_json: None,
        produces_image_version: true,
        undo_supported: true,
    });
    plan.stages.push(PipelineStage {
        stage_id: "stage_gap_5".into(),
        plan_id: "plan_p1_6_2_4_g".into(),
        stage_type: "stack".into(),
        sequence: 5,
        label: "Stack".into(),
        required: true,
        enabled: true,
        parameters_json: None,
        produces_image_version: true,
        undo_supported: true,
    });
    store.insert_plan(&plan).unwrap();

    let updated = insert_stage(&store, "plan_p1_6_2_4_g", "color_calibration").unwrap();
    let new = updated.stages.last().unwrap();
    assert_eq!(
        new.sequence, 6,
        "sequence is max(existing) + 1, not stages.len()"
    );
}
