import { useApplicationStore } from "./application";
import { useProjectContextStore } from "./projectContext";
import { workspaceState } from "@/state/workspace";
import { versionStore } from "@/state/versions";
import { comparisonState } from "@/state/comparison";
import { resetImportWizard } from "@/state/import-wizard";
import { resetAiEnhancement } from "@/state/ai-enhancement";
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
} from "@/lib/pipeline-plan-store";

function resetPipelinePlanStore() {
  activePlan.set(null);
  lastError.set(null);
  lastRunOutcome.set(null);
  planGenerationStatus.set("idle");
  planList.set([]);
  previewRunsForStageExecution.update(() => ({}));
  recommendationsForPlan.update(() => ({}));
  resumablePlans.set([]);
  runStatus.set("idle");
  stageExecutions.update(() => ({}));
}

export function useLifecycle() {
  const app = useApplicationStore();
  const context = useProjectContextStore();

  async function openProject(project) {
    closeProject();
    context.open(project);
    app.openProject(project);
    void workspaceState.load(project);
    void versionStore.load(project.project_id);
    void comparisonState.load(project.project_id);
  }

  function closeProject() {
    context.close();
    app.closeProject();
    workspaceState.reset();
    versionStore.reset();
    resetAiEnhancement();
    comparisonState.reset();
    resetPipelinePlanStore();
    resetImportWizard();
  }

  return { openProject, closeProject };
}
