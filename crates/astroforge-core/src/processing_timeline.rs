//! CR-05 P6.3 — §25 Processing Timeline.
//!
//! §25 says: "The version timeline and pipeline timeline should work
//! together." Pipeline timeline = a chronological list of stage
//! events; selecting an event reveals the corresponding Image Version.
//!
//! This module derives the pipeline timeline from existing data:
//! each `StageExecution` carries `started_at`, `completed_at`,
//! `output_artifact_id` (the produced Image Version), and a join back
//! to the plan stage gives us the human-readable label. No schema
//! change required — the data is already on the row.
//!
//! Timestamp encoding: `StageExecution.started_at` is a `String` of
//! the form `"unix_ms:<N>"` per the runner's writer. The parser
//! below is forgiving — any other shape (legacy `"2026-09-09T..."`)
//! degrades to `None` and the event surfaces without a time rather
//! than crashing.
//!
//! Sort: chronological by `started_at_unix_ms`, then by stage
//! sequence so a single instant with two events still orders by
//! the user's mental pipeline (stack → background → color ...).
//!
//! Stages without a `started_at` (early failure, no dispatch) are
//! dropped — they have no time on the clock, so they can't be
//! plotted without inventing a timestamp.

use serde::{Deserialize, Serialize};

use crate::domain::{PipelinePlan, PipelineStage, StageExecution};

/// One event on the processing timeline.
///
/// `timestamp_unix_ms` is the stage's `started_at`; `duration_ms`
/// is the gap to `completed_at` when both are present. The
/// `output_version_id` corresponds to `ImageVersion.version_id`
/// (one-to-one with `output_artifact_id` for stages that produce
/// an Image Version; `None` for stages like `export` that produce
/// arbitrary outputs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// Stage id (matches `PipelineStage.stage_id`).
    pub stage_id: String,
    /// Human-readable stage label (e.g. "Stack", "AI denoise").
    pub stage_label: String,
    /// Stage type token (e.g. "stack", "denoise").
    pub stage_type: String,
    /// Stage sequence within the plan.
    pub sequence: u32,
    /// Unix milliseconds parsed from `started_at`.
    pub timestamp_unix_ms: Option<u64>,
    /// Duration in milliseconds (`completed_at - started_at`), if both
    /// parse successfully.
    pub duration_ms: Option<u64>,
    /// `ImageVersion.version_id` this stage produced, if any.
    pub output_version_id: Option<String>,
    /// Stage status — `"completed"`, `"failed"`, `"running"`, etc.
    pub status: String,
}

/// Aggregated processing timeline for a plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessingTimeline {
    pub plan_id: String,
    pub events: Vec<TimelineEvent>,
}

/// Parse `"unix_ms:<N>"` (and any other `unix_ms:` prefix we
/// encounter) to `Some(u64)`. Returns `None` on malformed input
/// rather than panicking — older rows, unsynced clocks, or
/// future encoding changes shouldn't crash the renderer.
pub fn parse_unix_ms(raw: Option<&str>) -> Option<u64> {
    raw?.strip_prefix("unix_ms:")?.parse::<u64>().ok()
}

