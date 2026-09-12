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

// ─── CR-05 R2 — Project overview IPC ───────────────────────────────────────
//
// Truthful checklist state for the §8 Overview. The five booleans
// (imported / analyzed / processed / versioned / exported) are
// derived from the durable event log and project tables; the
// frontend no longer hardcodes them as `pending`.

export interface ProjectOverview {
  project_id: string;
  imported: boolean;
  analyzed: boolean;
  processed: boolean;
  versioned: boolean;
  exported: boolean;
}

export const projectOverview = (projectId: string): Promise<ProjectOverview> =>
  invoke("project_overview", { projectId });

// ─── CR-05 R3 — Image version listing IPC ──────────────────────────────────
//
// Real version timeline for the Compare workspace. Derived from
// the durable event log (`VersionCreated` events) plus any
// payload fields. The previous `versionStore.load` path used a
// hard-coded placeholder timeline; this slice replaces that with
// a Tauri command that returns the real state.

export interface ImageVersion {
  version_id: string;
  project_id: string;
  label: string;
  sequence: number;
  primary_artifact_id: string;
  source_version_id: string | null;
  created_at: string;
  hidden: boolean;
}

export interface ImageVersionListResponse {
  project_id: string;
  versions: ImageVersion[];
}

export const imageVersionList = (
  projectId: string,
): Promise<ImageVersionListResponse> =>
  invoke("image_version_list", { projectId });

// ─── CR-06 P1 — AI Enhancement Studio IPC shells ──────────────────────────
//
// P1 only persists the data model + provenance fields. The
// substantive behavior (analysis, recommendations, enhancement
// operations, masks, stacks, previews) lands in P2–P6; these
// wrappers expose the IPC surface so the TS types and the
// project-lifecycle reset hook can land alongside the schema
// migration without further changes.
//
// The wire shapes are JSON values (`unknown` in TS) so the
// frontend doesn't take a hard dependency on the Rust serde
// shapes until P2 defines the analysis / recommendation
// payload contracts. Consumers in P3+ will narrow these to
// concrete types once the contracts land.

export interface AnalyzeImageRequest {
  imageVersionId: string;
  width: number;
  height: number;
  channels: number;
  /// Row-major pixel data in [0, 1] float values. The Rust
  /// side reconstructs an `F32Image` from the dimensions +
  /// data and runs the analyzer on it. P4 wires the
  /// real read-artifact-and-analyze flow; P2 takes the
  /// pixels over IPC so the analyzer is testable
  /// end-to-end without a Tauri runtime.
  pixels: number[];
}

export interface AiOperationJson {
  operation_id: string;
  stage_run_id: string;
  model_id: string;
  model_version: string;
  model_hash?: string | null;
  runtime?: string | null;
  backend?: string | null;
  precision?: string | null;
  parameters_json?: string | null;
  seed?: number | null;
  deterministic: boolean;
  safety_classification:
    | "deterministic"
    | "perceptual"
    | "generative";
  experimental: boolean;
  input_artifact_id?: string | null;
  output_artifact_id?: string | null;
  engine_version?: string | null;
  tile_configuration?: string | null;
  resource_metrics?: string | null;
}

export interface AiOperationRecord extends AiOperationJson {
  /// CR-06 P1 — round-trip the safety-classification
  /// string in the same form as the IPC wire shape so
  /// consumers can branch on it directly.
}

export const analyzeImage = (
  request: AnalyzeImageRequest,
): Promise<unknown> => invoke("analyze_image", { request });

export interface GenerateRecommendationsRequest {
  project_id: string;
  image_version_id: string;
}

export interface GenerateRecommendationsResponse {
  report_json: string;
  recommendations: { items: unknown[] };
}

export const generateAiRecommendations = (
  request: GenerateRecommendationsRequest,
): Promise<GenerateRecommendationsResponse> =>
  invoke("generate_ai_recommendations", { request }) as Promise<GenerateRecommendationsResponse>;

// CR-06 P4 — enhancement stack + apply round + image
// version tracking. The Tauri commands land alongside
// `analyze_image` and `generate_ai_recommendations` in
// `commands_ai_enhancement`.

