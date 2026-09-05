<!--
  AppShell — the persistent shell that wraps all four layout modes.

  Always renders:
    - Header (logo + GPU badge + mode switcher)
    - Canvas backdrop layer (intensity per current mode)
    - Footer (status strip)
    - The active mode's content (via children snippet)

  Mode switcher lets the user override the context-derived mode.
  Available override modes depend on the current workflow stage.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { probeGpu, type GpuCapability } from "../lib/gpu";
  import { loadGallery } from "../lib/gallery";
  import {
    currentLayoutMode,
    modeSwitchAvailable,
    setModeOverride,
    clearModeOverride,
    labelForMode,
    availableOverrideModes,
    type LayoutMode,
    type AppStage,
  } from "../lib/layout-mode";
  import CanvasBackdrop from "./CanvasBackdrop.svelte";

  let {
    currentStage,
    children,
    onOpenProfiles,
  }: {
    currentStage: AppStage;
    children?: Snippet;
    /// Phase 1.5 PR-C: opens the ProfileManager modal. App.svelte wires
    /// this so the modal can be rendered outside AppShell's slot tree.
    onOpenProfiles?: () => void;
  } = $props();

  let gpuCapability: GpuCapability = $state("canvas2d");
  let gpuChecked = $state(false);

  async function init() {
    gpuCapability = probeGpu();
    gpuChecked = true;
    // Hydrate the gallery cache from the local rusqlite store (or
    // placeholder fallback if running outside Tauri). This way every
    // consumer of getGallery() sees the same data on first render.
    await loadGallery();
  }

  if (typeof document !== "undefined") {
    if (document.readyState !== "loading") {
      init();
    } else {
      document.addEventListener("DOMContentLoaded", init, { once: true });
    }
  }

  let overrides = $derived(availableOverrideModes(currentStage));
  let switcherActive = $derived($modeSwitchAvailable);
  let activeMode = $derived($currentLayoutMode);

  function pickOverride(mode: LayoutMode) {
    if (activeMode === mode) {
      clearModeOverride();
    } else {
      setModeOverride(mode);
    }
  }
</script>

<div class="app">
  <CanvasBackdrop />

  <header class="shell-header">
    <div class="logo">
      <svg width="24" height="24" viewBox="0 0 64 64" fill="none" aria-hidden="true">
        <circle cx="32" cy="32" r="20" stroke="currentColor" stroke-width="2.5" fill="none" />
        <circle cx="32" cy="32" r="6" fill="currentColor" />
        <path
          d="M32 6 L32 14 M32 50 L32 58 M6 32 L14 32 M50 32 L58 32"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
        />
      </svg>
      <span class="logo-text">AstroForge</span>
    </div>

    {#if switcherActive}
      <nav class="mode-switch" aria-label="Layout mode">
        {#each overrides as mode}
          <button
            type="button"
            class="mode-switch-btn"
            class:active={activeMode === mode}
            onclick={() => pickOverride(mode)}
            title="Switch to {labelForMode(mode)} layout"
          >{labelForMode(mode)}</button>
        {/each}
      </nav>
    {/if}

    {#if onOpenProfiles}
      <button
        type="button"
        class="profile-btn"
        onclick={onOpenProfiles}
        title="Manage saved pipeline profiles"
        aria-label="Open profile manager"
      >
        <span class="material-symbols-outlined">tune</span>
        <span>Profiles</span>
      </button>
    {/if}

    <div class="gpu-badge" class:gpu-checked={gpuChecked}>
      {#if gpuChecked}
        GPU: {gpuCapability === "webgpu" ? "WebGPU" : "WebGL"}
      {:else}
        Detecting GPU…
      {/if}
    </div>
  </header>

  <main class="shell-main">
    {#if children}
      {@render children()}
    {/if}
  </main>

  <footer class="shell-footer">
    <span>AstroForge v0.1.0</span>
    <span class="shell-footer-sep">·</span>
    <span>Full Pipeline — Deep Sky, Planetary &amp; Lunar</span>
    <span class="shell-footer-mode">Mode: {labelForMode(activeMode)}</span>
  </footer>
</div>

<style>
  .app {
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: var(--background);
    color: var(--on-surface);
    z-index: 1;
  }

  .shell-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-md);
    padding: 0 var(--sp-lg);
    height: 56px;
    background: var(--overlay-panel);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-bottom: 1px solid var(--outline-variant);
    flex-shrink: 0;
    position: relative;
    z-index: 10;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--cobalt-accent);
  }

  .logo-text {
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 600;
    color: var(--on-surface);
    letter-spacing: var(--ls-headline);
  }

  .mode-switch {
    display: flex;
    gap: 4px;
    padding: 4px;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
  }

  .mode-switch-btn {
    background: none;
    border: none;
    padding: 6px 12px;
    font-family: var(--font-data);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    color: var(--on-surface-variant);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: color var(--transition-base),
      background-color var(--transition-base);
  }

  .mode-switch-btn:hover {
    color: var(--on-surface);
  }

  .mode-switch-btn.active {
    background: var(--cobalt-accent);
    color: var(--on-primary);
  }

  .profile-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    color: var(--on-surface);
    border-radius: var(--radius-default);
    font-family: var(--font-data);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    cursor: pointer;
    transition: border-color var(--transition-base);
  }
  .profile-btn:hover {
    border-color: var(--cobalt-accent);
  }
  .profile-btn .material-symbols-outlined {
    font-size: 1rem;
  }

  .gpu-badge {
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    color: var(--on-surface-variant);
    padding: 4px 12px;
    border-radius: var(--radius-default);
    background: var(--surface-container-high);
  }

  .gpu-badge.gpu-checked {
    color: var(--success);
  }

  .shell-main {
    flex: 1;
    position: relative;
    z-index: 5;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .shell-footer {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: 0 var(--sp-lg);
    height: 32px;
    background: var(--overlay-panel);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-top: 1px solid var(--outline-variant);
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    color: var(--on-surface-variant);
    flex-shrink: 0;
    position: relative;
    z-index: 10;
  }

  .shell-footer-sep {
    color: var(--outline-variant);
  }

  .shell-footer-mode {
    margin-left: auto;
    color: var(--cobalt-accent);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
  }
</style>