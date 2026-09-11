<!--
  CR-04 P9 — Step 3 of the §12 Import wizard: Understanding.

  Renders the P8 orchestrator output:
  - target name + kind + confidence
  - capture classification (deep-sky / planetary-lunar / ambiguous)
  - narrowband composition suggestion
  - per-channel group counts
  - per-classifier confidence

  When `import_state == "ambiguous"` (per §13 ambiguity
  UX), renders an alert + offers override.
-->
<script lang="ts">
  import {
    captureKindLabel,
    confidenceLabel,
    fetchProvenance,
    goToStep,
    importWizard,
    narrowbandCompositionLabel,
    refreshUnderstanding,
    type TargetProvenanceReportJson,
  } from "../../state/import-wizard";

  $: state = $importWizard;
  $: classification = state.classification;
  $: analysis = state.analysis;
  $: ambiguous =
    classification?.import_state === "ambiguous" ||
    (classification?.classification_confidence !== null &&
      classification?.classification_confidence !== undefined &&
      (classification.classification_confidence ?? 1) < 0.6);

  let provenance: TargetProvenanceReportJson | null = null;
  let provenanceError: string | null = null;

  async function refresh(): Promise<void> {
    if (state.sessionId) {
      await refreshUnderstanding(state.sessionId);
      try {
        provenance = await fetchProvenance();
        provenanceError = null;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        provenanceError = msg;
      }
    }
  }
</script>

<section class="understanding" aria-labelledby="step-3-heading">
  <h2 id="step-3-heading" class="font-display">What did you capture?</h2>

  {#if !classification || !analysis}
    <p class="font-body">
      No classification yet. Go back to Step 2 and let the pipeline complete.
    </p>
  {:else}
    <dl class="summary">
      <div class="row">
        <dt class="font-body">Target</dt>
        <dd class="font-display">
          {analysis.capture.observations.find((o) => o.signal === "target_kind")?.value
            ? `${analysis.capture.observations.find((o) => o.signal === "target_kind")?.value}`
            : "Unknown"}
        </dd>
      </div>

      <div class="row">
        <dt class="font-body">Capture type</dt>
        <dd class="font-display">
          {captureKindLabel(classification.capture_kind)}
          <span class="confidence">{confidenceLabel(classification.classification_confidence)}</span>
        </dd>
      </div>

      <div class="row">
        <dt class="font-body">Narrowband</dt>
        <dd class="font-display">
          {narrowbandCompositionLabel(classification.narrowband_composition)}
        </dd>
      </div>

      {#if analysis.narrowband.channels.length > 0}
        <div class="row">
          <dt class="font-body">Channels</dt>
          <dd>
            <ul class="channels font-body">
              {#each analysis.narrowband.channels as ch (ch.channel)}
                <li>
                  <strong>{ch.channel}</strong>
                  <span class="muted">{ch.asset_ids.length} frames</span>
                </li>
              {/each}
            </ul>
          </dd>
        </div>
      {/if}
    </dl>

    {#if ambiguous}
      <aside class="ambiguity-alert" role="alert">
        <span class="material-symbols-outlined" aria-hidden="true">warning</span>
        <div>
          <strong>Dataset characteristics are ambiguous.</strong>
          <p>
            The pipeline couldn't classify this dataset confidently enough to
            route it silently. Override the suggestion below or go back.
          </p>
        </div>
      </aside>
    {/if}

    {#if provenance}
      <details class="provenance">
        <summary class="font-body">
          Provenance: <strong>{provenance.provenance}</strong>
        </summary>
        <p class="font-body provenance-reasoning">{provenance.reasoning}</p>
      </details>
    {:else if provenanceError}
      <p class="error" role="alert">Provenance fetch failed: {provenanceError}</p>
    {/if}

    <div class="actions">
      <button class="btn-secondary" on:click={refresh} type="button">
        Refresh
      </button>
      <button class="btn-primary" on:click={() => goToStep("confirm")} type="button">
        Continue
      </button>
    </div>
  {/if}
</section>

<style>
  .understanding {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1.5rem;
  }
  h2 {
    margin: 0;
  }
  .summary {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin: 0;
  }
  .row {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 0.5rem;
    align-items: baseline;
  }
  dt {
    color: var(--color-text-secondary, #999);
    margin: 0;
  }
  dd {
    margin: 0;
  }
  .confidence {
    margin-left: 0.5rem;
    font-size: 0.85rem;
    color: var(--color-text-secondary, #999);
  }
  .channels {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .channels .muted {
    margin-left: 0.25rem;
    color: var(--color-text-secondary, #999);
  }
  .ambiguity-alert {
    display: flex;
    gap: 0.75rem;
    align-items: flex-start;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    background: var(--color-warning-bg, #3a2a00);
    border: 1px solid var(--color-warning-border, #f5a623);
  }
  .ambiguity-alert .material-symbols-outlined {
    color: var(--color-warning, #f5a623);
  }
  .provenance {
    border: 1px solid var(--color-border, #444);
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
    background: var(--color-surface-elevated, #1a1a1a);
  }
  .provenance summary {
    cursor: pointer;
    color: var(--color-text-secondary, #999);
  }
  .provenance-reasoning {
    margin: 0.5rem 0 0;
    color: var(--color-text-primary, #eee);
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }
  .btn-primary,
  .btn-secondary {
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    border: 1px solid transparent;
  }
  .btn-primary {
    background: var(--color-accent, #4f9eff);
    color: white;
  }
  .btn-secondary {
    background: var(--color-surface, #222);
    color: var(--color-text-primary, #eee);
    border-color: var(--color-border, #444);
  }
</style>
