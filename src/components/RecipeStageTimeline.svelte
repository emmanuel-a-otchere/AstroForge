<!--
  CR-07 B15 — Recipe Stage Timeline.

  Companion to ProvenancePanel (B14). Renders the full
  vertical timeline of all `RecipeStage` rows from the
  Recipe profile: stage_id, enabled state, params
  (expanded in a <details> per stage). No IPC of its
  own: subscribes to the B13c provenanceStore like
  ProvenancePanel does.

  Honest scope:
    - Recipe-level data only. RecipeStage has no
      timestamps (those live on PipelineRun.stage_runs,
      not on the Recipe profile). The audit item
      demanding "processing history with timestamps"
      is per-RUN, not per-RECIPE; addressing it would
      require a new IPC (recipe_stage_runs_for_image_version)
      that joins ImageVersion -> PipelineRun ->
      StageRun, which is a different slice.
    - Stages preview in B14 ProvenancePanel is
      superseded by this component. The B14 preview
      stays as a fallback (still useful for at-a-glance
      sizing) but the timeline is the source of truth
      for full stage visibility.

  Five render states (same shape as ProvenancePanel):
    1. status === 'loading' -> spinner + "Loading
       stage timeline".
    2. status === 'error' -> error notice.
    3. status === 'loaded' && recipe === null ->
       "Profile not recorded" (same empty state as
       the panel; the timeline renders the same
       honest message rather than a redundant
       placeholder).
    4. status === 'loaded' && recipe !== null ->
       the full timeline: summary line (stage count,
       enabled vs disabled) + an ordered list of
       stage rows with expandable params.
    5. status === 'idle' -> "Select a version".
-->
<script lang="ts">
  import { provenanceStore, type ProvenanceRecipe } from "../state/provenance-store";

  interface Props {
    versionId: string;
    versionLabel: string;
  }
  const { versionId, versionLabel }: Props = $props();

  // CR-07 B15: kick the store to fetch on mount or
  // versionId change. Same pattern as ProvenancePanel;
  // the store's stale-load guard means both components
  // can fire concurrently without stepping on each
  // other.
  $effect(() => {
    void provenanceStore.load(versionId);
  });

  // Local derived reads of the store; only emit
  // values when the store's `versionId` is the one
  // we asked for, so a stale fetch for another
  // version doesn't bleed into this component.
  const recipe = $derived<ProvenanceRecipe | null>(
    $provenanceStore.versionId === versionId
      ? $provenanceStore.recipe
      : null,
  );
  const status = $derived(
    $provenanceStore.versionId === versionId
      ? $provenanceStore.status
      : "idle" as const,
  );
  const error = $derived<string | null>(
    $provenanceStore.versionId === versionId
      ? $provenanceStore.error
      : null,
  );

  // CR-07 B15: stage summary (counts total, enabled,
  // and disabled. Renders in the timeline header.
  const stageCounts = $derived(
    recipe
      ? {
          total: recipe.stages.length,
          enabled: recipe.stages.filter((s) => s.enabled).length,
          disabled: recipe.stages.filter((s) => !s.enabled).length,
        }
      : null,
  );

  // Param rendering: convert each HashMap<String,
  // serde_json::Value> to sorted entries so the
  // output is stable across renders. Long string
  // values are not truncated (the user wants to
  // see the full params; a future slice may add
  // copy-to-clipboard).
  const renderedParams = $derived(
    recipe
      ? recipe.stages.map((s) =>
          Object.entries(s.params).sort(([a], [b]) => a.localeCompare(b)),
        )
      : [],
  );

  function formatValue(v: unknown): string {
    if (v === null || v === undefined) return "—";
    if (typeof v === "string") return v;
    if (typeof v === "number" || typeof v === "boolean") return String(v);
    try {
      return JSON.stringify(v);
    } catch {
      return String(v);
    }
  }
</script>

