// src/lib/pipeline-plan-store.ts
//
// CR-05 P1 — frontend store for PipelinePlan (additive).
//
// The wizard `pipeline-store.ts` continues to own the live session
// execution path. This store owns the **plan** surface: generate a plan
// from a Session Understanding, render it as a human-readable
// processing journey, list plans for a project.
//
// P1 ships Auto mode end-to-end (generate → render → list). P2 wires
// the runner that produces StageExecution rows; P3 wires the
// recommendation engine; P5 exposes the Expert DAG view.
//
// State shape mirrors the IPC types in `astroforge-api.ts`. Stores use
// Svelte's `writable` so components can subscribe; the surface is small
// enough to keep the entire store in one file.

import { derived, get, writable, type Readable } from "svelte/store";

import {
  cancelPipelinePlan,
  createPipelinePlan,
  pausePipelinePlan,
  pipelinePlanGet,
  pipelinePlanListForProject,
  pipelinePlanListResumableForProject,
  pipelinePlanListStageExecutions,
  resumePipelinePlan,
  startPipelinePlan,
  type CreatePipelinePlanRequest,
  type PipelinePlanDto,
  type PipelinePlanMode,
  type PipelinePlanSummary,
  type PipelineStageDto,
  type RunOutcome,
  type StageExecutionSummary,
} from "./astroforge-api";

// ─── Stores ─────────────────────────────────────────────────────────────────

export const activePlan = writable<PipelinePlanDto | null>(null);
export const planList = writable<PipelinePlanSummary[]>([]);
export const planGenerationStatus = writable<"idle" | "generating" | "error">("idle");
export const lastError = writable<string | null>(null);

// ─── CR-05 P2 slice 1 — execution stores ─────────────────────────────────
//
// `runStatus` reflects the runner's lifecycle (idle | running | error).
// `stageExecutions` is a per-plan map keyed by plan_id so multiple
// plans can be observed concurrently. P2.5 adds `paused` to the
// `runStatus` enum and per-plan pause handles.

export const runStatus = writable<"idle" | "running" | "error">("idle");
export const lastRunOutcome = writable<RunOutcome | null>(null);
export const stageExecutions = writable<Record<string, StageExecutionSummary[]>>({});

// ─── CR-05 P2.5 — recovery stores ─────────────────────────────────────────
//
// `resumablePlans` lists every `Paused` plan for a project. The
// RecoveryBanner subscribes; clicking "Resume" calls resumePipelineRun
// and re-fetches both `activePlan` and `resumablePlans`.

export const resumablePlans = writable<PipelinePlanSummary[]>([]);

// Derived: a human-readable processing journey string per CR-05 §2.1
// ("M31 — Deep Sky OSC / ✓ Calibrate / ✓ Debayer / …"). Components
// render this in the ProcessWorkspace zone-A strip.
export const humanReadablePlan: Readable<string> = derived(activePlan, ($plan) => {
  if (!$plan) return "";
  const header = `${$plan.target_type.replace(/_/g, " ")} / ${$plan.mode}`;
  const lines = $plan.stages.map((s) => {
    const mark = s.required ? "✓" : "○";
    return `${mark} ${s.label}`;
  });
  return [header, ...lines].join("\n");
});

// ─── Operations ────────────────────────────────────────────────────────────

export async function generateAutoPlan(
  projectId: string,
  sessionId: string,
): Promise<PipelinePlanDto | null> {
  planGenerationStatus.set("generating");
  lastError.set(null);
  try {
    const request: CreatePipelinePlanRequest = {
      project_id: projectId,
      session_id: sessionId,
      recipe_id: "deep_sky_osc_balanced",
      mode: "auto",
      // P3 will pass the real Session Understanding from CR-04; P1 lets
      // the backend apply its default (Deep-Sky OSC Balanced) so the
      // Auto-mode smoke panel works without CR-04 integration.
      session_understanding: null,
    };
    const plan = await createPipelinePlan(request);
    activePlan.set(plan);
    return plan;
  } catch (err) {
    lastError.set(toMessage(err));
    planGenerationStatus.set("error");
    return null;
  } finally {
    if (get(planGenerationStatus) === "generating") {
      planGenerationStatus.set("idle");
    }
  }
}

export async function generateGuidedPlan(
  projectId: string,
  sessionId: string,
): Promise<PipelinePlanDto | null> {
  return generatePlanWithMode(projectId, sessionId, "guided");
}

export async function generateExpertPlan(
  projectId: string,
  sessionId: string,
): Promise<PipelinePlanDto | null> {
  return generatePlanWithMode(projectId, sessionId, "expert");
}

