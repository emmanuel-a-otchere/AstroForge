<!--
  Gallery — "Your Work" gallery, used in Mode B (Library sidebar) and
  Mode C (top strip). Tabs are Recent / Processing / Completed. Data is
  loaded from gallery.ts (placeholder for now; rusqlite in Phase 4).

  Renders tiles with target name, integration hours, palette, and a
  status badge. Tile click is a no-op in placeholder mode — Phase 4 will
  wire it to session load.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    loadGallery,
    filterByTab,
    statusBadgeClass,
    statusBadgeLabel,
    type GalleryItem,
    type GalleryTab,
  } from "../lib/gallery";

  export let variantLayout: "rail" | "grid" = "rail";
  /** Maximum number of tiles to show in this gallery instance. */
  export let maxTiles: number = 8;

  let items: GalleryItem[] = [];
  let activeTab: GalleryTab = "recent";
  let loading = true;

  onMount(async () => {
    items = await loadGallery();
    loading = false;
  });

  $: visibleItems = filterByTab(items, activeTab).slice(0, maxTiles);

  function setTab(t: GalleryTab) {
    activeTab = t;
  }
</script>

<section class="gallery" data-variant={variantLayout}>
  <header class="gallery-header">
    <h4 class="gallery-title">Your Work</h4>
    <nav class="gallery-tabs" aria-label="Gallery filter">
      <button
        type="button"
        class="gallery-tab"
        class:active={activeTab === "recent"}
        on:click={() => setTab("recent")}
      >Recent</button>
      <button
        type="button"
        class="gallery-tab"
        class:active={activeTab === "processing"}
        on:click={() => setTab("processing")}
      >Processing</button>
      <button
        type="button"
        class="gallery-tab"
        class:active={activeTab === "completed"}
        on:click={() => setTab("completed")}
      >Completed</button>
    </nav>
  </header>

  {#if loading}
    <div class="gallery-loading">Loading your work…</div>
  {:else if visibleItems.length === 0}
    <div class="gallery-empty">
      <span class="gallery-empty-label label-caps">No items</span>
      <span class="gallery-empty-hint">Nothing in this view yet.</span>
    </div>
  {:else}
    <ul class="gallery-list" data-variant={variantLayout}>
      {#each visibleItems as item (item.id)}
        <li class="gallery-tile">
          <div class="gallery-tile-thumb" aria-hidden="true"></div>
          <div class="gallery-tile-body">
            <div class="gallery-tile-target">{item.target}</div>
            <div class="gallery-tile-name" title={item.name}>{item.name}</div>
            <div class="gallery-tile-meta">
              {item.integrationHours.toFixed(2)}h · {item.palette}
            </div>
            <span class={statusBadgeClass(item.status)}>
              {statusBadgeLabel(item.status)}
            </span>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .gallery {
    display: flex;
    flex-direction: column;
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
    overflow: hidden;
    color: var(--on-surface);
  }

  .gallery-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-md);
    padding: var(--sp-md);
    border-bottom: 1px solid var(--outline-variant);
    flex-shrink: 0;
  }

  .gallery-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-body);
    font-weight: 600;
    letter-spacing: var(--ls-headline);
    color: var(--on-surface);
  }

  .gallery-tabs {
    display: flex;
    gap: var(--sp-md);
  }

  .gallery-tab {
    background: none;
    border: none;
    padding: 4px 0;
    font-family: var(--font-data);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    color: var(--on-surface-variant);
    border-bottom: 1px solid transparent;
    cursor: pointer;
    transition: color var(--transition-base),
      border-color var(--transition-base);
  }

  .gallery-tab:hover {
    color: var(--on-surface);
  }

  .gallery-tab.active {
    color: var(--cobalt-accent);
    border-bottom-color: var(--cobalt-accent);
  }

  .gallery-loading,
  .gallery-empty {
    padding: var(--sp-lg);
    text-align: center;
    color: var(--on-surface-variant);
    font-family: var(--font-body);
    font-size: var(--text-body);
  }

  .gallery-empty {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .gallery-empty-label {
    color: var(--outline);
  }

  .gallery-empty-hint {
    font-size: var(--text-metadata);
  }

  .gallery-list {
    list-style: none;
    margin: 0;
    padding: var(--sp-sm);
    overflow-y: auto;
    display: grid;
    gap: var(--sp-sm);
  }

  .gallery-list[data-variant="rail"] {
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  }

  .gallery-list[data-variant="grid"] {
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  }

  .gallery-tile {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
    transition: border-color var(--transition-base);
  }

  .gallery-tile:hover {
    border-color: var(--cobalt-accent);
  }

  .gallery-tile-thumb {
    aspect-ratio: 1.4;
    background: radial-gradient(
      circle at 50% 40%,
      rgba(203, 78, 61, 0.32),
      var(--surface-container-low)
    );
    border-radius: var(--radius-sm);
  }

  .gallery-tile-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .gallery-tile-target {
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    font-weight: 700;
    letter-spacing: var(--ls-data);
    color: var(--on-surface-variant);
    text-transform: uppercase;
  }

  .gallery-tile-name {
    font-family: var(--font-data);
    font-size: var(--text-data);
    font-weight: 500;
    color: var(--on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .gallery-tile-meta {
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    color: var(--on-surface-variant);
  }

  .gallery-status {
    display: inline-block;
    align-self: flex-start;
    margin-top: 4px;
    padding: 2px 6px;
    border-radius: var(--radius-full);
    font-family: var(--font-data);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
  }

  .gallery-status-completed {
    background: rgba(16, 185, 129, 0.15);
    color: var(--success);
  }

  .gallery-status-processing {
    background: rgba(203, 78, 61, 0.15);
    color: var(--cobalt-accent);
  }

  .gallery-status-pending {
    background: rgba(167, 138, 134, 0.15);
    color: var(--outline);
  }
</style>