<!--
  CR-07 B4 — decision panel (§17 + §18).

  Renders the B1 `ImageDecision` state machine for one Image
  Version and lets the user drive it:

      Working → Candidate → Preferred → Final
                    ↘ Rejected (from any non-terminal state)

  The backend (`apply_image_decision`) is the authoritative
  validator — the panel only offers the transitions the state
  machine permits (`can_promote_to`), mirrors terminal-state
  honesty, and surfaces backend rejections verbatim. Every
  transition carries an optional reason that lands in the
  append-only history (ADR-07.4: no prior state is rewritten).
-->
<script lang="ts">
  import { comparisonState } from "../state/comparison";
  import type { ImageDecisionState } from "../lib/astroforge-api";

  interface Props {
    versionId: string;
    versionLabel: string;
  }
  const { versionId, versionLabel }: Props = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);
  let reason = $state("");

  // Load the persisted decision once per version selection.
  // `ensureDecision` is idempotent; versions with no row yet
  // render the implicit `working` default below.
  $effect(() => {
    versionId;
    error = null;
    reason = "";
    void comparisonState.ensureDecision(versionId);
  });

  const decision = $derived($comparisonState.decisions[versionId] ?? null);
  const currentState = $derived<ImageDecisionState>(decision?.state ?? "working");

  const STATE_LABEL: Record<ImageDecisionState, string> = {
    working: "Working",
    candidate: "Candidate",
    preferred: "Preferred",
    final: "Final",
    rejected: "Rejected",
    reference: "Reference",
  };

  /** Mirror of `ImageDecisionState::next()` — the only promote
   *  target the state machine permits from each state. */
  const NEXT: Partial<Record<ImageDecisionState, ImageDecisionState>> = {
    working: "candidate",
    candidate: "preferred",
    preferred: "final",
  };

  const nextState = $derived(NEXT[currentState] ?? null);
  const terminal = $derived(
    currentState === "final" ||
      currentState === "rejected" ||
      currentState === "reference",
  );

  async function apply(target: ImageDecisionState) {
    busy = true;
    error = null;
    try {
      await comparisonState.applyDecision(
        versionId,
        target,
        reason.trim() || undefined,
      );
      reason = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function formatDate(iso: string): string {
    if (!iso) return "—";
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }
</script>

<section class="decision-panel" aria-label={`Decision for ${versionLabel}`}>
  <header class="panel-header">
    <span class="panel-tag font-label">Decision</span>
    <h3 class="panel-title font-display">{versionLabel}</h3>
    <span class="state-pill" data-state={currentState}>{STATE_LABEL[currentState]}</span>
  </header>

  {#if !decision}
    <p class="note font-body">
      No decision recorded yet. The version is implicitly
      <strong>Working</strong>; the first transition below creates
      the durable record.
    </p>
  {/if}

  {#if !terminal}
    <div class="actions">
      {#if nextState}
        <button
          type="button"
          class="cta primary"
          disabled={busy}
          onclick={() => apply(nextState)}
        >
          <span class="material-symbols-outlined" aria-hidden="true">
            arrow_upward
          </span>
          Promote to {STATE_LABEL[nextState]}
        </button>
      {/if}
      <button
        type="button"
        class="cta danger"
        disabled={busy}
        onclick={() => apply("rejected")}
      >
        <span class="material-symbols-outlined" aria-hidden="true">block</span>
        Reject
      </button>
    </div>
    <input
      class="reason-input font-body"
      type="text"
      placeholder="Reason (optional, recorded in history)"
      bind:value={reason}
      disabled={busy}
      aria-label="Decision reason"
    />
  {:else}
    <p class="note font-body">
      <span class="material-symbols-outlined" aria-hidden="true">lock</span>
      {STATE_LABEL[currentState]} is a terminal state — no further transitions
      (CR-07 §17).
    </p>
  {/if}

  {#if error}
    <p class="error font-body" role="alert">{error}</p>
  {/if}

  {#if decision && decision.history.length > 0}
    <details class="history">
      <summary class="font-label">
        History ({decision.history.length})
      </summary>
      <ol class="history-list">
        {#each [...decision.history].reverse() as entry (entry.at + entry.to)}
          <li class="history-entry">
            <span class="history-transition font-body">
              {entry.from ? STATE_LABEL[entry.from] : "—"} →
              {STATE_LABEL[entry.to]}
            </span>
            <span class="history-at font-body">{formatDate(entry.at)}</span>
            {#if entry.reason}
              <span class="history-reason font-body">{entry.reason}</span>
            {/if}
          </li>
        {/each}
      </ol>
    </details>
  {/if}
</section>

<style>
  .decision-panel {
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

  .state-pill {
    display: inline-block;
    padding: 2px var(--sp-xs);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .state-pill[data-state="working"] {
    background: rgba(158, 158, 158, 0.2);
    color: #b0bec5;
  }
  .state-pill[data-state="candidate"] {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
  }
  .state-pill[data-state="preferred"] {
    background: rgba(156, 39, 176, 0.2);
    color: #ba68c8;
  }
  .state-pill[data-state="final"] {
    background: rgba(76, 175, 80, 0.2);
    color: #81c784;
  }
  .state-pill[data-state="rejected"] {
    background: rgba(229, 57, 53, 0.2);
    color: #e57373;
  }
  .state-pill[data-state="reference"] {
    background: rgba(255, 179, 0, 0.2);
    color: #ffb300;
  }

  .actions {
    display: flex;
    gap: var(--sp-sm);
    flex-wrap: wrap;
  }

  .cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--outline-variant);
    cursor: pointer;
    font-family: inherit;
    font-size: 0.85rem;
  }

  .cta:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .cta.primary {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--on-primary);
  }

  .cta.danger {
    background: transparent;
    color: #e57373;
    border-color: rgba(229, 57, 53, 0.4);
  }

  .cta.danger:hover:not(:disabled) {
    background: rgba(229, 57, 53, 0.12);
  }

  .reason-input {
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface);
    font-size: 0.85rem;
    font-family: inherit;
  }

  .reason-input:focus {
    outline: none;
    border-color: var(--primary);
  }

  .note {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
  }

  .error {
    margin: 0;
    color: #e57373;
    font-size: 0.85rem;
  }

  .history summary {
    cursor: pointer;
    color: var(--on-surface-variant);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .history-list {
    margin: var(--sp-sm) 0 0;
    padding-left: var(--sp-lg);
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .history-entry {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .history-transition {
    color: var(--on-surface);
    font-size: 0.85rem;
  }

  .history-at {
    color: var(--on-surface-variant);
    font-size: 0.75rem;
  }

  .history-reason {
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    font-style: italic;
  }
</style>
