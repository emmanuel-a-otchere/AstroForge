// src/lib/astroforge-api.ts
//
// CR-02.6 — frontend wrapper for the durable Project / Pipeline-Run
// service layer. **This module is additive only.** Existing widgets
// continue to read from `sessionStore`, `gallery.ts`, etc. Nothing in
// this file wires any Svelte component to the Tauri commands — it just
// provides typed wrappers so future UI migration PRs have a single
// place to import from.
//
// The PR's single rule:
//   *"add the IPC surface; do not replace existing readers."*
//
// Future migration shape:
//   - Today: components import { createInitialSession } from "./pipeline-store"
//   - Tomorrow: components also import { createProject } from "./astroforge-api"
//     and the store reads project state through this layer.
//
// Each method here is a thin typed wrapper around `invoke()`. Errors
// surface to the caller as a thrown Error with the Rust message — the
// UI migrator is responsible for surfacing to the user.

import { invoke } from "@tauri-apps/api/core";

// ─── Project types (mirror Rust serde structs in commands_project.rs) ──

export type ProjectStatus = "Active" | "Archived" | "Exported";

export interface ProjectSummary {
  project_id: string;
  name: string;
  status: ProjectStatus;
  created_at: string;
  updated_at: string;
  active_session_id: string | null;
  application_version: string;
}

export type PipelineRunStatus =
  | "Queued"
  | "Running"
  | "Completed"
  | "Failed"
  | "Cancelled";

export interface PipelineRunSummary {
  run_id: string;
  project_id: string;
  status: PipelineRunStatus;
  recipe_id: string | null;
  started_at: string | null;
  completed_at: string | null;
}

export interface StageRunSummary {
  stage_run_id: string;
  run_id: string;
  stage_id: string;
  status: string;
  attempt: number;
  started_at: string | null;
  completed_at: string | null;
}

export interface RecoverSummary {
  project_id: string;
  status: ProjectStatus;
  repaired_dirs: string[];
}

// ─── Project lifecycle commands ────────────────────────────────────────

export const projectList = (): Promise<ProjectSummary[]> =>
  invoke("project_list");

export const projectGet = (projectId: string): Promise<ProjectSummary> =>
  invoke("project_get", { projectId });

export interface CreateProjectArgs {
  name: string;
  targetId?: string;
  applicationVersion: string;
}

export const projectCreate = (args: CreateProjectArgs): Promise<ProjectSummary> =>
  invoke("project_create", {
    name: args.name,
    targetId: args.targetId ?? null,
    applicationVersion: args.applicationVersion,
  });

export const projectOpen = (slug: string): Promise<ProjectSummary> =>
  invoke("project_open", { slug });

export const projectRename = (slug: string, newName: string): Promise<ProjectSummary> =>
  invoke("project_rename", { slug, newName });

export const projectArchive = (slug: string): Promise<void> =>
  invoke("project_archive", { slug });

export const projectDelete = (slug: string): Promise<void> =>
  // CR-02 §26 hard-delete gate — must be explicit on the wire.
  invoke("project_delete", { slug, confirm: true });

export const projectRecover = (slug: string): Promise<RecoverSummary> =>
  invoke("project_recover", { slug });

// ─── Pipeline-run commands (read-only in this scaffold) ────────────────

export const pipelineRunList = (projectId: string): Promise<PipelineRunSummary[]> =>
  invoke("pipeline_run_list", { projectId });

export const pipelineRunGet = (runId: string): Promise<PipelineRunSummary> =>
  invoke("pipeline_run_get", { runId });

export const pipelineRunListStages = (runId: string): Promise<StageRunSummary[]> =>
  invoke("pipeline_run_list_stages", { runId });

export const pipelineRunFindInterrupted = (): Promise<PipelineRunSummary[]> =>
  invoke("pipeline_run_find_interrupted");

// ─── CR-05 P1 — PipelinePlan commands (additive) ─────────────────────────
//
// PipelinePlan is the user-facing, human-readable processing workflow
// (CR-05 §3, §26). The wizard `mvp_pipeline` path still owns execution
// through P2; P1 lets the UI generate a Plan and render its stages. No
// component is wired to these yet — this is the IPC surface only.

