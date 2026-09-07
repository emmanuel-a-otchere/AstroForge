<!--
  CR-05 P3 slice 2.5 — IntelligencePanel component.

  Renders every recommendation emitted by the engine for the
  current plan as a card grid, with Apply / Dismiss / Reset
  controls per card. Per CR-05 §14 the engine's picks are pure
  functions of `QualityMetricSnapshot`; this panel is the
  consumption surface that lets the user act on them.

  Slice 2.5 ships apply + dismiss + reset (Option A — full UX).
  Apply merges the recommendation's `parameters` into the next
  pipeline stage's `parameters_json` (Tauri command does the
  merge server-side). Dismiss flips the row to `dismissed`
  without touching stage parameters. Reset returns the row to
  `pending` so the user can re-apply.

  Wiring ProcessWorkspace to mount this component is a separate
  P3 slice 2.6 PR (the wizard path stays untouched for slice
  2.5, mirroring how P2.6 slice 1 landed AutoPlanPanel first
  and wired ProcessWorkspace later).

  Props:
    planId — required to scope `recommendationsForPlan` and to
             pass back to apply/dismiss/reset so the store can
             patch the row in place.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import {
    applyRecommendationFor,
    dismissRecommendationFor,
    lastError,
    recommendationsForPlan,
    refreshRecommendations,
    resetRecommendationFor,
  } from "../lib/pipeline-plan-store";
  import type { RecommendationDto } from "../lib/astroforge-api";

  export let planId: string;

  // CR-05 P3 slice 2.5 — track which card is mid-mutation so we
  // can disable the buttons and show a spinner. Keyed by
  // recommendation id.
  let pendingAction: Record<string, "apply" | "dismiss" | "reset"> = {};

  onMount(() => {
    refreshRecommendations(planId);
  });

  $: recs = $recommendationsForPlan[planId] ?? [];

  function confidencePercent(r: RecommendationDto): string {
    // CR-05 P3 slice 2 — confidence is a 0.0..1.0 heuristic.
    return `${Math.round(r.confidence * 100)}%`;
  }

  function rationaleLabel(r: RecommendationDto): string {
    return r.decision_json.rationale || "—";
  }

  function parametersSummary(r: RecommendationDto): string {
    const params = r.decision_json.parameters ?? {};
    const keys = Object.keys(params);
    if (keys.length === 0) return "no parameter changes";
    const parts = keys
      .sort()
      .map((k) => `${k}=${JSON.stringify(params[k])}`);
    return parts.join(", ");
  }

  function isApplied(r: RecommendationDto): boolean {
    return r.user_decision === "applied";
  }
  function isDismissed(r: RecommendationDto): boolean {
    return r.user_decision === "dismissed";
  }
  function isPending(r: RecommendationDto): boolean {
    return !isApplied(r) && !isDismissed(r);
  }

  async function onApply(r: RecommendationDto) {
    pendingAction = { ...pendingAction, [r.id]: "apply" };
    try {
      await applyRecommendationFor(planId, r.id);
    } finally {
      const { [r.id]: _drop, ...rest } = pendingAction;
      pendingAction = rest;
    }
  }

  async function onDismiss(r: RecommendationDto) {
    pendingAction = { ...pendingAction, [r.id]: "dismiss" };
    try {
      await dismissRecommendationFor(planId, r.id);
    } finally {
      const { [r.id]: _drop, ...rest } = pendingAction;
      pendingAction = rest;
    }
  }

  async function onReset(r: RecommendationDto) {
    pendingAction = { ...pendingAction, [r.id]: "reset" };
    try {
      await resetRecommendationFor(planId, r.id);
    } finally {
      const { [r.id]: _drop, ...rest } = pendingAction;
      pendingAction = rest;
    }
  }
</script>

