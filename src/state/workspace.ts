// CR-03 P4 — workspace content store.
//
// Reads pipeline-run state for the active project and surfaces a
// derived set of stage statuses for the §8 Overview checklist and
// the §12 Process workspace. P4 ships the read path; the actual
// run/control UX lands in P4b alongside the wizard deprecation.
//
// CR-05 P4 slice 1 — the 'process' stage no longer reads from
// the legacy `pipeline_run_list` command (CR-02 wizard table).
// Instead it reads the active CR-05 `PipelinePlan`'s
// `StageExecutionSummary` rows so the checklist reflects what
// CR-05 actually runs. Until an active plan exists the legacy
// path is preserved (keeps CR-03 P4 behaviour intact for projects
// that have wizard runs but no CR-05 plan).
//
// CR-05 P4 slice 2 — `load(project)` also refreshes the most
// recent CR-05 plan's `StageExecutionSummary` rows so the
// `stageExecutions` store is warm the moment a project opens,
// without waiting for the user to navigate to Process.
//
// CR-05 R2 — `import / analyze / review / export` checklist
// states are no longer hard-coded `pending`. They come from
// `projectOverview(projectId)` (the `project_overview` Tauri
// command) which derives the booleans from the durable event
// log + per-table fallbacks. The previous derivation in this
// file is preserved for the `process` stage, which still uses
// the CR-05 plan/legacy-run path.

import { derived, get, writable } from "svelte/store";
import type { PipelineRunSummary, ProjectSummary, ProjectOverview } from "../lib/astroforge-api";
import type { StageExecutionSummary } from "../lib/astroforge-api";
import * as api from "../lib/astroforge-api";
import { activePlan, stageExecutions } from "../lib/pipeline-plan-store";

export type StageStatus = "complete" | "pending" | "blocked";

interface WorkspaceState {
  /** Last-loaded project (used to detect project changes). */
  project: ProjectSummary | null;
  runs: PipelineRunSummary[];
  loading: boolean;
  error: string | null;
  /**
   * R2: server-derived overview booleans. `null` while the IPC
   * call is in flight or when running outside Tauri (browser
   * dev mode). When `null`, the Overview shows the existing
   * CR-05 P4 fallback so the checklist still works.
   */
  overview: ProjectOverview | null;
  overviewLoading: boolean;
  overviewError: string | null;
}

const initial: WorkspaceState = {
  project: null,
  runs: [],
  loading: false,
  error: null,
  overview: null,
  overviewLoading: false,
  overviewError: null,
};

const internal = writable<WorkspaceState>(initial);

export const workspaceState = {
  subscribe: internal.subscribe,
  /** Load pipeline runs for the active project. */
  async load(project: ProjectSummary): Promise<void> {
    internal.update((s) => ({ ...s, project, loading: true, error: null }));
    try {
      const runs = await api.pipelineRunList(project.project_id);
      internal.set({ ...initial, project, runs, loading: false, error: null });
    } catch (e) {
      internal.set({
        ...initial,
        project,
        runs: [],
        loading: false,
        error: e instanceof Error ? e.message : String(e),
      });
    }
    // CR-05 P4 slice 2 — best-effort refresh of the project's
    // most recent CR-05 plan's stage executions. Failures are
    // swallowed (legacy `runs` already loaded; nothing else
    // breaks if this fails). Fire-and-forget so `load` does
    // not block the store transition.
    void refreshMostRecentPlanStageExecutions(project.project_id);
    // CR-05 R2 — load the truthful project overview.
    void refreshProjectOverview(project.project_id);
  },
  /** R2: refresh just the project overview (used after a
   *  successful apply, export, or analysis event). */
  async refreshOverview(): Promise<void> {
    const current = get(internal);
    if (!current.project) return;
    await refreshProjectOverview(current.project.project_id);
  },
  reset() {
    internal.set(initial);
  },
};

/// CR-05 P4 slice 2 — list the project's CR-05 plans, pick the
/// most recent (whatever the backend returns first; the list
/// endpoint sorts by `created_at desc` already in CR-05 P1),
/// and refresh its `StageExecutionSummary` rows into the
/// store so downstream consumers (slice 1's `stageStatuses`
/// derivation, the per-stage lists in `ProcessingControls`)
/// see the latest run state without requiring the user to
/// navigate to Process first.
async function refreshMostRecentPlanStageExecutions(
  projectId: string,
): Promise<void> {
  try {
    const plans = await api.pipelinePlanListForProject(projectId);
    const mostRecent = plans[0];
    if (!mostRecent) return;
    const execs = await api.pipelinePlanListStageExecutions(mostRecent.plan_id);
    stageExecutions.update((map) => ({ ...map, [mostRecent.plan_id]: execs }));
  } catch {
    // Best-effort. The store simply stays empty until a
    // navigation or runner callback populates it.
  }
}

