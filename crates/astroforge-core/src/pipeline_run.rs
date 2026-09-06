//! CR-02.5 — Pipeline persistence driver.
//!
//! The existing `Orchestrator` (orchestrator.rs) holds the DAG execution
//! engine itself; this module is the *persistence* wrapper. It drives the
//! DAG in topological order, records a PipelineRun + StageRunRecord per
//! stage (§11/§12), and surfaces a RecoveryReport so CR-02 §28 can resume
//! after an unexpected termination.
//!
//! Design notes.
//!
//! The v0 `Orchestrator::run` returns only after the full DAG; we can't
//! observe per-stage progress through it without changing its contract.
//! So this driver works directly against the `Stage` trait and
//! `PipelineDag::topological_order()` (the same primitives) and persists
//! between stages. The orchestrator stays untouched.
//!
//! On stage failure the run is marked Failed and the driver returns. The
//! caller can `list_stage_runs()` to inspect history.
//!
//! Artifact rows recorded against `pipeline_run_id` are the provenance
//! graph (CR-02 §16). This driver doesn't mint artifacts; it just carries
//! the run_id around so artifact writes elsewhere can attach to it.

use crate::domain::{PipelineRunStatus, ProjectEventKind, StageRunRecord};
use crate::domain_store::{DomainStore, DomainStoreError};
use crate::pipeline::{PipelineDag, Stage, StageContext, StageError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Outcome of `DomainStore::run_pipeline_persistent`.
#[derive(Debug, Clone)]
pub struct PipelineRunReport {
    pub run_id: String,
    pub stage_runs: Vec<String>,
    pub failed_at: Option<String>,
}

type Result<T> = std::result::Result<T, DomainStoreError>;

impl DomainStore {
    /// Create + execute a pipeline run, persisting every stage as it goes.
    ///
    /// `stages` is keyed by stage id, mirroring `Orchestrator::register_stage`.
    /// `application_version` and `engine_version` are recorded on the run
    /// row (CR-02 §31) so future reproductions can detect mismatches.
    #[allow(clippy::too_many_arguments)]
    pub fn run_pipeline_persistent(
        &self,
        project_id: &str,
        session_ids: &[String],
        recipe_id: Option<&str>,
        application_version: &str,
        engine_version: &str,
        dag: &PipelineDag,
        stages: &HashMap<String, Arc<dyn Stage>>,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<PipelineRunReport> {
        let order = dag
            .topological_order()
            .map_err(topological_order_to_sqlite)?;

        let run_id = self.create_pipeline_run(
            project_id,
            session_ids,
            recipe_id,
            application_version,
            engine_version,
        )?;
        self.mark_run_started(&run_id)?;

        let mut stage_run_ids = Vec::with_capacity(order.len());
        let mut report_failed_at: Option<String> = None;

        for stage_id in &order {
            let attempt = self.next_stage_attempt(&run_id, stage_id)?;
            let started_at = Instant::now();
            let stage_record = StageRunRecord {
                stage_run_id: String::new(), // minted by the store
                run_id: run_id.clone(),
                stage_id: stage_id.clone(),
                status: "running".into(),
                attempt,
                params_json: Some(serde_json::to_string(&params).unwrap_or_default()),
                metrics_json: None,
                error: None,
                started_at: Some(now_iso()),
                completed_at: None,
            };
            let sr_id = self.record_stage_run(&stage_record)?;

            let outcome = match stages.get(stage_id) {
                Some(stage) => {
                    let ctx = StageContext {
                        stage_id: stage_id.clone(),
                        params: params.clone(),
                    };
                    stage.run(&ctx)
                }
                None => Err(StageError::Failed(format!(
                    "stage {stage_id} not registered"
                ))),
            };

            let completed_at = now_iso();
            let (final_status, metrics_json, error) = match outcome {
                Ok(result) if result.success => (
                    "completed".to_string(),
                    if result.metrics.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_string(&result.metrics).unwrap_or_default())
                    },
                    None,
                ),
                Ok(result) => (
                    "failed".to_string(),
                    None,
                    result
                        .error
                        .or_else(|| Some("stage returned success=false".into())),
                ),
                Err(e) => ("failed".to_string(), None, Some(e.to_string())),
            };
            let _ = started_at; // duration currently unused; available for receipts later

            // Append a terminal attempt row (reruns add rows, per §12).
            let mut terminal = stage_record;
            terminal.status = final_status.clone();
            terminal.metrics_json = metrics_json;
            terminal.error = error.clone();
            terminal.completed_at = Some(completed_at.clone());
            self.record_stage_run(&terminal)?;

            stage_run_ids.push(sr_id);

            // Audit hook: stage-level event (StageCompleted kind reused —
            // it already exists in domain::ProjectEventKind).
            let _ = self.record_event(
                project_id,
                ProjectEventKind::StageCompleted,
                Some(&format!(
                    "{{\"run_id\":\"{run_id}\",\"stage_id\":\"{stage_id}\",\"status\":\"{final_status}\"}}"
                )),
            );

            if final_status == "failed" {
                report_failed_at = Some(stage_id.clone());
                self.mark_run_finished(&run_id, PipelineRunStatus::Failed)?;
                return Ok(PipelineRunReport {
                    run_id,
                    stage_runs: stage_run_ids,
                    failed_at: report_failed_at,
                });
            }
        }

        self.mark_run_finished(&run_id, PipelineRunStatus::Completed)?;
        Ok(PipelineRunReport {
            run_id,
            stage_runs: stage_run_ids,
            failed_at: report_failed_at,
        })
    }
}

