<script lang="ts">
  export let histogram: number[] = [];
  export let onStretch: (shadows: number, highlights: number, midtones: number) => void;
  export let onRevert: () => void;

  let shadows = 0;
  let highlights = 1;
  let midtones = 0.25;

  let maxBin = Math.max(...histogram, 1);

  $: bars = histogram.map((count) => (count / maxBin) * 100);

  function handleStretch() {
    onStretch(shadows, highlights, midtones);
  }
</script>

<div class="dialog-overlay">
  <div class="dialog">
    <h2>Histogram</h2>

    <div class="histogram-container">
      <div class="bars">
        {#each bars as height, i}
          <div class="bar" style="height: {height}%"></div>
        {/each}
      </div>
    </div>

    <div class="controls">
      <div class="field">
        <label for="shadows">Shadows</label>
        <input id="shadows" type="range" min="0" max="1" step="0.01" bind:value={shadows} />
        <span class="value">{shadows.toFixed(2)}</span>
      </div>
      <div class="field">
        <label for="midtones">Midtones</label>
        <input id="midtones" type="range" min="0.01" max="0.99" step="0.01" bind:value={midtones} />
        <span class="value">{midtones.toFixed(2)}</span>
      </div>
      <div class="field">
        <label for="highlights">Highlights</label>
        <input id="highlights" type="range" min="0" max="1" step="0.01" bind:value={highlights} />
        <span class="value">{highlights.toFixed(2)}</span>
      </div>
    </div>

    <div class="actions">
      <button class="btn-secondary" on:click={onRevert}>Revert to Auto</button>
      <button class="btn-primary" on:click={handleStretch}>Apply Stretch</button>
    </div>
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-scrim);
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
    width: 600px;
    max-width: 95vw;
  }

  h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin-bottom: 1rem;
  }

  .histogram-container {
    height: 150px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    padding: 0.5rem;
    margin-bottom: 1.5rem;
  }

  .bars {
    display: flex;
    align-items: flex-end;
    height: 100%;
    gap: 1px;
  }

  .bar {
    flex: 1;
    background: var(--accent);
    min-height: 1px;
    border-radius: 1px 1px 0 0;
    transition: height 0.1s ease;
  }

  .controls {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .field label {
    display: block;
    font-size: 0.8125rem;
    color: var(--text-secondary);
    margin-bottom: 0.25rem;
  }

  .field .value {
    display: block;
    font-size: 0.875rem;
    color: var(--accent);
    margin-top: 0.25rem;
  }

  input[type="range"] {
    width: 100%;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
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

  .btn-primary:hover {
    background: var(--accent-dim);
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }
</style>
