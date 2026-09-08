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
    createPreviewRunFor,
    lastError,
    lastRunOutcome,
    pausePipelineRun,
    previewRunsForStageExecution,
    refreshPreviewRuns,
    refreshStageExecutions,
    resumePipelineRun,
    runStatus,
    stageExecutions,
    startPipelineRun,
  } from "../lib/pipeline-plan-store";
  import { readPreviewArtifact } from "../lib/astroforge-api";

  export let planId: string;

  // Local view state: which stage's detail row is expanded.
  let expandedStageId: string | null = null;
  // CR-05 P4 slice 4 — which stage's preview panel is open.
  // Independent of `expandedStageId` so the user can browse
  // previews without expanding the JSON detail.
  let previewStageId: string | null = null;
  // CR-05 P4 slice 4 — keys of in-flight preview requests so
  // the button can show a spinner / disabled state.
  let previewPending: Record<string, boolean> = {};
  // CR-05 P4 slice 5 — base64 PNG payloads keyed by preview_id,
  // loaded lazily for completed previews that have an artifact.
  let previewImages: Record<string, string> = {};

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

  // CR-05 P4 slice 4 — toggle the inline preview panel and
  // fetch the stage's preview runs (lazy).
  async function togglePreview(stageExecutionId: string) {
    if (previewStageId === stageExecutionId) {
      previewStageId = null;
      return;
    }
    previewStageId = stageExecutionId;
    const rows = await refreshPreviewRuns(stageExecutionId);
    // Slice 5 — kick off image loads for completed previews.
    for (const row of rows) {
      void loadPreviewImage(row.preview_id, row.status, row.preview_artifact_id);
    }
  }

  async function onCreatePreview(stageExecutionId: string) {
    previewPending = { ...previewPending, [stageExecutionId]: true };
    try {
      const created = await createPreviewRunFor({
        stage_execution_id: stageExecutionId,
        source_version_id: planId, // reserved until image_versions land
        parameters_json: JSON.stringify({ scale_hint: 0.25 }),
        scale: 0.25,
        label: `Preview — ${stageExecutionId.slice(0, 12)}`,
      });
      if (created) {
        void loadPreviewImage(
          created.preview_id,
          created.status,
          created.preview_artifact_id,
        );
      }
    } finally {
      const { [stageExecutionId]: _drop, ...rest } = previewPending;
      previewPending = rest;
    }
  }

  // CR-05 P4 slice 5 — fetch the PNG bytes for a completed preview
  // and cache the base64 payload for the <img> tag. Failures are
  // non-fatal: the card falls back to showing status + error_json.
  async function loadPreviewImage(
    previewId: string,
    status: string,
    artifactId: string | null,
  ) {
    if (status !== "completed" || !artifactId) return;
    if (previewImages[previewId]) return;
    try {
      const b64 = await readPreviewArtifact(previewId);
      previewImages = { ...previewImages, [previewId]: b64 };
    } catch {
      // Leave uncached; the error banner on the card covers the
      // user-visible failure mode.
    }
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
            <!--
              CR-05 P4 slice 4 — Preview button + inline panel.
              The button toggles a per-stage preview list. The
              panel is independent of the JSON detail row above.
            -->
            <div class="stage-preview-row">
              <button
                type="button"
                class="preview-toggle font-label"
                on:click={() => togglePreview(exec.stage_execution_id)}
                aria-expanded={previewStageId === exec.stage_execution_id}
                aria-label={`Preview ${exec.stage_id}`}
              >
                <span
                  class="material-symbols-outlined preview-icon"
                  aria-hidden="true"
                >
                  visibility
                </span>
                Preview
              </button>
              <button
                type="button"
                class="preview-create font-label"
                on:click={() => onCreatePreview(exec.stage_execution_id)}
                disabled={previewPending[exec.stage_execution_id] === true}
                aria-label={`Create preview for ${exec.stage_id}`}
              >
                {previewPending[exec.stage_execution_id]
                  ? "Creating…"
                  : "New preview"}
              </button>
            </div>
            {#if previewStageId === exec.stage_execution_id}
              {@const previews =
                $previewRunsForStageExecution[exec.stage_execution_id] ?? []}
              <div
                class="preview-panel"
                aria-label={`Previews for ${exec.stage_id}`}
              >
                <h3 class="font-label">Previews ({previews.length})</h3>
                {#if previews.length === 0}
                  <p class="empty font-body">
                    No previews yet. Click <em>New preview</em> to create one.
                  </p>
                {:else}
                  <ul class="preview-list" role="list">
                    {#each previews as preview (preview.preview_id)}
                      <li
                        class="preview-item"
                        data-status={preview.status}
                      >
                        <header class="preview-header">
                          <span class="preview-label font-label">
                            {preview.label}
                          </span>
                          <span
                            class="preview-status font-label"
                            data-status={preview.status}
                          >
                            {preview.status}
                          </span>
                        </header>
                        <p class="preview-meta font-body">
                          scale: {preview.scale} ·
                          hash: <span class="font-mono">{preview.parameters_hash.slice(0, 12)}…</span> ·
                          {preview.started_at ?? "—"} → {preview.completed_at ?? "—"}
                        </p>
                        {#if previewImages[preview.preview_id]}
                          <img
                            class="preview-image"
                            src={`data:image/png;base64,${previewImages[preview.preview_id]}`}
                            alt={`Preview output for ${preview.label}`}
                          />
                        {/if}
                        <pre class="preview-params font-mono">{preview.parameters_json}</pre>
                        {#if preview.error_json}
                          <p class="preview-error font-body" role="alert">
                            {preview.error_json}
                          </p>
                        {/if}
                      </li>
                    {/each}
                  </ul>
                {/if}
              </div>
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

  /* CR-05 P4 slice 4 — inline preview controls + panel. */
  .stage-preview-row {
    display: flex;
    gap: var(--sp-xs);
    padding: var(--sp-xs) var(--sp-sm);
    border-top: 1px solid var(--outline-variant);
  }
  .preview-toggle,
  .preview-create {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline);
    border-radius: var(--radius-full);
    padding: 2px var(--sp-sm);
    font-size: 0.7rem;
    cursor: pointer;
  }
  .preview-create {
    background: transparent;
    border-color: var(--outline-variant);
  }
  .preview-create:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .preview-icon {
    font-size: 14px;
  }
  .preview-panel {
    padding: var(--sp-sm);
    background: var(--surface);
    border-top: 1px solid var(--outline-variant);
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }
  .preview-panel h3 {
    margin: 0;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--on-surface-variant);
  }
  .empty {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }
  .preview-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }
  .preview-item {
    padding: var(--sp-xs) var(--sp-sm);
    background: var(--surface-container-low);
    border-radius: var(--radius-sm);
    border-left: 3px solid var(--outline);
  }
  .preview-item[data-status="completed"] {
    border-left-color: var(--primary);
  }
  .preview-item[data-status="failed"] {
    border-left-color: #ff8a80;
  }
  .preview-header {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-sm);
  }
  .preview-label {
    font-size: 0.8rem;
    color: var(--on-surface);
  }
  .preview-status {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 2px var(--sp-sm);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
  }
  .preview-status[data-status="completed"] {
    background: rgba(111, 191, 115, 0.2);
    color: #6fbf73;
  }
  .preview-status[data-status="failed"] {
    background: rgba(229, 57, 53, 0.2);
    color: #ff8a80;
  }
  .preview-meta {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.75rem;
  }
  /* CR-05 P4 slice 5 — the rendered PNG, scaled to fit the panel. */
  .preview-image {
    max-width: 100%;
    max-height: 240px;
    object-fit: contain;
    border-radius: var(--radius-sm);
    background: var(--surface-container);
    image-rendering: pixelated;
  }
  .preview-params {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.7rem;
    background: var(--surface-container);
    padding: var(--sp-xs);
    border-radius: var(--radius-sm);
    overflow: auto;
  }
  .preview-error {
    margin: 0;
    color: #ff8a80;
    background: rgba(229, 57, 53, 0.12);
    padding: var(--sp-xs) var(--sp-sm);
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
  }
  .placeholder {
    color: var(--on-surface-variant);
    font-style: italic;
  }
</style>