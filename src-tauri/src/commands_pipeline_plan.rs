//! CR-05 P1 — Tauri command surface for PipelinePlan generation
//! (Decision D-CR05-3 + PLAN.md P1).
//!
//! **Additive only.** Existing widgets continue to read from
//! `sessionStore`, `pipeline-store`, etc. The commands below are dormant
//! until a future CR-05 PR wires the Svelte side to them.
//!
//! P1 ships `create_pipeline_plan` (auto mode produces a plan) +
//! `pipeline_plan_list_for_project`. P2 will add the runner that
//! produces `StageExecution` rows; P3 will wire the recommendation
//! engine; P5 will expose the Expert-mode DAG view.

use std::sync::{Arc, Mutex};

use astroforge_core::domain::{ObjectType, PipelinePlan, PipelinePlanStatus};
use astroforge_core::pipeline_plan::{
    builtin::deep_sky_osc_balanced,
    dispatch::{CalibrateHandler, DenoiseHandler, HandlerRegistry, StackHandler, StretchHandler},
    plan::{generate_plan as generate_plan_inner, GenerationContext, SessionUnderstanding},
    runner::{CancelHandle, PauseHandle, PipelineRunner, RunOutcome},
    AcquisitionMode, CalibrationAvailability,
};
use astroforge_core::pipeline_plans_store::{PipelinePlanStore, PipelinePlanStoreError};
use serde::{Deserialize, Serialize};
use tauri::State;

/// CR-05 P1 — Tauri-managed state for the PipelinePlan store.
pub struct PipelinePlanState {
    pub store: Arc<Mutex<PipelinePlanStore>>,
    /// P2 slice 1 — per-plan cancel handles. Keyed by plan_id so the
    /// UI can request cancel after start has returned.
    pub cancel_handles: Mutex<std::collections::HashMap<String, CancelHandle>>,
    /// P2.5 — per-plan pause handles. Independent of cancel handles
    /// (per Decision D-CR05-6).
    pub pause_handles: Mutex<std::collections::HashMap<String, PauseHandle>>,
    /// CR-05 P2.6 — handler registry. Each Tauri command constructs
    /// a fresh runner and clones this Arc.
    pub handler_registry: Arc<astroforge_core::pipeline_plan::dispatch::HandlerRegistry>,
    /// CR-05 P2.6 — DomainStore handle for handlers that need to
    /// load source assets / write artifacts. None is allowed for
    /// tests; production sets this in `main.rs` setup.
    pub domain_store: Option<Arc<astroforge_core::domain_store::DomainStore>>,
}

/// CR-05 P1 — input to `create_pipeline_plan`. Mirrors the `astroforge-api.ts`
/// `CreatePipelinePlanRequest` interface.
#[derive(Debug, Deserialize)]
pub struct CreatePipelinePlanRequest {
    pub project_id: String,
    pub session_id: String,
    /// Built-in recipe id. P1 only supports "deep_sky_osc_balanced";
    /// additional recipes land in later phases. Unknown ids fall back to
    /// "deep_sky_osc_balanced" for now (P3 adds the registry).
    pub recipe_id: Option<String>,
    /// "auto" | "guided" | "expert". Default "auto".
    pub mode: Option<String>,
    /// Optional explicit Session Understanding from CR-04 (P3 wires this
    /// for real; P1 lets the caller pass None to use a default).
    pub session_understanding: Option<SessionUnderstandingPayload>,
}

/// CR-05 P1 — frontend-facing Session Understanding shape. Mirrors the
/// TypeScript `SessionUnderstanding` interface.
#[derive(Debug, Deserialize)]
pub struct SessionUnderstandingPayload {
    pub target_type: String,
    pub acquisition: String,
    pub light_frame_count: u32,
    pub calibration: String,
    pub bayer_pattern: Option<String>,
    pub narrowband_filters: Vec<String>,
}

impl SessionUnderstandingPayload {
    fn into_domain(self) -> SessionUnderstanding {
        SessionUnderstanding {
            target_type: parse_target_type(&self.target_type),
            acquisition: parse_acquisition(&self.acquisition),
            light_frame_count: self.light_frame_count,
            calibration: parse_calibration(&self.calibration),
            bayer_pattern: self.bayer_pattern,
            narrowband_filters: self.narrowband_filters,
        }
    }
}

