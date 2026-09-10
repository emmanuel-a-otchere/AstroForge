// CR-05 R4 — centralized project lifecycle.
//
// The audit (docs/UI_WORKFLOW_AUDIT.md, recommendation R4) flagged
// that opening and closing a project was wired piecemeal across
// App.svelte, StudioShell, and keyboard-shortcuts — the close path
// only reset `projectContext` and `studioViewport`, leaving stale
// data in `workspaceState`, `versionStore`, and the per-plan
// `pipelinePlanStore` writables. R4 makes `projectLifecycle` the
// single source of truth for both transitions.
//
// `openProject(project)` and `closeProject()` are the canonical
// entry points. They coordinate every project-scoped store:
//   - `projectContext` — project identity (dirty bit, etc.)
//   - `studioViewport` — Studio overlay + active tab
//   - `workspaceState` — pipeline runs + project overview booleans
//   - `versionStore` — image version timeline
//   - `pipelinePlanStore` — active plan, stage executions, runs,
//     recommendations, preview runs, preview gate satisfaction,
//     run status, last outcome, resumable plans, plan generation
//     status, last error
//
// `openProject` always calls `closeProject` first, so the path is
// re-entry safe. Callers that only need to change the project
// (rather than refresh all data) can still call the lower-level
// store methods directly; this module is the lifecycle boundary.

import { projectContext } from "./project-context";
import { studioViewport } from "./application";
import { workspaceState } from "./workspace";
import { versionStore } from "./versions";
import { resetAiEnhancement } from "./ai-enhancement";
import {
  activePlan,
  lastError,
  lastRunOutcome,
  planGenerationStatus,
  planList,
  previewRunsForStageExecution,
  recommendationsForPlan,
  resumablePlans,
  runStatus,
  stageExecutions,
} from "../lib/pipeline-plan-store";
import type { ProjectSummary } from "../lib/astroforge-api";

/**
 * Open a project. Idempotent: calling open while a project is
 * already open re-resets every project-scoped store before
 * loading the new project's data. This avoids a class of bug
 * where stale data from project A leaks into the view for
 * project B (e.g. the Overview checklist showing A's pipeline
 * runs while loading B's data).
 */
export async function openProject(project: ProjectSummary): Promise<void> {
  // Always close first so any existing state is gone before the
  // new project's data lands.
  closeProject();
  projectContext.open(project);
  studioViewport.openProject({
    project_id: project.project_id,
    name: project.name,
  });
  // Refresh the durable reads. Failures are non-fatal: every
  // store handles its own error and the UI surfaces the
  // resulting `error` field. We don't await here because the
  // caller (App.svelte's project action) doesn't block on the
  // refresh — the Studio shell renders immediately and each
  // store self-populates as its IPC resolves.
  void workspaceState.load(project);
  void versionStore.load(project.project_id);
}

/**
 * Close the active project. Resets every project-scoped store
 * so re-opening either the same or a different project starts
 * from a clean slate. After this call the application context
 * is restored: no project is open, the Studio overlay is
 * hidden, and the project navigation target defaults to
 * `projects` so the user lands on a sensible screen.
 */
export function closeProject(): void {
  projectContext.close();
  studioViewport.closeProject();
  workspaceState.reset();
  versionStore.reset();
  // CR-06 P1 — reset the AI Enhancement Studio store so
  // reopening a project (or a different one) starts from a
  // clean slate. Per-image-version data lands in P2+ via
  // `loadAiEnhancementFor` when the user navigates to Enhance.
  resetAiEnhancement();
  resetPipelinePlanStore();
}

/**
 * Reset every writable in `pipelinePlanStore`. The store
 * exposes a flat list of writables (one per UI concern) rather
 * than a single typed object so each call site can subscribe to
 * exactly what it needs. R4 keeps that shape and just
 * centralizes the reset.
 */
function resetPipelinePlanStore(): void {
  activePlan.set(null);
  lastError.set(null);
  lastRunOutcome.set(null);
  planGenerationStatus.set("idle");
  planList.set([]);
  // `previewGateSatisfiedForStageExecution` is a derived store
  // keyed off `previewRunsForStageExecution`; clearing the
  // source is enough to reset it. (The derived store recomputes
  // to `{}` automatically.)
  previewRunsForStageExecution.update(() => ({}));
  recommendationsForPlan.update(() => ({}));
  resumablePlans.set([]);
  runStatus.set("idle");
  stageExecutions.update(() => ({}));
}

/**
 * Lifecycle facade. Re-exports the lifecycle plus a small
 * `subscribe` helper for components that just want a single
 * observable for "is a project open". The standalone
 * `hasOpenProject` derived in `project-context` is still the
 * canonical answer for "is a project open"; this is a
 * re-export so consumers can `import { hasOpenProject } from
 * "../state/project-lifecycle"` without splitting reads.
 */
export { hasOpenProject, activeProject, isProjectDirty } from "./project-context";
