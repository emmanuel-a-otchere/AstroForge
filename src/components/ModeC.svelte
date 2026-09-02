<!--
  ModeC — "Automagic Pro" layout. Gallery strip across the top + tabbed
  screen-cards below. Used when the user is configuring or tuning an
  automated pipeline run, with AI assistance. The AI surfaces context
  alongside the user's adjustments.

  Top: Gallery (2-column grid) showing the sessions the AI is working on.
  Bottom: Tabbed screen-card with Tuning / Stages / Output tabs.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import Gallery from "./Gallery.svelte";
  import ScreenCard from "./ScreenCard.svelte";
  import { galleryStore, type GalleryItem } from "../lib/gallery";

  let { children }: { children?: Snippet } = $props();

  let activeItem: GalleryItem | null = $state($galleryStore[2] ?? null);
  let activeTab: "tuning" | "stages" | "output" = $state("tuning");

  function pickItem(item: GalleryItem) {
    activeItem = item;
  }

  // Mock tuning values (would come from session store in Phase 4+)
  const tuningParams = {
    snr_target: 42,
    noise_reduction: 0.65,
    star_detection_sensitivity: 0.8,
    color_balance: "neutral",
    background_extraction: true,
    auto_crop: true,
  };

  const pipelineStages = [
    { name: "Calibration", status: "done", duration: "0:48" },
    { name: "Registration", status: "done", duration: "1:23" },
    { name: "Stacking", status: "active", duration: "4:12" },
    { name: "Stretch", status: "queued", duration: "—" },
    { name: "Color Calibration", status: "queued", duration: "—" },
    { name: "Denoise", status: "queued", duration: "—" },
    { name: "Output", status: "queued", duration: "—" },
  ];

  const outputPreview = {
    format: "FITS 16-bit",
    resolution: "6248 × 4176",
    fileSize: "124.6 MB",
    location: "~/AstroForge/output/M42-2026-09-01/",
  };
</script>

