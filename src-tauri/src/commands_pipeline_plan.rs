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
    dispatch::{
        BackgroundHandler, CalibrateHandler, DebayerHandler, DenoiseHandler, ExportHandler,
        HandlerRegistry, RegisterHandler, StackHandler, StretchHandler,
    },
    plan::{generate_plan as generate_plan_inner, GenerationContext, SessionUnderstanding},
    runner::{CancelHandle, PauseHandle, PipelineRunner, RunOutcome},
    AcquisitionMode, CalibrationAvailability,
};
use astroforge_core::adaptive::AdaptiveParameterSet;
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
    /// CR-05 P3 slice 2 — recommendation engine. When present,
    /// every successful stage dispatch that produced a metric
    /// snapshot triggers `engine.evaluate()` and persists the
    /// resulting rows. None means the engine is dormant (tests).
    pub recommendation_engine: Option<Arc<astroforge_core::recommendation::RecommendationEngine>>,
    /// CR-05 P4 slice 5 — directory where preview PNGs are written
    /// (`<projects_root>/previews`). `read_preview_artifact` refuses
    /// to serve any path outside this directory.
    pub previews_dir: std::path::PathBuf,
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
        PipelineRunner::with_engine(
            store,
            state.handler_registry.clone(),
            state.domain_store.clone(),
            state.recommendation_engine.clone(),
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
        PipelineRunner::with_engine(
            store,
            state.handler_registry.clone(),
            state.domain_store.clone(),
            state.recommendation_engine.clone(),
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

// ─── CR-05 P3 slice 2 — recommendation Tauri commands ───────────────

/// CR-05 P3 slice 2 — DTO mirroring `Recommendation` for IPC. The
/// JSON shape is stable so the Svelte side can serialise /
/// deserialise without translation.
#[derive(Debug, Serialize)]
pub struct RecommendationDto {
    pub id: String,
    pub stage_execution_id: String,
    pub rule_id: String,
    pub stage_type: String,
    pub decision_json: serde_json::Value,
    pub confidence: f64,
    pub evidence_summary: String,
    pub created_at: String,
    /// CR-05 P3 slice 2.5 — user decision lifecycle. One of
    /// "pending" (engine-only), "applied" (parameters merged into
    /// the next stage's `parameters_json`), or "dismissed" (user
    /// rejected). `None` rows round-trip as `null` on the
    /// frontend; the UI treats that as "pending".
    pub user_decision: Option<String>,
    /// CR-05 P3 slice 2.5 — `unix_ms:<ms>` timestamp of the user
    /// decision, if any.
    pub user_decision_at: Option<String>,
    /// CR-05 P3 slice 2.5 — when applied, the `pipeline_stages.stage_id`
    /// whose `parameters_json` was rewritten by the apply.
    pub applied_stage_id: Option<String>,
    /// CR-05 P4 slice 6 — id of the `preview_run` that satisfied
    /// the §11 preview-before-commit gate when this recommendation
    /// was applied. `None` for not-yet-applied or legacy rows.
    pub applied_with_preview_id: Option<String>,
}

impl From<&astroforge_core::recommendation::Recommendation> for RecommendationDto {
    fn from(r: &astroforge_core::recommendation::Recommendation) -> Self {
        // Re-serialise the decision so the frontend gets
        // `{stage_type, parameters, rationale}` in the standard
        // shape rather than the `parameters` flattened.
        let decision_json = serde_json::to_value(&r.decision)
            .unwrap_or_else(|_| serde_json::json!({"error": "serialise"}));
        Self {
            id: r.id.clone(),
            stage_execution_id: r.stage_execution_id.clone(),
            rule_id: r.rule_id.clone(),
            stage_type: r.decision.stage_type.clone(),
            decision_json,
            confidence: r.confidence,
            evidence_summary: r.evidence_summary.clone(),
            created_at: r.created_at.clone(),
            user_decision: r.user_decision.clone(),
            user_decision_at: r.user_decision_at.clone(),
            applied_stage_id: r.applied_stage_id.clone(),
            applied_with_preview_id: r.applied_with_preview_id.clone(),
        }
    }
}

/// CR-05 P3 slice 2 — list every recommendation emitted for a
/// given stage execution. Returns an empty list when no
/// recommendation has been generated (no metric snapshot yet, or
/// snapshot was uniform / failed).
#[tauri::command]
pub fn get_recommendations_for_stage_execution(
    state: State<'_, PipelinePlanState>,
    stage_execution_id: String,
) -> Result<Vec<RecommendationDto>, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let recs = store
        .list_recommendations_for_stage_execution(&stage_execution_id)
        .map_err(store_err_to_string)?;
    Ok(recs.iter().map(RecommendationDto::from).collect())
}

/// CR-05 P3 slice 2 — list every recommendation emitted for every
/// stage execution in a plan. Used by the IntelligencePanel to
/// show the full set of current picks without the UI needing to
/// iterate over stage executions.
#[tauri::command]
pub fn get_recommendations_for_plan(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
) -> Result<Vec<RecommendationDto>, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let recs = store
        .list_recommendations_for_plan(&plan_id)
        .map_err(store_err_to_string)?;
    Ok(recs.iter().map(RecommendationDto::from).collect())
}