// Minimal stub so we don't pull in chrono just for an audit timestamp; the
// sqlite-side writes (`started_at`, `completed_at`, event `created_at`) use
// `datetime('now')` and that's the canonical source of truth. This helper
// exists only for the StageRunRecord row we record immediately so the
// string field is never empty/null — its value is replaced by the
// SQL-write on completion.
fn now_iso() -> String {
    "1970-01-01T00:00:00Z".to_string()
}

fn topological_order_to_sqlite<E: std::fmt::Display>(_e: E) -> DomainStoreError {
    DomainStoreError::Sqlite(rusqlite::Error::InvalidQuery)
}

// ─── Test-only Stage impls ─────────────────────────────────────────────────

#[cfg(test)]
mod test_stages {
    use super::*;
    use crate::pipeline::{Stage, StageContext, StageError, StageResult};

    pub struct EchoStage {
        pub id: String,
    }
    impl Stage for EchoStage {
        fn id(&self) -> &str {
            &self.id
        }
        fn run(&self, _ctx: &StageContext) -> std::result::Result<StageResult, StageError> {
            Ok(StageResult {
                stage_id: self.id.clone(),
                success: true,
                metrics: HashMap::new(),
                error: None,
            })
        }
    }

    pub struct FailStage {
        pub id: String,
        pub msg: String,
    }
    impl Stage for FailStage {
        fn id(&self) -> &str {
            &self.id
        }
        fn run(&self, _ctx: &StageContext) -> std::result::Result<StageResult, StageError> {
            Ok(StageResult {
                stage_id: self.id.clone(),
                success: false,
                metrics: HashMap::new(),
                error: Some(self.msg.clone()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_stages::{EchoStage, FailStage};
    use super::*;
    use crate::pipeline::PipelineDag;
    use std::path::PathBuf;

    fn store() -> DomainStore {
        DomainStore::new(&PathBuf::from(":memory:")).unwrap()
    }

    fn project(s: &DomainStore) -> String {
        s.create_project("M42", None, "0.1.0").unwrap()
    }

    fn dag(order: &[&str]) -> PipelineDag {
        // Build a linear DAG with the requested order. Edges are required
        // for topological_order to honor sequence (with no edges every
        // stage has in_degree 0).
        let stages = order.iter().map(|s| s.to_string()).collect();
        let mut edges = Vec::new();
        for w in order.windows(2) {
            edges.push((w[0].to_string(), w[1].to_string()));
        }
        PipelineDag { stages, edges }
    }

    #[test]
    fn run_persistent_persists_stages_and_marks_completed() {
        let s = store();
        let pid = project(&s);
        let mut stages: HashMap<String, Arc<dyn Stage>> = HashMap::new();
        stages.insert(
            "ingest".into(),
            Arc::new(EchoStage {
                id: "ingest".into(),
            }),
        );
        stages.insert(
            "stretch".into(),
            Arc::new(EchoStage {
                id: "stretch".into(),
            }),
        );

        let report = s
            .run_pipeline_persistent(
                &pid,
                &[],
                None,
                "0.1.0",
                "engine-1.0",
                &dag(&["ingest", "stretch"]),
                &stages,
                HashMap::new(),
            )
            .unwrap();
        assert!(report.failed_at.is_none());
        assert_eq!(report.stage_runs.len(), 2);
        assert_eq!(
            s.get_pipeline_run(&report.run_id).unwrap().status,
            PipelineRunStatus::Completed
        );

        // Per-stage rows: each stage gets a "running" row + a terminal
        // "completed" row (CR-02 §29 §12 — keep all attempts).
        let all = s.list_stage_runs(&report.run_id).unwrap();
        assert_eq!(all.len(), 4);
        // Terminal row per stage.
        for (stage_id, expected) in [("ingest", "completed"), ("stretch", "completed")] {
            let terminal = all
                .iter()
                .find(|r| r.stage_id == stage_id && r.status == expected)
                .unwrap_or_else(|| panic!("missing terminal row for {stage_id}"));
            assert_eq!(terminal.attempt, 1);
        }
        // Both stages recorded.
        let mut stages_seen: Vec<&str> = all.iter().map(|r| r.stage_id.as_str()).collect();
        stages_seen.sort_unstable();
        stages_seen.dedup();
        assert_eq!(stages_seen, vec!["ingest", "stretch"]);
    }

    #[test]
    fn run_persistent_marks_failed_when_stage_fails() {
        let s = store();
        let pid = project(&s);
        let mut stages: HashMap<String, Arc<dyn Stage>> = HashMap::new();
        stages.insert(
            "ingest".into(),
            Arc::new(EchoStage {
                id: "ingest".into(),
            }),
        );
        stages.insert(
            "denoise".into(),
            Arc::new(FailStage {
                id: "denoise".into(),
                msg: "out of VRAM".into(),
            }),
        );
        stages.insert(
            "stretch".into(),
            Arc::new(EchoStage {
                id: "stretch".into(),
            }),
        );

        let report = s
            .run_pipeline_persistent(
                &pid,
                &[],
                None,
                "0.1.0",
                "engine-1.0",
                &dag(&["ingest", "denoise", "stretch"]),
                &stages,
                HashMap::new(),
            )
            .unwrap();
        assert_eq!(report.failed_at.as_deref(), Some("denoise"));
        assert_eq!(
            s.get_pipeline_run(&report.run_id).unwrap().status,
            PipelineRunStatus::Failed
        );
        // Only the stages up to the failure ran. Each of ingest and
        // denoise contributes a "running" + a terminal row; stretch
        // never started.
        let all = s.list_stage_runs(&report.run_id).unwrap();
        assert_eq!(all.len(), 4);
        let stage_ids: Vec<&str> = all.iter().map(|r| r.stage_id.as_str()).collect();
        assert!(
            !stage_ids.contains(&"stretch"),
            "stretch should not have run"
        );
        let mut distinct = stage_ids.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct, vec!["denoise", "ingest"]);
        // ingest's terminal row is completed; denoise's terminal row is failed.
        let ingest_term = all
            .iter()
            .find(|r| r.stage_id == "ingest" && r.status != "running")
            .unwrap();
        let denoise_term = all
            .iter()
            .find(|r| r.stage_id == "denoise" && r.status != "running")
            .unwrap();
        assert_eq!(ingest_term.status, "completed");
        assert_eq!(denoise_term.status, "failed");
        assert_eq!(denoise_term.error.as_deref(), Some("out of VRAM"));

        // CR-02 §28: this run is exactly what find_interrupted_runs picks
        // up... well, Failed is terminal so it WON'T — but it ensures
        // completed_at was set.
        assert!(s
            .get_pipeline_run(&report.run_id)
            .unwrap()
            .completed_at
            .is_some());
        assert!(s.find_interrupted_runs().unwrap().is_empty());
    }

    #[test]
    fn rerunning_a_stage_increments_attempt_counter() {
        let s = store();
        let pid = project(&s);
        let mut stages: HashMap<String, Arc<dyn Stage>> = HashMap::new();
        stages.insert(
            "stretch".into(),
            Arc::new(EchoStage {
                id: "stretch".into(),
            }),
        );

        let first = s
            .run_pipeline_persistent(
                &pid,
                &[],
                None,
                "0.1.0",
                "engine-1.0",
                &dag(&["stretch"]),
                &stages,
                HashMap::new(),
            )
            .unwrap();
        let second = s
            .run_pipeline_persistent(
                &pid,
                &[],
                None,
                "0.1.0",
                "engine-1.0",
                &dag(&["stretch"]),
                &stages,
                HashMap::new(),
            )
            .unwrap();

        let all = s.list_stage_runs(&first.run_id).unwrap();
        let mut stretch = all.iter().filter(|r| r.stage_id == "stretch").cloned();
        assert_eq!(stretch.next().unwrap().attempt, 1);

        let all2 = s.list_stage_runs(&second.run_id).unwrap();
        let mut stretch2 = all2.iter().filter(|r| r.stage_id == "stretch").cloned();
        assert_eq!(stretch2.next().unwrap().attempt, 1);

        // Direct API also tracks attempts per run.
        assert_eq!(s.next_stage_attempt(&first.run_id, "stretch").unwrap(), 2);
    }
}