export type PipelinePlanMode = "auto" | "guided" | "expert";

export type PipelinePlanStatus =
  | "draft"
  | "ready"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export interface PipelineStageDto {
  stage_id: string;
  plan_id: string;
  stage_type: string;
  sequence: number;
  label: string;
  required: boolean;
  enabled: boolean;
  parameters_json: string | null;
  produces_image_version: boolean;
  undo_supported: boolean;
}

export interface PipelinePlanDto {
  plan_id: string;
  project_id: string;
  session_id: string;
  recipe_id: string | null;
  mode: PipelinePlanMode;
  target_type: string;
  status: PipelinePlanStatus;
  created_at: string;
  schema_version: number;
  stages: PipelineStageDto[];
}

export interface PipelinePlanSummary {
  plan_id: string;
  project_id: string;
  session_id: string;
  recipe_id: string | null;
  mode: PipelinePlanMode;
  target_type: string;
  status: PipelinePlanStatus;
  created_at: string;
  schema_version: number;
}

export interface SessionUnderstandingPayload {
  target_type: "deep_sky" | "planet" | "lunar" | "solar" | "unknown";
  acquisition: "osc" | "mono" | "narrowband" | "planetary" | "lunar" | "solar";
  light_frame_count: number;
  calibration: "none" | "partial" | "full";
  bayer_pattern: string | null;
  narrowband_filters: string[];
}

export interface CreatePipelinePlanRequest {
  project_id: string;
  session_id: string;
  recipe_id?: string | null;
  mode?: PipelinePlanMode;
  session_understanding?: SessionUnderstandingPayload | null;
}

export const createPipelinePlan = (
  request: CreatePipelinePlanRequest,
): Promise<PipelinePlanDto> =>
  invoke("create_pipeline_plan", { request });

export const pipelinePlanListForProject = (
  projectId: string,
): Promise<PipelinePlanSummary[]> =>
  invoke("pipeline_plan_list_for_project", { projectId });

export const pipelinePlanGet = (planId: string): Promise<PipelinePlanDto> =>
  invoke("pipeline_plan_get", { planId });

// ─── CR-05 P2 slice 1 — execution commands (additive) ─────────────────────
//
// Slice 1 ships start + cancel only. Pause / resume land in P2.5. The
// runner is synchronous: `startPipelinePlan` returns once the run has
// finished (or been cancelled). The frontend reflects per-stage state
// via `pipelinePlanListStageExecutions`.

export type RunOutcome = "completed" | "cancelled" | "failed" | "paused";

export type StageExecutionStatus =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "skipped";

export interface StageExecutionSummary {
  stage_execution_id: string;
  plan_id: string;
  stage_id: string;
  attempt: number;
  status: string;
  started_at: string | null;
  completed_at: string | null;
  error_json: string | null;
}

export const startPipelinePlan = (planId: string): Promise<RunOutcome> =>
  invoke("start_pipeline_run", { planId });

export const cancelPipelinePlan = (planId: string): Promise<boolean> =>
  invoke("cancel_pipeline_run", { planId });

export const pipelinePlanListStageExecutions = (
  planId: string,
): Promise<StageExecutionSummary[]> =>
  invoke("pipeline_plan_list_stage_executions", { planId });

// ─── CR-05 P2.5 — pause + resume + recovery commands (additive) ───────────
//
// `pausePipelinePlan` flips the runner's pause flag (no-op if no run
// is in flight). `resumePipelinePlan` re-loads the plan, refuses if
// not in Paused, and continues from the first stage with no
// completed StageExecution row. `pipelinePlanListResumableForProject`
// feeds RecoveryBanner.svelte on project open.

export const pausePipelinePlan = (planId: string): Promise<boolean> =>
  invoke("pause_pipeline_run", { planId });

export const resumePipelinePlan = (planId: string): Promise<RunOutcome> =>
  invoke("resume_pipeline_run", { planId });

export const pipelinePlanListResumableForProject = (
  projectId: string,
): Promise<PipelinePlanSummary[]> =>
  invoke("pipeline_plan_list_resumable_for_project", { projectId });

