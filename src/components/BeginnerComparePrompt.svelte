<!--
  CR-07 §24 — Beginner comparison "Which do you prefer?" prompt.

  Renders a simple two-button preference question above
  the existing side-by-side comparison for beginner
  sessions. Each button shows the version's label and
  a 1-paragraph explanation of what choosing this
  version means. Clicking a button calls
  `applyImageDecision(versionId, "preferred")` via the
  existing IPC and collapses the prompt.

  Per CR-07 §24: "Side-by-side A vs B is simple by
  default. Missing: the 'Which do you prefer? [Natural]
  [AI Enhanced]' with one-paragraph explanation beneath
  each." This component ships that prompt.

  §21 invariant: the two options are equal-weight
  preference buttons, not a winner-pick. There is no
  AI ranking here. The user decides; the system
  records the decision.

  The prompt is always rendered when both versions are
  selected (this is the default view per the audit),
  but is collapsed once a preference is recorded so
  the existing comparison tools (which carry the
  decision into the broader workflow) become the
  primary surface.
-->
<script lang="ts">
  import { applyImageDecision } from "../lib/astroforge-api";
  import type { QualityProfile } from "../lib/astroforge-api";

  type Props = {
    /** ID of the "natural" version (typically the
     *  unprocessed baseline). */
    aId: string | null;
    /** Human label for the natural version (shown
     *  on the button). */
    aLabel: string;
    /** ID of the "AI enhanced" version (the version
     *  with the AI's processing applied). */
    bId: string | null;
    /** Human label for the AI enhanced version. */
    bLabel: string;
    /** Optional: the QualityProfile the user has
     *  selected in the picker. Threaded through to
     *  applyImageDecision so the decision row
     *  carries the user's intent (C-A3.5 fix). */
    selectedQualityProfile?: QualityProfile | null;
    /** Once a preference is recorded, this fires so
     *  the parent can refresh derived state. */
    onDecided?: () => void;
  };

  let {
    aId,
    aLabel,
    bId,
    bLabel,
    selectedQualityProfile = null,
    onDecided,
  }: Props = $props();

  // Collapsed after the user picks an option.
  let collapsed = $state(false);
  // Which side the user picked (only meaningful while
  // collapsed === true).
  let pickedSide = $state<"a" | "b" | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function pick(side: "a" | "b"): Promise<void> {
    const id = side === "a" ? aId : bId;
    if (!id) {
      error =
        side === "a"
          ? "Natural version is not selected."
          : "AI enhanced version is not selected.";
      return;
    }
    busy = true;
    error = null;
    try {
      await applyImageDecision(
        id,
        "preferred",
        undefined,
        selectedQualityProfile ?? undefined,
      );
      pickedSide = side;
      collapsed = true;
      if (onDecided) onDecided();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function expand(): void {
    collapsed = false;
    pickedSide = null;
    error = null;
  }
</script>

{#if aId && bId}
  <section
    class="beginner-prompt"
    aria-labelledby="beginner-heading"
  >
    <h2 id="beginner-heading" class="prompt-heading">
      Which do you prefer?
    </h2>

    {#if collapsed && pickedSide}
      <p class="collapsed">
        You picked
        <strong>
          {pickedSide === "a" ? aLabel : bLabel}
        </strong>
        as the preferred version. You can still change
        your mind below.
        <button
          type="button"
          class="change-mind"
          onclick={expand}
        >
          Change my pick
        </button>
      </p>
    {:else}
      <div class="prompt-grid">
        <button
          type="button"
          class="prompt-card"
          data-side="a"
          disabled={busy}
          onclick={() => pick("a")}
          aria-label={`Pick ${aLabel} as preferred`}
        >
          <span class="card-title">{aLabel}</span>
          <span class="card-tag">Natural</span>
          <p class="card-body">
            Keep what the original capture gives you.
            This is the version as the camera/sensor
            recorded it, with no AI processing applied.
            Best when you want the most faithful
            representation of the scene.
          </p>
        </button>
        <button
          type="button"
          class="prompt-card"
          data-side="b"
          disabled={busy}
          onclick={() => pick("b")}
          aria-label={`Pick ${bLabel} as preferred`}
        >
          <span class="card-title">{bLabel}</span>
          <span class="card-tag">AI Enhanced</span>
          <p class="card-body">
            Apply the AI's processing to bring out faint
            details, smooth noise, and balance the
            dynamic range. Best when the original capture
            looks dim, noisy, or flat and you want a
            punchier result.
          </p>
        </button>
      </div>
    {/if}

    {#if error}
      <p class="prompt-error" role="alert">
        Could not record your preference: {error}
      </p>
    {/if}
  </section>
{/if}

<style>
  .beginner-prompt {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    margin: 0 0 1rem 0;
    background: var(--surface-1, #1a1a1a);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: 6px;
  }

  .prompt-heading {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text, #ddd);
    font-family: var(--font-display, sans-serif);
  }

  .prompt-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  @media (max-width: 640px) {
    .prompt-grid {
      grid-template-columns: 1fr;
    }
  }

  .prompt-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.4rem;
    padding: 1rem;
    background: var(--surface-2, #222);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: 6px;
    color: var(--text, #ddd);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition:
      background 0.12s ease,
      border-color 0.12s ease,
      transform 0.08s ease;
  }

  .prompt-card:hover:not(:disabled) {
    background: var(--surface-3, #2a2a2a);
    border-color: var(--accent, #4a90e2);
  }

  .prompt-card:active:not(:disabled) {
    transform: translateY(1px);
  }

  .prompt-card:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .card-title {
    font-size: 1.05rem;
    font-weight: 600;
    font-family: var(--font-display, sans-serif);
  }

  .card-tag {
    display: inline-block;
    padding: 0.1rem 0.5rem;
    background: var(--accent-soft, #2a3a55);
    color: var(--accent, #4a90e2);
    border-radius: 3px;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .card-body {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-dim, #aaa);
    line-height: 1.5;
  }

  .collapsed {
    margin: 0;
    font-size: 0.9rem;
    color: var(--text-dim, #aaa);
  }

  .collapsed strong {
    color: var(--text, #ddd);
  }

  .change-mind {
    margin-left: 0.5rem;
    padding: 0.2rem 0.6rem;
    background: transparent;
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: 3px;
    color: var(--accent, #4a90e2);
    font-size: 0.8rem;
    font-family: inherit;
    cursor: pointer;
  }

  .change-mind:hover {
    background: var(--accent-soft, #2a3a55);
  }

  .prompt-error {
    margin: 0;
    color: #f87171;
    font-size: 0.85rem;
  }
</style>