<!--
  ModeA — "Load" layout. Floating overlay cards on a persistent canvas.

  Used by currentStep in { select-files, session-setup, review-frames }.
  Renders the actual existing workflow screen-card in an overlay panel
  positioned over the persistent canvas (Gallery + canvas placeholders).

  Uses Svelte 5 snippet API to accept arbitrary overlay content without
  requiring direct-child slot placement.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import Gallery from "./Gallery.svelte";

  let { children }: { children?: Snippet } = $props();
</script>

<div class="mode-a">
  <!-- Persistent canvas area behind the overlay -->
  <div class="mode-a-canvas">
    <Gallery columns={2} />
  </div>

  <!-- Overlay card slot -->
  <div class="mode-a-overlay">
    {#if children}
      {@render children()}
    {/if}
  </div>
</div>

<style>
  .mode-a {
    flex: 1;
    display: flex;
    flex-direction: column;
    position: relative;
    min-height: 0;
  }

  .mode-a-canvas {
    flex: 1;
    padding: var(--sp-lg);
    overflow-y: auto;
  }

  .mode-a-overlay {
    position: fixed;
    top: 80px;
    right: var(--sp-lg);
    width: 480px;
    max-height: calc(100vh - 56px - 80px - 32px);
    z-index: 20;
  }

  @media (max-width: 1024px) {
    .mode-a-overlay {
      position: fixed;
      top: auto;
      right: 0;
      bottom: 32px;
      left: 0;
      width: 100%;
      max-height: 60vh;
      border-radius: var(--radius-default) var(--radius-default) 0 0;
    }
  }
</style>