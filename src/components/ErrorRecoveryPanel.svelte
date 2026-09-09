<script lang="ts">
  /**
   * CR-05 P6.1 — §28 Error and Recovery UX.
   *
   * Renders the four-question template from CR-05 §28:
   *   1. What happened?       → header + `what_happened` body
   *   2. What was preserved?  → `what_was_preserved`
   *   3. What can AstroForge do next? → suggested actions
   *   4. What can the user do?         → same actions, re-stated
   *
   * Buttons emit semantic events via `on:retryOptimized`,
   * `on:retry`, `on:adjustProcessing`, `on:skipStage`, `on:cancel`,
   * `on:contactSupport`. Consumers (ProcessingControls) decide what
   * each action means at runtime — the panel itself is dumb.
   *
   * Honest disclosure: the panel never re-derives or edits the
   * error message. The Rust runner (P6.1) populates `StageError`
   * with classification logic; this component is the presentational
   * mirror.
   */
  import type { StageErrorDto, SuggestedActionDto } from "../lib/astroforge-api";

  export let error: StageErrorDto;
  /** Optional stage label for the header line, e.g. "Stack".
   *  Falls back to the first word of `what_happened`. */
  export let stageLabel: string | null = null;

  function dispatchAction(action: SuggestedActionDto) {
    switch (action.kind) {
      case "retry_optimized":
        // The actual retry with smaller tiles is implemented in a
        // future slice (P6.1b). For now we surface the click so the
        // UI knows the user picked it.
        dispatchEvent(new CustomEvent("retryOptimized", { detail: action }));
        break;
      case "retry":
        dispatchEvent(new CustomEvent("retry", { detail: action }));
        break;
      case "adjust_processing":
        dispatchEvent(new CustomEvent("adjustProcessing", { detail: action }));
        break;
      case "skip_stage":
        dispatchEvent(new CustomEvent("skipStage", { detail: action }));
        break;
      case "cancel":
        dispatchEvent(new CustomEvent("cancel", { detail: action }));
        break;
      case "contact_support":
        dispatchEvent(new CustomEvent("contactSupport", { detail: action }));
        break;
    }
  }

  function headerTitle(): string {
    if (stageLabel) return `${stageLabel} could not complete`;
    // Derive header from the leading clause of `what_happened`,
    // which is the human stage label followed by ":".
    const colon = error.what_happened.indexOf(":");
    if (colon > 0 && colon < 40) {
      return `${error.what_happened.slice(0, colon)} could not complete`;
    }
    return "Processing could not complete";
  }

  function primaryRecommendation(): string {
    if (error.suggested_actions.length === 0) {
      return "";
    }
    const first = error.suggested_actions[0];
    return `Recommended: ${first.label.toLowerCase()}.`;
  }

  function isPrimary(kind: SuggestedActionDto["kind"]): boolean {
    // Primary = the action we want the user to take first.
    // Per §28 example, "Retry Optimized" is the recommended action
    // for the OOM case. For other cases, the first action in the
    // list is primary by convention.
    return kind === "retry_optimized";
  }
</script>

<aside class="error-recovery-panel" data-testid="error-recovery-panel">
  <header>
    <h3>{headerTitle()}</h3>
  </header>

  <dl class="error-recovery-grid">
    <dt>What was preserved</dt>
    <dd>{error.what_was_preserved}</dd>

    <dt>What happened</dt>
    <dd>{error.what_happened}</dd>

    <dt>Recommended</dt>
    <dd>
      {#if error.suggested_actions.length > 0}
        {primaryRecommendation()}
      {:else}
        AstroForge has no automated next step for this failure.
      {/if}
    </dd>
  </dl>

  {#if error.suggested_actions.length > 0}
    <div class="actions">
      {#each error.suggested_actions as action}
        <button
          type="button"
          class="action"
          class:primary={isPrimary(action.kind)}
          data-action-kind={action.kind}
          on:click={() => dispatchAction(action)}
        >
          {action.label}
        </button>
      {/each}
    </div>
  {/if}
</aside>

<style>
  .error-recovery-panel {
    border: 1px solid var(--color-error-border, #c33);
    background: var(--color-error-bg, #fbeaea);
    border-radius: 6px;
    padding: 1rem 1.25rem;
    margin: 0.75rem 0;
    color: var(--color-error-text, #400);
  }

  .error-recovery-panel header h3 {
    margin: 0 0 0.5rem 0;
    color: var(--color-error-heading, #800);
    font-size: 1rem;
    font-weight: 600;
  }

  .error-recovery-grid {
    display: grid;
    grid-template-columns: max-content 1fr;
    column-gap: 0.75rem;
    row-gap: 0.4rem;
    margin: 0.5rem 0 0.75rem 0;
    font-size: 0.92rem;
  }

  .error-recovery-grid dt {
    font-weight: 600;
    color: var(--color-error-heading, #800);
  }

  .error-recovery-grid dd {
    margin: 0;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .action {
    appearance: none;
    border: 1px solid var(--color-error-border, #c33);
    background: white;
    color: var(--color-error-heading, #800);
    padding: 0.4rem 0.85rem;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-weight: 500;
  }

  .action:hover {
    background: var(--color-error-bg-hover, #f5d5d5);
  }

  .action.primary {
    background: var(--color-error-primary-bg, #800);
    color: white;
    border-color: var(--color-error-primary-bg, #800);
  }

  .action.primary:hover {
    background: var(--color-error-primary-bg-hover, #600);
    border-color: var(--color-error-primary-bg-hover, #600);
  }
</style>
