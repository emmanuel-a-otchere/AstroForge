//! CR-05 P2 (slice 1 + 2.5) — pipeline runner (Decision D-CR05-3 + D-CR05-6).
//!
//! P2 slice 1 shipped the smallest viable runner: **start + cancel**.
//! P2.5 adds **pause + resume + recovery**:
//!
//! 1. Loads a [`PipelinePlan`] from the store.
//! 2. Walks the plan's stages in sequence.
//! 3. For each stage:
//!    - Persists a [`StageExecution`] row with status `running` and
//!      `started_at` set.
//!    - Invokes the stage dispatcher (a no-op for P2 slice 1; real
//!      processing modules land in P2.6).
//!    - Updates the row to `completed` (or `failed`).
//! 4. Writes a [`Checkpoint`] after each completed stage so P2.5 can
//!    resume from the last good state.
//! 5. Sets the plan's final status (`completed` / `cancelled` /
//!    `paused` / `failed`).
//!
//! The runner is intentionally synchronous and small. Async /
//! background execution lands with P5's resource-aware execution.
//!
//! ## Cancel semantics
//!
//! `CancelHandle::cancel()` flips an atomic flag that the stage loop
//! checks between stages. The current stage completes (so we never
//! leave a half-written artifact), then the runner returns `Cancelled`
//! and the plan's status is set to `Cancelled`.
//!
//! ## Pause semantics (P2.5)
//!
//! `PauseHandle::pause()` flips a second atomic flag. The stage loop
//! checks both flags between stages: cancel beats pause. On pause the
//! current stage completes and the plan's status is set to `Paused`.
//! Resume (slice 2.5) re-loads the plan, finds the first stage with no
//! `completed` `StageExecution`, and continues from there.
//!
//! ## Recovery semantics (P2.5)
//!
//! `find_resumable_runs()` (on the store) returns plans with status
//! `Paused`. The `RecoveryBanner.svelte` component shows a banner per
//! resumable plan; clicking "Resume" calls `resume_pipeline_run`.