fn parse_target_type(s: &str) -> ObjectType {
    match s {
        "deep_sky" | "deepsky" => ObjectType::DeepSky,
        "planet" => ObjectType::Planet,
        "lunar" => ObjectType::Lunar,
        "solar" => ObjectType::Solar,
        _ => ObjectType::Unknown,
    }
}

fn parse_acquisition(s: &str) -> AcquisitionMode {
    match s {
        "osc" => AcquisitionMode::Osc,
        "mono" => AcquisitionMode::Mono,
        "narrowband" => AcquisitionMode::Narrowband,
        "planetary" => AcquisitionMode::Planetary,
        "lunar" => AcquisitionMode::Lunar,
        "solar" => AcquisitionMode::Solar,
        _ => AcquisitionMode::Osc,
    }
}

fn parse_calibration(s: &str) -> CalibrationAvailability {
    match s {
        "none" => CalibrationAvailability::None,
        "partial" => CalibrationAvailability::Partial,
        "full" => CalibrationAvailability::Full,
        _ => CalibrationAvailability::None,
    }
}

/// CR-05 P1 — frontend-facing PipelinePlan summary (mirrors the
/// TypeScript `PipelinePlan` interface). Returns the full plan including
/// the ordered `stages` array — the P2 runner will re-load this by id.
#[derive(Debug, Serialize)]
pub struct PipelinePlanDto {
    pub plan_id: String,
    pub project_id: String,
    pub session_id: String,
    pub recipe_id: Option<String>,
    pub mode: String,
    pub target_type: String,
    pub status: String,
    pub created_at: String,
    pub schema_version: u32,
    pub stages: Vec<PipelineStageDto>,
}

#[derive(Debug, Serialize)]
pub struct PipelineStageDto {
    pub stage_id: String,
    pub plan_id: String,
    pub stage_type: String,
    pub sequence: u32,
    pub label: String,
    pub required: bool,
    pub enabled: bool,
    pub parameters_json: Option<String>,
    pub produces_image_version: bool,
    pub undo_supported: bool,
}

impl From<&PipelinePlan> for PipelinePlanDto {
    fn from(p: &PipelinePlan) -> Self {
        Self {
            plan_id: p.plan_id.clone(),
            project_id: p.project_id.clone(),
            session_id: p.session_id.clone(),
            recipe_id: p.recipe_id.clone(),
            mode: p.mode.clone(),
            target_type: format!("{:?}", p.target_type).to_lowercase(),
            status: format!("{:?}", p.status).to_lowercase(),
            created_at: p.created_at.clone(),
            schema_version: p.schema_version,
            stages: p.stages.iter().map(PipelineStageDto::from).collect(),
        }
    }
}

impl From<&astroforge_core::domain::PipelineStage> for PipelineStageDto {
    fn from(s: &astroforge_core::domain::PipelineStage) -> Self {
        Self {
            stage_id: s.stage_id.clone(),
            plan_id: s.plan_id.clone(),
            stage_type: s.stage_type.clone(),
            sequence: s.sequence,
            label: s.label.clone(),
            required: s.required,
            enabled: s.enabled,
            parameters_json: s.parameters_json.clone(),
            produces_image_version: s.produces_image_version,
            undo_supported: s.undo_supported,
        }
    }
}

#[tauri::command]
pub fn create_pipeline_plan(
    state: State<'_, PipelinePlanState>,
    request: CreatePipelinePlanRequest,
) -> Result<PipelinePlanDto, String> {
    let (recipe_stages, recipe_target) = match request.recipe_id.as_deref() {
        None | Some("") | Some("deep_sky_osc_balanced") => deep_sky_osc_balanced(),
        Some(other) => {
            // P1 only ships one recipe; unknown ids fall back. P3 adds
            // the registry and rejects unknown ids explicitly.
            let _ = other;
            deep_sky_osc_balanced()
        }
    };

    let session_understanding = request
        .session_understanding
        .map(SessionUnderstandingPayload::into_domain)
        .unwrap_or_else(SessionUnderstanding::deep_sky_osc_balanced);

    let ctx = GenerationContext {
        project_id: request.project_id.clone(),
        session_id: request.session_id.clone(),
        session_understanding,
        recipe_id: Some("recipe_deep_sky_osc_balanced".into()),
        mode: request.mode.unwrap_or_else(|| "auto".into()),
        generated_at_unix_ms: now_unix_ms(),
    };

    let plan = generate_plan_inner(&ctx, &recipe_stages, recipe_target)
        .map_err(|e| format!("plan generation failed: {e}"))?;

    let store = state.store.lock().map_err(lock_err)?;
    store.insert_plan(&plan).map_err(store_err_to_string)?;

    Ok(PipelinePlanDto::from(&plan))
}

