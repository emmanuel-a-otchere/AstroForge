<script lang="ts">
  import type { BayerPattern } from "../lib/ipc";

  export let onConfirm: (data: BayerPromptResult) => void;
  export let onCancel: () => void;

  let selectedTelescope = "";
  let selectedPattern: BayerPattern | null = null;
  let showPatternSelect = false;

  interface BayerPromptResult {
    telescope: string | null;
    pattern: BayerPattern | null;
  }

  const telescopes = [
    "Seestar S50",
    "Seestar S30",
    "Unistellar eVscope",
    "Vaonis Stellina",
    "Vaonis Vespera",
    "Dwarf II",
    "Other",
  ];

  const patterns: BayerPattern[] = ["RGGB", "BGGR", "GRBG", "GBRG"];

  function selectTelescope(name: string) {
    selectedTelescope = name;
    showPatternSelect = name === "Other";
    if (name !== "Other") {
      selectedPattern = "RGGB";
    }
  }

  function handleConfirm() {
    onConfirm({
      telescope: selectedTelescope || null,
      pattern: selectedPattern,
    });
  }
</script>

<div class="dialog-overlay">
  <div class="dialog">
    <h2>Raw Bayer Detection</h2>
    <p class="subtitle">
      This file may be raw Bayer data. What telescope are you using?
    </p>

    <div class="telescope-grid">
      {#each telescopes as scope}
        <button
          class="telescope-btn"
          class:selected={selectedTelescope === scope}
          on:click={() => selectTelescope(scope)}
        >
          {scope}
        </button>
      {/each}
    </div>

    {#if showPatternSelect}
      <div class="pattern-select">
        <p>Select Bayer pattern:</p>
        <div class="pattern-grid">
          {#each patterns as pattern}
            <button
              class="pattern-btn"
              class:selected={selectedPattern === pattern}
              on:click={() => (selectedPattern = pattern)}
            >
              {pattern}
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="actions">
      <button class="btn-secondary" on:click={onCancel}>Assume RGB</button>
      <button
        class="btn-primary"
        disabled={!selectedTelescope || (showPatternSelect && !selectedPattern)}
        on:click={handleConfirm}
      >
        Confirm
      </button>
    </div>
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    padding: 1.5rem;
    width: 480px;
    max-width: 90vw;
  }

  h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin-bottom: 0.25rem;
  }

  .subtitle {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 1.25rem;
  }

  .telescope-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .telescope-btn {
    padding: 0.625rem;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.875rem;
    transition: all 0.15s ease;
  }

  .telescope-btn:hover {
    border-color: var(--accent);
  }

  .telescope-btn.selected {
    background: var(--accent);
    color: var(--bg-primary);
    border-color: var(--accent);
  }

  .pattern-select {
    margin-bottom: 1rem;
  }

  .pattern-select p {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .pattern-grid {
    display: flex;
    gap: 0.5rem;
  }

  .pattern-btn {
    padding: 0.5rem 1rem;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.875rem;
    font-family: monospace;
  }

  .pattern-btn:hover {
    border-color: var(--accent);
  }

  .pattern-btn.selected {
    background: var(--accent);
    color: var(--bg-primary);
    border-color: var(--accent);
  }

  .actions {
    display: flex;
    justify-content: space-between;
    margin-top: 1.5rem;
  }

  .btn-primary,
  .btn-secondary {
    padding: 0.5rem 1.25rem;
    border-radius: 0.375rem;
    font-size: 0.9375rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn-primary {
    background: var(--accent);
    color: var(--bg-primary);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-dim);
  }

  .btn-primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }
</style>
