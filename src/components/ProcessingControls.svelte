<!--
  CR-05 P2 slice 1 — ProcessingControls component.

  Renders Start / Cancel buttons for a PipelinePlan. The runner is
  synchronous: clicking Start blocks until the plan finishes (or is
  cancelled); the per-stage state appears below the buttons.

  Pause / Resume buttons land in P2.5 alongside the recovery banner.

  Props:
    planId — the plan to control. Required.
-->
<script lang="ts">
  import {
    activePlan,
    cancelPipelineRun,
    lastError,
    lastRunOutcome,
    pausePipelineRun,
    refreshStageExecutions,
    resumePipelineRun,
    runStatus,
    stageExecutions,
    startPipelineRun,
  } from "../lib/pipeline-plan-store";

  export let planId: string;

  // Local view state: which stage's detail row is expanded.
  let expandedStageId: string | null = null;

  $: plan = $activePlan?.plan_id === planId ? $activePlan : null;
  $: executions = $stageExecutions[planId] ?? [];
  // The plan is "paused" if the backend reports that status. Slice 2.5
  // surfaces a Resume button in that case instead of Start.
  $: planStatus = plan?.status ?? "ready";

  async function onStart() {
    await startPipelineRun(planId);
  }

  async function onPause() {
    await pausePipelineRun(planId);
    await refreshStageExecutions(planId);
  }

  async function onResume() {
    await resumePipelineRun(planId);
  }

  async function onCancel() {
    await cancelPipelineRun(planId);
    await refreshStageExecutions(planId);
  }

  function toggle(stageId: string) {
    expandedStageId = expandedStageId === stageId ? null : stageId;
  }
</script>

