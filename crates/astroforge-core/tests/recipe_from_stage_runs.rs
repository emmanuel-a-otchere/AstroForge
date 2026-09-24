//! CR-08 §22 round 2 / Slice B tests:
//! `recipe_from_stage_runs` (pure function).
//!
//! The function is the execution-history sibling of
//! `recipe_from_pipeline_plan` (§14). It builds a Recipe
//! from the actual `StageRunRecord`s the engine
//! produced, picking the terminal attempt per stage so
//! reruns don't smuggle stale params into the saved
//! Recipe. Tests pin the round-2 contract:
//! - An empty stage-runs list produces a Recipe with
//!   zero stages but the §14-shaped defaults.
//! - A 3-stage completed run produces 3 enabled
//!   RecipeStages with the terminal attempt's parsed
//!   params.
//! - A mixed run (2 completed + 1 failed) produces 3
//!   stages; the failed stage has `enabled = false`.
//! - A rerun scenario (attempts 1 and 2 for one stage)
//!   keeps the attempt-2 params (terminal wins).
//! - A malformed `params_json` falls back to an empty
//!   HashMap (no panic).

use astroforge_core::domain::StageRunRecord;
use astroforge_core::recipe::{recipe_from_stage_runs, QualityProfile};

fn stage_run(
    stage_id: &str,
    attempt: u32,
    status: &str,
    params_json: Option<&str>,
) -> StageRunRecord {
    StageRunRecord {
        stage_run_id: format!("sr-{stage_id}-{attempt}"),
        run_id: "pr-test-001".to_string(),
        stage_id: stage_id.to_string(),
        status: status.to_string(),
        attempt,
        params_json: params_json.map(|s| s.to_string()),
        metrics_json: None,
        error: if status == "failed" {
            Some("test error".to_string())
        } else {
            None
        },
        started_at: Some("2026-09-24T00:00:00Z".to_string()),
        completed_at: Some("2026-09-24T00:00:01Z".to_string()),
    }
}

#[test]
fn empty_stage_runs_produces_recipe_with_zero_stages() {
    let recipe = recipe_from_stage_runs(&[], "pr-test-001", "Empty", Some("deep_sky"));

    assert_eq!(recipe.name, "Empty");
    assert_eq!(recipe.target_type, "deep_sky");
    assert_eq!(recipe.version, 1);
    assert_eq!(recipe.parent_version, None);
    assert_eq!(recipe.branch, "main");
    assert_eq!(recipe.quality_profile, QualityProfile::Natural);
    assert!(!recipe.is_system);
    assert!(recipe.required_models.is_empty());
    assert!(recipe.flags.is_empty());
    assert!(recipe.stages.is_empty(), "empty input → zero RecipeStages");
    // Description still records the run id so the
    // provenance line is honest even on empty runs.
    assert!(recipe.description.contains("pr-test-001"));
    assert!(recipe.description.contains("0 stages"));
    assert!(recipe.description.contains("0 completed"));
}

