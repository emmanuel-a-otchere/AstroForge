<script lang="ts">
  /**
   * CR-05 P6.2 — §23 AI Boundary card.
   *
   * Renders the §23 example badge when the most-recent stage
   * crosses the AI boundary:
   *
   *   AI Enhancement
   *   Model: AstroForge Detail v1.2
   *   Type: Perceptual enhancement
   *   Deterministic: Yes
   *   Seed: N/A
   *
   * For non-AI stages (`uses_ai = false` or label absent), the
   * card renders nothing — the §23 acceptance is that AI stages
   * carry their provenance badge, not that classical stages show a
   * "not AI" line.
   *
   * Honest disclosure: this card never *adds* AI provenance that
   * isn't on the row. If `latest_ai_label` is null (pre-P6.2 data
   * or unparseable JSON), the card renders nothing — same UX as a
   * stage without AI provenance. We never fabricate the badge.
   */
  import type {
    AiBoundaryLabelDto,
    ProcessingMetricsDto,
  } from "../lib/astroforge-api";

  export let metrics: ProcessingMetricsDto | null;

  function formatSeed(seed: number | null): string {
    if (seed === null) return "N/A";
    return String(seed);
  }

  function formatDeterministic(deterministic: boolean): string {
    return deterministic ? "Yes" : "No";
  }

  function typeBadge(label: AiBoundaryLabelDto): string {
    // §23 example wording is "Perceptual enhancement". Today every
    // AI stage is perceptual; this branch is the single point to
    // update when CR-06 introduces new AI kinds ("Generative", etc.).
    if (!label.uses_ai) return "Deterministic processing";
    return "Perceptual enhancement";
  }
</script>

{#if metrics?.latest_ai_label?.uses_ai}
  <aside class="ai-boundary-card" data-testid="ai-boundary-card">
    <header>
      <strong class="ai-heading">AI Enhancement</strong>
    </header>
    <dl>
      <dt>Model</dt>
      <dd>{metrics.latest_ai_label.model_id ?? "Unknown"}</dd>

      <dt>Type</dt>
      <dd>{typeBadge(metrics.latest_ai_label)}</dd>

      <dt>Deterministic</dt>
      <dd>{formatDeterministic(metrics.latest_ai_label.deterministic)}</dd>

      <dt>Seed</dt>
      <dd>{formatSeed(metrics.latest_ai_label.seed)}</dd>
    </dl>
    <!-- Per §23 — anticipate CR-06. -->
    <p class="ai-warning">
      This operation may alter structures beyond conventional image processing.
    </p>
  </aside>
{/if}

<style>
  .ai-boundary-card {
    border: 1px solid var(--color-ai-border, #5b3aa8);
    background: var(--color-ai-bg, #f4ecff);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    margin: 0.75rem 0;
    color: var(--color-ai-text, #2a1450);
  }

  .ai-heading {
    color: var(--color-ai-heading, #5b3aa8);
    font-size: 0.95rem;
    letter-spacing: 0.02em;
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    column-gap: 0.75rem;
    row-gap: 0.25rem;
    margin: 0.4rem 0;
    font-size: 0.9rem;
  }

  dt {
    font-weight: 600;
    color: var(--color-ai-heading, #5b3aa8);
  }

  dd {
    margin: 0;
  }

  .ai-warning {
    margin: 0.4rem 0 0 0;
    font-size: 0.85rem;
    font-style: italic;
    color: var(--color-ai-warning, #5b3aa8);
  }
</style>