async function generatePlanWithMode(
  projectId: string,
  sessionId: string,
  mode: PipelinePlanMode,
): Promise<PipelinePlanDto | null> {
  planGenerationStatus.set("generating");
  lastError.set(null);
  try {
    const plan = await createPipelinePlan({
      project_id: projectId,
      session_id: sessionId,
      recipe_id: "deep_sky_osc_balanced",
      mode,
      session_understanding: null,
    });
    activePlan.set(plan);
    return plan;
  } catch (err) {
    lastError.set(toMessage(err));
    planGenerationStatus.set("error");
    return null;
  } finally {
    if (get(planGenerationStatus) === "generating") {
      planGenerationStatus.set("idle");
    }
  }
}

export async function refreshPlanList(projectId: string): Promise<void> {
  try {
    const list = await pipelinePlanListForProject(projectId);
    planList.set(list);
  } catch (err) {
    lastError.set(toMessage(err));
  }
}

export async function loadPlan(planId: string): Promise<PipelinePlanDto | null> {
  try {
    const plan = await pipelinePlanGet(planId);
    activePlan.set(plan);
    return plan;
  } catch (err) {
    lastError.set(toMessage(err));
    return null;
  }
}

// ─── CR-05 P2 slice 1 — execution operations ──────────────────────────────
//
// `startPipelineRun` blocks until the runner returns. P5's
// resource-aware execution will introduce async / backgrounded
// execution; slice 1 keeps the synchronous, immediately-resolving
// shape so the UI can mirror the wizard's button-press UX.

export async function startPipelineRun(
  planId: string,
): Promise<RunOutcome | null> {
  runStatus.set("running");
  lastError.set(null);
  lastRunOutcome.set(null);
  try {
    const outcome = await startPipelinePlan(planId);
    lastRunOutcome.set(outcome);
    // Refresh per-stage state so the UI reflects the persisted rows.
    await refreshStageExecutions(planId);
    return outcome;
  } catch (err) {
    lastError.set(toMessage(err));
    runStatus.set("error");
    return null;
  } finally {
    if (get(runStatus) === "running") {
      runStatus.set("idle");
    }
  }
}

export async function cancelPipelineRun(planId: string): Promise<boolean> {
  try {
    return await cancelPipelinePlan(planId);
  } catch (err) {
    lastError.set(toMessage(err));
    return false;
  }
}

export async function refreshStageExecutions(
  planId: string,
): Promise<StageExecutionSummary[]> {
  try {
    const execs = await pipelinePlanListStageExecutions(planId);
    stageExecutions.update((map) => ({ ...map, [planId]: execs }));
    return execs;
  } catch (err) {
    lastError.set(toMessage(err));
    return [];
  }
}

// ─── CR-05 P2.5 — pause / resume / refresh operations ─────────────────────

export async function pausePipelineRun(planId: string): Promise<boolean> {
  try {
    return await pausePipelinePlan(planId);
  } catch (err) {
    lastError.set(toMessage(err));
    return false;
  }
}

export async function resumePipelineRun(
  planId: string,
): Promise<RunOutcome | null> {
  runStatus.set("running");
  lastError.set(null);
  lastRunOutcome.set(null);
  try {
    const outcome = await resumePipelinePlan(planId);
    lastRunOutcome.set(outcome);
    await refreshStageExecutions(planId);
    return outcome;
  } catch (err) {
    lastError.set(toMessage(err));
    runStatus.set("error");
    return null;
  } finally {
    if (get(runStatus) === "running") {
      runStatus.set("idle");
    }
  }
}

export async function refreshResumablePlans(
  projectId: string,
): Promise<PipelinePlanSummary[]> {
  try {
    const list = await pipelinePlanListResumableForProject(projectId);
    resumablePlans.set(list);
    return list;
  } catch (err) {
    lastError.set(toMessage(err));
    return [];
  }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

export function requiredStages(plan: PipelinePlanDto): PipelineStageDto[] {
  return plan.stages.filter((s) => s.required);
}

export function optionalStages(plan: PipelinePlanDto): PipelineStageDto[] {
  return plan.stages.filter((s) => !s.required);
}

export function progressFraction(plan: PipelinePlanDto): number {
  // P1 doesn't track per-stage execution yet (P2 wires it). For now
  // "progress" reflects how many stages the user has enabled, which is
  // a useful proxy for "how ready is this plan to run".
  if (plan.stages.length === 0) return 0;
  const enabled = plan.stages.filter((s) => s.enabled).length;
  return enabled / plan.stages.length;
}

function toMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}