export interface StackOperationJson {
  operation_id: string;
  recommendation_id: string | null;
  parameters_json: string;
  enabled: boolean;
  needs_preview?: boolean;
}

export interface EnhancementStackJson {
  stack_id: string;
  project_id: string;
  source_image_version_id: string;
  operations: StackOperationJson[];
  branched_from_version_id: string | null;
  created_at: string;
}

export interface CreateEnhancementStackRequest {
  project_id: string;
  source_image_version_id: string;
  operations: StackOperationJson[];
}

export const enhancementStackCreate = (
  request: CreateEnhancementStackRequest,
): Promise<{ stack: EnhancementStackJson }> =>
  invoke("enhancement_stack_create", { request }) as Promise<{
    stack: EnhancementStackJson;
  }>;

export const enhancementStackGet = (
  stackId: string,
): Promise<EnhancementStackJson | null> =>
  invoke("enhancement_stack_get", { stackId }) as Promise<
    EnhancementStackJson | null
  >;

export type StackMutation =
  | { Reorder: { operation_id: string; new_index: number } }
  | { SetEnabled: { operation_id: string; enabled: boolean } }
  | { Remove: { operation_id: string } }
  | { MarkNeedsPreview: { operation_id: string } }
  | { Append: StackOperationJson };

export const enhancementStackApplyMutation = (
  stackId: string,
  mutation: StackMutation,
): Promise<EnhancementStackJson> =>
  invoke("enhancement_stack_apply_mutation", {
    stackId,
    mutation,
  }) as Promise<EnhancementStackJson>;

export interface BranchEnhancementStackRequest {
  source_stack_id: string;
  cutoff: number;
  new_image_version_id: string;
}

export const enhancementStackBranch = (
  request: BranchEnhancementStackRequest,
): Promise<EnhancementStackJson> =>
  invoke("enhancement_stack_branch", { request }) as Promise<
    EnhancementStackJson
  >;

export type SafetyClassification =
  | "deterministic"
  | "perceptual"
  | "generative";

export type OperationCategory =
  | "cleanup"
  | "denoise"
  | "restoration"
  | "star"
  | "detail"
  | "upscale"
  | "inpaint"
  | "background";

export interface OperationRegistryEntryJson {
  operation_id: string;
  display_name: string;
  category: OperationCategory;
  safety_classification: SafetyClassification;
  description: string;
  default_parameters_json: string;
  requires_region: boolean;
}

export const enhancementOperationsList = (): Promise<
  OperationRegistryEntryJson[]
> =>
  invoke("enhancement_operations_list") as Promise<
    OperationRegistryEntryJson[]
  >;

export const operationsRegistryList = (): Promise<
  OperationRegistryEntryJson[]
> =>
  invoke("operations_registry_list") as Promise<
    OperationRegistryEntryJson[]
  >;

export interface ApplyAiOperationRequest {
  project_id: string;
  source_image_version_id: string;
  operation_id: string;
  parameters_json: string;
  preview_id?: string | null;
}

export interface ApplyAiOperationResponse {
  image_version: ImageVersionJson;
  outcome: { operation_id: string; result_image_version_id: string };
  operation_row: { operation_id: string };
}

export const enhancementApplyOperation = (
  request: ApplyAiOperationRequest,
): Promise<ApplyAiOperationResponse> =>
  invoke("enhancement_apply_operation", { request }) as Promise<
    ApplyAiOperationResponse
  >;

export interface ImageVersionJson {
  version_id: string;
  project_id: string;
  label: string;
  sequence: number;
  primary_artifact_id: string;
  source_version_id: string | null;
  created_at: string;
  hidden: boolean;
}

export const imageVersionListForProject = (
  projectId: string,
): Promise<{ items: ImageVersionJson[] }> =>
  invoke("image_version_list_for_project", {
    projectId,
  }) as Promise<{ items: ImageVersionJson[] }>;

export const imageVersionGet = (
  versionId: string,
): Promise<ImageVersionJson | null> =>
  invoke("image_version_get", { versionId }) as Promise<
    ImageVersionJson | null
  >;

/**
 * CR-07 — read an applied Image Version's primary artifact as
 * base64-encoded 16-bit TIFF bytes plus dimensions. The Zone B
 * canvas decodes the bytes in a Web Worker so the main thread
 * stays responsive. Path-confined to the project's applied
 * directory; the backend refuses artifacts outside that dir.
 */
