<!--
  ModeB — "Library" layout. 3-column: Gallery | Canvas | Workflow.

  Used by currentStep in { landing, processing }. Left rail holds the
  Gallery, center is the canvas (where the workflow card sits), right
  is the active workflow panel.

  In landing, the center is empty + brand; in processing, the center
  hosts the PreviewCanvas and the right shows WizardBottomSheet /
  NodeSidebar / ParameterSidebar.

  Uses Svelte 5 snippets so the caller doesn't have to keep slot
  elements as direct children (which conflicts with {#if} blocks).
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import Gallery from "./Gallery.svelte";

  let {
    canvas,
    workflow,
  }: { canvas?: Snippet; workflow?: Snippet } = $props();
</script>

<div class="mode-b">
  <aside class="mode-b-rail">
    <Gallery variantLayout="rail" maxTiles={8} />
  </aside>

  <main class="mode-b-canvas">
    {#if canvas}
      {@render canvas()}
    {/if}
  </main>

  <aside class="mode-b-workflow">
    {#if workflow}
      {@render workflow()}
    {/if}
  </aside>
</div>

<style>
  .mode-b {
    flex: 1;
    display: grid;
    grid-template-columns: 320px 1fr 360px;
    min-height: 0;
    overflow: hidden;
  }

  .mode-b-rail,
  .mode-b-workflow {
    background: var(--surface-container-low);
    border-color: var(--outline-variant);
    overflow: hidden;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .mode-b-rail {
    border-right: 1px solid var(--outline-variant);
  }

  .mode-b-workflow {
    border-left: 1px solid var(--outline-variant);
  }

  .mode-b-canvas {
    flex: 1;
    overflow: auto;
    background: var(--background);
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  @media (max-width: 1024px) {
    .mode-b {
      grid-template-columns: 1fr;
      grid-template-rows: 200px 1fr 240px;
    }
    .mode-b-rail {
      border-right: none;
      border-bottom: 1px solid var(--outline-variant);
    }
    .mode-b-workflow {
      border-left: none;
      border-top: 1px solid var(--outline-variant);
    }
  }
</style>