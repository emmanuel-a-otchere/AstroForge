<!--
  CR-07 B7: Comparison Scope picker (CR-07 §7).

  Three scopes per the CR:
    - Whole image (default; the existing global comparison)
    - Selected region (the user draws a rectangle on the canvas)
    - Specific feature (semantic features AstroForge recognises;
      for B7 we ship a fixed feature list; feature-aware detectors
      land in a later slice)

  When the user picks "Selected region" and clicks "Draw", the
  parent enables drawMode on the canvases; shift-drag (or click+drag
  in draw mode) captures a rectangle, which the parent then renders
  as a region overlay on both A and B.
-->
<script lang="ts">
  export type RegionScope = "whole" | "selected" | "feature";
  export type RegionRect = {
    x0: number;
    y0: number;
    x1: number;
    y1: number;
  };

  interface Props {
    scope: RegionScope;
    onScopeChange: (scope: RegionScope) => void;
    drawMode: boolean;
    onDrawToggle: () => void;
    onClearRegion: () => void;
    hasRegion: boolean;
    selectedFeature: string | null;
    onFeatureChange: (feature: string | null) => void;
  }
  const {
    scope,
    onScopeChange,
    drawMode,
    onDrawToggle,
    onClearRegion,
    hasRegion,
    selectedFeature,
    onFeatureChange,
  }: Props = $props();

  const FEATURES: ReadonlyArray<{ id: string; label: string }> = [
    { id: "nebula", label: "Nebula" },
    { id: "stars", label: "Stars" },
    { id: "background", label: "Background" },
    { id: "galaxy_core", label: "Galaxy core" },
  ];

  const SCOPE_LABEL: Record<RegionScope, string> = {
    whole: "Whole image",
    selected: "Selected region",
    feature: "Specific feature",
  };
</script>

<section class="region-picker" aria-label="Comparison scope">
  <header class="picker-header">
    <span class="picker-tag font-label">Scope</span>
    <h3 class="picker-title font-display">Comparison area</h3>
  </header>

  <div class="scope-radios" role="radiogroup" aria-label="Comparison scope">
    {#each (["whole", "selected", "feature"] as RegionScope[]) as s (s)}
      <label class="scope-option" data-active={scope === s}>
        <input
          type="radio"
          name="region-scope"
          value={s}
          checked={scope === s}
          onchange={() => onScopeChange(s)}
        />
        <span class="scope-option-label font-body">{SCOPE_LABEL[s]}</span>
      </label>
    {/each}
  </div>

  {#if scope === "selected"}
    <div class="scope-controls">
      <button
        type="button"
        class="cta primary"
        onclick={onDrawToggle}
        aria-pressed={drawMode}
      >
        <span class="material-symbols-outlined" aria-hidden="true">
          {drawMode ? "close" : "crop"}
        </span>
        {drawMode ? "Cancel draw" : "Draw region"}
      </button>
      <button
        type="button"
        class="cta"
        onclick={onClearRegion}
        disabled={!hasRegion}
      >
        <span class="material-symbols-outlined" aria-hidden="true">backspace</span>
        Clear region
      </button>
    </div>
    {#if drawMode && !hasRegion}
      <p class="hint font-body">
        Drag a rectangle on either canvas. Shift-drag also works.
      </p>
    {:else if hasRegion}
      <p class="hint font-body">
        Region captured. Click "Draw region" to redraw.
      </p>
    {/if}
  {:else if scope === "feature"}
    <div class="feature-picker">
      <label class="feature-label font-label" for="feature-pick">
        AstroForge feature
      </label>
      <select
        id="feature-pick"
        class="feature-select font-body"
        value={selectedFeature ?? ""}
        onchange={(ev) => {
          const v = (ev.target as HTMLSelectElement).value;
          onFeatureChange(v === "" ? null : v);
        }}
      >
        <option value="">Pick a feature…</option>
        {#each FEATURES as f (f.id)}
          <option value={f.id}>{f.label}</option>
        {/each}
      </select>
      <p class="hint font-body">
        Feature-scoped comparison uses semantic masks where
        available; falls back to whole-image metrics otherwise.
      </p>
    </div>
  {/if}
</section>

<style>
  .region-picker {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    padding: var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
  }

  .picker-header {
    display: flex;
    align-items: baseline;
    gap: var(--sp-sm);
  }

  .picker-tag {
    font-size: 0.7rem;
    color: var(--primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .picker-title {
    margin: 0;
    font-size: 1rem;
  }

  .scope-radios {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-xs);
  }

  .scope-option {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: var(--sp-xs) var(--sp-sm);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    cursor: pointer;
    background: var(--surface-container-low);
    color: var(--on-surface);
    font-size: 0.85rem;
  }

  .scope-option input {
    margin: 0;
  }

  .scope-option[data-active="true"] {
    border-color: var(--primary);
    background: rgba(74, 144, 255, 0.12);
  }

  .scope-controls {
    display: flex;
    gap: var(--sp-sm);
    flex-wrap: wrap;
  }

  .feature-picker {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .feature-label {
    font-size: 0.7rem;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .feature-select {
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface);
    font-size: 0.85rem;
    font-family: inherit;
  }

  .feature-select:focus {
    outline: none;
    border-color: var(--primary);
  }

  .cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--outline-variant);
    background: transparent;
    color: var(--on-surface);
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

  .hint {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.8rem;
  }
</style>