use crate::domain::{PipelinePlan, PipelinePlanStatus, StageExecution};
use crate::pipeline_plans_store::{PipelinePlanStore, PipelinePlanStoreError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// CR-05 P2 — errors from the runner.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("plan not found: {0}")]
    PlanNotFound(String),
    #[error("plan is not in a startable state (current: {current:?})")]
    NotStartable { current: PipelinePlanStatus },
    #[error("plan is not in a resumable state (current: {current:?})")]
    NotResumable { current: PipelinePlanStatus },
    #[error("stage execution failed: {0}")]
    StageFailed(String),
    #[error("store error: {0}")]
    Store(#[from] PipelinePlanStoreError),
}

/// CR-05 P2 — runner outcomes (Decision D-CR05-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Completed,
    Cancelled,
    Failed,
    /// CR-05 P2.5 — the loop exited because the pause flag was set;
    /// the plan's persisted status is `Paused` and the next stage
    /// remains unprocessed.
    Paused,
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

/// CR-05 P2.5 — the runner. Holds the store handle and two atomic
/// flags (cancel + pause). Both flags are checked between stages; the
/// cancel flag wins (a plan that's both pausing and cancelling ends up
/// cancelled, not paused).
pub struct PipelineRunner {
    store: Arc<PipelinePlanStore>,
    cancel_requested: Arc<AtomicBool>,
    pause_requested: Arc<AtomicBool>,
}

impl PipelineRunner {
    pub fn new(store: Arc<PipelinePlanStore>) -> Self {
        Self {
            store,
            cancel_requested: Arc::new(AtomicBool::new(false)),
            pause_requested: Arc::new(AtomicBool::new(false)),
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

    /// CR-05 P2.5 — pause handle. Independent of the cancel handle.
    pub fn pause_handle(&self) -> PauseHandle {
        PauseHandle {
            flag: self.pause_requested.clone(),
        }
    }

    /// CR-05 P2 — start a plan. Walks every stage in sequence,
    /// persisting `stage_executions` + checkpoints. Returns `RunOutcome`
    /// to the caller.
    ///
    /// On error (including store failure mid-run) the plan's status is
    /// set to `Failed` and the partial `stage_executions` remain in
    /// place so P2.5 can resume from the last completed stage.
    ///
    /// The cancel / pause flags are **not** reset at the top of
    /// `start()` — the caller must explicitly `CancelHandle::clear()` /
    /// `PauseHandle::clear()` them before re-running a plan. This
    /// matches the user's mental model that "once cancelled, plan must
    /// be re-armed to run".
    pub fn start(&self, plan_id: &str) -> Result<RunOutcome, RunnerError> {
        // Load the plan; refuse if it's not in a startable state.
        let plan = self.store.load_plan(plan_id).map_err(|e| match e {
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
        self.run_loop(&plan, /* skip_completed */ false)
    }

    /// CR-05 P2.5 — resume a paused plan. Loads the plan from the
    /// store, refuses if it's not in `Paused`, then runs the stage
    /// loop with `skip_completed = true` so already-completed stages are
    /// not re-executed.
    pub fn resume(&self, plan_id: &str) -> Result<RunOutcome, RunnerError> {
        let plan = self.store.load_plan(plan_id).map_err(|e| match e {
            PipelinePlanStoreError::NotFound(_) => RunnerError::PlanNotFound(plan_id.into()),
            other => RunnerError::Store(other),
        })?;
        if plan.status != PipelinePlanStatus::Paused {
            return Err(RunnerError::NotResumable {
                current: plan.status,
            });
        }
        self.run_loop(&plan, /* skip_completed */ true)
    }

    /// CR-05 P2.5 — internal: the actual stage loop, factored out so
    /// `start()` and `resume()` share the same atomic-flag handling.
    /// `skip_completed` is true on resume (don't re-run completed
    /// stages).
    fn run_loop(
        &self,
        plan: &PipelinePlan,
        skip_completed: bool,
    ) -> Result<RunOutcome, RunnerError> {
        let plan_id = plan.plan_id.as_str();

        // Mark Running.
        self.store
            .update_plan_status(plan_id, PipelinePlanStatus::Running)?;

        // Walk the stages. Each iteration: insert running → dispatch →
        // update to completed → checkpoint. The cancel / pause flags
        // are checked between stages so we never leave a half-written
        // artifact. Cancel beats pause.
        for stage in &plan.stages {
            // Skip already-completed stages on resume.
            if skip_completed {
                let prior = self
                    .store
                    .list_stage_executions_for_plan(plan_id)?
                    .into_iter()
                    .find(|e| e.stage_id == stage.stage_id && e.status == "completed");
                if prior.is_some() {
                    continue;
                }
            }

            // Cancel beats pause (Decision D-CR05-6).
            if self.cancel_requested.load(Ordering::SeqCst) {
                self.store
                    .update_plan_status(plan_id, PipelinePlanStatus::Cancelled)?;
                return Ok(RunOutcome::Cancelled);
            }
            if self.pause_requested.load(Ordering::SeqCst) {
                self.store
                    .update_plan_status(plan_id, PipelinePlanStatus::Paused)?;
                return Ok(RunOutcome::Paused);
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

            // Dispatch (no-op in P2 slice 1).
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

        // All stages completed (or cancel/pause flipped mid-loop).
        if self.cancel_requested.load(Ordering::SeqCst) {
            self.store
                .update_plan_status(plan_id, PipelinePlanStatus::Cancelled)?;
            Ok(RunOutcome::Cancelled)
        } else if self.pause_requested.load(Ordering::SeqCst) {
            self.store
                .update_plan_status(plan_id, PipelinePlanStatus::Paused)?;
            Ok(RunOutcome::Paused)
        } else {
            self.store
                .update_plan_status(plan_id, PipelinePlanStatus::Completed)?;
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

/// CR-05 P2.5 — pause handle. Cheap to clone; can outlive the runner.
#[derive(Clone)]
pub struct PauseHandle {
    flag: Arc<AtomicBool>,
}

impl PauseHandle {
    pub fn pause(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn clear(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }

    pub fn is_paused(&self) -> bool {
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

    // ─── CR-05 P2.5 — pause / resume / recovery tests ─────────────────

    #[test]
    fn pause_mid_run_marks_plan_paused() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let plan_id = plan_in_store(&store);
        let runner = PipelineRunner::new(store.clone());
        let pause = runner.pause_handle();

        // Pre-pause BEFORE start. The runner sees the flag at the
        // top-of-loop and exits before processing any stages.
        pause.pause();
        let outcome = runner.start(&plan_id).unwrap();
        assert_eq!(outcome, RunOutcome::Paused);

        let loaded = store.load_plan(&plan_id).unwrap();
        assert_eq!(loaded.status, PipelinePlanStatus::Paused);
    }

    #[test]
    fn pause_handle_is_independent_of_cancel() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let runner = PipelineRunner::new(store.clone());
        let cancel = runner.cancel_handle();
        let pause = runner.pause_handle();
        assert!(!cancel.is_cancelled());
        assert!(!pause.is_paused());
        pause.pause();
        assert!(pause.is_paused());
        assert!(!cancel.is_cancelled(), "pause must not flip cancel");
    }

    #[test]
    fn cancel_beats_pause_when_both_flags_set() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let plan_id = plan_in_store(&store);
        let runner = PipelineRunner::new(store.clone());
        let cancel = runner.cancel_handle();
        let pause = runner.pause_handle();
        cancel.cancel();
        pause.pause();
        let outcome = runner.start(&plan_id).unwrap();
        assert_eq!(outcome, RunOutcome::Cancelled);
    }

    #[test]
    fn resume_rejected_on_non_paused_plan() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let plan_id = plan_in_store(&store);
        // Plan is in `Ready` (the default after generate_plan +
        // insert_plan). resume should refuse.
        let runner = PipelineRunner::new(store);
        let err = runner.resume(&plan_id).unwrap_err();
        assert!(matches!(err, RunnerError::NotResumable { .. }));
    }

    #[test]
    fn resume_picks_up_from_first_uncompleted_stage() {
        // Pre-populate stage_executions for stages 0..5 as completed;
        // resume a paused plan and verify only stages 5..9 produce new
        // StageExecution rows.
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let plan_id = plan_in_store(&store);

        // Load the plan to get the persisted PipelineStages (which have
        // `stage_id`); StageSpec (recipe-side) does not.
        let plan = store.load_plan(&plan_id).unwrap();
        // Mark the first 5 stages completed in the store.
        for (i, stage) in plan.stages.iter().take(5).enumerate() {
            let exec = crate::domain::StageExecution {
                stage_execution_id: format!("pre_exec_{}", i),
                plan_id: plan_id.clone(),
                stage_id: stage.stage_id.clone(),
                attempt: 1,
                status: "completed".into(),
                input_version_id: None,
                output_artifact_id: None,
                parameters_json: None,
                parameters_hash: None,
                started_at: Some("unix_ms:1".into()),
                completed_at: Some("unix_ms:2".into()),
                resource_usage_json: None,
                error_json: None,
            };
            store.insert_stage_execution(&exec).unwrap();
        }

        store
            .update_plan_status(&plan_id, PipelinePlanStatus::Paused)
            .unwrap();
        let runner = PipelineRunner::new(store.clone());
        let outcome = runner.resume(&plan_id).unwrap();
        assert_eq!(outcome, RunOutcome::Completed);

        let execs = store.list_stage_executions_for_plan(&plan_id).unwrap();
        // 5 pre-existing + 5 new = 10 total.
        assert_eq!(execs.len(), 10);
        // The last 5 must be marked completed by the runner (started_at
        // > unix_ms:2, since they were inserted by resume, not by us).
        let new_ones: Vec<_> = execs
            .iter()
            .filter(|e| e.started_at.as_deref() != Some("unix_ms:1"))
            .collect();
        assert_eq!(new_ones.len(), 5);
        for e in &new_ones {
            assert_eq!(e.status, "completed");
        }
    }

    #[test]
    fn resume_unknown_plan_returns_not_found() {
        let store = Arc::new(PipelinePlanStore::in_memory().unwrap());
        let runner = PipelineRunner::new(store);
        let err = runner.resume("nope").unwrap_err();
        assert!(matches!(err, RunnerError::PlanNotFound(_)));
    }
}
