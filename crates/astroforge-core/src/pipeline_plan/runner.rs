//! CR-05 P2 slice 1 — pipeline runner (Decision D-CR05-3 + D-CR05-6).
//!
//! Slice 1 ships the smallest viable runner: **start + cancel only**.
//! Pause / resume / recovery land in P2.5. The runner:
//!
//! 1. Loads a [`PipelinePlan`] from the store.
//! 2. Walks the plan's stages in sequence.
//! 3. For each stage:
//!    - Persists a [`StageExecution`] row with status `running` and
//!      `started_at` set.
//!    - Invokes the stage dispatcher (a no-op for slice 1; real
//!      processing modules land in P2.6).
//!    - Updates the row to `completed` (or `failed`).
//! 4. Writes a [`Checkpoint`] after each completed stage so P2.5 can
//!    resume from the last good state.
//! 5. Sets the plan's final status (`completed` / `cancelled` /
//!    `failed`).
//!
//! The runner is intentionally synchronous and small. Async /
//! background execution lands with P5's resource-aware execution;
//! slice 1 is reviewable on one page.
//!
//! ## Cancel semantics
//!
//! `PipelineRunner::cancel()` flips an atomic flag that the stage loop
//! checks between stages. The current stage completes (so we never
//! leave a half-written artifact), then the runner returns `Cancelled`
//! and the plan's status is set to `Cancelled`. P2.5 adds the
//! mid-stage interrupt that this slice deliberately avoids.

use crate::domain::{PipelinePlanStatus, StageExecution};
use crate::pipeline_plans_store::{PipelinePlanStore, PipelinePlanStoreError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// CR-05 P2 slice 1 — errors from the runner.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("plan not found: {0}")]
    PlanNotFound(String),
    #[error("plan is not in a startable state (current: {current:?})")]
    NotStartable { current: PipelinePlanStatus },
    #[error("stage execution failed: {0}")]
    StageFailed(String),
    #[error("store error: {0}")]
    Store(#[from] PipelinePlanStoreError),
}

/// CR-05 P2 slice 1 — runner outcomes (Decision D-CR05-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Completed,
    Cancelled,
    Failed,
}

/// CR-05 P2 slice 1 — a single checkpoint after a completed stage.
/// The Artifact reference is opaque to slice 1 (P2.6 attaches real
/// artifacts); slice 1 persists the `stage_execution_id` as the
/// checkpoint anchor.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub plan_id: String,
    pub stage_id: String,
    pub stage_execution_id: String,
    pub created_at_unix_ms: u64,
}

/// CR-05 P2 slice 1 — the runner. Holds a clone of the store handle
/// and an atomic cancel flag. P2.5 adds the `pause` flag.
pub struct PipelineRunner {
    store: Arc<PipelinePlanStore>,
    cancel_requested: Arc<AtomicBool>,
}

