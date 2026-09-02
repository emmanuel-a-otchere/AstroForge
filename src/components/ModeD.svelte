<!--
  ModeD — "Refine" layout. Full-screen screen-card on full-intensity
  canvas backdrop. Used when the user is doing detailed image refinement
  — stretch, denoise, histogram, color, sharpen. The image IS the screen.

  Tabs at the top of the card. Single column of sliders per tab. Right
  side of the card shows the image preview (placeholder gradient for now;
  will become PreviewCanvas in Phase 6 alignment).
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import ScreenCard from "./ScreenCard.svelte";
  import { galleryStore, type GalleryItem } from "../lib/gallery";

  let { children }: { children?: Snippet } = $props();

  let activeItem: GalleryItem | null = $state(
    $galleryStore.find((it: GalleryItem) => it.status === "completed") ?? $galleryStore[0] ?? null
  );
  let activeTab: "stretch" | "denoise" | "histogram" | "color" | "sharpen" =
    $state("stretch");

  // Mock parameter values per tab
  const params = {
    stretch: { blackPoint: 0.02, midtones: 0.45, highlights: 0.98, saturation: 1.1 },
    denoise: { strength: 0.4, luminanceOnly: true, edgePreserve: 0.7 },
    histogram: { mode: "linear", clipping: 0.001 },
    color: { r: 1.0, g: 0.98, b: 1.02, sat: 1.05 },
    sharpen: { amount: 0.5, radius: 1.2, threshold: 0.05 },
  };

  function setItem(item: GalleryItem) {
    activeItem = item;
  }
</script>

