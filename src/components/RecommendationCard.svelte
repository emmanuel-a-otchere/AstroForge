<!--
  CR-03 P5 — AI recommendation card.

  One card per pending recommendation. Each card shows:
    - Kind + title
    - What it would change (description)
    - Why the AI suggests it (rationale)
    - Accept / Reject buttons

  Accept/Reject update the versionStore optimistically. The real
  IPC that creates a new ImageVersion on accept lands in P5a.
-->
<script lang="ts">
  import {
    versionStore,
    type AiRecommendation,
    type RecommendationStatus,
  } from "../state/versions";

  let { recommendation }: { recommendation: AiRecommendation } = $props();

  function handleDecision(status: RecommendationStatus) {
    versionStore.applyRecommendationStatus(recommendation.id, status);
  }

  function kindLabel(kind: AiRecommendation["kind"]): string {
    switch (kind) {
      case "tone_adjustment": return "Tone";
      case "star_reduction": return "Stars";
      case "color_balance": return "Color";
      case "noise_reduction": return "Noise";
      case "stretch_recommendation": return "Stretch";
    }
  }
</script>

<article class="rec-card" data-status={recommendation.status}>
  <header class="rec-header">
    <span class="kind-pill font-label">{kindLabel(recommendation.kind)}</span>
    <h3 class="rec-title font-display">{recommendation.title}</h3>
    <span class="status-pill" data-status={recommendation.status}>
      {recommendation.status}
    </span>
  </header>

  <section class="rec-body">
    <p class="rec-section-label font-label">What this changes</p>
    <p class="rec-description font-body">{recommendation.description}</p>

    <p class="rec-section-label font-label">Why</p>
    <p class="rec-rationale font-body">{recommendation.rationale}</p>
  </section>

  {#if recommendation.status === "pending"}
    <footer class="rec-actions">
      <button
        type="button"
        class="reject-cta font-display"
        onclick={() => handleDecision("rejected")}
      >
        Reject
      </button>
      <button
        type="button"
        class="accept-cta font-display"
        onclick={() => handleDecision("accepted")}
      >
        Accept
      </button>
    </footer>
  {:else}
    <footer class="rec-decided">
      <p class="decided-message font-body">
        {recommendation.status === "accepted"
          ? "Accepted — applied as v{n} of the project."
          : recommendation.status === "rejected"
            ? "Rejected — no changes made."
            : "Superseded by a later version."}
      </p>
    </footer>
  {/if}
</article>

<style>
  .rec-card {
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    padding: var(--sp-lg);
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .rec-card[data-status="accepted"] {
    border-color: rgba(111, 191, 115, 0.4);
  }

  .rec-card[data-status="rejected"] {
    border-color: rgba(229, 57, 53, 0.4);
    opacity: 0.7;
  }

  .rec-header {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--sp-sm);
  }

  .kind-pill {
    font-size: 0.7rem;
    padding: 2px var(--sp-sm);
    border-radius: var(--radius-full);
    background: var(--primary-container);
    color: var(--on-primary-container);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .rec-title {
    margin: 0;
    font-size: 1.05rem;
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

  .status-pill[data-status="accepted"] {
    background: rgba(111, 191, 115, 0.2);
    color: #6fbf73;
  }

  .status-pill[data-status="rejected"] {
    background: rgba(229, 57, 53, 0.2);
    color: #ff8a80;
  }

  .rec-body {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .rec-section-label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--on-surface-variant);
    margin: 0;
  }

  .rec-description,
  .rec-rationale {
    margin: 0;
    font-size: 0.9rem;
    color: var(--on-surface);
    line-height: 1.45;
  }

  .rec-actions {
    display: flex;
    gap: var(--sp-sm);
    justify-content: flex-end;
  }

  .accept-cta {
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    padding: var(--sp-sm) var(--sp-lg);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .accept-cta:hover {
    background: var(--primary-container);
    color: var(--on-primary-container);
  }

  .reject-cta {
    background: transparent;
    color: var(--on-surface-variant);
    border: 1px solid var(--outline-variant);
    padding: var(--sp-sm) var(--sp-lg);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.9rem;
  }

  .reject-cta:hover {
    color: #ff8a80;
    border-color: #ff8a80;
  }

  .decided-message {
    font-size: 0.85rem;
    color: var(--on-surface-variant);
    margin: 0;
    font-style: italic;
  }
</style>