/// CR-05 R2 — fetch the truthful project overview. On success,
/// the five checklist booleans are stored on `workspaceState`.
/// On failure (Tauri unavailable in browser dev, project
/// removed, etc.) the failure is recorded but the existing
/// `process` derivation still works — the Overview renders
/// the server-derived booleans when present and the CR-05 P4
/// fallback otherwise.
async function refreshProjectOverview(projectId: string): Promise<void> {
  internal.update((s) => ({ ...s, overviewLoading: true, overviewError: null }));
  try {
    const overview = await api.projectOverview(projectId);
    internal.update((s) => ({
      ...s,
      overview,
      overviewLoading: false,
      overviewError: null,
    }));
  } catch (e) {
    internal.update((s) => ({
      ...s,
      overview: null,
      overviewLoading: false,
      overviewError: e instanceof Error ? e.message : String(e),
    }));
  }
}

/** Status for each stage of the §8 checklist. R2 derivation:
 *  - `import` → `overview.imported` (any source asset on the
 *    project's sessions, OR a `SourceImported` event)
 *  - `analyze` → `overview.analyzed` (`AnalysisCompleted` event)
 *  - `process` → unchanged from CR-05 P4 (CR-05 plan/legacy runs)
 *  - `review` → `overview.versioned` (`VersionCreated` event)
 *  - `export` → `overview.exported` (`ExportCreated` event)
 *
 *  When the overview is not yet loaded (loading or errored),
 *  the derivation falls back to `pending` so the checklist
 *  doesn't show a false "complete" on a fresh project before
 *  the IPC resolves. */
export const stageStatuses = derived(
  [internal, activePlan, stageExecutions],
  ([$s, $plan, $executions]): Record<
    "import" | "analyze" | "process" | "review" | "export",
    StageStatus
  > => {
    // CR-05 P4 slice 1 — derive the 'process' stage from the
    // active CR-05 plan. R5 retired the legacy wizard
    // fallback: when no plan exists the §8 checklist reports
    // `pending` (an honest "no processing data yet") rather
    // than walking the CR-02 wizard table.
    const processStatus = $plan
      ? computeProcessStatusFromPlan(
          $plan.stages,
          $executions[$plan.plan_id] ?? [],
        )
      : "pending";

    const o = $s.overview;
    const ready = o !== null;
    return {
      import: ready && o!.imported ? "complete" : "pending",
      analyze: ready && o!.analyzed ? "complete" : "pending",
      process: processStatus,
      review: ready && o!.versioned ? "complete" : "pending",
      export: ready && o!.exported ? "complete" : "pending",
    };
  },
);

/// CR-05 P4 slice 1 — derive the 'process' stage from the
/// active CR-05 plan. Any failed stage execution marks the
/// whole plan blocked; all executions completed or skipped
/// marks it complete; otherwise pending (covers running /
/// queued / never-attempted stages).
function computeProcessStatusFromPlan(
  planStages: readonly { stage_id: string }[],
  executions: readonly StageExecutionSummary[],
): StageStatus {
  if (planStages.length === 0) return "pending";
  // Index executions by stage_id; we collapse retries (attempt
  // > 1) by taking the highest-attempt execution per stage.
  const latestByStage = new Map<string, StageExecutionSummary>();
  for (const exec of executions) {
    const prev = latestByStage.get(exec.stage_id);
    if (!prev || exec.attempt > prev.attempt) {
      latestByStage.set(exec.stage_id, exec);
    }
  }
  let anyFailed = false;
  let anyInFlight = false;
  let allResolved = true;
  for (const stage of planStages) {
    const exec = latestByStage.get(stage.stage_id);
    if (!exec) {
      // Stage hasn't been attempted yet.
      allResolved = false;
      continue;
    }
    switch (exec.status) {
      case "failed":
        anyFailed = true;
        allResolved = false;
        break;
      case "completed":
      case "skipped":
        // Resolved — keep allResolved true.
        break;
      case "pending":
      case "running":
        anyInFlight = true;
        allResolved = false;
        break;
      default:
        // Unknown status string — treat as in-flight so the
        // checklist doesn't prematurely go green.
        anyInFlight = true;
        allResolved = false;
        break;
    }
  }
  if (anyFailed) return "blocked";
  if (allResolved && !anyInFlight) return "complete";
  return "pending";
}

/// CR-05 R5 — legacy wizard fallback retired. The previous
/// `computeProcessStatusFromLegacyRuns` read CR-02 wizard
/// `pipeline_runs` rows. R5 removes the call site: every
/// project going forward creates a CR-05 plan via
/// `create_pipeline_plan`, and the legacy wizard
/// `pipeline_run_session` is preserved at the IPC layer only
/// for backwards compatibility. The `runs` field on
/// `WorkspaceState` is kept (so consumers like the Overview's
/// pipeline-runs list can still render historical data) but
/// no longer drives the §8 checklist.
///
/// The function itself is preserved (renamed with a
/// `_DEPRECATED` suffix) so the previous behavior remains
/// grep-able and recoverable. No live code calls it.
function computeProcessStatusFromLegacyRuns_DEPRECATED(
  runs: readonly PipelineRunSummary[],
): StageStatus {
  if (runs.length === 0) return "pending";
  const latest = runs[runs.length - 1];
  switch (latest.status) {
    case "Completed":
      return "complete";
    case "Failed":
    case "Cancelled":
      return "blocked";
    case "Queued":
    case "Running":
      return "pending";
  }
}