#[tauri::command]
pub fn pipeline_plan_list_for_project(
    state: State<'_, PipelinePlanState>,
    project_id: String,
) -> Result<Vec<astroforge_core::pipeline_plans_store::PipelinePlanSummary>, String> {
    let store = state.store.lock().map_err(lock_err)?;
    store
        .list_plans_for_project(&project_id)
        .map_err(store_err_to_string)
}

#[tauri::command]
pub fn pipeline_plan_get(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
) -> Result<PipelinePlanDto, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let plan = store.load_plan(&plan_id).map_err(store_err_to_string)?;
    Ok(PipelinePlanDto::from(&plan))
}

// ─── CR-05 P2 slice 1 — start + cancel commands ────────────────────────────
//
// Pause / resume land in P2.5. The runner is synchronous and small:
// start walks the plan's stages, persisting stage_executions, and
// returns RunOutcome. cancel flips an atomic flag that the runner
// checks between stages; the current stage completes (no half-written
// artifacts) before the runner returns RunOutcome::Cancelled.

/// CR-05 P2 slice 1 — start a plan. Returns the run outcome
/// (`completed` / `cancelled` / `failed`).
///
/// P2.5 — also registers the pause handle so the UI can pause during
/// execution.
///
/// P2.6 — uses the handler registry + shared DomainStore so real
/// stages (Stack) execute instead of no-op.
#[tauri::command]
pub fn start_pipeline_run(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
) -> Result<RunOutcomeResponse, String> {
    let runner = {
        let store = state.store.clone();
        PipelineRunner::with_handlers(
            store,
            state.handler_registry.clone(),
            state.domain_store.clone(),
        )
    };
    let cancel_handle = runner.cancel_handle();
    let pause_handle = runner.pause_handle();

    // Register the handles so the UI can flip them later.
    {
        let mut cancels = state.cancel_handles.lock().map_err(lock_err)?;
        cancels.insert(plan_id.clone(), cancel_handle.clone());
    }
    {
        let mut pauses = state.pause_handles.lock().map_err(lock_err)?;
        pauses.insert(plan_id.clone(), pause_handle.clone());
    }

    let result = runner.start(&plan_id);

    // Clean up both handles now that the run has finished.
    {
        let mut cancels = state.cancel_handles.lock().map_err(lock_err)?;
        cancels.remove(&plan_id);
    }
    {
        let mut pauses = state.pause_handles.lock().map_err(lock_err)?;
        pauses.remove(&plan_id);
    }

    match result {
        Ok(outcome) => Ok(RunOutcomeResponse::from(outcome)),
        Err(e) => Err(e.to_string()),
    }
}

/// CR-05 P2.5 — pause a running plan. The runner notices the flag
/// between stages, finishes the current stage, and exits with
/// `RunOutcome::Paused`. Idempotent — returns `false` if no live run
/// is in flight.
#[tauri::command]
pub fn pause_pipeline_run(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
) -> Result<bool, String> {
    let mut handles = state.pause_handles.lock().map_err(lock_err)?;
    if let Some(handle) = handles.get_mut(&plan_id) {
        handle.pause();
        Ok(true)
    } else {
        Ok(false)
    }
}

