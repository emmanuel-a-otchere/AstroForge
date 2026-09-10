<!--
  CR-06 P3 — Recommendation List.

  Renders the structured recommendations emitted by the
  AI recommendation engine (CR-06 §7 / §27) for the
  currently-loaded Image Version. Each row shows:

    - the operation name (denoise_luminance, etc.)
    - a plain-English reason (the rationale the engine
      produced, with the observation that triggered it)
    - confidence badge (Low / Medium / High) derived
      from the recommendation's confidence score
    - risk level badge (low / medium / high)
    - safety classification chip (Deterministic /
      Perceptual / Generative) — Generative rows get
      an accent border so the user notices the
      fabrication caveat
    - expected effect
    - sequencing note (when the engine moved a row to
      satisfy the §23 ordering rules)
    - model candidates + recommended model
    - estimated runtime + estimated memory

  Empty state: when the user has not yet run
  `analyze_image`, the list shows a hint to run the
  analyzer. When the analyzer ran but produced no
  recommendations, the list shows a "No recommendations"
  message.

  The data flow: `RecommendationList` reads from
  `aiEnhancementStore.recommendations` (an array of
  `RecommendationJson`). The `loadAiEnhancementFor`
  helper in `ai-enhancement.ts` populates both
  `analysis` and `recommendations` from the IPC layer;
  P3 adds the `generateAiRecommendations` call so the
  recommendation array is populated after analysis.
-->
<script lang="ts">
  import { aiEnhancementStore } from "../state/ai-enhancement";

  type Confidence = "Low" | "Medium" | "High";
  type SafetyClassification = "deterministic" | "perceptual" | "generative";

  interface ModelCandidateJson {
    name: string;
    version: string;
    sha256: string;
    tile_size: number;
  }

  interface RecommendationJson {
    operation: string;
    reason: string;
    confidence: number;
    evidence: string[];
    affected_regions: string[];
    expected_effect: string;
    risk_level: string;
    model_candidates: ModelCandidateJson[];
    recommended_model: string | null;
    estimated_runtime: string | null;
    estimated_memory: string | null;
    classification: SafetyClassification;
  }

  interface SequencingNoteJson {
    moved: string;
    before_after: string;
    note: string;
  }

  interface RecommendationReportJson {
    project_id: string;
    image_version_id: string;
    recommendations: RecommendationJson[];
    sequencing_notes: SequencingNoteJson[];
    engine_version: string;
    created_at: string;
  }

  function confidenceToBadge(score: number): Confidence {
    if (score >= 0.85) return "High";
    if (score >= 0.6) return "Medium";
    return "Low";
  }

  function confidencePercent(score: number): number {
    return Math.round(score * 100);
  }

  function operationLabel(op: string): string {
    // Convert snake_case to a readable title; "denoise_luminance" → "Denoise Luminance".
    return op
      .split("_")
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(" ");
  }

  function safetyLabel(c: SafetyClassification): string {
    switch (c) {
      case "deterministic":
        return "Deterministic";
      case "perceptual":
        return "Perceptual";
      case "generative":
        return "Generative";
    }
  }

  function safetyRequiresDisclosure(c: SafetyClassification): boolean {
    // Per CR-06 §16, Perceptual and Generative operations
    // surface a banner so the user understands the trust
    // implications.
    return c !== "deterministic";
  }

  $: report = $aiEnhancementStore.recommendationsReport as RecommendationReportJson | null;
  $: items = (report?.recommendations ?? []) as RecommendationJson[];
  $: notes = (report?.sequencing_notes ?? []) as SequencingNoteJson[];
</script>