<div class="mode-d">
  {#if activeItem}
    <ScreenCard
      kicker="Refine · {activeItem.target}"
      title="Image Refinement"
      variant="overlay"
      maxWidth="1100px"
    >
      <div class="refine-layout">
        <aside class="refine-tabs" aria-label="Refinement panels">
          <button
            type="button"
            class="refine-tab"
            class:active={activeTab === "stretch"}
            onclick={() => (activeTab = "stretch")}
          >
            <span class="material-symbols-outlined">auto_awesome_motion</span>
            Stretch
          </button>
          <button
            type="button"
            class="refine-tab"
            class:active={activeTab === "denoise"}
            onclick={() => (activeTab = "denoise")}
          >
            <span class="material-symbols-outlined">blur_on</span>
            Denoise
          </button>
          <button
            type="button"
            class="refine-tab"
            class:active={activeTab === "histogram"}
            onclick={() => (activeTab = "histogram")}
          >
            <span class="material-symbols-outlined">bar_chart</span>
            Histogram
          </button>
          <button
            type="button"
            class="refine-tab"
            class:active={activeTab === "color"}
            onclick={() => (activeTab = "color")}
          >
            <span class="material-symbols-outlined">palette</span>
            Color
          </button>
          <button
            type="button"
            class="refine-tab"
            class:active={activeTab === "sharpen"}
            onclick={() => (activeTab = "sharpen")}
          >
            <span class="material-symbols-outlined">center_focus_strong</span>
            Sharpen
          </button>
        </aside>

        <section class="refine-controls">
          {#if activeTab === "stretch"}
            <div class="control-grid">
              <div class="control">
                <label class="control-label">Black point</label>
                <input type="range" min="0" max="100" value={params.stretch.blackPoint * 100} />
                <span class="control-value">{params.stretch.blackPoint}</span>
              </div>
              <div class="control">
                <label class="control-label">Midtones</label>
                <input type="range" min="0" max="100" value={params.stretch.midtones * 100} />
                <span class="control-value">{params.stretch.midtones}</span>
              </div>
              <div class="control">
                <label class="control-label">Highlights</label>
                <input type="range" min="0" max="100" value={params.stretch.highlights * 100} />
                <span class="control-value">{params.stretch.highlights}</span>
              </div>
              <div class="control">
                <label class="control-label">Saturation</label>
                <input type="range" min="0" max="200" value={params.stretch.saturation * 100} />
                <span class="control-value">{params.stretch.saturation}</span>
              </div>
            </div>
          {:else if activeTab === "denoise"}
            <div class="control-grid">
              <div class="control">
                <label class="control-label">Strength</label>
                <input type="range" min="0" max="100" value={params.denoise.strength * 100} />
                <span class="control-value">{params.denoise.strength}</span>
              </div>
              <div class="control">
                <label class="control-label">Edge preservation</label>
                <input type="range" min="0" max="100" value={params.denoise.edgePreserve * 100} />
                <span class="control-value">{params.denoise.edgePreserve}</span>
              </div>
              <div class="control toggle">
                <label class="control-label">Luminance only</label>
                <input type="checkbox" checked={params.denoise.luminanceOnly} />
              </div>
            </div>
          {:else if activeTab === "histogram"}
            <div class="histogram-view">
              <svg viewBox="0 0 400 120" preserveAspectRatio="none" class="histogram-svg" aria-label="Histogram">
                <defs>
                  <linearGradient id="hist-grad" x1="0" x2="0" y1="0" y2="1">
                    <stop offset="0" stop-color="var(--primary)" stop-opacity="0.6" />
                    <stop offset="1" stop-color="var(--primary)" stop-opacity="0.05" />
                  </linearGradient>
                </defs>
                {#each Array(48) as _, i}
                  {@const x = (i / 48) * 400}
                  {@const y = 120 - (Math.sin(i * 0.3) * 35 + Math.cos(i * 0.7) * 22 + 60)}
                  {@const h = 120 - y}
                  <rect {x} {y} width={400 / 48 - 1} height={h} fill="url(#hist-grad)" />
                {/each}
              </svg>
              <div class="histogram-labels">
                <span>0%</span>
                <span>50%</span>
                <span>100%</span>
              </div>
            </div>
          {:else if activeTab === "color"}
            <div class="control-grid">
              <div class="control">
                <label class="control-label">Red gain</label>
                <input type="range" min="0" max="200" value={params.color.r * 100} />
                <span class="control-value">{params.color.r}</span>
              </div>
              <div class="control">
                <label class="control-label">Green gain</label>
                <input type="range" min="0" max="200" value={params.color.g * 100} />
                <span class="control-value">{params.color.g}</span>
              </div>
              <div class="control">
                <label class="control-label">Blue gain</label>
                <input type="range" min="0" max="200" value={params.color.b * 100} />
                <span class="control-value">{params.color.b}</span>
              </div>
              <div class="control">
                <label class="control-label">Saturation</label>
                <input type="range" min="0" max="200" value={params.color.sat * 100} />
                <span class="control-value">{params.color.sat}</span>
              </div>
            </div>
          {:else if activeTab === "sharpen"}
            <div class="control-grid">
              <div class="control">
                <label class="control-label">Amount</label>
                <input type="range" min="0" max="100" value={params.sharpen.amount * 100} />
                <span class="control-value">{params.sharpen.amount}</span>
              </div>
              <div class="control">
                <label class="control-label">Radius</label>
                <input type="range" min="0" max="50" value={params.sharpen.radius * 10} />
                <span class="control-value">{params.sharpen.radius}</span>
              </div>
              <div class="control">
                <label class="control-label">Threshold</label>
                <input type="range" min="0" max="100" value={params.sharpen.threshold * 100} />
                <span class="control-value">{params.sharpen.threshold}</span>
              </div>
            </div>
          {/if}
        </section>

        <section class="refine-preview">
          <div class="preview-placeholder" aria-label="Image preview">
            <span class="material-symbols-outlined preview-icon">image</span>
            <div class="preview-label">{activeItem.target}</div>
            <span class="preview-meta">{activeItem.integrationHours}h · {activeItem.palette}</span>
          </div>
        </section>
      </div>

      {#snippet footer()}
      <div class="refine-actions">
        <button class="btn-secondary" type="button">Reset</button>
        <button class="btn-secondary" type="button">Compare</button>
        <button class="btn-primary" type="button">Apply</button>
      </div>
      {/snippet}
    </ScreenCard>
  {:else}
    <div class="mode-d-empty">No completed sessions to refine yet.</div>
  {/if}

  <aside class="refine-sessions">
    <div class="refine-sessions-label label-caps">Pick session</div>
    {#each $galleryStore.filter((it: GalleryItem) => it.status === "completed") as item}
      <button
        type="button"
        class="refine-session-chip"
        class:active={activeItem?.id === item.id}
        onclick={() => setItem(item)}
      >{item.target}</button>
    {/each}
  </aside>
</div>

<style>
  .mode-d {
    flex: 1;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    gap: var(--sp-md);
    padding: var(--sp-lg);
    overflow-y: auto;
    min-height: 0;
  }

  .mode-d :global(.screen-card) {
    width: 100%;
    max-width: 1100px;
  }

  .mode-d-empty {
    color: var(--on-surface);
    padding: var(--sp-xl);
    text-align: center;
    align-self: center;
  }

  .refine-layout {
    display: grid;
    grid-template-columns: 160px 1fr 320px;
    gap: var(--sp-md);
    min-height: 380px;
  }

  .refine-tabs {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
    align-self: start;
  }

  .refine-tab {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: 10px 12px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--on-surface-variant);
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    cursor: pointer;
    text-align: left;
    transition: color var(--transition-base),
      background-color var(--transition-base);
  }

  .refine-tab .material-symbols-outlined {
    font-size: 18px;
  }

  .refine-tab:hover {
    color: var(--on-surface);
    background: var(--surface-container-high);
  }

  .refine-tab.active {
    background: var(--cobalt-accent);
    color: var(--on-primary);
  }

  .refine-controls {
    padding: var(--sp-sm);
    background: var(--surface-container-lowest);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
  }

  .control-grid {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .control {
    display: grid;
    grid-template-columns: 160px 1fr 60px;
    align-items: center;
    gap: var(--sp-md);
  }

  .control.toggle {
    display: flex;
    justify-content: space-between;
    grid-template-columns: none;
  }

  .control-label {
    color: var(--on-surface);
    font-family: var(--font-body);
  }

  .control input[type="range"] {
    accent-color: var(--cobalt-accent);
  }

  .control-value {
    font-family: var(--font-data);
    color: var(--on-surface);
    text-align: right;
  }

  .control input[type="checkbox"] {
    accent-color: var(--cobalt-accent);
    width: 18px;
    height: 18px;
  }

  .histogram-view {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .histogram-svg {
    width: 100%;
    height: 200px;
    background: var(--surface-container);
    border-radius: var(--radius-sm);
  }

  .histogram-labels {
    display: flex;
    justify-content: space-between;
    color: var(--on-surface-variant);
    font-family: var(--font-data);
    font-size: var(--text-metadata);
  }

  .refine-preview {
    background: var(--surface-container-lowest);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
    padding: var(--sp-md);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .preview-placeholder {
    width: 100%;
    aspect-ratio: 3 / 2;
    background:
      radial-gradient(circle at 25% 30%, var(--tile-glow-warm-strong), transparent 50%),
      radial-gradient(circle at 80% 70%, var(--tile-glow-cool-strong), transparent 50%),
      var(--surface-container-high);
    border-radius: var(--radius-sm);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    color: var(--on-surface);
  }

  .preview-icon {
    font-size: 48px;
    opacity: 0.4;
    color: var(--on-surface-variant);
  }

  .preview-label {
    font-family: var(--font-display);
    font-size: var(--text-body);
    font-weight: 600;
  }

  .preview-meta {
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    color: var(--on-surface);
  }

  .refine-actions {
    display: flex;
    gap: var(--sp-sm);
  }

  .btn-primary,
  .btn-secondary {
    padding: 8px 16px;
    border-radius: var(--radius-md);
    font-family: var(--font-body);
    font-size: var(--text-body);
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn-primary {
    background: var(--cobalt-accent);
    color: var(--surface);
  }

  .btn-primary:hover {
    background: var(--primary-container);
  }

  .btn-secondary {
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
  }

  .btn-secondary:hover {
    border-color: var(--cobalt-accent);
  }

  .refine-sessions {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
    align-self: stretch;
    min-width: 160px;
  }

  .refine-sessions-label {
    color: var(--primary);
    font-family: var(--font-data);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    margin-bottom: 4px;
  }

  .refine-session-chip {
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
    padding: 8px 12px;
    color: var(--on-surface);
    font-family: var(--font-body);
    cursor: pointer;
    text-align: left;
    transition: border-color var(--transition-fast);
  }

  .refine-session-chip:hover {
    border-color: var(--cobalt-accent);
  }

  .refine-session-chip.active {
    border-color: var(--cobalt-accent);
    background: var(--overlay-card);
  }
</style>