export interface ImageArtifactResponse {
  base64_data: string;
  mime_type: string;
  byte_size: number;
  width: number;
  height: number;
  channels: number;
}

export const readImageArtifact = (
  versionId: string,
): Promise<ImageArtifactResponse> =>
  invoke("read_image_artifact", { versionId }) as Promise<
    ImageArtifactResponse
  >;

// CR-06 P5 — region-aware mask system. The mask
// engine in `astroforge-core::masks` produces JSON
// payloads via `astroforge_core::masks::encoding`;
// the Tauri commands persist the rows.

export interface AiMaskJson {
  mask_id: string;
  project_id: string;
  image_version_id: string;
  provenance: string;
  parents_json: string | null;
  mask_json: string;
  created_at: string;
}

export interface CreateAiMaskRequest {
  project_id: string;
  image_version_id: string;
  provenance: string;
  parents_json?: string | null;
  mask_json: string;
}

export const createAiMask = (
  request: CreateAiMaskRequest,
): Promise<{ mask: AiMaskJson }> =>
  invoke("create_ai_mask", { request }) as Promise<{ mask: AiMaskJson }>;

export interface UpdateAiMaskRequest {
  mask_id: string;
  mask_json: string;
}

export const updateAiMask = (
  request: UpdateAiMaskRequest,
): Promise<{ mask: AiMaskJson }> =>
  invoke("update_ai_mask", { request }) as Promise<{ mask: AiMaskJson }>;

export const aiMaskGet = (maskId: string): Promise<AiMaskJson | null> =>
  invoke("ai_mask_get", { maskId }) as Promise<AiMaskJson | null>;

export const aiMaskListForVersion = (
  imageVersionId: string,
): Promise<{ items: AiMaskJson[] }> =>
  invoke("ai_mask_list_for_version", {
    imageVersionId,
  }) as Promise<{ items: AiMaskJson[] }>;

export type AutoMaskTarget = "stars" | "background" | "bright_core";

export interface BuildAutoMaskRequest {
  image_version_id: string;
  width: number;
  height: number;
  channels: number;
  pixels: number[];
  target: AutoMaskTarget;
}

export interface BuildAutoMaskResponse {
  mask_json: string;
  provenance: string;
  width: number;
  height: number;
}

export const buildAutoMask = (
  request: BuildAutoMaskRequest,
): Promise<BuildAutoMaskResponse> =>
  invoke("build_auto_mask", { request }) as Promise<BuildAutoMaskResponse>;

export type CompositeMaskOp = "union" | "intersect" | "difference";

export interface ComposeMaskRequest {
  project_id: string;
  image_version_id: string;
  parent_a_id: string;
  parent_b_id: string;
  op: CompositeMaskOp;
}

export const composeMask = (
  request: ComposeMaskRequest,
): Promise<{ mask: AiMaskJson }> =>
  invoke("compose_mask", { request }) as Promise<{ mask: AiMaskJson }>;

// CR-06 P6 — quality gate report. The orchestrator
// runs the ten §37 checks against the (source,
// result) pixel pair and returns the verdict +
// per-gate findings. P6 wires the engine end-to-end
// with the apply round; the verdict drives the
// QualityGatePanel surface.

export type QualityVerdict = "ok" | "info" | "warning" | "failure";
export type Severity = "ok" | "info" | "warning" | "failure";
export type GateId =
  | "clipping"
  | "noise_amplification"
  | "star_artifacts"
  | "halos"
  | "ringing"
  | "false_structures"
  | "color_shifts"
  | "edge_artifacts"
  | "segmentation_leakage"
  | "excessive_smoothing";

export interface GateFindingJson {
  gate: GateId;
  severity: Severity;
  message: string;
  score: number;
  recommendation?: string | null;
}

export interface QualityGateReportJson {
  verdict: QualityVerdict;
  findings: GateFindingJson[];
  source_image_version_id: string;
  result_image_version_id: string;
  operation_id: string;
}

export interface RunAiQualityReportRequest {
  source_image_version_id: string;
  result_image_version_id: string;
  operation_id: string;
  width: number;
  height: number;
  channels: number;
  source_pixels: number[];
  result_pixels: number[];
}