<section class="recommendation-list" aria-label="AI recommendations">
  <header>
    <h3>Recommended Enhancements</h3>
    {#if report}
      <p class="meta">
        engine <code>{report.engine_version}</code> · generated
        <code>{report.created_at}</code>
      </p>
    {/if}
  </header>

  {#if !report}
    <p class="empty">
      No recommendations yet. Analyze the image first, then the recommendation
      engine will propose an enhancement sequence.
    </p>
  {:else if items.length === 0}
    <p class="empty">
      The analyzer found no characteristics that warrant an enhancement. The
      image is in good shape as-is.
    </p>
  {:else}
    {#if notes.length > 0}
      <ul class="sequencing-notes" aria-label="Sequencing adjustments">
        {#each notes as note}
          <li>
            <strong>{operationLabel(note.moved)}</strong>: {note.note}
          </li>
        {/each}
      </ul>
    {/if}
    <ol class="recommendations" aria-label="Recommendations">
      {#each items as rec, index}
        {@const conf = confidenceToBadge(rec.confidence)}
        {@const disclosure = safetyRequiresDisclosure(rec.classification)}
        <li
          class="recommendation-card"
          class:generative={rec.classification === "generative"}
        >
          <div class="row top">
            <span class="step-number">{index + 1}</span>
            <span class="operation">{operationLabel(rec.operation)}</span>
            <span class="confidence-badge confidence-{conf.toLowerCase()}">
              {conf} · {confidencePercent(rec.confidence)}%
            </span>
            <span class="risk-badge risk-{rec.risk_level.toLowerCase()}">
              {rec.risk_level} risk
            </span>
            <span class="safety-badge safety-{rec.classification}">
              {safetyLabel(rec.classification)}
            </span>
          </div>

          {#if disclosure}
            <p class="disclosure">
              This operation is <strong>{rec.classification}</strong> — the
              result is not a literal measurement of the original scene. The
              enhancement modifies pixel values and may smooth or synthesise
              detail.
            </p>
          {/if}

          <p class="reason">{rec.reason}</p>

          {#if rec.expected_effect}
            <p class="expected">
              <span class="label">Expected:</span>
              {rec.expected_effect}
            </p>
          {/if}

          {#if rec.evidence.length > 0}
            <p class="evidence">
              <span class="label">Evidence:</span>
              {rec.evidence.map(operationLabel).join(", ")}
            </p>
          {/if}

          <div class="row bottom">
            {#if rec.recommended_model}
              <span class="model">
                Model:
                <code>{rec.recommended_model}</code>
                {#if rec.model_candidates.length > 1}
                  (+{rec.model_candidates.length - 1} alternatives)
                {/if}
              </span>
            {/if}
            {#if rec.estimated_runtime}
              <span class="resource">≈ {rec.estimated_runtime}</span>
            {/if}
            {#if rec.estimated_memory}
              <span class="resource">{rec.estimated_memory}</span>
            {/if}
          </div>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .recommendation-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem;
  }
  .meta {
    margin: 0.25rem 0 0 0;
    color: var(--color-text-muted, #9aa3b2);
    font-size: 0.85em;
  }
  .empty {
    padding: 1rem;
    border: 1px dashed var(--color-border, #2a2f3a);
    border-radius: 6px;
    color: var(--color-text-muted, #9aa3b2);
    margin: 0;
  }
  .sequencing-notes {
    list-style: none;
    padding: 0.75rem 1rem;
    margin: 0;
    border-left: 3px solid var(--color-accent, #4f9cff);
    background: var(--color-surface-muted, rgba(79, 156, 255, 0.08));
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    font-size: 0.9em;
  }
  .recommendations {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    counter-reset: recommendation;
  }
  .recommendation-card {
    padding: 0.75rem 1rem;
    border: 1px solid var(--color-border, #2a2f3a);
    border-radius: 6px;
    background: var(--color-surface, #15181f);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .recommendation-card.generative {
    border-left: 3px solid var(--color-accent, #ff8a4f);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }
  .row.bottom {
    font-size: 0.85em;
    color: var(--color-text-muted, #9aa3b2);
  }
  .step-number {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: 50%;
    background: var(--color-surface-muted, #1f242d);
    color: var(--color-text, #e6e9ef);
    font-size: 0.85em;
    font-weight: 600;
  }
  .operation {
    font-weight: 600;
    font-size: 1.05em;
  }
  .confidence-badge,
  .risk-badge,
  .safety-badge {
    padding: 0.15rem 0.5rem;
    border-radius: 4px;
    font-size: 0.8em;
    font-weight: 500;
  }
  .confidence-high {
    background: rgba(80, 200, 120, 0.15);
    color: #50c878;
  }
  .confidence-medium {
    background: rgba(255, 195, 0, 0.15);
    color: #ffc300;
  }
  .confidence-low {
    background: rgba(160, 160, 160, 0.15);
    color: #a0a0a0;
  }
  .risk-low {
    background: rgba(80, 200, 120, 0.15);
    color: #50c878;
  }
  .risk-medium {
    background: rgba(255, 195, 0, 0.15);
    color: #ffc300;
  }
  .risk-high {
    background: rgba(255, 100, 100, 0.15);
    color: #ff6464;
  }
  .safety-deterministic {
    background: rgba(80, 200, 120, 0.15);
    color: #50c878;
  }
  .safety-perceptual {
    background: rgba(255, 195, 0, 0.15);
    color: #ffc300;
  }
  .safety-generative {
    background: rgba(255, 138, 79, 0.2);
    color: #ff8a4f;
  }
  .disclosure {
    margin: 0;
    padding: 0.4rem 0.6rem;
    background: var(--color-surface-muted, #1f242d);
    border-left: 2px solid var(--color-accent, #4f9cff);
    border-radius: 3px;
    font-size: 0.85em;
    color: var(--color-text-muted, #c0c5d0);
  }
  .reason,
  .expected,
  .evidence {
    margin: 0;
    font-size: 0.9em;
    line-height: 1.4;
  }
  .label {
    font-weight: 600;
    color: var(--color-text-muted, #9aa3b2);
    margin-right: 0.25rem;
  }
  .model code {
    background: var(--color-surface-muted, #1f242d);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
    font-size: 0.9em;
  }
  .resource {
    padding: 0.15rem 0.4rem;
    background: var(--color-surface-muted, #1f242d);
    border-radius: 3px;
    font-family: ui-monospace, monospace;
  }
</style>