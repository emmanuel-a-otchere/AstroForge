//! CR-05 P5 slice 4 — §24 / §27 `get_processing_metrics` aggregator.
//!
//! Pure aggregation of `(PipelinePlan, &[StageExecution])` into a
//! `ProcessingMetrics` payload. Lives in `astroforge-core` (rather
//! than `src-tauri`) so it is exercised by the CI workspace test
//! run; `src-tauri` is a binary crate outside the workspace.
//!
//! ## What this returns
//!
//! - `stage_count` / `completed_count` / `completion_ratio` — for
//!   the §24 progress header.
//! - `latest_stage_id` / `latest_stage_type` — what the user just
//!   ran; the DAG view keys node colouring off these.
//! - `latest_metrics` — deserialised `ImageMetrics` from the most
//!   recent completed stage's `metric_snapshot_json`. None when the
//!   stage didn't persist metrics (older rows or stages that don't
//!   emit them).
//! - `latest_resource_budget` — deserialised `ExecutionBudget`
//!   from the same stage's `resource_usage_json` (slice 2 persistence).
//! - `adaptive_parameters` — §12 output derived from `latest_metrics`,
//!   so the UI can render the recommendation banner without a
//!   second IPC round-trip.

use crate::adaptive::{derive_adaptive_parameters, AdaptiveParameterSet, ImageMetrics};
use crate::ai_boundary::AiBoundaryLabel;
use crate::domain::{PipelinePlan, StageExecution};
use crate::resource::ExecutionBudget;
use crate::stage_error::StageError;
use serde::{Deserialize, Serialize};

/// Aggregate payload — same shape as the Tauri `ProcessingMetricsDto`
/// but lives in core so the aggregation can be unit-tested without
/// a Tauri State. The Tauri command serde-converts this directly to
/// its own DTO via `serde_json::to_value` or just renames the fields;
/// see `src-tauri/src/commands_pipeline_plan.rs` for the wiring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMetrics {
    pub plan_id: String,
    pub stage_count: u32,
    pub completed_count: u32,
    /// `0.0..=1.0`. `0.0` when no stages have completed yet.
    pub completion_ratio: f64,
    pub latest_stage_id: Option<String>,
    pub latest_stage_type: Option<String>,
    pub latest_metrics: Option<ImageMetrics>,
    pub latest_resource_budget: Option<ExecutionBudget>,
    /// §12 adaptive output derived from `latest_metrics`. `None` when
    /// no metrics are available (the engine itself never refuses to
    /// make progress, but the UI surfaces absence honestly rather
    /// than fabricating a recommendation).
    pub adaptive_parameters: Option<AdaptiveParameterSet>,
    /// CR-05 P6.2 (§23 AI Boundary) — AI label from the most-recent
    /// execution. `None` when `ai_label_json` is missing (pre-P6.2
    /// rows) or unparseable.
    pub latest_ai_label: Option<AiBoundaryLabel>,
    /// §28 structured error from the most-recent failure. `None`
    /// when the latest stage hasn't failed or when `error_json`
    /// is malformed (legacy rows pre-P6.1 used the bare
    /// `{"what_happened":"..."}` shape).
    pub latest_stage_error: Option<StageError>,
}

/// Pure aggregation. No IO, no system calls — exhaustive unit tests
/// below cover the selection rules.
pub fn aggregate_processing_metrics(
    plan: &PipelinePlan,
    execs: &[StageExecution],
) -> ProcessingMetrics {
    let stage_count = plan.stages.len() as u32;
    let completed_count = execs.iter().filter(|e| e.status == "completed").count() as u32;
    let completion_ratio = if stage_count == 0 {
        0.0
    } else {
        completed_count as f64 / stage_count as f64
    };

    // The "latest" stage is the most-recently-completed one, or — if
    // nothing has completed yet — the most-recently-started. Ties
    // broken by `sequence` so the view is deterministic.
    let latest = execs
        .iter()
        .max_by(|a, b| latest_key(a, plan).cmp(&latest_key(b, plan)));

    let latest_stage_id = latest.map(|e| e.stage_id.clone());
    // `stage_type` is on the plan, not on the execution — join back.
    let latest_stage_type = latest.and_then(|e| {
        plan.stages
            .iter()
            .find(|s| s.stage_id == e.stage_id)
            .map(|s| s.stage_type.clone())
    });
    let latest_resource_budget = latest.and_then(|e| {
        e.resource_usage_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<ExecutionBudget>(s).ok())
    });

    // Bridge metric_snapshot_json → ImageMetrics. Older rows may have
    // null or a non-metrics JSON; serde_json::from_str is forgiving.
    let latest_metrics = latest.and_then(|e| {
        e.metric_snapshot_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<ImageMetrics>(s).ok())
    });

    let adaptive_parameters = latest_metrics.as_ref().map(derive_adaptive_parameters);

    // CR-05 P6.2 (§23) — `latest_ai_label` is sourced from the same
    // `latest` row that drives `latest_metrics`. Pre-P6.2 rows have
    // `ai_label_json = None` and surface as `None` here; the
    // StageCard renders no badge in that case (absence, not a
    // "no AI" line).
    let latest_ai_label = latest.and_then(|e| {
        e.ai_label_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<AiBoundaryLabel>(s).ok())
    });

    // CR-05 P6.1 (§28) — `latest_stage_error` is sourced from the
    // most-recent *failed* execution, NOT from the same `latest`
    // row that drives `latest_metrics`. A user who watches a stage
    // fail wants to see the recovery panel immediately, even if a
    // prior stage completed successfully.
    let latest_failed = execs
        .iter()
        .filter(|e| e.status == "failed")
        .max_by_key(|e| e.completed_at.clone().unwrap_or_default());
    let latest_stage_error = latest_failed.and_then(|e| {
        e.error_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<StageError>(s).ok())
    });

    ProcessingMetrics {
        plan_id: plan.plan_id.clone(),
        stage_count,
        completed_count,
        completion_ratio,
        latest_stage_id,
        latest_stage_type,
        latest_metrics,
        latest_resource_budget,
        adaptive_parameters,
        latest_ai_label,
        latest_stage_error,
    }
}