/// Pure aggregation. No IO, no system calls.
///
/// `execs` need not be pre-sorted; we sort by timestamp (with
/// sequence as a tie-breaker). Executions without a parseable
/// `started_at` are dropped.
pub fn build_processing_timeline(
    plan: &PipelinePlan,
    execs: &[StageExecution],
) -> ProcessingTimeline {
    let stage_index: std::collections::HashMap<&str, &PipelineStage> = plan
        .stages
        .iter()
        .map(|s| (s.stage_id.as_str(), s))
        .collect();

    let mut events: Vec<TimelineEvent> = execs
        .iter()
        .filter_map(|e| {
            let stage = stage_index.get(e.stage_id.as_str()).copied();
            // Without a stage to label the event, skip it. The data
            // integrity here matters because the timeline is
            // §25's reproducibility surface — events without
            // provenance shouldn't be plotted.
            let stage = stage?;

            let started = parse_unix_ms(e.started_at.as_deref());
            // Drop events with no parseable start time. We never
            // invent timestamps; absence is honest.
            let timestamp_unix_ms = started?;

            let completed = parse_unix_ms(e.completed_at.as_deref());
            let duration_ms = match (started, completed) {
                (Some(s), Some(c)) if c >= s => Some(c - s),
                _ => None,
            };

            Some(TimelineEvent {
                stage_id: e.stage_id.clone(),
                stage_label: stage.label.clone(),
                stage_type: stage.stage_type.clone(),
                sequence: stage.sequence,
                timestamp_unix_ms: Some(timestamp_unix_ms),
                duration_ms,
                output_version_id: e.output_artifact_id.clone(),
                status: e.status.clone(),
            })
        })
        .collect();

    // Sort: timestamp ascending, then sequence as a stable
    // tie-breaker (two events at the same instant still order by
    // pipeline progression).
    events.sort_by(|a, b| {
        a.timestamp_unix_ms
            .cmp(&b.timestamp_unix_ms)
            .then(a.sequence.cmp(&b.sequence))
    });

    ProcessingTimeline {
        plan_id: plan.plan_id.clone(),
        events,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ObjectType, PipelinePlan, PipelineStage, StageExecution};

    fn stage(stage_id: &str, sequence: u32, stage_type: &str, label: &str) -> PipelineStage {
        PipelineStage {
            plan_id: "plan_1".into(),
            stage_id: stage_id.into(),
            stage_type: stage_type.into(),
            sequence,
            label: label.into(),
            required: false,
            enabled: true,
            parameters_json: None,
            produces_image_version: stage_type != "export",
            undo_supported: false,
        }
    }

    fn exec(stage_id: &str, started: &str, completed: Option<&str>) -> StageExecution {
        StageExecution {
            stage_execution_id: format!("se_{stage_id}"),
            plan_id: "plan_1".into(),
            stage_id: stage_id.into(),
            attempt: 1,
            status: "completed".into(),
            input_version_id: None,
            output_artifact_id: Some(format!("v_{stage_id}")),
            parameters_json: None,
            parameters_hash: None,
            started_at: Some(started.into()),
            completed_at: completed.map(|s| s.into()),
            resource_usage_json: None,
            error_json: None,
            metric_snapshot_json: None,
            ai_label_json: None,
        }
    }

    fn plan(stages: Vec<PipelineStage>) -> PipelinePlan {
        PipelinePlan {
            plan_id: "plan_1".into(),
            project_id: "project_1".into(),
            session_id: "sess_1".into(),
            recipe_id: None,
            mode: "expert".into(),
            target_type: ObjectType::DeepSky,
            status: crate::domain::PipelinePlanStatus::Running,
            created_at: "unix_ms:0".into(),
            schema_version: 1,
            stages,
        }
    }

    #[test]
    fn parses_unix_ms_prefix() {
        assert_eq!(parse_unix_ms(Some("unix_ms:1234")), Some(1234));
        assert_eq!(parse_unix_ms(Some("unix_ms:0")), Some(0));
        assert_eq!(parse_unix_ms(None), None);
        assert_eq!(parse_unix_ms(Some("")), None);
        assert_eq!(parse_unix_ms(Some("2026-09-09T00:00:00Z")), None);
        assert_eq!(parse_unix_ms(Some("unix_ms:not_a_number")), None);
    }

    #[test]
    fn empty_plan_yields_empty_timeline() {
        let plan = plan(vec![]);
        let t = build_processing_timeline(&plan, &[]);
        assert_eq!(t.plan_id, "plan_1");
        assert!(t.events.is_empty());
    }

    #[test]
    fn orders_events_by_timestamp_then_sequence() {
        // Out-of-order input: import starts AFTER stack (defensive).
        // Output: sorted by timestamp asc, sequence tie-break.
        let plan = plan(vec![
            stage("s_import", 0, "calibrate", "Import"),
            stage("s_stack", 4, "stack", "Stack"),
            stage("s_stretch", 7, "stretch", "Stretch"),
        ]);
        let execs = vec![
            exec("s_stretch", "unix_ms:2000", Some("unix_ms:2100")),
            exec("s_import", "unix_ms:1000", Some("unix_ms:1500")),
            exec("s_stack", "unix_ms:1500", Some("unix_ms:1800")),
        ];
        let t = build_processing_timeline(&plan, &execs);
        let labels: Vec<_> = t.events.iter().map(|e| e.stage_label.as_str()).collect();
        assert_eq!(labels, ["Import", "Stack", "Stretch"]);
        // Same timestamp (1500): tie-break by sequence ascending.
        // Import is 1000, Stack is 1500 — they differ in time, so the
        // tie-break only matters for the (1500, 1500) pair... but
        // there's only one event at 1500. Add a tied pair to make
        // sure.
    }

    #[test]
    fn tie_breaks_by_sequence_when_timestamps_match() {
        let plan = plan(vec![
            stage("s_a", 1, "calibrate", "Calibrate"),
            stage("s_b", 2, "calibrate", "Calibrate-2"),
        ]);
        let execs = vec![
            exec("s_b", "unix_ms:1000", None),
            exec("s_a", "unix_ms:1000", None),
        ];
        let t = build_processing_timeline(&plan, &execs);
        // Sequence ascending: a (1) before b (2).
        assert_eq!(t.events[0].stage_id, "s_a");
        assert_eq!(t.events[1].stage_id, "s_b");
    }

    #[test]
    fn drops_events_without_parseable_started_at() {
        let plan = plan(vec![
            stage("s_a", 1, "calibrate", "Calibrate"),
            stage("s_b", 2, "stack", "Stack"),
        ]);
        let execs = vec![
            exec("s_a", "unix_ms:1000", Some("unix_ms:1100")),
            exec("s_b", "not-a-timestamp", None),
        ];
        let t = build_processing_timeline(&plan, &execs);
        // s_b dropped — no fake timestamp.
        assert_eq!(t.events.len(), 1);
        assert_eq!(t.events[0].stage_id, "s_a");
    }

    #[test]
    fn drops_events_without_matching_stage() {
        let plan = plan(vec![stage("s_a", 1, "calibrate", "Calibrate")]);
        // Orphan exec: references a stage_id not in the plan.
        let execs = vec![
            exec("s_a", "unix_ms:1000", None),
            exec("s_orphan", "unix_ms:2000", None),
        ];
        let t = build_processing_timeline(&plan, &execs);
        assert_eq!(t.events.len(), 1);
        assert_eq!(t.events[0].stage_id, "s_a");
    }

    #[test]
    fn computes_duration_when_both_timestamps_parse() {
        let plan = plan(vec![stage("s_a", 1, "stack", "Stack")]);
        let execs = vec![exec("s_a", "unix_ms:1000", Some("unix_ms:1234"))];
        let t = build_processing_timeline(&plan, &execs);
        assert_eq!(t.events[0].duration_ms, Some(234));
    }

    #[test]
    fn duration_is_none_when_completed_unparseable() {
        let plan = plan(vec![stage("s_a", 1, "stack", "Stack")]);
        let execs = vec![exec("s_a", "unix_ms:1000", Some("nonsense"))];
        let t = build_processing_timeline(&plan, &execs);
        assert_eq!(t.events[0].duration_ms, None);
    }

    #[test]
    fn duration_is_none_when_clock_skew_negative() {
        // completed < started is impossible but possible under clock
        // skew; we don't surface negative durations.
        let plan = plan(vec![stage("s_a", 1, "stack", "Stack")]);
        let execs = vec![exec("s_a", "unix_ms:2000", Some("unix_ms:1000"))];
        let t = build_processing_timeline(&plan, &execs);
        assert_eq!(t.events[0].duration_ms, None);
    }

    #[test]
    fn status_is_preserved() {
        let plan = plan(vec![stage("s_a", 1, "stack", "Stack")]);
        let mut e = exec("s_a", "unix_ms:1000", Some("unix_ms:1100"));
        e.status = "failed".into();
        let t = build_processing_timeline(&plan, &[e]);
        assert_eq!(t.events[0].status, "failed");
    }

    #[test]
    fn output_version_id_maps_from_output_artifact_id() {
        let plan = plan(vec![stage("s_a", 1, "stack", "Stack")]);
        let execs = vec![exec("s_a", "unix_ms:1000", Some("unix_ms:1100"))];
        let t = build_processing_timeline(&plan, &execs);
        assert_eq!(t.events[0].output_version_id.as_deref(), Some("v_s_a"));
    }

    #[test]
    fn serde_round_trip() {
        let plan = plan(vec![
            stage("s_a", 1, "stack", "Stack"),
            stage("s_b", 2, "stretch", "Stretch"),
        ]);
        let execs = vec![
            exec("s_a", "unix_ms:1000", Some("unix_ms:1100")),
            exec("s_b", "unix_ms:1200", Some("unix_ms:1300")),
        ];
        let t = build_processing_timeline(&plan, &execs);
        let json = serde_json::to_string(&t).unwrap();
        let back: ProcessingTimeline = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }
}