export const runAiQualityReport = (
  request: RunAiQualityReportRequest,
): Promise<{ verdict: QualityVerdict; report: QualityGateReportJson }> =>
  invoke("run_ai_quality_report", {
    request,
  }) as Promise<{ verdict: QualityVerdict; report: QualityGateReportJson }>;

export const aiOperationGet = (
  operationId: string,
): Promise<AiOperationJson | null> =>
  invoke("ai_operation_get", { operationId }) as Promise<AiOperationJson | null>;

export const aiOperationListForStage = (
  stageRunId: string,
): Promise<{ items: AiOperationJson[] }> =>
  invoke("ai_operation_list_for_stage", { stageRunId });

export const imageAnalysisLatest = (
  imageVersionId: string,
): Promise<unknown> =>
  invoke("image_analysis_latest", { imageVersionId });

export const imageRegionList = (
  imageVersionId: string,
): Promise<{ items: unknown[] }> =>
  invoke("image_region_list", { imageVersionId });

export const aiRecommendationListForVersion = (
  imageVersionId: string,
): Promise<{ items: unknown[] }> =>
  invoke("ai_recommendation_list_for_version", { imageVersionId });

export const aiMaskList = (
  imageVersionId: string,
): Promise<{ items: unknown[] }> =>
  invoke("ai_mask_list", { imageVersionId });

export const enhancementStackListForSource = (
  imageVersionId: string,
): Promise<{ items: unknown[] }> =>
  invoke("enhancement_stack_list_for_source", { imageVersionId });

export const enhancementPreviewListForOperation = (
  operationId: string,
): Promise<{ items: unknown[] }> =>
  invoke("enhancement_preview_list_for_operation", { operationId });

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

// ─── CR-05 P6 slice 1b (§9 Retry + Skip) ───────────────────────────

/** §9 — UI-side retry variant. Mirrors the Rust `RetryKind`.
 *  Today `optimized` and `as_is` produce the same runner behaviour;
 *  the distinction is preserved at the IPC boundary for traceability
 *  and so a future PR can wire real differentiation. */
export type RetryKind = "optimized" | "as_is";

/** §9 — UI payload for `retry_stage` / `skip_stage`. Mirrors
 *  src-tauri::StageExecutionDto. */
export interface StageExecutionDto {
  stage_execution_id: string;
  plan_id: string;
  stage_id: string;
  attempt: number;
  status: string;
  output_artifact_id: string | null;
  started_at: string | null;
  completed_at: string | null;
  error_json: string | null;
}

/** §9 — `retry_stage(plan_id, stage_id, kind)`.
 *  Re-dispatches a single stage that previously failed.
 *  Returns the new exec row (attempt = prev + 1). */
export const retryStage = (
  planId: string,
  stageId: string,
  kind: RetryKind,
): Promise<StageExecutionDto> =>
  invoke("retry_stage", { planId, stageId, kind });

/** §9 — `skip_stage(plan_id, stage_id)`.
 *  Marks a stage's latest exec row as `skipped` and flips the
 *  plan back to `Ready` so the user can Resume. Refuses on
 *  required stages. */
export const skipStage = (
  planId: string,
  stageId: string,
): Promise<StageExecutionDto> =>
  invoke("skip_stage", { planId, stageId });

// ─── CR-05 R1 — AI model catalog IPC ────────────────────────────────────────
//
// Read-only wrapper around the `ai_model_list` Tauri command. Surfaces the
// canonical AI model list (denoise, super-resolution, de-jpeg, star
// segmentation, cloud score, color calibration, trail inpainting) to the
// application-level AI Models screen. No download flow is exposed in this
// slice; `installed` always returns `false` and a follow-up tranche wires
// the actual install path.

export interface AiModelInfo {
  name: string;
  version: string;
  stage: string;
  license: string;
  size_bytes: number;
  input_channels: number;
  input_tile_size: number;
  output_channels: number;
  scale_factor: number;
  installed: boolean;
}

export const aiModelList = (): Promise<AiModelInfo[]> =>
  invoke("ai_model_list");

