<!--
  Gallery — "Your Work" gallery, used in Mode B (Library sidebar) and
  Mode C (top strip). Tabs are Recent / Processing / Completed. Data is
  loaded from gallery.ts (placeholder for now; rusqlite swap-in Phase 4).

  Layout:
    columns=1  -> narrow side rail (Mode B default)
    columns=2  -> top strip (Mode C)
    columns=3  -> reserved for future wide contexts
-->
<script lang="ts">
  import { galleryStore, filterByTab, type GalleryItem, type GalleryTab } from "../lib/gallery";

  let {
    columns = 1,
    activeItemId = null,
    onSelect = (_item: GalleryItem) => {},
  }: {
    columns?: 1 | 2 | 3;
    activeItemId?: string | null;
    onSelect?: (item: GalleryItem) => void;
  } = $props();

  let activeTab: GalleryTab = $state("recent");

  let visibleItems: GalleryItem[] = $derived(filterByTab($galleryStore, activeTab));
  let counts = $derived({
    recent: $galleryStore.length,
    processing: $galleryStore.filter((i) => i.status === "processing").length,
    completed: $galleryStore.filter((i) => i.status === "completed").length,
  });

  function pickItem(item: GalleryItem) {
    onSelect(item);
  }
</script>

<section class="gallery" data-columns={columns}>
  <header class="gallery-header">
    <div class="gallery-title">
      <span class="gallery-kicker label-caps">Your Work</span>
      <h2 class="gallery-h">Sessions</h2>
    </div>

    <nav class="gallery-tabs" aria-label="Gallery filter">
      <button
        type="button"
        class="gallery-tab"
        class:active={activeTab === "recent"}
        onclick={() => (activeTab = "recent")}
      >Recent <span class="tab-count">{counts.recent}</span></button>
      <button
        type="button"
        class="gallery-tab"
        class:active={activeTab === "processing"}
        onclick={() => (activeTab = "processing")}
      >Processing <span class="tab-count">{counts.processing}</span></button>
      <button
        type="button"
        class="gallery-tab"
        class:active={activeTab === "completed"}
        onclick={() => (activeTab = "completed")}
      >Completed <span class="tab-count">{counts.completed}</span></button>
    </nav>
  </header>

  <div class="gallery-scroll">
    {#if visibleItems.length === 0}
      <div class="gallery-empty">No sessions in this category yet.</div>
    {:else}
      <div class="gallery-grid" data-columns={columns}>
        {#each visibleItems as item (item.id)}
          <button
            type="button"
            class="gallery-tile"
            class:active={activeItemId === item.id}
            onclick={() => pickItem(item)}
            aria-label="Open session {item.target}"
          >
            <div class="tile-preview">
              <span class="material-symbols-outlined tile-icon">image</span>
              <span class="tile-status status-{item.status}">{item.status}</span>
            </div>
            <div class="tile-body">
              <div class="tile-name">{item.target}</div>
              <div class="tile-meta-primary">
                {item.integrationHours}h · {item.palette}
              </div>
              <div class="tile-meta-secondary">
                {item.name} · {new Date(item.updatedAt).toLocaleDateString()}
              </div>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</section>

<style>
  .gallery {
    display: flex;
    flex-direction: column;
    background: var(--surface-container-low);
    border-right: 1px solid var(--outline-variant);
    height: 100%;
    min-height: 0;
  }

  .gallery[data-columns="2"],
  .gallery[data-columns="3"] {
    border-right: none;
    border-bottom: 1px solid var(--outline-variant);
    height: auto;
    max-height: 280px;
  }

  .gallery-header {
    padding: var(--sp-md);
    border-bottom: 1px solid var(--outline-variant);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .gallery-title {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .gallery-kicker {
    color: var(--primary);
    font-family: var(--font-data);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
  }

  .gallery-h {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-headline-mobile);
    font-weight: 600;
    line-height: var(--lh-headline);
    color: var(--on-surface);
  }

  .gallery-tabs {
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
  }

  .gallery-tab {
    flex: 1;
    background: none;
    border: none;
    padding: 4px 6px;
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    font-weight: 700;
    color: var(--on-surface-variant);
    border-radius: var(--radius-sm);
    cursor: pointer;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    transition: color var(--transition-base),
      background-color var(--transition-base);
  }

  .gallery-tab:hover {
    color: var(--on-surface);
  }

  .gallery-tab.active {
    background: var(--cobalt-accent);
    color: var(--on-primary);
  }

  .tab-count {
    display: inline-block;
    margin-left: 4px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--overlay-tile-idle);
    color: inherit;
    font-size: 0.85em;
  }

  .gallery-tab.active .tab-count {
    background: var(--overlay-soft);
  }

  .gallery-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--sp-md);
  }

  .gallery-grid {
    display: grid;
    gap: var(--sp-sm);
  }

  .gallery-grid[data-columns="1"] {
    grid-template-columns: 1fr;
  }

  .gallery-grid[data-columns="2"] {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .gallery-grid[data-columns="3"] {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .gallery-empty {
    color: var(--on-surface-variant);
    padding: var(--sp-lg);
    text-align: center;
    border: 1px dashed var(--outline-variant);
    border-radius: var(--radius-default);
  }

  .gallery-tile {
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
    text-align: left;
    padding: 0;
    cursor: pointer;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    color: inherit;
    font-family: inherit;
    transition: border-color var(--transition-fast),
      transform var(--transition-fast);
  }

  .gallery-tile:hover {
    border-color: var(--cobalt-accent);
    transform: translateY(-1px);
  }

  .gallery-tile.active {
    border-color: var(--cobalt-accent);
    box-shadow: 0 0 0 1px var(--cobalt-accent),
      0 8px 24px var(--glow-cobalt-soft);
  }

  .tile-preview {
    position: relative;
    height: 80px;
    background:
      radial-gradient(circle at 30% 20%, var(--tile-glow-warm), transparent 60%),
      radial-gradient(circle at 80% 80%, var(--tile-glow-cool), transparent 60%),
      var(--surface-container-highest);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--on-surface-variant);
  }

  .tile-icon {
    font-size: 32px;
    opacity: 0.4;
  }

  .tile-status {
    position: absolute;
    top: 6px;
    right: 6px;
    padding: 2px 8px;
    border-radius: 999px;
    font-family: var(--font-data);
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    background: var(--overlay-strong);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }

  .status-completed {
    color: var(--success-fg);
    border: 1px solid var(--success-border);
  }

  .status-processing {
    color: var(--primary);
    border: 1px solid var(--primary-container);
    animation: pulse 2s ease-in-out infinite;
  }

  .status-pending {
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }

  .tile-body {
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tile-name {
    font-family: var(--font-display);
    font-size: var(--text-body);
    font-weight: 600;
    color: var(--on-surface);
    line-height: var(--lh-body);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tile-meta-primary {
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    font-weight: 500;
    color: var(--on-surface);
    line-height: var(--lh-data);
  }

  .tile-meta-secondary {
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    color: var(--on-surface-variant);
    line-height: var(--lh-data);
  }
</style>