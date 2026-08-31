<script lang="ts">
  export let beforeUrl: string = "";
  export let afterUrl: string = "";
  export let metrics: Record<string, number> = {};
  export let onRevertToAuto: () => void = () => {};
  export let onSavePreset: () => void = () => {};

  let sliderPos = 50;

  function handleSlider(e: Event) {
    const target = e.target as HTMLInputElement;
    sliderPos = parseFloat(target.value);
  }
</script>

<div class="compare-container">
  <div class="images">
    <div class="image-before">
      <img src={beforeUrl} alt="Before" />
    </div>
    <div class="image-after" style="clip-path: inset(0 0 0 {sliderPos}%)">
      <img src={afterUrl} alt="After" />
    </div>
    <div class="slider-handle" style="left: {sliderPos}%">
      <div class="handle-line"></div>
      <div class="handle-knob">↔</div>
    </div>
    <input
      type="range"
      min="0"
      max="100"
      step="1"
      value={sliderPos}
      on:input={handleSlider}
      class="slider-input"
    />
  </div>

  {#if Object.keys(metrics).length > 0}
    <div class="metrics">
      <h3>Quality Metrics</h3>
      <div class="metric-grid">
        {#each Object.entries(metrics) as [key, value]}
          <div class="metric">
            <span class="metric-label">{key}</span>
            <span class="metric-value">{value.toFixed(2)}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <div class="actions">
    <button class="btn-secondary" on:click={onRevertToAuto}>Revert to Auto</button>
    <button class="btn-secondary" on:click={onSavePreset}>Save as Preset</button>
  </div>
</div>

<style>
  .compare-container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .images {
    position: relative;
    width: 100%;
    height: 300px;
    overflow: hidden;
    border-radius: 0.5rem;
    background: var(--bg-primary);
  }

  .image-before,
  .image-after {
    position: absolute;
    inset: 0;
  }

  .image-before img,
  .image-after img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .image-after {
    transition: clip-path 0.05s linear;
  }

  .slider-handle {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--accent);
    pointer-events: none;
    transform: translateX(-50%);
  }

  .handle-line {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 2px;
    background: var(--accent);
  }

  .handle-knob {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--accent);
    color: var(--bg-primary);
    border-radius: 50%;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
    font-weight: bold;
  }

  .slider-input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: ew-resize;
  }

  .metrics h3 {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .metric-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 0.5rem;
  }

  .metric {
    background: var(--bg-tertiary);
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    display: flex;
    flex-direction: column;
  }

  .metric-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .metric-value {
    font-size: 1rem;
    font-weight: 600;
    color: var(--accent);
  }

  .actions {
    display: flex;
    gap: 0.75rem;
  }

  .btn-secondary {
    padding: 0.5rem 1rem;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.875rem;
  }

  .btn-secondary:hover {
    background: var(--border);
  }
</style>