// ─── CR-05 P3 slice 2.5 — user-decision Tauri commands ───────────────────
//
// `apply_recommendation` merges the recommended `parameters` into
// the next pipeline stage's `parameters_json` and marks the
// recommendation as `applied`. `dismiss_recommendation` flips
// the recommendation to `dismissed` without touching stage
// parameters. `reset_recommendation` returns the row to
// `pending` (lets the user re-apply).
//
// All three return the updated `RecommendationDto` so the
// frontend can refresh the panel in one round-trip without a
// second `get_recommendations_for_plan` call.

/// CR-05 P3 slice 2.5 — response shape returned by apply /
/// dismiss / reset. Same as the list endpoint, but a single
/// object so the UI can update the row in place.
#[derive(Debug, Serialize)]
pub struct RecommendationUpdateResult {
    pub recommendation: RecommendationDto,
    pub applied_stage_id: Option<String>,
}

/// CR-05 P3 slice 2.5 — generate an `unix_ms:<ms>` timestamp in
/// the same shape the rest of the pipeline uses.
fn now_unix_ms_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{}", ms)
}

/// CR-05 P4 slice 6 — inverse of `now_unix_ms_string`. Extracts
/// the integer millisecond value from a `unix_ms:<n>` string.
/// Returns `None` for any other shape (defensive — we don't want
/// the gate to silently bypass on bad input).
fn parse_unix_ms(s: &str) -> Option<i64> {
    s.strip_prefix("unix_ms:").and_then(|tail| tail.parse().ok())
}

#[tauri::command]
pub fn apply_recommendation(
    state: State<'_, PipelinePlanState>,
    recommendation_id: String,
) -> Result<RecommendationUpdateResult, String> {
    let timestamp = now_unix_ms_string();
    // CR-05 P4 slice 6 — preview-before-commit gate (§11).
    //
    // Resolve the recommendation's stage_execution_id first so we
    // can look up a qualifying preview in the domain store. The
    // preview must:
    //   - belong to the same stage_execution_id
    //   - have status = completed
    //   - have completed_at > recommendation.created_at
    // (the last condition ensures the user previewed after the
    // recommendation was emitted — i.e. they actually saw what
    // the recommendation is changing).
    //
    // If no qualifying preview exists, return a typed error so
    // the UI can surface "preview required" instead of allowing
    // a blind apply.
    let stage_execution_id = {
        let store = state.store.lock().map_err(lock_err)?;
        store
            .stage_execution_id_for_recommendation(&recommendation_id)
            .map_err(store_err_to_string)?
    };

    let preview = {
        let domain_store = state
            .domain_store
            .clone()
            .ok_or_else(|| "domain store is unavailable".to_string())?;
        let store = state.store.lock().map_err(lock_err)?;
        let rec = store
            .get_recommendation_by_id(&recommendation_id)
            .map_err(store_err_to_string)?;
        drop(store);
        // The recommendation.created_at is `unix_ms:<ms>`; extract
        // the integer part for the cutoff comparison.
        let cutoff_ms = parse_unix_ms(&rec.created_at)
            .ok_or_else(|| format!("recommendation has malformed created_at: {}", rec.created_at))?;
        domain_store
            .latest_completed_preview_for_stage_execution(&stage_execution_id, cutoff_ms)
            .map_err(|e| format!("preview gate lookup failed: {e}"))?
    };
    let preview_id = match preview {
        Some(p) => p.preview_id,
        None => {
            return Err(format!(
                "preview required: no completed preview found for stage_execution '{stage_execution_id}' after the recommendation was emitted"
            ));
        }
    };

    // Two passes: first mutate (commit), then re-read inside the
    // same store so the returned row reflects the post-update
    // state. The lock is released between passes so the
    // re-read path stays simple.
    let applied_stage_id = {
        let store = state.store.lock().map_err(lock_err)?;
        let (_id, applied_stage_id) = store
            .apply_recommendation(&recommendation_id, &timestamp, &preview_id)
            .map_err(store_err_to_string)?;
        applied_stage_id
    };
    let store = state.store.lock().map_err(lock_err)?;
    let recs = store
        .list_recommendations_for_stage_execution(&stage_execution_id)
        .map_err(store_err_to_string)?;
    let rec = recs
        .into_iter()
        .find(|r| r.id == recommendation_id)
        .ok_or_else(|| format!("recommendation vanished after apply: {recommendation_id}"))?;
    Ok(RecommendationUpdateResult {
        recommendation: RecommendationDto::from(&rec),
        applied_stage_id,
    })
}