// ─── CR-05 P3 slice 2 — recommendation engine API (additive) ────────────
//
// Mirrors the slice 2 / 2.5 Rust commands. `IntelligencePanel.svelte`
// reads `pipelinePlanListRecommendations(planId)` on mount and on
// every `refreshStageExecutions` tick. The mutation endpoints return
// a `RecommendationUpdateResult` that the panel uses to update the
// row in place.

export type RecommendationUserDecision = "pending" | "applied" | "dismissed";

export interface RecommendationDto {
  id: string;
  stage_execution_id: string;
  rule_id: string;
  stage_type: string;
  decision_json: ProcessingDecisionJson;
  confidence: number;
  evidence_summary: string;
  created_at: string;
  // CR-05 P3 slice 2.5 — user-decision lifecycle. Null is treated
  // as "pending" by the UI for backward compatibility.
  user_decision: RecommendationUserDecision | null;
  user_decision_at: string | null;
  applied_stage_id: string | null;
  /** CR-05 P4 slice 6 — id of the `preview_run` that satisfied
   * the §11 gate when this recommendation was applied. Null
   * for not-yet-applied or legacy rows. */
  applied_with_preview_id: string | null;
}

export interface ProcessingDecisionJson {
  stage_type: string;
  parameters: Record<string, unknown>;
  rationale: string;
}

export interface RecommendationUpdateResult {
  recommendation: RecommendationDto;
  applied_stage_id: string | null;
  /** CR-05 P4 slice 6 — preview id provenance for the apply. */
  applied_with_preview_id?: string | null;
}

export const pipelinePlanListRecommendations = (
  planId: string,
): Promise<RecommendationDto[]> =>
  invoke("get_recommendations_for_plan", { planId });

export const pipelinePlanListRecommendationsForStageExecution = (
  stageExecutionId: string,
): Promise<RecommendationDto[]> =>
  invoke("get_recommendations_for_stage_execution", {
    stageExecutionId,
  });

// CR-05 P3 slice 2.5 — user-decision mutation commands. All three
// return the updated `RecommendationUpdateResult` so the panel can
// update the row in place.

export const applyRecommendation = (
  recommendationId: string,
): Promise<RecommendationUpdateResult> =>
  invoke("apply_recommendation", { recommendationId });

export const dismissRecommendation = (
  recommendationId: string,
): Promise<RecommendationUpdateResult> =>
  invoke("dismiss_recommendation", { recommendationId });

export const resetRecommendation = (
  recommendationId: string,
): Promise<RecommendationUpdateResult> =>
  invoke("reset_recommendation", { recommendationId });

// ─── CR-05 P4 slice 4 — preview-before-commit API (additive) ─────────────
//
// Mirrors the slice 4 Rust commands. Slice 4 ships lifecycle
// wiring only — the actual stage-handler integration (running a
// handler at scale and writing a real preview image) lands in
// slice 5. For now `createPreviewRun` immediately returns a
// PreviewRun with status `completed` + a synthesized placeholder
// artifact so the UI can exercise the read-back path.

export interface PreviewRunDto {
  preview_id: string;
  stage_execution_id: string;
  source_version_id: string;
  preview_artifact_id: string | null;
  parameters_json: string;
  parameters_hash: string;
  status: "pending" | "running" | "completed" | "failed";
  scale: number;
  label: string;
  error_json: string | null;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
}

export interface CreatePreviewRunRequest {
  stage_execution_id: string;
  source_version_id: string;
  parameters_json: string;
  scale?: number | null;
  label?: string | null;
}

export const createPreviewRun = (
  request: CreatePreviewRunRequest,
): Promise<PreviewRunDto> =>
  invoke("create_preview_run", { request });

export const listPreviewRunsForStageExecution = (
  stageExecutionId: string,
): Promise<PreviewRunDto[]> =>
  invoke("list_preview_runs_for_stage_execution", { stageExecutionId });

export const getPreviewRun = (
  previewId: string,
): Promise<PreviewRunDto> => invoke("get_preview_run", { previewId });

/**
 * CR-05 P4 slice 5 — read a preview's PNG artifact back as base64.
 * Only files inside the app's previews directory are served; the
 * asset protocol stays disabled.
 */