<section class="stage-timeline" aria-label={`Stage timeline for ${versionLabel}`}>
  <header class="panel-header">
    <span class="panel-tag font-label">Stage Timeline</span>
    <h3 class="panel-title font-display">{versionLabel}</h3>
  </header>

  {#if status === "loading"}
    <p class="status-note font-body" aria-live="polite">
      <span class="material-symbols-outlined" aria-hidden="true">progress_activity</span>
      Loading stage timeline…
    </p>
  {:else if status === "error"}
    <div class="error" role="alert">
      <p class="error-text font-body">
        <span class="material-symbols-outlined" aria-hidden="true">error</span>
        {error ?? "Unknown error"}
      </p>
      <button
        type="button"
        class="retry-cta"
        onclick={() => provenanceStore.load(versionId)}
        aria-label="Retry loading stage timeline"
      >
        Retry
      </button>
    </div>
  {:else if status === "loaded" && recipe === null}
    <div class="empty">
      <p class="empty-title font-body">
        <span class="material-symbols-outlined" aria-hidden="true">info</span>
        Profile not recorded
      </p>
      <p class="empty-body font-body">
        No Recipe profile linked to this image version, so
        no stage timeline to display. See the ProvenancePanel
        for the same honest empty state.
      </p>
    </div>
  {:else if status === "loaded" && recipe !== null}
    {#if stageCounts && stageCounts.total === 0}
      <p class="empty-body font-body">No stages recorded on this recipe.</p>
    {:else if stageCounts}
      <div class="timeline-summary font-body">
        <span class="summary-item">
          <strong>{stageCounts.total}</strong>
          {stageCounts.total === 1 ? "stage" : "stages"}
        </span>
        {#if stageCounts.disabled > 0}
          <span class="summary-item">
            <strong>{stageCounts.enabled}</strong> enabled
          </span>
          <span class="summary-item summary-disabled">
            <strong>{stageCounts.disabled}</strong> disabled
          </span>
        {:else if stageCounts.enabled > 0}
          <span class="summary-item">
            all <strong>{stageCounts.enabled}</strong> enabled
          </span>
        {/if}
      </div>

      <ol class="stage-list">
        {#each recipe.stages as s, i (i)}
          <li class="stage-card" data-enabled={s.enabled}>
            <div class="stage-card-header">
              <span class="stage-num">{i + 1}</span>
              <span class="stage-name font-body">{s.stageId}</span>
              {#if !s.enabled}
                <span class="stage-disabled">disabled</span>
              {/if}
            </div>
            {#if renderedParams[i] && renderedParams[i].length > 0}
              <details class="stage-params-details">
                <summary class="stage-params-summary font-label">
                  Parameters ({renderedParams[i].length})
                </summary>
                <table class="stage-params-table font-body">
                  <thead>
                    <tr>
                      <th class="param-key-head">Key</th>
                      <th class="param-value-head">Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each renderedParams[i] as [k, v] (k)}
                      <tr>
                        <td class="param-key">{k}</td>
                        <td class="param-value">{formatValue(v)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </details>
            {:else}
              <p class="stage-no-params font-body">No parameters.</p>
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
  {:else}
    <p class="status-note font-body">Select a version to view the stage timeline.</p>
  {/if}
</section>

<style>
  .stage-timeline {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
  }

  .panel-tag {
    font-size: 0.7rem;
    color: var(--primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .panel-title {
    margin: 0;
    font-size: 1rem;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-note {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    color: var(--on-surface-variant);
    margin: 0;
  }

  .error {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    padding: var(--sp-sm);
    background: rgba(244, 67, 54, 0.1);
    border-left: 3px solid #f44336;
    border-radius: var(--radius-sm);
  }

  .error-text {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    color: #ef9a9a;
    margin: 0;
  }

  .retry-cta {
    align-self: flex-start;
    padding: var(--sp-xs) var(--sp-sm);
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .empty {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
    padding: var(--sp-sm);
    background: var(--surface-container-high);
    border-left: 3px solid var(--outline-variant);
    border-radius: var(--radius-sm);
  }

  .empty-title {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    color: var(--on-surface);
    margin: 0;
    font-weight: 500;
  }

  .empty-body {
    color: var(--on-surface-variant);
    margin: 0;
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .timeline-summary {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-md);
    padding: var(--sp-xs) var(--sp-sm);
    background: var(--surface-container-high);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    color: var(--on-surface-variant);
  }

  .summary-item strong {
    color: var(--on-surface);
    font-weight: 500;
  }

  .summary-disabled {
    color: #ffab91;
  }

  .stage-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .stage-card {
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .stage-card[data-enabled="false"] {
    opacity: 0.65;
  }

  .stage-card-header {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: var(--sp-sm);
    background: var(--surface-container);
    border-bottom: 1px solid var(--outline-variant);
  }

  .stage-num {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: 50%;
    background: var(--surface-container-high);
    color: var(--primary);
    font-size: 0.7rem;
    flex-shrink: 0;
    font-weight: 500;
  }

  .stage-name {
    flex: 1;
    color: var(--on-surface);
    font-weight: 500;
  }

  .stage-disabled {
    font-size: 0.7rem;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    padding: 2px var(--sp-xs);
    background: rgba(255, 171, 145, 0.1);
    border-radius: var(--radius-full);
  }

  .stage-params-details {
    padding: 0;
  }

  .stage-params-summary {
    padding: var(--sp-xs) var(--sp-sm);
    cursor: pointer;
    color: var(--on-surface-variant);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    user-select: none;
  }

  .stage-params-summary:hover {
    background: var(--surface-container);
  }

  .stage-params-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }

  .stage-params-table th,
  .stage-params-table td {
    text-align: left;
    padding: var(--sp-xs) var(--sp-sm);
    border-top: 1px solid var(--outline-variant);
  }

  .param-key-head,
  .param-value-head {
    color: var(--on-surface-variant);
    text-transform: uppercase;
    font-size: 0.65rem;
    letter-spacing: 0.05em;
    font-weight: 500;
    background: var(--surface-container);
  }

  .param-key {
    color: var(--on-surface-variant);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    width: 40%;
  }

  .param-value {
    color: var(--on-surface);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    word-break: break-all;
  }

  .stage-no-params {
    margin: 0;
    padding: var(--sp-sm);
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    font-style: italic;
  }
</style>