<!--
  CR-06 P4 — Enhancement Operation Card.

  One card per operation in the AI Enhancement Studio's
  Zone C "Operations" rail (CR-06 §14). Renders:

    - operation display name
    - category + safety-classification chip
    - description (one-liner)
    - default parameters (rendered as a code block)
    - "Apply" button (calls the `onApply` callback with
      the JSON parameters string)

  The card does NOT own the apply state — the parent
  (EnhancementStudio) coordinates the apply round via
  `applyOperation` so the stack engine can refresh and
  Zone A's version timeline can update.
-->
<script lang="ts">
  import type { OperationRegistryEntryJson } from "../lib/astroforge-api";

  export let operation: OperationRegistryEntryJson;
  export let onApply: (parametersJson: string) => void = () => {};

  function safetyLabel(c: string): string {
    return c.charAt(0).toUpperCase() + c.slice(1);
  }

  function categoryLabel(c: string): string {
    return c.charAt(0).toUpperCase() + c.slice(1);
  }

  function prettyJson(raw: string): string {
    try {
      return JSON.stringify(JSON.parse(raw), null, 2);
    } catch {
      return raw;
    }
  }

  function handleApply(): void {
    onApply(operation.default_parameters_json);
  }
</script>

<article class="operation-card" aria-label={operation.display_name}>
  <header>
    <h4 class="op-title">{operation.display_name}</h4>
    <span class="op-id"><code>{operation.operation_id}</code></span>
  </header>
  <div class="badges">
    <span class="badge category">{categoryLabel(operation.category)}</span>
    <span class="badge safety-{operation.safety_classification}">
      {safetyLabel(operation.safety_classification)}
    </span>
    {#if operation.requires_region}
      <span class="badge region">Requires region</span>
    {/if}
  </div>
  <p class="description">{operation.description}</p>
  <details class="parameters">
    <summary>Default parameters</summary>
    <pre><code>{prettyJson(operation.default_parameters_json)}</code></pre>
  </details>
  <button type="button" class="apply" on:click={handleApply}>
    Apply
  </button>
</article>

<style>
  .operation-card {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.6rem 0.75rem;
    border: 1px solid var(--color-border, #2a2f3a);
    border-radius: 6px;
    background: var(--color-surface-muted, #1f242d);
  }
  .operation-card.generative {
    border-left: 3px solid var(--color-accent, #ff8a4f);
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .op-title {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }
  .op-id code {
    font-size: 0.7rem;
    color: var(--color-text-muted, #9aa3b2);
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .badge {
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    font-size: 0.7rem;
    font-weight: 500;
  }
  .badge.category {
    background: rgba(79, 156, 255, 0.15);
    color: #4f9cff;
  }
  .badge.safety-deterministic {
    background: rgba(80, 200, 120, 0.15);
    color: #50c878;
  }
  .badge.safety-perceptual {
    background: rgba(255, 195, 0, 0.15);
    color: #ffc300;
  }
  .badge.safety-generative {
    background: rgba(255, 138, 79, 0.2);
    color: #ff8a4f;
  }
  .badge.region {
    background: rgba(160, 160, 160, 0.15);
    color: #a0a0a0;
  }
  .description {
    margin: 0;
    font-size: 0.8rem;
    line-height: 1.3;
    color: var(--color-text-muted, #c0c5d0);
  }
  .parameters summary {
    cursor: pointer;
    font-size: 0.75rem;
    color: var(--color-text-muted, #9aa3b2);
  }
  .parameters pre {
    margin: 0.3rem 0 0 0;
    padding: 0.3rem 0.5rem;
    background: var(--color-surface, #15181f);
    border-radius: 3px;
    font-size: 0.7rem;
    overflow-x: auto;
  }
  .apply {
    align-self: flex-start;
    background: var(--color-accent, #4f9cff);
    color: var(--color-on-accent, #fff);
    border: none;
    border-radius: 4px;
    padding: 0.25rem 0.6rem;
    font-size: 0.8rem;
    cursor: pointer;
  }
</style>