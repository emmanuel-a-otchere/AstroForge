<!--
  CR-03 P3 — Studio Overview workspace (§8).

  Shows the processing-state checklist (§8) for the open project:
    - Import: have we got source data?
    - Analyze: do we have an astrometric solution?
    - Process: is there at least one completed pipeline run?
    - Review: is there a final image version?
    - Export: has anything been exported?

  P3 ships the layout with deterministic placeholders for the
  actual stage status (P4 wires real pipeline-run data; P5 wires
  image version status). The layout itself is final.
-->
<script lang="ts">
  import { activeProject } from "../state/project-context";
  import { stageStatuses } from "../state/workspace";

  interface ChecklistStage {
    id: "import" | "analyze" | "process" | "review" | "export";
    label: string;
    description: string;
  }

  const stageDefs: ChecklistStage[] = [
    {
      id: "import",
      label: "Import",
      description: "Source files loaded into the project workspace.",
    },
    {
      id: "analyze",
      label: "Analyze",
      description: "Plate-solve, calibration, and target classification.",
    },
    {
      id: "process",
      label: "Process",
      description: "At least one completed pipeline run.",
    },
    {
      id: "review",
      label: "Review",
      description: "Final image version is rendered and reviewed.",
    },
    {
      id: "export",
      label: "Export",
      description: "At least one image version exported.",
    },
  ];
</script>

<section class="overview" aria-labelledby="overview-title">
  <header class="overview-header">
    <h1 id="overview-title" class="font-display">Project Overview</h1>
    <p class="overview-subtitle font-body">
      Track where the project is in the pipeline. P3 ships the
      checklist; P4 wires real stage status; P5 wires the image
      version timeline and AI recommendations.
    </p>
  </header>

  {#if $activeProject}
    <div class="project-summary">
      <span class="font-label">Project</span>
      <span class="project-name font-display">{$activeProject.name}</span>
      <span class="status-pill" data-status={$activeProject.status}>
        {$activeProject.status}
      </span>
    </div>
  {/if}

  <ol class="checklist" aria-label="Processing-stage checklist">
    {#each stageDefs as stage, idx (stage.id)}
      <li class="checklist-item" data-status={$stageStatuses[stage.id]}>
        <div class="step-number font-label">{idx + 1}</div>
        <div class="step-body">
          <h2 class="step-title font-display">{stage.label}</h2>
          <p class="step-description font-body">{stage.description}</p>
        </div>
        <div class="step-state" aria-label="Status: {$stageStatuses[stage.id]}">
          {#if $stageStatuses[stage.id] === "complete"}
            <span class="material-symbols-outlined state-icon complete" aria-hidden="true">
              check_circle
            </span>
          {:else if $stageStatuses[stage.id] === "blocked"}
            <span class="material-symbols-outlined state-icon blocked" aria-hidden="true">
              block
            </span>
          {:else}
            <span class="material-symbols-outlined state-icon pending" aria-hidden="true">
              schedule
            </span>
          {/if}
        </div>
      </li>
    {/each}
  </ol>

  <footer class="overview-footer">
    <p class="hint font-body">
      The Overview reflects the latest pipeline run and image version.
      Real-time stage status lights up in P4 alongside the Process
      workspace.
    </p>
  </footer>
</section>

<style>
  .overview {
    display: flex;
    flex-direction: column;
    gap: var(--sp-lg);
    padding: var(--sp-xl);
    max-width: 1100px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    color: var(--on-surface);
  }

  .overview-header h1 {
    font-size: 1.75rem;
    margin: 0 0 var(--sp-xs) 0;
  }

  .overview-subtitle {
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 70ch;
    font-size: 0.9rem;
  }

  .project-summary {
    display: flex;
    align-items: baseline;
    gap: var(--sp-sm);
    padding: var(--sp-md) var(--sp-lg);
    background: var(--surface-container-low);
    border-radius: var(--radius-md);
    border: 1px solid var(--outline-variant);
  }

  .project-summary .project-name {
    font-size: 1.1rem;
    font-weight: 600;
  }

  .status-pill {
    font-size: 0.7rem;
    padding: 2px var(--sp-sm);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .status-pill[data-status="Active"] {
    background: rgba(111, 191, 115, 0.2);
    color: #6fbf73;
  }

  .status-pill[data-status="Archived"] {
    background: rgba(255, 179, 0, 0.2);
    color: #ffb300;
  }

  .status-pill[data-status="Exported"] {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
  }

  .checklist {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .checklist-item {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--sp-md);
    padding: var(--sp-md) var(--sp-lg);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
  }

  .checklist-item[data-status="complete"] {
    border-color: rgba(111, 191, 115, 0.4);
  }

  .checklist-item[data-status="blocked"] {
    border-color: rgba(229, 57, 53, 0.4);
  }

  .step-number {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
  }

  .checklist-item[data-status="complete"] .step-number {
    background: rgba(111, 191, 115, 0.2);
    color: #6fbf73;
  }

  .step-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .step-title {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .step-description {
    margin: 0;
    font-size: 0.85rem;
    color: var(--on-surface-variant);
  }

  .state-icon {
    font-size: 28px;
  }

  .state-icon.complete {
    color: #6fbf73;
  }

  .state-icon.pending {
    color: var(--on-surface-variant);
  }

  .state-icon.blocked {
    color: #ff8a80;
  }

  .overview-footer .hint {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
    margin: 0;
  }
</style>