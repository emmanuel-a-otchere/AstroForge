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

import { derived, writable } from "svelte/store";
import type { PipelineRunSummary, ProjectSummary } from "../lib/astroforge-api";
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
}

const initial: WorkspaceState = {
  project: null,
  runs: [],
  loading: false,
  error: null,
};

const internal = writable<WorkspaceState>(initial);

export const workspaceState = {
  subscribe: internal.subscribe,
  /** Load pipeline runs for the active project. */
  async load(project: ProjectSummary): Promise<void> {
    internal.update((s) => ({ ...s, project, loading: true, error: null }));
    try {
      const runs = await api.pipelineRunList(project.project_id);
      internal.set({ project, runs, loading: false, error: null });
    } catch (e) {
      internal.set({
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

/** Status for each stage of the §8 checklist. P4 derives the
 *  'process' stage from the most recent pipeline run. Import /
 *  Analyze / Review / Export are still placeholders pending P5
 *  + a real source-asset event log. */
export const stageStatuses = derived(
  [internal, activePlan, stageExecutions],
  ([$s, $plan, $executions]): Record<
    "import" | "analyze" | "process" | "review" | "export",
    StageStatus
  > => {
    // CR-05 P4 slice 1 — prefer the active CR-05 plan's
    // StageExecutionSummary rows over the legacy CR-02
    // `pipeline_run_list` once a plan exists. The legacy path
    // remains the fallback so the Overview still lights up the
    // checklist for projects that have wizard runs but no CR-05
    // plan yet.
    const processStatus =
      $plan !== null
        ? computeProcessStatusFromPlan($plan.stages, $executions[$plan.plan_id] ?? [])
        : computeProcessStatusFromLegacyRuns($s.runs);
    return {
      import: "pending",
      analyze: "pending",
      process: processStatus,
      review: "pending",
      export: "pending",
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

function computeProcessStatusFromLegacyRuns(
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