/// Sort key for "latest stage" selection — completed-first then by
/// `completed_at` (parsed as u64; 0 if missing or unparseable) then
/// by `sequence` from the matching plan stage. Stable: equal keys
/// preserve input order.
fn latest_key(e: &StageExecution, plan: &PipelinePlan) -> (bool, u64, u32) {
    let completed_at_ms = e
        .completed_at
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let sequence = plan
        .stages
        .iter()
        .find(|s| s.stage_id == e.stage_id)
        .map(|s| s.sequence)
        .unwrap_or(0);
    (e.status == "completed", completed_at_ms, sequence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PipelinePlanStatus, PipelineStage};

    fn test_stage(seq: u32, type_: &str) -> PipelineStage {
        PipelineStage {
            plan_id: "plan_test".into(),
            stage_id: format!("stage_{seq}"),
            stage_type: type_.to_string(),
            label: type_.to_string(),
            sequence: seq,
            required: true,
            enabled: true,
            parameters_json: None,
            produces_image_version: true,
            undo_supported: false,
        }
    }

    fn test_plan(stages: Vec<PipelineStage>) -> PipelinePlan {
        PipelinePlan {
            plan_id: "plan_test".into(),
            project_id: "project_test".into(),
            session_id: "sess_test".into(),
            recipe_id: Some("deep_sky_osc_balanced".into()),
            mode: "expert".into(),
            target_type: crate::domain::ObjectType::DeepSky,
            status: PipelinePlanStatus::Ready,
            created_at: "0".into(),
            schema_version: 1,
            stages,
        }
    }

    fn test_exec(
        stage_id: &str,
        status: &str,
        completed_at: Option<u64>,
        metric_snapshot_json: Option<String>,
        resource_usage_json: Option<String>,
    ) -> StageExecution {
        // `seq` argument removed — `sequence` lives on the plan
        // stage, not on the execution. Kept off the helper to keep
        // the call sites focused.
        StageExecution {
            stage_execution_id: format!("exec_{stage_id}"),
            plan_id: "plan_test".into(),
            stage_id: stage_id.to_string(),
            attempt: 1,
            status: status.to_string(),
            input_version_id: None,
            output_artifact_id: None,
            parameters_json: None,
            parameters_hash: None,
            started_at: completed_at.map(|t| format!("{t}")),
            completed_at: completed_at.map(|t| format!("{t}")),
            resource_usage_json,
            error_json: None,
            metric_snapshot_json,
            // CR-05 P6.2 (§23) — AI label is populated by the runner
            // for real StageExecution rows; the test fixture leaves it
            // None (see ai_boundary::tests for dedicated coverage).
            ai_label_json: None,
        }
    }

    #[test]
    fn empty_plan_reports_zero_completion() {
        let plan = test_plan(vec![]);
        let metrics = aggregate_processing_metrics(&plan, &[]);
        assert_eq!(metrics.stage_count, 0);
        assert_eq!(metrics.completed_count, 0);
        assert_eq!(metrics.completion_ratio, 0.0);
        assert!(metrics.latest_stage_id.is_none());
        assert!(metrics.latest_metrics.is_none());
    }

    #[test]
    fn uncompleted_plan_reports_zero_completion_ratio() {
        let plan = test_plan(vec![test_stage(0, "calibrate"), test_stage(1, "stack")]);
        let execs = vec![test_exec("stage_0", "running", None, None, None)];
        let metrics = aggregate_processing_metrics(&plan, &execs);
        assert_eq!(metrics.stage_count, 2);
        assert_eq!(metrics.completed_count, 0);
        assert_eq!(metrics.completion_ratio, 0.0);
    }

    #[test]
    fn half_completed_plan_reports_half_ratio() {
        let plan = test_plan(vec![test_stage(0, "calibrate"), test_stage(1, "stack")]);
        let execs = vec![test_exec("stage_0", "completed", Some(1000), None, None)];
        let metrics = aggregate_processing_metrics(&plan, &execs);
        assert_eq!(metrics.completed_count, 1);
        assert!((metrics.completion_ratio - 0.5).abs() < 1e-9);
    }

    #[test]
    fn latest_stage_is_most_recently_completed() {
        let plan = test_plan(vec![
            test_stage(0, "calibrate"),
            test_stage(1, "stack"),
            test_stage(2, "stretch"),
        ]);
        let execs = vec![
            test_exec("stage_0", "completed", Some(1000), None, None),
            test_exec("stage_1", "completed", Some(2000), None, None),
            test_exec("stage_2", "running", None, None, None),
        ];
        let metrics = aggregate_processing_metrics(&plan, &execs);
        assert_eq!(metrics.latest_stage_id.as_deref(), Some("stage_1"));
        assert_eq!(metrics.latest_stage_type.as_deref(), Some("stack"));
    }

    #[test]
    fn adaptive_parameters_derived_from_latest_metrics() {
        let plan = test_plan(vec![test_stage(0, "stack")]);
        let metrics_json = serde_json::json!({
            "snr": 60.0,
            "fwhm": 1.8,
            "star_count": 250,
            "background_gradient": 0.0,
            "mean": 0.20,
            "stddev": 0.003,
        })
        .to_string();
        let execs = vec![test_exec(
            "stage_0",
            "completed",
            Some(1000),
            Some(metrics_json),
            None,
        )];
        let metrics = aggregate_processing_metrics(&plan, &execs);
        let adaptive = metrics
            .adaptive_parameters
            .expect("adaptive_parameters must be derived from latest_metrics");
        let noise = adaptive.noise.expect("noise profile must be derived");
        assert_eq!(noise.kind, crate::adaptive::NoiseKind::Low);
        assert_eq!(noise.label, "Low");
    }

    #[test]
    fn adaptive_parameters_omitted_when_no_metrics_available() {
        let plan = test_plan(vec![test_stage(0, "calibrate")]);
        let execs = vec![test_exec("stage_0", "completed", Some(1000), None, None)];
        let metrics = aggregate_processing_metrics(&plan, &execs);
        assert!(
            metrics.adaptive_parameters.is_none(),
            "no metrics → no adaptive parameters"
        );
    }

    #[test]
    fn latest_resource_budget_parsed_when_persisted() {
        let plan = test_plan(vec![test_stage(0, "stack")]);
        let budget_json = serde_json::to_string(&ExecutionBudget::default()).unwrap();
        let execs = vec![test_exec(
            "stage_0",
            "completed",
            Some(1000),
            None,
            Some(budget_json),
        )];
        let metrics = aggregate_processing_metrics(&plan, &execs);
        assert!(metrics.latest_resource_budget.is_some());
    }

    #[test]
    fn malformed_metrics_json_does_not_panic() {
        let plan = test_plan(vec![test_stage(0, "calibrate")]);
        let execs = vec![test_exec(
            "stage_0",
            "completed",
            Some(1000),
            Some("not json".into()),
            None,
        )];
        let metrics = aggregate_processing_metrics(&plan, &execs);
        assert!(metrics.latest_metrics.is_none());
        assert!(metrics.adaptive_parameters.is_none());
    }

    #[test]
    fn failed_stage_error_surfaces_in_aggregator() {
        let plan = test_plan(vec![test_stage(0, "calibrate"), test_stage(1, "stack")]);
        let err = crate::stage_error::StageError::from_failure("stack", 1, "out of memory");
        let err_json = serde_json::to_string(&err).unwrap();
        let mut failed_exec = test_exec("stage_1", "failed", Some(2000), None, None);
        failed_exec.error_json = Some(err_json);
        let execs = vec![
            test_exec("stage_0", "completed", Some(1000), None, None),
            failed_exec,
        ];
        let metrics = aggregate_processing_metrics(&plan, &execs);
        let surf = metrics
            .latest_stage_error
            .expect("latest_stage_error must be populated when a stage failed");
        assert!(surf.what_happened.contains("Stacking"));
        assert!(surf.what_happened.contains("out of memory"));
        assert!(surf.what_was_preserved.contains("1 Image Version"));
        assert!(!surf.suggested_actions.is_empty());
    }

    #[test]
    fn completed_status_wins_even_when_running_stage_has_later_timestamp() {
        let plan = test_plan(vec![test_stage(0, "calibrate"), test_stage(1, "stack")]);
        let execs = vec![
            test_exec("stage_0", "completed", Some(500), None, None),
            test_exec("stage_1", "running", Some(9999), None, None),
        ];
        let metrics = aggregate_processing_metrics(&plan, &execs);
        // stage_0 is the only completed one → wins over a later-
        // timestamped running stage.
        assert_eq!(metrics.latest_stage_id.as_deref(), Some("stage_0"));
    }
}