<section class="intelligence-panel" aria-label="Recommendation intelligence panel">
  <header class="panel-header">
    <h2 class="font-display">Intelligence</h2>
    <p class="font-body panel-description">
      Per-stage picks emitted by the recommendation engine. Apply merges the
      recommended parameters into the next pipeline stage; Dismiss suppresses
      the recommendation without changing stage parameters. Reset returns the
      row to <em>pending</em> for re-apply.
    </p>
  </header>

  {#if recs.length === 0}
    <p class="empty font-body" role="status">
      No recommendations yet. Recommendations appear after a stage executes and
      produces a quality metric snapshot.
    </p>
  {:else}
    <ul class="rec-list" role="list">
      {#each recs as r (r.id)}
        {@const applied = isApplied(r)}
        {@const dismissed = isDismissed(r)}
        {@const pending = isPending(r)}
        {@const busy = pendingAction[r.id] !== undefined}
        <li class="rec-card" class:applied class:dismissed data-status={r.user_decision ?? "pending"}>
          <header class="rec-header">
            <span class="rule-id font-mono" title={r.rule_id}>{r.rule_id}</span>
            <span class="stage-type font-mono" title={`stage_type=${r.stage_type}`}>
              {r.stage_type}
            </span>
            <span class="confidence font-mono" title="engine confidence">
              {confidencePercent(r)}
            </span>
          </header>

          <p class="rationale font-body">{rationaleLabel(r)}</p>

          <p class="parameters font-mono" title="parameters that apply will write">
            {parametersSummary(r)}
          </p>

          <p class="evidence font-mono" title="evidence summary">
            <span class="evidence-label">evidence:</span> {r.evidence_summary}
          </p>

          <footer class="rec-footer">
            <span class="status font-mono" data-status={r.user_decision ?? "pending"}>
              {applied ? "applied" : dismissed ? "dismissed" : "pending"}
              {#if r.user_decision_at}
                <span class="at font-body"> · {r.user_decision_at}</span>
              {/if}
              {#if r.applied_stage_id}
                <span class="applied-to font-body">
                  · wrote to <span class="font-mono">{r.applied_stage_id}</span>
                </span>
              {/if}
            </span>

            <div class="actions">
              {#if pending || dismissed}
                <button
                  type="button"
                  class="btn apply"
                  on:click={() => onApply(r)}
                  disabled={busy}
                  aria-label={`Apply recommendation ${r.rule_id}`}
                >
                  {busy && pendingAction[r.id] === "apply" ? "Applying…" : "Apply"}
                </button>
              {/if}
              {#if pending || applied}
                <button
                  type="button"
                  class="btn dismiss"
                  on:click={() => onDismiss(r)}
                  disabled={busy}
                  aria-label={`Dismiss recommendation ${r.rule_id}`}
                >
                  {busy && pendingAction[r.id] === "dismiss" ? "Dismissing…" : "Dismiss"}
                </button>
              {/if}
              {#if applied || dismissed}
                <button
                  type="button"
                  class="btn reset"
                  on:click={() => onReset(r)}
                  disabled={busy}
                  aria-label={`Reset recommendation ${r.rule_id}`}
                >
                  {busy && pendingAction[r.id] === "reset" ? "Resetting…" : "Reset"}
                </button>
              {/if}
            </div>
          </footer>
        </li>
      {/each}
    </ul>
  {/if}

  {#if $lastError}
    <p class="error font-body" role="alert">{$lastError}</p>
  {/if}
</section>

<style>
  .intelligence-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-lg);
    background: var(--surface-container);
    border-radius: var(--radius-md);
  }

  .panel-header h2 {
    margin: 0 0 var(--sp-xs);
    font-size: 1.15rem;
  }
  .panel-description {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }

  .empty {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.9rem;
  }

  .rec-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .rec-card {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border-left: 4px solid var(--outline);
    border-radius: var(--radius-sm);
  }
  .rec-card.applied {
    border-left-color: var(--primary);
  }
  .rec-card.dismissed {
    border-left-color: var(--on-surface-variant);
    opacity: 0.75;
  }

  .rec-header {
    display: flex;
    gap: var(--sp-md);
    align-items: baseline;
    font-size: 0.85rem;
  }
  .rule-id {
    font-weight: 600;
    color: var(--on-surface);
  }
  .stage-type {
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .confidence {
    margin-left: auto;
    color: var(--primary);
  }

  .rationale {
    margin: 0;
    color: var(--on-surface);
    font-size: 0.95rem;
  }
  .parameters {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    overflow-wrap: anywhere;
  }
  .evidence {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.75rem;
  }
  .evidence-label {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-right: var(--sp-xs);
  }

  .rec-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: var(--sp-xs);
    gap: var(--sp-sm);
  }
  .status {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .status[data-status="applied"] {
    color: var(--primary);
  }
  .status[data-status="dismissed"] {
    color: var(--on-surface-variant);
  }
  .at,
  .applied-to {
    text-transform: none;
    letter-spacing: 0;
    color: var(--on-surface-variant);
  }

  .actions {
    display: flex;
    gap: var(--sp-xs);
  }
  .btn {
    border: 1px solid var(--outline);
    background: var(--surface-container-high);
    color: var(--on-surface);
    border-radius: var(--radius-full);
    padding: var(--sp-xs) var(--sp-md);
    font-size: 0.85rem;
    cursor: pointer;
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .btn.apply {
    background: var(--primary);
    color: var(--on-primary);
    border-color: transparent;
  }

  .error {
    color: #ff8a80;
    background: rgba(229, 57, 53, 0.12);
    padding: var(--sp-xs) var(--sp-sm);
    border-radius: var(--radius-sm);
  }
</style>