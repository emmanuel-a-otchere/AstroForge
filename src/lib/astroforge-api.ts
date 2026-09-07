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
}

export interface ProcessingDecisionJson {
  stage_type: string;
  parameters: Record<string, unknown>;
  rationale: string;
}

export interface RecommendationUpdateResult {
  recommendation: RecommendationDto;
  applied_stage_id: string | null;
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