// CR-03 P4 — workspace content store.
//
// Reads pipeline-run state for the active project and surfaces a
// derived set of stage statuses for the §8 Overview checklist and
// the §12 Process workspace. P4 ships the read path; the actual
// run/control UX lands in P4b alongside the wizard deprecation.

import { derived, writable } from "svelte/store";
import type { PipelineRunSummary, ProjectSummary } from "../lib/astroforge-api";
import * as api from "../lib/astroforge-api";

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
  },
  reset() {
    internal.set(initial);
  },
};

/** Status for each stage of the §8 checklist. P4 derives the
 *  'process' stage from the most recent pipeline run. Import /
 *  Analyze / Review / Export are still placeholders pending P5
 *  + a real source-asset event log. */
export const stageStatuses = derived(
  internal,
  ($s): Record<"import" | "analyze" | "process" | "review" | "export", StageStatus> => {
    const processStatus: StageStatus = computeProcessStatus($s.runs);
    return {
      import: "pending",
      analyze: "pending",
      process: processStatus,
      review: "pending",
      export: "pending",
    };
  },
);

function computeProcessStatus(runs: readonly PipelineRunSummary[]): StageStatus {
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