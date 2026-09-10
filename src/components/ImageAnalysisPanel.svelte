<!--
  CR-06 P2 — Image Analysis Panel.

  Renders the structured observations from an
  `ImageAnalysisReport` per CR-06 §6 (the "Observation →
  Evidence → Confidence → Recommendation" intelligence
  contract). The panel reads its report from the
  `aiEnhancementStore` (set via `loadAiEnhancementFor`)
  and surfaces:

    - a 10-row observation table: noise, background,
      clipping, chromatic noise, local contrast, stars,
      nebula, bright core, hot pixels, trails
    - per-row confidence score (Low / Medium / High)
    - the evidence region (clickable to highlight the
      source region in the canvas — P5 wires the
      highlight overlay)
    - the structured sub-fields (star_count,
      faint_structure_count, has_bright_core, etc.) used
      by the P3 recommendation engine

  P3+ replaces the "Recommendations" rail with the live
  output of `aiRecommendationListForVersion`. P2 ships
  the panel with a clear placeholder so the gap is
  visible.
-->
<script lang="ts">
  import { aiEnhancementStore, generateRecommendationsFor } from "../state/ai-enhancement";
  import RecommendationList from "./RecommendationList.svelte";

  interface ObservationJson {
    name: string;
    label: string;
    value: number;
    evidence_region?: [number, number, number, number] | null;
    confidence: "Low" | "Medium" | "High";
  }

  interface ImageAnalysisJson {
    image_version_id: string;
    width: number;
    height: number;
    channels: number;
    observations: ObservationJson[];
    star_count: number;
    faint_structure_count: number;
    has_bright_core: boolean;
    hot_pixel_count: number;
    trail_artifact_count: number;
    engine_version: string;
    created_at: string;
  }

  function confidenceScore(c: ObservationJson["confidence"]): number {
    switch (c) {
      case "High":
        return 0.92;
      case "Medium":
        return 0.75;
      case "Low":
      default:
        return 0.5;
    }
  }

  function confidenceLabel(score: number): string {
    return `${Math.round(score * 100)}%`;
  }

  $: report = $aiEnhancementStore.analysis as ImageAnalysisJson | null;
  $: imageVersionId = report?.image_version_id ?? "";

  async function onGenerateRecommendations(): Promise<void> {
    if (!imageVersionId) return;
    try {
      await generateRecommendationsFor({
        project_id: "",
        image_version_id: imageVersionId,
      });
    } catch (err) {
      // The store records the error; the rail renders it
      // inline via the existing error state.
      console.error("generateRecommendationsFor failed", err);
    }
  }
</script>

<section class="image-analysis-panel" aria-label="Image analysis">
  <header>
    <h3>Image Intelligence</h3>
    {#if report}
      <p class="meta">
        {report.width} × {report.height} · {report.channels} channels ·
        engine <code>{report.engine_version}</code>
      </p>
    {/if}
  </header>

  {#if $aiEnhancementStore.loading}
    <p class="status">Loading analysis…</p>
  {:else if $aiEnhancementStore.error}
    <p class="status error">{$aiEnhancementStore.error}</p>
  {:else if !report}
    <p class="status placeholder">
      No analysis yet. Run <code>analyze_image</code> on the
      active Image Version to populate the intelligence
      profile.
    </p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Observation</th>
          <th>Value</th>
          <th>Confidence</th>
        </tr>
      </thead>
      <tbody>
        {#each report.observations as obs (obs.name)}
          <tr>
            <td>
              <span class="name">{obs.name}</span>
              <span class="label">{obs.label}</span>
            </td>
            <td class="value">
              {Number.isInteger(obs.value) ? obs.value : obs.value.toFixed(3)}
            </td>
            <td class="confidence">
              <span class="bar" data-level={obs.confidence}>
                <span
                  class="bar-fill"
                  style:width="{confidenceScore(obs.confidence) * 100}%"
                ></span>
              </span>
              <span class="confidence-label">
                {obs.confidence} ·
                {confidenceLabel(confidenceScore(obs.confidence))}
              </span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>

    <aside class="rail">
      <div class="rail-header">
        <h4>Recommendations</h4>
        <button
          type="button"
          class="generate-button"
          on:click={onGenerateRecommendations}
          disabled={!imageVersionId || $aiEnhancementStore.loading}
        >
          Generate
        </button>
      </div>
      <RecommendationList />
    </aside>
  {/if}
</section>

<style>
  .image-analysis-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-sm);
  }
  header h3 {
    margin: 0 0 var(--sp-xs) 0;
  }
  .meta {
    margin: 0;
    font-size: 0.8rem;
    color: var(--on-surface-variant);
  }
  .meta code {
    font-size: 0.75rem;
  }
  .status {
    margin: 0;
    padding: var(--sp-md);
    border-radius: var(--radius-md);
    background: var(--surface-container);
    color: var(--on-surface-variant);
  }
  .status.error {
    color: var(--on-error-container);
    background: var(--error-container);
  }
  .status.placeholder {
    border-left: 3px solid var(--primary);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  thead th {
    text-align: left;
    padding: var(--sp-xs) var(--sp-sm);
    border-bottom: 1px solid var(--outline-variant);
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.7rem;
    font-weight: 500;
  }
  tbody tr {
    border-bottom: 1px solid var(--outline-variant);
  }
  tbody tr:last-child {
    border-bottom: none;
  }
  tbody td {
    padding: var(--sp-sm);
    vertical-align: middle;
  }
  .name {
    display: block;
    font-weight: 500;
  }
  .label {
    display: block;
    font-size: 0.75rem;
    color: var(--on-surface-variant);
  }
  .value {
    font-family: ui-monospace, "SF Mono", monospace;
    font-size: 0.8rem;
  }
  .confidence {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    min-width: 8rem;
  }
  .bar {
    flex: 1 1 auto;
    height: 6px;
    background: var(--surface-container-high);
    border-radius: 3px;
    overflow: hidden;
  }
  .bar-fill {
    display: block;
    height: 100%;
    background: var(--primary);
  }
  .bar[data-level="Low"] .bar-fill {
    background: var(--tertiary);
  }
  .bar[data-level="Medium"] .bar-fill {
    background: var(--secondary);
  }
  .bar[data-level="High"] .bar-fill {
    background: var(--primary);
  }
  .confidence-label {
    font-size: 0.7rem;
    color: var(--on-surface-variant);
    white-space: nowrap;
  }
  .rail {
    border-top: 1px solid var(--outline-variant);
    padding-top: var(--sp-md);
  }
  .rail h4 {
    margin: 0 0 var(--sp-xs) 0;
  }
  .rail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-sm);
    margin: 0 0 var(--sp-xs) 0;
  }
  .generate-button {
    padding: var(--sp-xs) var(--sp-sm);
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 0.85rem;
  }
  .generate-button:disabled {
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    cursor: not-allowed;
  }
  .placeholder {
    margin: 0;
    padding: var(--sp-sm);
    background: var(--surface-container);
    border-left: 3px solid var(--primary);
    border-radius: var(--radius-sm);
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }
</style>