<div class="mode-c">
  <Gallery columns={2} activeItemId={activeItem?.id} onSelect={pickItem} />

  <main class="mode-c-body">
    {#if activeItem}
      <header class="mode-c-header">
        <div>
          <div class="mode-c-kicker label-caps">Automagic Pro</div>
          <h2 class="mode-c-title">{activeItem.target}</h2>
          <div class="mode-c-subtitle">{activeItem.name} · {activeItem.integrationHours}h · {activeItem.palette}</div>
        </div>
        <div class="mode-c-status">
          <span class="status-pill status-processing">STAGE 3 / 7</span>
        </div>
      </header>

      <nav class="mode-c-tabs" aria-label="Automagic Pro panel">
        <button
          type="button"
          class="mode-c-tab"
          class:active={activeTab === "tuning"}
          onclick={() => (activeTab = "tuning")}
        >Tuning</button>
        <button
          type="button"
          class="mode-c-tab"
          class:active={activeTab === "stages"}
          onclick={() => (activeTab = "stages")}
        >Stages</button>
        <button
          type="button"
          class="mode-c-tab"
          class:active={activeTab === "output"}
          onclick={() => (activeTab = "output")}
        >Output</button>
      </nav>

      <div class="mode-c-panel">
        <ScreenCard kicker="Automagic Pro" title={null}>
          {#if activeTab === "tuning"}
            <div class="tuning-grid">
              <div class="tuning-row">
                <label class="tuning-label">SNR target</label>
                <div class="tuning-control">
                  <input type="range" min="0" max="100" value={tuningParams.snr_target} />
                  <span class="tuning-value">{tuningParams.snr_target}</span>
                </div>
              </div>
              <div class="tuning-row">
                <label class="tuning-label">Noise reduction</label>
                <div class="tuning-control">
                  <input type="range" min="0" max="100" value={tuningParams.noise_reduction * 100} />
                  <span class="tuning-value">{tuningParams.noise_reduction}</span>
                </div>
              </div>
              <div class="tuning-row">
                <label class="tuning-label">Star detection sensitivity</label>
                <div class="tuning-control">
                  <input type="range" min="0" max="100" value={tuningParams.star_detection_sensitivity * 100} />
                  <span class="tuning-value">{tuningParams.star_detection_sensitivity}</span>
                </div>
              </div>
              <div class="tuning-row">
                <label class="tuning-label">Color balance</label>
                <div class="tuning-control">
                  <select class="tuning-select">
                    <option>Neutral</option>
                    <option>Auto-detect</option>
                    <option>Custom</option>
                  </select>
                </div>
              </div>
              <div class="tuning-row toggle-row">
                <label class="tuning-label">Background extraction</label>
                <input type="checkbox" checked={tuningParams.background_extraction} class="tuning-toggle" />
              </div>
              <div class="tuning-row toggle-row">
                <label class="tuning-label">Auto-crop to subject</label>
                <input type="checkbox" checked={tuningParams.auto_crop} class="tuning-toggle" />
              </div>
            </div>

            <aside class="ai-banner">
              <span class="material-symbols-outlined">auto_awesome</span>
              <div>
                <div class="ai-banner-title">AI suggestion</div>
                <div class="ai-banner-body">
                  Raising star detection sensitivity to 0.85 will recover 14
                  marginal detections in the outer halo. Currently 0.80.
                </div>
              </div>
              <button class="btn-primary ai-banner-apply" type="button">Apply</button>
            </aside>
          {:else if activeTab === "stages"}
            <ol class="stage-list">
              {#each pipelineStages as stage, i}
                <li class="stage-item stage-{stage.status}">
                  <span class="stage-index">{i + 1}</span>
                  <span class="stage-name">{stage.name}</span>
                  <span class="stage-status">{stage.status.toUpperCase()}</span>
                  <span class="stage-duration">{stage.duration}</span>
                </li>
              {/each}
            </ol>
          {:else if activeTab === "output"}
            <dl class="output-grid">
              <dt>Format</dt><dd>{outputPreview.format}</dd>
              <dt>Resolution</dt><dd>{outputPreview.resolution}</dd>
              <dt>File size</dt><dd>{outputPreview.fileSize}</dd>
              <dt>Location</dt><dd>{outputPreview.location}</dd>
            </dl>
            <div class="output-actions">
              <button class="btn-secondary" type="button">Open folder</button>
              <button class="btn-primary" type="button">Export</button>
            </div>
          {/if}
        </ScreenCard>
      </div>
    {:else}
      <div class="mode-c-empty">Pick a session from the gallery to start.</div>
    {/if}
  </main>
</div>

<style>
  .mode-c {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .mode-c-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: var(--sp-md);
    gap: var(--sp-sm);
    overflow: hidden;
  }

  .mode-c-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-md);
  }

  .mode-c-kicker {
    color: var(--primary);
    font-family: var(--font-data);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
  }

  .mode-c-title {
    margin: 2px 0 0 0;
    font-family: var(--font-display);
    font-size: var(--text-headline-mobile);
    font-weight: 600;
    color: var(--on-surface);
    line-height: var(--lh-headline);
  }

  .mode-c-subtitle {
    color: var(--on-surface);
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    margin-top: 2px;
  }

  .status-pill {
    padding: 4px 10px;
    border-radius: 999px;
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
  }

  .status-processing {
    background: var(--cobalt-accent);
    color: var(--on-primary);
  }

  .mode-c-tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--outline-variant);
  }

  .mode-c-tab {
    background: none;
    border: none;
    padding: 10px 16px;
    font-family: var(--font-data);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    color: var(--on-surface-variant);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color var(--transition-base),
      border-color var(--transition-base);
  }

  .mode-c-tab:hover {
    color: var(--on-surface);
  }

  .mode-c-tab.active {
    color: var(--cobalt-accent);
    border-bottom-color: var(--cobalt-accent);
  }

  .mode-c-panel {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .mode-c-panel :global(.screen-card) {
    flex: 1;
  }

  .mode-c-empty {
    color: var(--on-surface-variant);
    padding: var(--sp-xl);
    text-align: center;
  }

  /* Tuning tab */
  .tuning-grid {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    margin-bottom: var(--sp-md);
  }

  .tuning-row {
    display: grid;
    grid-template-columns: 200px 1fr;
    align-items: center;
    gap: var(--sp-md);
  }

  .tuning-row.toggle-row {
    display: flex;
    justify-content: space-between;
  }

  .tuning-label {
    color: var(--on-surface);
    font-family: var(--font-body);
  }

  .tuning-control {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
  }

  .tuning-control input[type="range"] {
    flex: 1;
    accent-color: var(--cobalt-accent);
  }

  .tuning-value {
    font-family: var(--font-data);
    color: var(--on-surface);
    min-width: 40px;
    text-align: right;
  }

  .tuning-select {
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    padding: 6px 10px;
    font-family: var(--font-body);
  }

  .tuning-toggle {
    accent-color: var(--cobalt-accent);
    width: 18px;
    height: 18px;
  }

  .ai-banner {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: rgba(255, 180, 168, 0.08);
    border: 1px solid var(--primary-container);
    border-radius: var(--radius-default);
  }

  .ai-banner .material-symbols-outlined {
    color: var(--primary);
    font-size: 24px;
    flex-shrink: 0;
  }

  .ai-banner-title {
    color: var(--primary);
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    margin-bottom: 2px;
  }

  .ai-banner-body {
    color: var(--on-surface);
    font-family: var(--font-body);
    line-height: var(--lh-body);
  }

  .ai-banner-apply {
    align-self: center;
  }

  /* Stages tab */
  .stage-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .stage-item {
    display: grid;
    grid-template-columns: 28px 1fr auto auto;
    align-items: center;
    gap: var(--sp-sm);
    padding: 10px 12px;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
    color: var(--on-surface);
    font-family: var(--font-body);
  }

  .stage-done {
    border-color: rgba(185, 240, 197, 0.3);
  }

  .stage-active {
    border-color: var(--cobalt-accent);
    background: rgba(203, 78, 61, 0.08);
  }

  .stage-index {
    font-family: var(--font-data);
    color: var(--on-surface-variant);
    font-size: var(--text-metadata);
    text-align: center;
  }

  .stage-name {
    font-weight: 500;
  }

  .stage-status {
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    color: var(--on-surface-variant);
  }

  .stage-done .stage-status {
    color: #b9f0c5;
  }

  .stage-active .stage-status {
    color: var(--primary);
  }

  .stage-duration {
    font-family: var(--font-data);
    color: var(--on-surface);
    font-size: var(--text-metadata);
    font-variant-numeric: tabular-nums;
  }

  /* Output tab */
  .output-grid {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 8px var(--sp-md);
    margin: 0 0 var(--sp-md) 0;
  }

  .output-grid dt {
    color: var(--on-surface-variant);
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
  }

  .output-grid dd {
    color: var(--on-surface);
    font-family: var(--font-body);
    margin: 0;
  }

  .output-actions {
    display: flex;
    justify-content: flex-end;
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
</style>