export const readPreviewArtifact = (previewId: string): Promise<string> =>
  invoke("read_preview_artifact", { previewId });

export const deletePreviewRun = (
  previewId: string,
): Promise<void> => invoke("delete_preview_run", { previewId });
// ─── CR-05 P4 slice 7 — §21 Resource-Aware Execution ─────────────────────
// Mirrors `crates/astroforge-core/src/resource.rs` (serde snake_case).

export type ExecutionBackend =
  | "cpu"
  | "cuda"
  | "direct_ml"
  | "core_ml"
  | "open_vino";

export type Precision = "f16" | "f32";

export interface GpuInfo {
  name: string;
  backend: ExecutionBackend;
  vram_bytes: number | null;
}

export interface RecommendedExecution {
  backend: ExecutionBackend;
  tile_size: number;
  thread_count: number;
  precision: Precision;
  memory_budget_bytes: number;
}

export interface ResourceSnapshot {
  cpu_model: string;
  logical_cores: number;
  physical_cores: number | null;
  total_memory_bytes: number;
  available_memory_bytes: number;
  gpus: GpuInfo[];
  recommended: RecommendedExecution;
}

/**
 * Detect the device and return the snapshot + derived execution
 * recommendation. Recomputed on demand so it reflects current memory
 * pressure; infallible by design (probes degrade gracefully).
 */
export const getResourceSnapshot = (): Promise<ResourceSnapshot> =>
  invoke("get_resource_snapshot");

// ─── CR-05 P5 slice 1 — backend enumeration + §22 execution budget ───────
// Mirrors `crates/astroforge-core/src/resource.rs` (serde snake_case).

export interface BackendCapability {
  backend: ExecutionBackend;
  available: boolean;
  device: string | null;
  /** Why the backend is unavailable — render verbatim in Expert mode. */
  unavailable_reason: string | null;
}

export interface ExecutionBudget {
  memory_budget_bytes: number;
  requires_tiling: boolean;
  tile_size: number;
  thread_count: number;
  /** §22 pre-flight warning copy when memory is tight; null when fine. */
  warning: string | null;
}

/** Advertised capability of every §21 backend on this device. */
export const listBackendCapabilities = (): Promise<BackendCapability[]> =>
  invoke("list_backend_capabilities");

/** Pre-flight budget for a stage input of `datasetSizeBytes` uncompressed. */
export const deriveExecutionBudget = (
  datasetSizeBytes: number,
): Promise<ExecutionBudget> =>
  invoke("derive_execution_budget", { datasetSizeBytes });

/** CR-05 P5 slice 2 (§22) — pre-flight budget from a stage's
 *  parameters_json. Returns the same shape as `deriveExecutionBudget`
 *  but reads the dataset_size_bytes from the JSON rather than as a
 *  separate argument, mirroring how the runner resolves it. */
export const stageExecutionBudget = (
  parametersJson: string | null,
): Promise<ExecutionBudget> =>
  invoke("stage_execution_budget", { parametersJson });

// ─── CR-05 P5 slice 4 (§24 + §27) — aggregate metrics ────────────────

/** CR-05 §14 metric list, subset exposed to the Expert DAG view. */
export interface ImageMetricsDto {
  snr: number;
  fwhm: number;
  star_count: number;
  background_gradient: number;
  mean: number;
  stddev: number;
}

/** CR-05 §12 noise banding — see `adaptive::NoiseKind` on the core. */
export type NoiseKindDto = "low" | "moderate" | "high" | "extreme";

export interface NoiseProfileDto {
  kind: NoiseKindDto;
  recommended_strength: number;
  label: string;
  reason: string;
}

export interface SharpeningProfileDto {
  strength: number;
  reason: string;
}

/** §12 adaptive parameter set. Every field is optional so the UI can
 *  gracefully degrade when only partial data is available (e.g. a
 *  stage that hasn't emitted metrics yet). */
export interface AdaptiveParameterSetDto {
  noise?: NoiseProfileDto;
  sharpening?: SharpeningProfileDto;
}