// ─── CR-04 P8 — Import Understanding IPC ────────────────────────────────
//
// Wires P3..P7 (frame classification, target detection, session grouping,
// capture analysis, narrowband detection) into the import wizard. The UI
// (P9) reads these wrappers to render the Understanding panel + the
// ambiguity dialog (per CR-04 §13).

export type ImportState =
  | "created"
  | "scanned"
  | "understanding"
  | "ambiguous"
  | "confirmed"
  | "materialised";

export type CaptureKind = "deep_sky" | "planetary_lunar" | "ambiguous";

export type NarrowbandComposition =
  | "none"
  | "mono"
  | "hoo"
  | "sho"
  | "lrgb"
  | "hoo_or_sho";

export interface CaptureObservationJson {
  signal: string;
  value: string;
  weight: number;
  confidence: number;
  favours: CaptureKind;
}

export interface CaptureClassificationJson {
  kind: CaptureKind;
  confidence: number;
  observations: CaptureObservationJson[];
  ambiguous: boolean;
}

export interface NarrowbandObservationJson {
  signal: string;
  value: string;
  weight: number;
  confidence: number;
  resolves_to: string | null;
}

export interface ChannelGroupJson {
  channel: string;
  asset_ids: string[];
}

export interface NarrowbandAnalysisJson {
  channels: ChannelGroupJson[];
  composition_suggestion: NarrowbandComposition;
  confidence: number;
  observations: NarrowbandObservationJson[];
}

export interface SessionAnalysisJson {
  classifications: Array<{
    kind: string;
    confidence: number;
    observations: Array<{
      signal: string;
      value: string;
      weight: number;
      confidence: number;
    }>;
  }>;
  capture: CaptureClassificationJson;
  narrowband: NarrowbandAnalysisJson;
  aggregate_confidence: number;
}

export interface SessionClassificationJson {
  session_id: string;
  import_state: ImportState;
  capture_kind: CaptureKind | null;
  narrowband_composition: NarrowbandComposition | null;
  classification_confidence: number | null;
  classification_metadata: string | null;
}

export interface ImportAnalysisResultJson {
  session_id: string;
  analysis: SessionAnalysisJson;
  classification: SessionClassificationJson;
}

export interface AssetMetadataInputJson {
  path: string;
  metadata: Record<string, unknown>;
}

export const importAnalyseSession = (
  projectId: string,
  sessionId: string,
  assets: AssetMetadataInputJson[],
  targetName: string | null,
): Promise<ImportAnalysisResultJson> =>
  invoke("import_analyse_session", {
    projectId,
    sessionId,
    assets,
    targetName,
  });

export const importGetUnderstanding = (
  sessionId: string,
): Promise<SessionClassificationJson | null> =>
  invoke("import_get_understanding", { sessionId });

export const importConfirm = (
  sessionId: string,
): Promise<SessionClassificationJson> =>
  invoke("import_confirm", { sessionId });

export const importOverrideClassification = (
  sessionId: string,
  captureKind: CaptureKind | null,
  narrowbandComposition: NarrowbandComposition | null,
): Promise<SessionClassificationJson> =>
  invoke("import_override_classification", {
    sessionId,
    captureKind,
    narrowbandComposition,
  });

export const importSetMaterialised = (
  sessionId: string,
): Promise<SessionClassificationJson> =>
  invoke("import_set_materialised", { sessionId });

// CR-04 P10 — AI provenance. Returns the source of
// authority for the target classification
// (deterministic / ai_stub_requested / user_override)
// + a reasoning string the UI surfaces on the
// Understanding panel.

export type ClassificationProvenance =
  | "deterministic"
  | "ai_stub_requested"
  | "user_override";

export interface TargetProvenanceReportJson {
  session_id: string;
  provenance: ClassificationProvenance;
  reasoning: string;
}

export const importGetTargetProvenance = (
  sessionId: string,
): Promise<TargetProvenanceReportJson> =>
  invoke("import_get_target_provenance", { sessionId });

export function provenanceLabel(
  provenance: ClassificationProvenance,
): string {
  switch (provenance) {
    case "deterministic":
      return "Deterministic";
    case "ai_stub_requested":
      return "AI confirming…";
    case "user_override":
      return "User correction applied";
  }
}