/// CR-05 P2.5 — resume a paused plan. Walks stages from the first one
/// without a completed `StageExecution` row; returns the run outcome.
#[tauri::command]
pub fn resume_pipeline_run(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
) -> Result<RunOutcomeResponse, String> {
    let runner = {
        let store = state.store.clone();
        PipelineRunner::with_handlers(
            store,
            state.handler_registry.clone(),
            state.domain_store.clone(),
        )
    };
    // Pause handle is re-registered so the UI can pause mid-resume.
    let pause_handle = runner.pause_handle();
    {
        let mut pauses = state.pause_handles.lock().map_err(lock_err)?;
        pauses.insert(plan_id.clone(), pause_handle);
    }

    let result = runner.resume(&plan_id);

    {
        let mut pauses = state.pause_handles.lock().map_err(lock_err)?;
        pauses.remove(&plan_id);
    }

    match result {
        Ok(outcome) => Ok(RunOutcomeResponse::from(outcome)),
        Err(e) => Err(e.to_string()),
    }
}

/// CR-05 P2.5 — list resumable plans for a project (status = Paused).
/// Used by `RecoveryBanner.svelte` to render the per-project banner on
/// Project open.
#[tauri::command]
pub fn pipeline_plan_list_resumable_for_project(
    state: State<'_, PipelinePlanState>,
    project_id: String,
) -> Result<Vec<astroforge_core::pipeline_plans_store::PipelinePlanSummary>, String> {
    let store = state.store.lock().map_err(lock_err)?;
    store
        .list_resumable_plans_for_project(&project_id)
        .map_err(store_err_to_string)
}

/// CR-05 P2 slice 1 — cancel a running plan. The runner notices the
/// flag between stages, finishes the current stage, and exits with
/// `RunOutcome::Cancelled`. Idempotent — calling cancel on a plan that
/// has no live handle is a no-op (returns `false` so the UI can show
/// "no run to cancel").
#[tauri::command]
pub fn cancel_pipeline_run(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
) -> Result<bool, String> {
    let mut handles = state.cancel_handles.lock().map_err(lock_err)?;
    if let Some(handle) = handles.get_mut(&plan_id) {
        handle.cancel();
        Ok(true)
    } else {
        Ok(false)
    }
}

/// CR-05 P2.5 — frontend-facing outcome shape.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcomeResponse {
    Completed,
    Cancelled,
    Failed,
    Paused,
}

impl From<RunOutcome> for RunOutcomeResponse {
    fn from(o: RunOutcome) -> Self {
        match o {
            RunOutcome::Completed => Self::Completed,
            RunOutcome::Cancelled => Self::Cancelled,
            RunOutcome::Failed => Self::Failed,
            RunOutcome::Paused => Self::Paused,
        }
    }
}

/// CR-05 P2 slice 1 — frontend-facing stage execution summary.
#[derive(Debug, Serialize)]
pub struct StageExecutionSummary {
    pub stage_execution_id: String,
    pub plan_id: String,
    pub stage_id: String,
    pub attempt: u32,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_json: Option<String>,
}

impl From<&astroforge_core::domain::StageExecution> for StageExecutionSummary {
    fn from(e: &astroforge_core::domain::StageExecution) -> Self {
        Self {
            stage_execution_id: e.stage_execution_id.clone(),
            plan_id: e.plan_id.clone(),
            stage_id: e.stage_id.clone(),
            attempt: e.attempt,
            status: e.status.clone(),
            started_at: e.started_at.clone(),
            completed_at: e.completed_at.clone(),
            error_json: e.error_json.clone(),
        }
    }
}

/// CR-05 P2 slice 1 — list stage executions for a plan. Used by the
/// ProcessingControls component to render per-stage state.
#[tauri::command]
pub fn pipeline_plan_list_stage_executions(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
) -> Result<Vec<StageExecutionSummary>, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let execs = store
        .list_stage_executions_for_plan(&plan_id)
        .map_err(store_err_to_string)?;
    Ok(execs.iter().map(StageExecutionSummary::from).collect())
}

fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn lock_err<T>(_: std::sync::PoisonError<T>) -> String {
    "pipeline plan store mutex poisoned".to_string()
}

fn store_err_to_string(e: PipelinePlanStoreError) -> String {
    e.to_string()
}

// Touch the unused status enum so the import isn't dead-code; P2 will
// extend it.
#[allow(dead_code)]
fn _touch_status() -> PipelinePlanStatus {
    PipelinePlanStatus::Draft
}