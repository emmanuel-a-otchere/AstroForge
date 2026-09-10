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
  import {
    getResourceSnapshot,
    getProcessingMetrics,
    getProcessingTimeline,
    readPreviewArtifact,
    cancelPipelinePlan,
    retryStage,
    skipStage,
    stageExecutionBudget,
    type ExecutionBackend,
    type ExecutionBudget,
    type Precision,
    type ProcessingMetricsDto,
    type ResourceSnapshot,
    type ProcessingTimelineDto,
  } from "../lib/astroforge-api";
  import ExpertDagView from "./ExpertDagView.svelte";
  import StageCard from "./StageCard.svelte";
  import ErrorRecoveryPanel from "./ErrorRecoveryPanel.svelte";
  import ProcessingTimeline from "./ProcessingTimeline.svelte";

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

  // CR-05 P4 slice 7 (§21) — resource snapshot for the advanced
  // "Execution resources" inspect block. Loaded lazily on first expand
  // and refreshed on every subsequent expand so memory pressure is current.
  let resourceSnapshot: ResourceSnapshot | null = null;
  let resourceError: string | null = null;
  let resourceLoading = false;

  // R7 — presentational-only actions surface a clear, dismissable
  // notice instead of `console.info` stubs. The two recovery
  // actions (`adjustProcessing`, `contactSupport`) and the
  // timeline-row click handler are placeholders for follow-up
  // tranches (P6.1c, P6.3); the notice makes the gap visible
  // rather than hiding it behind silent no-ops.
  type NoticeKind = "adjustProcessing" | "contactSupport" | "timelineSelect";
  let notice: { kind: NoticeKind; message: string } | null = null;

  function showNotice(kind: NoticeKind, message: string) {
    notice = { kind, message };
  }

  function dismissNotice() {
    notice = null;
  }

  // CR-05 P5 slice 4 (§27) — aggregate metrics for the Expert DAG view.
  // Refreshed whenever the active plan changes or a new stage completes.
  let processingMetrics: ProcessingMetricsDto | null = null;
  let processingMetricsLoading = false;

  async function loadProcessingMetrics() {
    if (processingMetricsLoading) return;
    processingMetricsLoading = true;
    try {
      processingMetrics = await getProcessingMetrics(planId);
    } catch (err) {
      // Failure is non-fatal — the DAG view still renders without
      // metrics; the recommendation banner just stays hidden.
      console.warn("[ProcessingControls] getProcessingMetrics failed", err);
      processingMetrics = null;
    } finally {
      processingMetricsLoading = false;
    }
  }

  // CR-05 P6 slice 3 (§25) — per-stage processing timeline. Same
  // refresh strategy as `processingMetrics`: pull on plan change
  // and after each completion; failure is non-fatal (the panel
  // surfaces an empty-state message).
  let processingTimeline: ProcessingTimelineDto | null = null;
  let processingTimelineLoading = false;

  async function loadProcessingTimeline() {
    if (processingTimelineLoading) return;
    processingTimelineLoading = true;
    try {
      processingTimeline = await getProcessingTimeline(planId);
    } catch (err) {
      console.warn(
        "[ProcessingControls] getProcessingTimeline failed",
        err,
      );
      processingTimeline = null;
    } finally {
      processingTimelineLoading = false;
    }
  }

  // CR-05 P6.1b (§9) — ErrorRecoveryPanel action handlers. Each
  // action triggers a backend call and refreshes the metrics
  // + timeline so the UI reflects the new state. AdjustProcessing
  // and ContactSupport remain presentational for now (no backend
  // wiring yet — they'll land in P6.1c with the recommendation
  // engine's tuning affordances).

  /** Stage id of the currently-failed stage (from metrics). Used
   *  as the target for retry_stage / skip_stage. */
  $: failedStageId = processingMetrics?.latest_stage_id ?? null;

  async function handleRetryOptimized() {
    if (!failedStageId) return;
    try {
      await retryStage(planId, failedStageId, "optimized");
      // Refresh so the recovery panel disappears (on success)
      // or stays with the new error (on retry-failure).
      await Promise.all([loadProcessingMetrics(), loadProcessingTimeline()]);
    } catch (err) {
      console.warn("[ProcessingControls] retryStage failed", err);
    }
  }

  async function handleRetryAsIs() {
    if (!failedStageId) return;
    try {
      await retryStage(planId, failedStageId, "as_is");
      await Promise.all([loadProcessingMetrics(), loadProcessingTimeline()]);
    } catch (err) {
      console.warn("[ProcessingControls] retryStage (as_is) failed", err);
    }
  }

  async function handleSkipStage() {
    if (!failedStageId) return;
    try {
      await skipStage(planId, failedStageId);
      await Promise.all([loadProcessingMetrics(), loadProcessingTimeline()]);
    } catch (err) {
      console.warn("[ProcessingControls] skipStage failed", err);
    }
  }

  function handleAdjustProcessing() {
    // R7 — P6.1c: opens a tuning dialog wired to the
    // recommendation engine. The follow-up tranche isn't
    // landed yet; surface a clear notice rather than a
    // silent no-op so the user knows the action is a
    // placeholder.
    showNotice(
      "adjustProcessing",
      "Adjust processing lands in the P6.1c follow-up tranche. The recovery panel's Retry and Skip actions are live today.",
    );
  }

  function handleContactSupport() {
    // R7 — P6.1c: opens the support bundle flow (CR-07
    // territory). Surface a clear notice rather than a
    // silent no-op.
    showNotice(
      "contactSupport",
      "Support bundle flow lands in the P6.1c follow-up tranche. The recovery panel's Cancel action remains available.",
    );
  }

  function handleCancel() {
    // Cancel the whole plan via the existing cancel command. The
    // cancel handle is owned by the backend (PipelinePlanState);
    // the IPC flips the same atomic flag the runner checks
    // between stages.
    cancelPipelinePlan(planId).catch((err: unknown) => {
      console.warn("[ProcessingControls] cancelPipelinePlan failed", err);
    });
  }

  async function loadResourceSnapshot() {
    if (resourceLoading) return;
    resourceLoading = true;
    resourceError = null;
    try {
      resourceSnapshot = await getResourceSnapshot();
    } catch (err) {
      resourceError = err instanceof Error ? err.message : String(err);
    } finally {
      resourceLoading = false;
    }
  }

  const BACKEND_LABELS: Record<ExecutionBackend, string> = {
    cpu: "CPU",
    cuda: "CUDA",
    direct_ml: "DirectML",
    core_ml: "CoreML",
    open_vino: "OpenVINO",
  };
  const PRECISION_LABELS: Record<Precision, string> = {
    f16: "float16",
    f32: "float32",
  };

  function formatGiB(bytes: number): string {
    return `${(bytes / 1024 ** 3).toFixed(1)} GiB`;
  }

  // CR-05 P5 slice 2 (§22) — pre-flight budget for the active plan's
  // first stage. Re-runs on `plan` change and on Start click so memory
  // pressure is fresh. The §22 "more memory than available" warning is
  // surfaced verbatim when the budget requires tiling.
  let prefetchedBudget: ExecutionBudget | null = null;
  let prefetchedBudgetError: string | null = null;
  let prefetchedBudgetLoading = false;

  async function refreshPreflightBudget() {
    if (!plan || plan.stages.length === 0) {
      prefetchedBudget = null;
      prefetchedBudgetError = null;
      return;
    }
    prefetchedBudgetLoading = true;
    prefetchedBudgetError = null;
    try {
      prefetchedBudget = await stageExecutionBudget(
        plan.stages[0].parameters_json,
      );
    } catch (err) {
      prefetchedBudget = null;
      prefetchedBudgetError = err instanceof Error ? err.message : String(err);
    } finally {
      prefetchedBudgetLoading = false;
    }
  }

  $: if (plan) void refreshPreflightBudget();

  async function onStart() {
    // CR-05 P5 slice 2 (§22) — refresh pre-flight on Start so memory
    // pressure is current. The warning is advisory (matches §22 copy);
    // we don't block the run, only surface the message.
    await refreshPreflightBudget();
    await startPipelineRun(planId);
  }

  $: plan = $activePlan?.plan_id === planId ? $activePlan : null;
  $: executions = $stageExecutions[planId] ?? [];
  // CR-05 P5 slice 4 — re-fetch the §27 aggregate metrics whenever
  // either the active plan or its stage executions change, so the
  // DAG view's recommendation banner stays current during a run.
  $: if (plan || executions.length > 0) void loadProcessingMetrics();
  $: if (plan || executions.length > 0) void loadProcessingTimeline();
  // The plan is "paused" if the backend reports that status. Slice 2.5
  // surfaces a Resume button in that case instead of Start.
  $: planStatus = plan?.status ?? "ready";

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

  <!-- CR-05 P5 slice 2 (§22) — pre-flight memory warning surfaced
       verbatim from `derive_execution_budget` when memory is tight.
       Advisory per the spec; not blocking. -->
  {#if prefetchedBudget?.warning}
    <p class="memory-warning font-body" role="alert" data-testid="memory-warning">
      {prefetchedBudget.warning}
    </p>
  {:else if prefetchedBudgetError}
    <p class="error font-body" role="alert">{prefetchedBudgetError}</p>
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

  <!-- CR-05 P4 slice 7 (§21) — advanced users can inspect what the
       engine picked on their behalf. The default posture stays
       "AstroForge optimized processing for this device". -->
  <details
    class="resource-inspect"
    on:toggle={(e) => {
      if ((e.currentTarget as HTMLDetailsElement).open) void loadResourceSnapshot();
    }}
  >
    <summary class="font-label" data-testid="resource-inspect-toggle">
      Execution resources
    </summary>
    {#if resourceLoading}
      <p class="font-body resource-note">Detecting device…</p>
    {:else if resourceError}
      <p class="error font-body" role="alert">{resourceError}</p>
    {:else if resourceSnapshot}
      <p class="font-body resource-note">
        AstroForge optimized processing for this device.
      </p>
      <dl class="resource-grid font-body" data-testid="resource-snapshot">
        <div>
          <dt>CPU</dt>
          <dd>
            {resourceSnapshot.cpu_model} · {resourceSnapshot.logical_cores}
            threads{#if resourceSnapshot.physical_cores}
              ({resourceSnapshot.physical_cores} cores){/if}
          </dd>
        </div>
        <div>
          <dt>GPU</dt>
          <dd>
            {#if resourceSnapshot.gpus.length > 0}
              {resourceSnapshot.gpus
                .map(
                  (g) =>
                    `${g.name} (${BACKEND_LABELS[g.backend]}${g.vram_bytes ? `, ${formatGiB(g.vram_bytes)}` : ""})`,
                )
                .join(" · ")}
            {:else}
              None detected — CPU execution
            {/if}
          </dd>
        </div>
        <div>
          <dt>Memory</dt>
          <dd>
            {formatGiB(resourceSnapshot.available_memory_bytes)} available of
            {formatGiB(resourceSnapshot.total_memory_bytes)}
          </dd>
        </div>
        <div>
          <dt>Tile size</dt>
          <dd>{resourceSnapshot.recommended.tile_size}px</dd>
        </div>
        <div>
          <dt>Precision</dt>
          <dd>{PRECISION_LABELS[resourceSnapshot.recommended.precision]}</dd>
        </div>
        <div>
          <dt>Backend</dt>
          <dd>{BACKEND_LABELS[resourceSnapshot.recommended.backend]}</dd>
        </div>
        <div>
          <dt>Threads</dt>
          <dd>{resourceSnapshot.recommended.thread_count}</dd>
        </div>
        <div>
          <dt>Memory budget</dt>
          <dd>{formatGiB(resourceSnapshot.recommended.memory_budget_bytes)}</dd>
        </div>
      </dl>
    {/if}
  </details>

  <!-- CR-05 P5 slice 4 (§24 Pipeline Visualization + §27 metrics).
       Renders the plan as a DAG with the latest adaptive recommendation
       banner underneath. Pulls `get_processing_metrics` lazily; the
       component itself handles the "not loaded" state. -->
  <ExpertDagView
    {plan}
    metrics={processingMetrics}
    stageStatus={Object.fromEntries(
      executions.map((e) => [e.stage_id, e.status]),
    )}
  />

  <!-- CR-05 P6.1 (§28) — when the most-recent stage failed, surface
       the structured recovery panel. The panel reads from
       `processingMetrics.latest_stage_error`, which the backend
       populates from StageExecution.error_json via the slice 4
       metrics aggregator. Renders above any other guidance so the
       user sees it first. -->
  {#if processingMetrics?.latest_stage_error}
    <ErrorRecoveryPanel
      error={processingMetrics.latest_stage_error}
      stageLabel={processingMetrics.latest_stage_type ?? null}
      on:retryOptimized={handleRetryOptimized}
      on:retry={handleRetryAsIs}
      on:skipStage={handleSkipStage}
      on:cancel={handleCancel}
      on:adjustProcessing={handleAdjustProcessing}
      on:contactSupport={handleContactSupport}
    />
  {/if}

  <!-- CR-05 P6.2 (§23) — render the AI provenance badge for the
       most-recent stage. Component handles the non-AI / null cases
       internally (renders nothing). -->
  <StageCard metrics={processingMetrics} />

  <!-- R7 — dismissable notice for presentational actions
       (Adjust processing, Contact support, timeline reveal).
       Rendered above the timeline so the user sees the gap
       right after clicking. -->
  {#if notice}
    <aside
      class="followup-notice font-body"
      role="status"
      aria-live="polite"
      data-kind={notice.kind}
    >
      <span class="material-symbols-outlined" aria-hidden="true">info</span>
      <p>{notice.message}</p>
      <button
        type="button"
        class="dismiss"
        on:click={dismissNotice}
        aria-label="Dismiss notice"
      >
        <span class="material-symbols-outlined" aria-hidden="true">close</span>
      </button>
    </aside>
  {/if}

  <!-- CR-05 P6 slice 3 (§25) — per-stage processing timeline.
       Click a row to reveal the corresponding Image Version (the
       actual reveal wiring is downstream of P6.3; the panel
       surfaces the version_id for the parent to act on). -->
  <section class="timeline-section" data-testid="timeline-section">
    <h3>Processing Timeline</h3>
    <ProcessingTimeline
      events={processingTimeline?.events ?? []}
      onSelect={(versionId) => {
        // R7 — P6.3: clicking a timeline row reveals the
        // corresponding Image Version. The actual reveal
        // wiring (ImageViewer.focus(versionId) or a branch-
        // and-compare UI) lands downstream of P6.3. For now
        // we surface a clear notice with the version id so
        // the user can correlate the click with the panel.
        showNotice(
          "timelineSelect",
          `Reveal for image version ${versionId} lands in the P6.3 follow-up tranche. The version id is now logged for reference.`,
        );
      }}
    />
  </section>
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
  /* CR-05 P4 slice 7 (§21) — resource inspect block. */
  .resource-inspect {
    border-top: 1px solid var(--outline-variant, var(--outline));
    padding-top: var(--sp-sm);
  }
  .resource-inspect summary {
    cursor: pointer;
    font-size: 0.85rem;
    color: var(--on-surface-variant);
  }
  .resource-note {
    margin: var(--sp-xs) 0;
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    font-style: italic;
  }
  .resource-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--sp-xs) var(--sp-md);
    margin: var(--sp-xs) 0 0 0;
  }
  .resource-grid dt {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--on-surface-variant);
  }
  .resource-grid dd {
    margin: 0;
    font-size: 0.85rem;
    color: var(--on-surface);
  }
  /* CR-05 P5 slice 2 (§22) — pre-flight memory warning, distinct
     from the run-error red. Uses the accent token so the user sees
     it as "informational about memory pressure", not a failure. */
  .memory-warning {
    margin: 0;
    color: var(--on-surface);
    background: var(--surface-container-high);
    border-left: 3px solid var(--primary);
    padding: var(--sp-xs) var(--sp-sm);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
  }

  /* R7 — follow-up notice. Same shape as the memory warning so
     users learn one visual idiom for "informational, not a
     failure". */
  .followup-notice {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-sm);
    margin: 0;
    color: var(--on-surface);
    background: var(--surface-container-high);
    border-left: 3px solid var(--primary);
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
  }

  .followup-notice .material-symbols-outlined {
    color: var(--primary);
    flex: 0 0 auto;
  }

  .followup-notice p {
    flex: 1 1 auto;
    margin: 0;
    font-size: 0.85rem;
  }

  .followup-notice .dismiss {
    background: transparent;
    border: none;
    color: var(--on-surface-variant);
    cursor: pointer;
    padding: 2px;
    border-radius: var(--radius-sm);
  }

  .followup-notice .dismiss:hover {
    background: var(--surface-container);
    color: var(--on-surface);
  }
</style>