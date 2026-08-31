<script lang="ts">
  import type { AnalysisResult } from "../lib/image-analysis";
  import type { TargetType, Verbosity } from "../lib/ipc";

  export let onConfirm: (data: InitialDialogData) => void;
  export let onCancel: () => void;
  export let analysis: AnalysisResult | null = null;

  let targetName = "";
  let cameraType: "osc" | "smart_telescope" = "smart_telescope";
  let focalLength = "";
  let lightsOnly = false;
  let includeDithering = false;
  let detectedObjectType: "deep_sky" | "planetary" | "lunar" | "" = "";
  let userOverrodeObjectType = false;

  interface InitialDialogData {
    targetName: string;
    cameraType: string;
    focalLength: number | null;
    lightsOnly: boolean;
    includeDithering: boolean;
    objectType: string;
  }

  $: if (analysis && !focalLength && analysis.focalLength) {
    focalLength = String(analysis.focalLength);
  }
  $: if (analysis && !userOverrodeObjectType && analysis.detectedObjectType) {
    detectedObjectType = analysis.detectedObjectType;
  }

  function handleSubmit() {
    onConfirm({
      targetName: targetName.trim(),
      cameraType,
      focalLength: focalLength ? parseFloat(focalLength) : null,
      lightsOnly,
      includeDithering,
      objectType: detectedObjectType,
    });
  }

  function handleObjectTypeChange() {
    userOverrodeObjectType = true;
  }

  const objectTypeLabels: Record<string, string> = {
    deep_sky: "Deep-sky (nebula, galaxy)",
    planetary: "Planetary (planet)",
    lunar: "Lunar (Moon)",
  };
</script>

<div class="dialog-overlay">
  <div class="dialog">
    <h2>What did you shoot?</h2>
    <p class="subtitle">Tell AstroForge about your session so it can pick the best defaults.</p>

    {#if analysis}
      <div class="analysis-banner">
        <div class="analysis-title">Auto-detected from your files</div>
        <div class="analysis-grid">
          {#if analysis.focalLength}
            <div class="analysis-item">
              <span class="analysis-label">Focal length</span>
              <span class="analysis-value">{analysis.focalLength} mm</span>
              <span class="analysis-source">{analysis.focalLengthSource}</span>
            </div>
          {/if}
          {#if analysis.detectedObjectType}
            <div class="analysis-item">
              <span class="analysis-label">Object type</span>
              <span class="analysis-value">{objectTypeLabels[analysis.detectedObjectType] ?? analysis.detectedObjectType}</span>
              <span class="analysis-source confidence-{analysis.detectionConfidence}">
                {analysis.detectionReason}
              </span>
            </div>
          {/if}
          <div class="analysis-item">
            <span class="analysis-label">Frames</span>
            <span class="analysis-value">{analysis.totalFrames} total</span>
            <span class="analysis-source">
              {analysis.lightCount} light · {analysis.darkCount} dark · {analysis.flatCount} flat · {analysis.biasCount} bias
            </span>
          </div>
          {#if analysis.avgExposure !== null}
            <div class="analysis-item">
              <span class="analysis-label">Avg exposure</span>
              <span class="analysis-value">{analysis.avgExposure.toFixed(1)}s</span>
              <span class="analysis-source">from {analysis.lightCount} light frame{analysis.lightCount !== 1 ? "s" : ""}</span>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <form on:submit|preventDefault={handleSubmit}>
      <div class="field">
        <label for="target">Target name (optional)</label>
        <input id="target" type="text" bind:value={targetName} placeholder="e.g. Orion Nebula" />
      </div>

      <div class="field">
        <span class="field-label">Object type</span>
        <div class="radio-group">
          {#each Object.entries(objectTypeLabels) as [value, label]}
            <label class="radio">
              <input
                type="radio"
                bind:group={detectedObjectType}
                value={value}
                on:change={handleObjectTypeChange}
              />
              <span>{label}</span>
            </label>
          {/each}
        </div>
        {#if analysis?.detectedObjectType && !userOverrodeObjectType}
          <p class="auto-filled-note">Auto-detected — change if this doesn't look right.</p>
        {/if}
      </div>

      <div class="field">
        <span class="field-label">Camera type</span>
        <div class="radio-group">
          <label class="radio">
            <input type="radio" bind:group={cameraType} value="smart_telescope" />
            <span>Smart telescope</span>
          </label>
          <label class="radio">
            <input type="radio" bind:group={cameraType} value="osc" />
            <span>OSC / DSLR</span>
          </label>
        </div>
      </div>

      <div class="field">
        <label for="focal">Telescope focal length (mm)</label>
        <input id="focal" type="number" bind:value={focalLength} placeholder="e.g. 250" min="0" />
        {#if analysis?.focalLength}
          <p class="auto-filled-note">Auto-detected from {analysis.focalLengthSource.toLowerCase()} — edit if needed.</p>
        {/if}
      </div>

      <div class="field-row">
        <label class="checkbox">
          <input type="checkbox" bind:checked={lightsOnly} />
          <span>I only have lights (skip calibration)</span>
        </label>
      </div>

      <div class="field-row">
        <label class="checkbox">
          <input type="checkbox" bind:checked={includeDithering} />
          <span>Include dithering info</span>
        </label>
      </div>

      <div class="actions">
        <button type="button" class="btn-secondary" on:click={onCancel}>Cancel</button>
        <button type="submit" class="btn-primary">Continue</button>
      </div>
    </form>
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
    padding: 2rem;
    width: 520px;
    max-width: 90vw;
    max-height: 90vh;
    overflow-y: auto;
  }

  h2 {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
  }

  .subtitle {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 1.5rem;
  }

  .analysis-banner {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 1rem;
    margin-bottom: 1.5rem;
  }

  .analysis-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 0.75rem;
  }

  .analysis-grid {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .analysis-item {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .analysis-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .analysis-value {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .analysis-source {
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .analysis-source.confidence-high {
    color: var(--success);
  }

  .analysis-source.confidence-medium {
    color: var(--warning);
  }

  .analysis-source.confidence-low {
    color: var(--text-muted);
  }

  .auto-filled-note {
    font-size: 0.75rem;
    color: var(--accent);
    margin-top: 0.25rem;
    margin-bottom: 0;
  }

  .field {
    margin-bottom: 1.25rem;
  }

  .field label,
  .field .field-label,
  .field-row label {
    display: block;
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 0.375rem;
  }

  input[type="text"],
  input[type="number"] {
    width: 100%;
    padding: 0.5rem 0.75rem;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    color: var(--text-primary);
    font-size: 0.9375rem;
    outline: none;
  }

  input[type="text"]:focus,
  input[type="number"]:focus {
    border-color: var(--accent);
  }

  .radio-group {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem 1.5rem;
  }

  .radio {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    cursor: pointer;
    margin-bottom: 0;
  }

  .radio span {
    font-size: 0.9375rem;
    color: var(--text-primary);
  }

  .checkbox {
    display: flex !important;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .checkbox span {
    font-size: 0.9375rem;
    color: var(--text-primary);
  }

  .field-row {
    margin-bottom: 1rem;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
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

  .btn-primary:hover {
    background: var(--accent-dim);
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }

  .btn-secondary:hover {
    background: var(--border);
  }
</style>