/** §22 execution budget from the runner's persisted `resource_usage_json`. */
export interface ExecutionBudgetDto {
  memory_budget_bytes: number;
  requires_tiling: boolean;
  tile_size: number;
  thread_count: number;
  warning: string | null;
}

/** CR-05 §27 `get_processing_metrics` payload. Aggregates stage
 *  executions + the latest metrics + the adaptive engine's current
 *  parameter set, so the Expert DAG view can render in a single IPC. */
export interface ProcessingMetricsDto {
  plan_id: string;
  stage_count: number;
  completed_count: number;
  completion_ratio: number;
  latest_stage_id: string | null;
  latest_stage_type: string | null;
  latest_metrics: ImageMetricsDto | null;
  latest_resource_budget: ExecutionBudgetDto | null;
  adaptive_parameters: AdaptiveParameterSetDto | null;
  /** CR-05 P6.2 (§23) — AI label for the most-recent execution.
   *  null when `ai_label_json` is missing (pre-P6.2 rows). */
  latest_ai_label: AiBoundaryLabelDto | null;
  /** CR-05 P6.1 (§28) — populated when the most-recent execution
   *  is failed; the UI renders `ErrorRecoveryPanel` from this. */
  latest_stage_error: StageErrorDto | null;
}

/** §27 — single-roundtrip aggregate metrics for the Expert DAG view. */
export const getProcessingMetrics = (
  planId: string,
): Promise<ProcessingMetricsDto> =>
  invoke("get_processing_metrics", { planId });

// ─── CR-05 P6 slice 2 (§23 AI Boundary) ─────────────────────────────

/** §23 — AI boundary label per stage. Mirrors
 *  astroforge_core::ai_boundary::AiBoundaryLabel. */
export interface AiBoundaryLabelDto {
  /** Whether the stage crosses the AI boundary. */
  uses_ai: boolean;
  /** Model identifier; null for classical stages. */
  model_id: string | null;
  /** Whether the stage is bit-reproducible. */
  deterministic: boolean;
  /** RNG seed; null when uses_ai is false or the stage has no seed. */
  seed: number | null;
}

// ─── CR-05 P6 slice 1 (§28 Error and Recovery UX) ───────────────────

/** §28 — SuggestedActionKind enum. Variants match the Rust
 *  `SuggestedActionKind` and the snake_case renames in serde. */
export type SuggestedActionKind =
  | "retry_optimized"
  | "retry"
  | "adjust_processing"
  | "skip_stage"
  | "cancel"
  | "contact_support";

/** §28 — Single suggested next action, surfaced as a button. */
export interface SuggestedActionDto {
  label: string;
  kind: SuggestedActionKind;
}

/** §28 — Structured stage error payload. Round-trips through
 *  StageExecution.error_json. Mirrors astroforge_core::stage_error::StageError. */
export interface StageErrorDto {
  what_happened: string;
  what_was_preserved: string;
  suggested_actions: SuggestedActionDto[];
}


// ─── CR-05 P6 slice 3 (§25 Processing Timeline) ────────────────────

/** §25 — single event on the processing timeline. Mirrors
 *  astroforge_core::processing_timeline::TimelineEvent. */
export interface TimelineEventDto {
  stage_id: string;
  /** Human-readable label, e.g. "Stack", "AI denoise". */
  stage_label: string;
  /** Stage type token, e.g. "stack", "denoise". */
  stage_type: string;
  sequence: number;
  /** Unix milliseconds parsed from `started_at`. null for legacy
   *  rows where the timestamp couldn't be parsed. */
  timestamp_unix_ms: number | null;
  /** `completed_at - started_at` in ms, when both parse cleanly. */
  duration_ms: number | null;
  /** `ImageVersion.version_id` this stage produced. */
  output_version_id: string | null;
  status: string;
}

/** §25 — processing timeline for a plan. */
export interface ProcessingTimelineDto {
  plan_id: string;
  events: TimelineEventDto[];
}

/** §25 — single-roundtrip timeline fetch for the timeline panel. */
export const getProcessingTimeline = (
  planId: string,
): Promise<ProcessingTimelineDto> =>
  invoke("get_processing_timeline", { planId });