#[test]
fn three_completed_stages_produce_three_enabled_recipe_stages() {
    let runs = vec![
        stage_run("stacking", 1, "completed", Some(r#"{"gain": 1.5}"#)),
        stage_run("denoise", 1, "completed", Some(r#"{"strength": 0.7}"#)),
        stage_run("sharpen", 1, "completed", Some(r#"{"amount": 0.4}"#)),
    ];
    let recipe = recipe_from_stage_runs(&runs, "pr-test-001", "Three Up", Some("deep_sky"));

    assert_eq!(recipe.stages.len(), 3);
    // Order: first-attempt-first, which matches the
    // input order in this case.
    assert_eq!(recipe.stages[0].stage_id, "stacking");
    assert!(recipe.stages[0].enabled);
    assert_eq!(
        recipe.stages[0].params.get("gain").and_then(|v| v.as_f64()),
        Some(1.5)
    );
    assert_eq!(recipe.stages[1].stage_id, "denoise");
    assert!(recipe.stages[1].enabled);
    assert_eq!(
        recipe.stages[1]
            .params
            .get("strength")
            .and_then(|v| v.as_f64()),
        Some(0.7)
    );
    assert_eq!(recipe.stages[2].stage_id, "sharpen");
    assert!(recipe.stages[2].enabled);
    assert_eq!(
        recipe.stages[2]
            .params
            .get("amount")
            .and_then(|v| v.as_f64()),
        Some(0.4)
    );
    // Description counts all-completed.
    assert!(recipe.description.contains("3 stages"));
    assert!(recipe.description.contains("3 completed"));
}

#[test]
fn mixed_completed_and_failed_stages_keep_failed_disabled() {
    let runs = vec![
        stage_run("stacking", 1, "completed", Some(r#"{"gain": 1.0}"#)),
        stage_run("denoise", 1, "completed", Some(r#"{"strength": 0.5}"#)),
        stage_run("sharpen", 1, "failed", Some(r#"{"amount": 0.3}"#)),
    ];
    let recipe = recipe_from_stage_runs(&runs, "pr-test-001", "Mixed", None);

    assert_eq!(
        recipe.stages.len(),
        3,
        "all stages surface; only enabled flag differs"
    );
    assert!(recipe.stages[0].enabled);
    assert!(recipe.stages[1].enabled);
    assert!(
        !recipe.stages[2].enabled,
        "failed terminal attempt must record enabled=false so the user can re-enable in RecipeEditor"
    );
    // No explicit target_type supplied → "unknown".
    assert_eq!(recipe.target_type, "unknown");
    assert!(recipe.description.contains("3 stages"));
    assert!(recipe.description.contains("2 completed"));
}

#[test]
fn rerun_scenario_keeps_terminal_attempt_params() {
    // First attempt of stacking fails; second attempt
    // succeeds with different params. The terminal
    // attempt (attempt=2) wins.
    let runs = vec![
        stage_run("stacking", 1, "failed", Some(r#"{"gain": 1.0}"#)),
        stage_run(
            "stacking",
            2,
            "completed",
            Some(r#"{"gain": 2.5, "rerun": true}"#),
        ),
        stage_run("denoise", 1, "completed", Some(r#"{"strength": 0.6}"#)),
    ];
    let recipe = recipe_from_stage_runs(&runs, "pr-test-001", "Rerun", Some("planet"));

    // Two unique stages: stacking + denoise (not three).
    assert_eq!(recipe.stages.len(), 2);
    // Stacking comes first (its attempt-1 row was first
    // in the slice), enabled = (terminal.status ==
    // "completed") = true (attempt 2 completed).
    assert_eq!(recipe.stages[0].stage_id, "stacking");
    assert!(
        recipe.stages[0].enabled,
        "terminal completed attempt → enabled"
    );
    assert_eq!(
        recipe.stages[0].params.get("gain").and_then(|v| v.as_f64()),
        Some(2.5),
        "terminal attempt's params win; attempt-1's gain=1.0 must NOT leak through"
    );
    assert_eq!(
        recipe.stages[0]
            .params
            .get("rerun")
            .and_then(|v| v.as_bool()),
        Some(true),
        "terminal attempt's extra param surfaces"
    );
    assert_eq!(recipe.stages[1].stage_id, "denoise");
    assert!(recipe.stages[1].enabled);
    assert!(recipe.description.contains("2 stages"));
    assert!(recipe.description.contains("2 completed"));
}

#[test]
fn malformed_params_json_falls_back_to_empty_hashmap() {
    // Three different malformed payloads — none should
    // panic; all should produce RecipeStage with empty
    // params but the right enabled flag.
    let runs = vec![
        stage_run("alpha", 1, "completed", None),
        stage_run("beta", 1, "completed", Some("")),
        stage_run("gamma", 1, "completed", Some("{not valid json")),
    ];
    let recipe = recipe_from_stage_runs(&runs, "pr-test-001", "Malformed", Some("deep_sky"));

    assert_eq!(recipe.stages.len(), 3);
    for stage in &recipe.stages {
        assert!(
            stage.params.is_empty(),
            "malformed/missing params_json → empty HashMap (no panic)"
        );
        assert!(stage.enabled);
    }
}