/// CR-05 P3 slice 2.5 — resolve a recommendation id back to its
/// `stage_execution_id` so the row can be reloaded after the
/// apply / dismiss / reset pass. Done via a dedicated store
/// method that takes a `&Connection` (it runs inside an
/// already-locked connection).
fn stage_execution_id_for_recommendation(
    store: &astroforge_core::pipeline_plans_store::PipelinePlanStore,
    recommendation_id: &str,
) -> Result<String, astroforge_core::pipeline_plans_store::PipelinePlanStoreError> {
    store.stage_execution_id_for_recommendation(recommendation_id)
}

#[tauri::command]
pub fn dismiss_recommendation(
    state: State<'_, PipelinePlanState>,
    recommendation_id: String,
) -> Result<RecommendationUpdateResult, String> {
    let timestamp = now_unix_ms_string();
    {
        let store = state.store.lock().map_err(lock_err)?;
        store
            .dismiss_recommendation(&recommendation_id, &timestamp)
            .map_err(store_err_to_string)?;
    }
    let store = state.store.lock().map_err(lock_err)?;
    let stage_execution_id = stage_execution_id_for_recommendation(&store, &recommendation_id)
        .map_err(store_err_to_string)?;
    let recs = store
        .list_recommendations_for_stage_execution(&stage_execution_id)
        .map_err(store_err_to_string)?;
    let rec = recs
        .into_iter()
        .find(|r| r.id == recommendation_id)
        .ok_or_else(|| format!("recommendation vanished after dismiss: {recommendation_id}"))?;
    Ok(RecommendationUpdateResult {
        recommendation: RecommendationDto::from(&rec),
        applied_stage_id: rec.applied_stage_id.clone(),
        // The DTO `From<&Recommendation>` constructor populates
        // `applied_with_preview_id`, so a manual `rec.clone()` or
        // `RecommendationDto::from(&rec)` would be safer here. The
        // direct field-by-field copy below predates slice 6; the
        // missing field is filled in to keep the DTO consistent
        // with `RecommendationDto::from`.
        applied_with_preview_id: rec.applied_with_preview_id.clone(),
    })
}