<section class="controls" aria-label="Pipeline run controls">
  <header class="controls-header">
    <h2 class="font-display">Run controls</h2>
    {#if plan}
      <p class="font-body plan-meta">
        {plan.stages.length} stages ·
        {plan.mode} mode ·
        {plan.target_type.replace(/_/g, " ")} ·
        <span class="plan-status" data-status={planStatus}>
          {planStatus}
        </span>
      </p>
    {:else}
      <p class="font-body plan-meta">No active plan selected.</p>
    {/if}
  </header>

  <div class="button-row">
    <button
      type="button"
      class="start font-label"
      on:click={onStart}
      disabled={!plan || $runStatus === "running" || planStatus === "paused"}
      data-testid="start-pipeline"
    >
      {$runStatus === "running" ? "Running…" : "Start"}
    </button>
    <button
      type="button"
      class="pause font-label"
      on:click={onPause}
      disabled={$runStatus !== "running"}
      data-testid="pause-pipeline"
    >
      Pause
    </button>
    <button
      type="button"
      class="resume font-label"
      on:click={onResume}
      disabled={!plan || planStatus !== "paused"}
      data-testid="resume-pipeline"
    >
      {$runStatus === "running" ? "Resuming…" : "Resume"}
    </button>
    <button
      type="button"
      class="cancel font-label"
      on:click={onCancel}
      disabled={$runStatus !== "running"}
      data-testid="cancel-pipeline"
    >
      Cancel
    </button>
  </div>

  {#if $lastRunOutcome}
    <p class="outcome font-body" data-outcome={$lastRunOutcome}>
      Last run outcome: <strong>{$lastRunOutcome}</strong>
    </p>
  {/if}

  {#if $lastError}
    <p class="error font-body" role="alert">{$lastError}</p>
  {/if}

  {#if executions.length > 0}
    <div class="stage-table" aria-label="Stage executions">
      <h3 class="font-label">Stages ({executions.length})</h3>
      <ul class="stage-list">
        {#each executions as exec (exec.stage_execution_id)}
          <li
            class="stage-row"
            data-status={exec.status}
            data-testid="stage-row"
          >
            <button
              type="button"
              class="stage-row-button font-body"
              on:click={() => toggle(exec.stage_id)}
              aria-expanded={expandedStageId === exec.stage_id}
            >
              <span class="stage-status" data-status={exec.status}>
                {exec.status}
              </span>
              <span class="stage-id font-label">{exec.stage_id}</span>
              <span class="stage-time font-label">
                {exec.started_at ?? "—"}
              </span>
            </button>
            {#if expandedStageId === exec.stage_id}
              <pre class="stage-detail font-label">{JSON.stringify(exec, null, 2)}</pre>
            {/if}
          </li>
        {/each}
      </ul>
    </div>
  {:else}
    <p class="placeholder font-body">
      No stage executions yet. Click <em>Start</em> to run the plan.
    </p>
  {/if}
</section>

<style>
  .controls {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-lg);
    background: var(--surface-container);
    border-radius: var(--radius-md);
  }
  .controls-header h2 {
    margin: 0 0 var(--sp-xs) 0;
    font-size: 1.1rem;
  }
  .plan-meta {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }
  .button-row {
    display: flex;
    gap: var(--sp-sm);
  }
  .start {
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    border-radius: var(--radius-full);
    padding: var(--sp-xs) var(--sp-md);
    cursor: pointer;
  }
  .start:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .cancel {
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline);
    border-radius: var(--radius-full);
    padding: var(--sp-xs) var(--sp-md);
    cursor: pointer;
  }
  .cancel:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .pause {
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline);
    border-radius: var(--radius-full);
    padding: var(--sp-xs) var(--sp-md);
    cursor: pointer;
  }
  .pause:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .resume {
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline);
    border-radius: var(--radius-full);
    padding: var(--sp-xs) var(--sp-md);
    cursor: pointer;
  }
  .resume:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .outcome {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }
  .outcome[data-outcome="completed"] strong {
    color: #6fbf73;
  }
  .outcome[data-outcome="cancelled"] strong {
    color: #ffb300;
  }
  .outcome[data-outcome="paused"] strong {
    color: #ffb300;
  }
  .outcome[data-outcome="failed"] strong {
    color: #ff8a80;
  }
  .plan-status {
    padding: 2px var(--sp-sm);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .plan-status[data-status="completed"] {
    background: rgba(111, 191, 115, 0.2);
    color: #6fbf73;
  }
  .plan-status[data-status="paused"] {
    background: rgba(255, 179, 0, 0.2);
    color: #ffb300;
  }
  .plan-status[data-status="failed"],
  .plan-status[data-status="cancelled"] {
    background: rgba(229, 57, 53, 0.2);
    color: #ff8a80;
  }
  .error {
    color: #ff8a80;
    background: rgba(229, 57, 53, 0.12);
    padding: var(--sp-xs) var(--sp-sm);
    border-radius: var(--radius-sm);
  }
  .stage-table h3 {
    margin: 0 0 var(--sp-xs) 0;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--on-surface-variant);
  }
  .stage-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }
  .stage-row {
    background: var(--surface-container-low);
    border-radius: var(--radius-sm);
  }
  .stage-row-button {
    display: grid;
    grid-template-columns: 100px 1fr auto;
    gap: var(--sp-sm);
    width: 100%;
    background: transparent;
    border: none;
    color: inherit;
    padding: var(--sp-xs) var(--sp-sm);
    cursor: pointer;
    text-align: left;
  }
  .stage-status {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 2px var(--sp-sm);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
  }
  .stage-status[data-status="completed"] {
    background: rgba(111, 191, 115, 0.2);
    color: #6fbf73;
  }
  .stage-status[data-status="running"] {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
  }
  .stage-status[data-status="failed"] {
    background: rgba(229, 57, 53, 0.2);
    color: #ff8a80;
  }
  .stage-id {
    color: var(--on-surface-variant);
    font-family: var(--font-mono);
    font-size: 0.75rem;
  }
  .stage-time {
    color: var(--on-surface-variant);
    font-size: 0.7rem;
  }
  .stage-detail {
    margin: 0;
    padding: var(--sp-sm);
    background: var(--surface);
    color: var(--on-surface);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 0.75rem;
  }
  .placeholder {
    color: var(--on-surface-variant);
    font-style: italic;
  }
</style>