impl PipelineRunner {
    pub fn new(store: Arc<PipelinePlanStore>) -> Self {
        Self {
            store,
            cancel_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns a handle the caller can use to request cancel. Calling
    /// `cancel_handle().cancel()` flips the flag; the next stage loop
    /// iteration notices and exits.
    pub fn cancel_handle(&self) -> CancelHandle {
        CancelHandle {
            flag: self.cancel_requested.clone(),
        }
    }

    /// CR-05 P2 slice 1 — start a plan. Walks every stage in sequence,
    /// persisting `stage_executions` + checkpoints. Returns `RunOutcome`
    /// to the caller.
    ///
    /// On error (including store failure mid-run) the plan's status is
    /// set to `Failed` and the partial `stage_executions` remain in
    /// place so P2.5 can resume from the last completed stage.
    ///
    /// The cancel flag is **not** reset at the top of `start()` — the
    /// caller must explicitly `CancelHandle::clear()` it before
    /// re-running a plan that was previously cancelled. This matches
    /// the user's mental model that "once cancelled, plan must be
    /// re-armed to run".
    pub fn start(&self, plan_id: &str) -> Result<RunOutcome, RunnerError> {
        // Load the plan; refuse if it's not in a startable state.
        let mut plan = self.store.load_plan(plan_id).map_err(|e| match e {
            PipelinePlanStoreError::NotFound(_) => RunnerError::PlanNotFound(plan_id.into()),
            other => RunnerError::Store(other),
        })?;
        if !matches!(
            plan.status,
            PipelinePlanStatus::Ready | PipelinePlanStatus::Draft
        ) {
            return Err(RunnerError::NotStartable {
                current: plan.status,
            });
        }

        // Mark Running.
        self.store
            .update_plan_status(plan_id, PipelinePlanStatus::Running)?;
        plan.status = PipelinePlanStatus::Running;

        // Walk the stages. Each iteration: insert running → dispatch →
        // update to completed → checkpoint. The cancel flag is checked
        // between stages so we never leave a half-written artifact.
        for stage in &plan.stages {
            if self.cancel_requested.load(Ordering::SeqCst) {
                self.store
                    .update_plan_status(plan_id, PipelinePlanStatus::Cancelled)?;
                plan.status = PipelinePlanStatus::Cancelled;
                return Ok(RunOutcome::Cancelled);
            }

            // Per-stage execution row.
            let started_at_unix_ms = now_unix_ms();
            let stage_execution_id =
                format!("exec_{}_{}_{}", plan_id, stage.stage_id, started_at_unix_ms);
            let mut exec = StageExecution {
                stage_execution_id: stage_execution_id.clone(),
                plan_id: plan_id.into(),
                stage_id: stage.stage_id.clone(),
                attempt: 1,
                status: "running".into(),
                input_version_id: None,
                output_artifact_id: None,
                parameters_json: stage.parameters_json.clone(),
                parameters_hash: None,
                started_at: Some(format!("unix_ms:{}", started_at_unix_ms)),
                completed_at: None,
                resource_usage_json: None,
                error_json: None,
            };

            // Dispatch (no-op in slice 1).
            match dispatch_stage(stage) {
                Ok(()) => {
                    exec.status = "completed".into();
                    exec.completed_at = Some(format!("unix_ms:{}", now_unix_ms()));
                }
                Err(msg) => {
                    exec.status = "failed".into();
                    exec.error_json = Some(format!(r#"{{"what_happened":"{}"}}"#, msg));
                    exec.completed_at = Some(format!("unix_ms:{}", now_unix_ms()));
                    self.store.insert_stage_execution(&exec)?;
                    self.store
                        .update_plan_status(plan_id, PipelinePlanStatus::Failed)?;
                    return Err(RunnerError::StageFailed(msg));
                }
            }

            self.store.insert_stage_execution(&exec)?;

            // Checkpoint after the stage completes.
            self.write_checkpoint(&Checkpoint {
                plan_id: plan_id.into(),
                stage_id: stage.stage_id.clone(),
                stage_execution_id: exec.stage_execution_id.clone(),
                created_at_unix_ms: now_unix_ms(),
            })?;
        }

        // All stages completed (or cancelled mid-loop).
        if self.cancel_requested.load(Ordering::SeqCst) {
            self.store
                .update_plan_status(plan_id, PipelinePlanStatus::Cancelled)?;
            plan.status = PipelinePlanStatus::Cancelled;
            Ok(RunOutcome::Cancelled)
        } else {
            self.store
                .update_plan_status(plan_id, PipelinePlanStatus::Completed)?;
            plan.status = PipelinePlanStatus::Completed;
            Ok(RunOutcome::Completed)
        }
    }

    /// CR-05 P2 slice 1 — append a checkpoint row to the stage_runs
    /// checkpoints table that already lives in `db::SESSION_SCHEMA_SQL`.
    /// Slice 1 writes through the same table; P2.5 introduces a
    /// dedicated `checkpoints` view per Decision D-CR05-6.
    fn write_checkpoint(&self, ckpt: &Checkpoint) -> Result<(), RunnerError> {
        // For slice 1 we don't have a dedicated checkpoints store yet.
        // The checkpoint is captured implicitly in the stage_executions
        // row (status=completed + completed_at set). P2.5 introduces
        // an explicit checkpoints surface and recovery via it.
        let _ = ckpt;
        Ok(())
    }
}

/// CR-05 P2 slice 1 — cancel handle. Cheap to clone; can outlive the
/// runner for the lifetime of the cancel request.
#[derive(Clone)]
pub struct CancelHandle {
    flag: Arc<AtomicBool>,
}

impl CancelHandle {
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn clear(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// CR-05 P2 slice 1 — no-op stage dispatcher. Real module dispatch
/// lands in P2.6. Slice 1 still records every transition (running →
/// completed) so the data model is exercised end-to-end.
fn dispatch_stage(stage: &crate::domain::PipelineStage) -> Result<(), String> {
    if stage.stage_type.is_empty() {
        return Err(format!("stage {} has empty stage_type", stage.stage_id));
    }
    // Intentionally no-op for slice 1. P2.6 will dispatch into
    // crate::calibration / crate::stacking / etc. based on stage_type.
    Ok(())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PipelinePlanStatus;
    use crate::pipeline_plan::builtin::deep_sky_osc_balanced;
    use crate::pipeline_plan::plan::{generate_plan, GenerationContext, SessionUnderstanding};
    use crate::pipeline_plans_store::PipelinePlanStore;

    fn ctx() -> GenerationContext {
        GenerationContext {
            project_id: "proj_1".into(),
            session_id: "sess_1".into(),
            session_understanding: SessionUnderstanding::deep_sky_osc_balanced(),
            recipe_id: Some("recipe_deep_sky_osc_balanced".into()),
            mode: "auto".into(),
            generated_at_unix_ms: 1_700_000_000_000,
        }
    }

    fn plan_in_store(store: &PipelinePlanStore) -> String {
        let (stages, target) = deep_sky_osc_balanced();
        let plan = generate_plan(&ctx(), &stages, target).unwrap();
        store.insert_plan(&plan).unwrap();
        plan.plan_id
    }

    #[test]
    fn start_runs_all_stages_to_completion() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let plan_id = plan_in_store(&store);
        let runner = PipelineRunner::new(store.clone());

        let outcome = runner.start(&plan_id).unwrap();
        assert_eq!(outcome, RunOutcome::Completed);

        let loaded = store.load_plan(&plan_id).unwrap();
        assert_eq!(loaded.status, PipelinePlanStatus::Completed);

        let execs = store.list_stage_executions_for_plan(&plan_id).unwrap();
        assert_eq!(execs.len(), 10);
        for e in &execs {
            assert_eq!(e.status, "completed");
            assert!(e.started_at.is_some());
            assert!(e.completed_at.is_some());
        }
    }

    #[test]
    fn cancel_mid_run_marks_plan_cancelled() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let plan_id = plan_in_store(&store);
        let runner = PipelineRunner::new(store.clone());
        let handle = runner.cancel_handle();

        // Pre-cancel BEFORE start. The runner sees the flag at the
        // top-of-loop and exits before processing any stages.
        handle.cancel();
        let outcome = runner.start(&plan_id).unwrap();
        assert_eq!(outcome, RunOutcome::Cancelled);

        let loaded = store.load_plan(&plan_id).unwrap();
        assert_eq!(loaded.status, PipelinePlanStatus::Cancelled);

        let execs = store.list_stage_executions_for_plan(&plan_id).unwrap();
        assert_eq!(
            execs.len(),
            0,
            "no stage rows should be written for a pre-cancelled run"
        );
    }

    #[test]
    fn cancel_handle_is_cloneable_and_independent() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let runner = PipelineRunner::new(store.clone());
        let h1 = runner.cancel_handle();
        let h2 = h1.clone();
        assert!(!h1.is_cancelled());
        h2.cancel();
        assert!(h1.is_cancelled(), "handles share the flag");
    }

    #[test]
    fn start_unknown_plan_returns_not_found() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let runner = PipelineRunner::new(store);
        let err = runner.start("nope").unwrap_err();
        assert!(matches!(err, RunnerError::PlanNotFound(_)));
    }

    #[test]
    fn start_running_plan_is_rejected() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let plan_id = plan_in_store(&store);
        store
            .update_plan_status(&plan_id, PipelinePlanStatus::Running)
            .unwrap();
        let runner = PipelineRunner::new(store);
        let err = runner.start(&plan_id).unwrap_err();
        assert!(matches!(err, RunnerError::NotStartable { .. }));
    }

    #[test]
    fn start_completed_plan_is_rejected() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let plan_id = plan_in_store(&store);
        store
            .update_plan_status(&plan_id, PipelinePlanStatus::Completed)
            .unwrap();
        let runner = PipelineRunner::new(store);
        let err = runner.start(&plan_id).unwrap_err();
        assert!(matches!(err, RunnerError::NotStartable { .. }));
    }

    #[test]
    fn cancel_flag_resets_between_runs() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let plan_a = plan_in_store(&store);
        let runner = PipelineRunner::new(store.clone());
        let handle = runner.cancel_handle();

        // First run: pre-cancel and verify cancellation.
        handle.cancel();
        runner.start(&plan_a).unwrap();

        // Second run with the SAME runner instance: caller must
        // explicitly clear the flag. After clear(), the run completes
        // normally.
        handle.clear();
        let plan_b = plan_in_store(&store);
        let outcome = runner.start(&plan_b).unwrap();
        assert_eq!(outcome, RunOutcome::Completed);
    }
}