#[tauri::command]
pub fn reset_recommendation(
    state: State<'_, PipelinePlanState>,
    recommendation_id: String,
) -> Result<RecommendationUpdateResult, String> {
    {
        let store = state.store.lock().map_err(lock_err)?;
        store
            .reset_recommendation(&recommendation_id)
            .map_err(store_err_to_string)?;
    }
    let store = state.store.lock().map_err(lock_err)?;
    let stage_execution_id = stage_execution_id_for_recommendation(&store, &recommendation_id)
        .map_err(store_err_to_string)?;
    let recs = store
        .list_recommendations_for_stage_execution(&stage_execution_id)
        .map_err(store_err_to_string)?;
    let rec = recs
        .into_iter()
        .find(|r| r.id == recommendation_id)
        .ok_or_else(|| format!("recommendation vanished after reset: {recommendation_id}"))?;
    Ok(RecommendationUpdateResult {
        recommendation: RecommendationDto::from(&rec),
        applied_stage_id: None,
    })
}

// ─── CR-05 P5 slice 4 — §24 Pipeline Visualization + §27 metrics ─────────

/// CR-05 P5 slice 4 (§27) — UI payload for `get_processing_metrics`.
/// Mirror of `astroforge_core::processing_metrics::ProcessingMetrics`;
/// lives in the Tauri layer so the IPC contract is stable when the
/// core type evolves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMetricsDto {
    pub plan_id: String,
    pub stage_count: u32,
    pub completed_count: u32,
    /// `0.0..=1.0`. `0.0` when no stages have completed yet.
    pub completion_ratio: f64,
    pub latest_stage_id: Option<String>,
    pub latest_stage_type: Option<String>,
    pub latest_metrics: Option<astroforge_core::adaptive::ImageMetrics>,
    /// Persisted `resource_usage_json` from the most recent completed
    /// stage, deserialised into `ExecutionBudget`. `None` when no
    /// stage has completed or the runner hasn't persisted a budget yet.
    pub latest_resource_budget: Option<astroforge_core::resource::ExecutionBudget>,
    /// §12 adaptive output derived from `latest_metrics`. `None` when
    /// no metrics are available yet (the §12 promise of "never refuse
    /// to make progress" means the engine always returns *something*,
    /// but we surface the absence to the UI rather than fabricate).
    pub adaptive_parameters: Option<AdaptiveParameterSet>,
    /// CR-05 P6.2 (§23 AI Boundary) — AI label from the most-recent
    /// stage execution. `None` when `ai_label_json` is missing
    /// (pre-P6.2 row) or unparseable.
    pub latest_ai_label: Option<astroforge_core::ai_boundary::AiBoundaryLabel>,
    /// CR-05 P6.1 (§28) — structured error from the most-recent
    /// failed stage, parsed from `error_json`. `None` when no stage
    /// has failed or the persisted shape is pre-P6.1.
    pub latest_stage_error: Option<astroforge_core::stage_error::StageError>,
}

impl From<astroforge_core::processing_metrics::ProcessingMetrics> for ProcessingMetricsDto {
    fn from(m: astroforge_core::processing_metrics::ProcessingMetrics) -> Self {
        Self {
            plan_id: m.plan_id,
            stage_count: m.stage_count,
            completed_count: m.completed_count,
            completion_ratio: m.completion_ratio,
            latest_stage_id: m.latest_stage_id,
            latest_stage_type: m.latest_stage_type,
            latest_metrics: m.latest_metrics,
            latest_resource_budget: m.latest_resource_budget,
            adaptive_parameters: m.adaptive_parameters,
            latest_ai_label: m.latest_ai_label,
            latest_stage_error: m.latest_stage_error,
        }
    }
}

/// CR-05 P5 slice 4 (§24 + §27) — `get_processing_metrics`.
///
/// Aggregates persisted stage executions into a single UI-friendly
/// payload so `ExpertDagView` doesn't need to make 5+ round trips.
/// Pure function of the store — no IO, no system calls.
#[tauri::command]
pub fn get_processing_metrics(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
) -> Result<ProcessingMetricsDto, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let plan = store
        .get_pipeline_plan(&plan_id)
        .map_err(store_err_to_string)?;
    let execs = store
        .list_stage_executions_for_plan(&plan_id)
        .map_err(store_err_to_string)?;
    // §27 aggregate lives in astroforge-core so it can be unit-tested
    // as part of the workspace `cargo test --workspace` run that CI
    // executes (src-tauri is a binary-only crate outside the workspace).
    let payload = astroforge_core::processing_metrics::aggregate_processing_metrics(&plan, &execs);
    Ok(payload.into())
}