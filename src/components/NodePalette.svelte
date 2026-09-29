<script lang="ts">
  // CR-10 P1.6.2.1 / Slice P1.6.2.1:
  // NodePalette skeleton. Reads the canonical NodeCatalog
  // from the P1.6.1.3 bridge, groups entries by the seven
  // palette groups defined in AstroForge_Spec_v1.4.0.md
  // §7.7.3, and renders a collapsible per-group list.
  //
  // Click-to-insert is a stub: this slice only renders and
  // dispatches the user's selection (logged via the
  // `onSelect` event + console). The actual graph mutation
  // IPC lands in P1.6.4.1 (graph_to_recipe IPC) / P1.6.3.1
  // (update_graph IPC).

  import { onMount } from "svelte";
  import {
    loadNodeCatalog,
    getCachedNodeCatalog,
    type NodeCatalog,
    type NodeCatalogEntry,
  } from "../lib/node-catalog";
  import {
    PALETTE_GROUP_ORDER,
    PALETTE_GROUP_DESCRIPTIONS,
    PALETTE_GROUP_EMPTY_MESSAGES,
    groupEntriesByPalette,
    countPaletteEntries,
    paletteRootClassName,
    type PaletteGroupName,
  } from "../lib/node-palette-logic";

  // ─── Props ─────────────────────────────────────────────────────────────────

  /** Optional CSS class for the root container. */
  let className: string = "";
  export { className as class };

  /** Optional disabled flag (fades the palette + disables clicks). */
  export let disabled: boolean = false;

  /**
   * Fired when the user clicks an entry. The host
   * (P1.6.2.2's `ProcessWorkspace`) decides where to
   * insert it (constrained layout: append to active
   * pipeline; free-form layout: drop at cursor). This
   * slice does not yet have an insert IPC: handlers
   * can simply log the selection.
   */
  export let onSelect: ((entry: NodeCatalogEntry) => void) | undefined =
    undefined;

  // ─── State ─────────────────────────────────────────────────────────────────

  let catalog: NodeCatalog | null = null;
  let loadError: string | null = null;
  let search: string = "";

  // ─── Group mapping ─────────────────────────────────────────────────────────
  // The 12 user-facing stages map to the seven palette
  // groups defined in AstroForge_Spec_v1.4.0.md §7.7.3.
  // The mapping + descriptions + order live in
  // `node-palette-logic.ts` so they can be unit-tested
  // without booting Svelte in jsdom. The empty message
  // per group is also centralized.

  // Stable in-memory collapsed state. Seven groups,
  // all initially expanded. The view toggle is a
  // per-group click that flips one entry.
  function defaultCollapsed(): Record<PaletteGroupName, boolean> {
    const out = {} as Record<PaletteGroupName, boolean>;
    for (const g of PALETTE_GROUP_ORDER) out[g] = false;
    return out;
  }

  let collapsed: Record<PaletteGroupName, boolean> = defaultCollapsed();

  // ─── Lifecycle ─────────────────────────────────────────────────────────────

  onMount(() => {
    // If the catalog was already loaded by a
    // previous slice, skip the IPC call.
    catalog = getCachedNodeCatalog();
    if (catalog !== null) return;

    loadNodeCatalog()
      .then((c) => {
        catalog = c;
      })
      .catch((err) => {
        loadError =
          err instanceof Error
            ? err.message
            : "Failed to load NodeCatalog from the Rust side";
      });
  });

  // ─── Derived ───────────────────────────────────────────────────────────────

  $: filteredByGroup =
    catalog !== null
      ? groupEntriesByPalette(catalog, search)
      : new Map<PaletteGroupName, NodeCatalogEntry[]>();

  $: paletteEntryCount =
    catalog !== null ? countPaletteEntries(catalog) : 0;

  function toggle(group: PaletteGroupName): void {
    collapsed[group] = !collapsed[group];
    // Svelte 4 reactivity: reassign to trigger.
    collapsed = collapsed;
  }

  function emptyMessageFor(group: PaletteGroupName): string {
    return (
      PALETTE_GROUP_EMPTY_MESSAGES[group] ?? "No matching entries"
    );
  }

  function handleClick(entry: NodeCatalogEntry): void {
    if (disabled) return;
    if (onSelect !== undefined) {
      onSelect(entry);
      return;
    }
    // Default stub: log to the console. The IPC
    // wiring lands in P1.6.2.4 / P1.6.3.1 /
    // P1.6.4.1 (graph_to_recipe): for now we
    // surface the intent in the dev console so
    // the click is visible.
    // eslint-disable-next-line no-console
    console.log(
      `[NodePalette] selected stage_type='${entry.stage_type}' label='${entry.label}'`,
    );
  }

  // ─── Render ────────────────────────────────────────────────────────────────
</script>

<div
  class={paletteRootClassName(className, disabled)}
  data-testid="node-palette"
  role="region"
  aria-label="Node palette"
>
  <header class="palette-header">
    <h3 class="palette-title">Node Palette</h3>
    <p class="palette-subtitle">
      {catalog === null
        ? "Loading catalog..."
        : `${paletteEntryCount} stages`}
    </p>
  </header>

  <div class="palette-search">
    <input
      type="search"
      placeholder="Search by label or hint"
      bind:value={search}
      aria-label="Search palette entries"
      disabled={catalog === null || disabled}
    />
  </div>

  {#if loadError !== null}
    <div class="palette-error" role="alert">
      <strong>Failed to load catalog.</strong>
      <span>{loadError}</span>
    </div>
  {/if}

  {#if catalog !== null && catalog.entries.length === 0}
    <div class="palette-empty" role="status">
      No catalog entries available. (Browser-mode fallback: open the
      AstroForge Tauri shell to load the NodeCatalog.)
    </div>
  {/if}

  <div class="palette-groups">
    {#each PALETTE_GROUP_ORDER as group (group)}
      {@const entries = filteredByGroup.get(group) ?? []}
      {@const isCollapsed = collapsed[group]}
      {@const isEmpty = entries.length === 0}
      <section class="palette-group" data-group={group} data-empty={isEmpty}>
        <button
          class="group-header"
          type="button"
          aria-expanded={!isCollapsed}
          aria-controls={`group-${group}`}
          on:click={() => toggle(group)}
        >
          <span class="group-caret" aria-hidden="true">
            {isCollapsed ? "▶" : "▼"}
          </span>
          <span class="group-label">{group}</span>
          <span class="group-count" aria-label={`${entries.length} entries`}>
            {entries.length}
          </span>
        </button>
        <p class="group-description">{PALETTE_GROUP_DESCRIPTIONS[group]}</p>

        {#if !isCollapsed}
          <ul class="group-entries" id={`group-${group}`}>
            {#each entries as entry (entry.stage_type)}
              <li>
                <button
                  class="entry"
                  type="button"
                  data-stage-type={entry.stage_type}
                  data-ai={entry.ai_uses_ai}
                  on:click={() => handleClick(entry)}
                  disabled={disabled}
                  title={entry.description}
                >
                  <span class="entry-label">{entry.label}</span>
                  {#if entry.ai_uses_ai}
                    <span
                      class="entry-ai-badge"
                      aria-label="AI-powered stage"
                      title="AI-powered"
                    >
                      AI
                    </span>
                  {/if}
                </button>
              </li>
            {/each}
            {#if isEmpty}
              <li class="group-empty">{emptyMessageFor(group)}</li>
            {/if}
          </ul>
        {/if}
      </section>
    {/each}
  </div>
</div>

<style>
  .node-palette {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    width: 280px;
    min-width: 280px;
    max-width: 320px;
    height: 100%;
    overflow-y: auto;
    background: var(--surface-container-low, #1a1a1a);
    color: var(--on-surface, #e0e0e0);
    border-right: 1px solid var(--outline-variant, #333);
    font-size: 14px;
    box-sizing: border-box;
  }

  .node-palette.disabled {
    opacity: 0.5;
    pointer-events: none;
  }

  .palette-header {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .palette-title {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--on-surface, #e0e0e0);
  }

  .palette-subtitle {
    margin: 0;
    font-size: 12px;
    color: var(--on-surface-variant, #999);
  }

  .palette-search input {
    width: 100%;
    padding: 8px 12px;
    background: var(--surface-container, #222);
    color: var(--on-surface, #e0e0e0);
    border: 1px solid var(--outline-variant, #444);
    border-radius: 6px;
    font-size: 13px;
    box-sizing: border-box;
  }

  .palette-error {
    padding: 8px 12px;
    background: var(--error-container, #5a1a1a);
    color: var(--on-error-container, #fbb);
    border-radius: 6px;
    font-size: 12px;
  }

  .palette-empty {
    padding: 12px;
    color: var(--on-surface-variant, #999);
    font-size: 12px;
    font-style: italic;
    text-align: center;
  }

  .palette-groups {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .palette-group {
    border: 1px solid var(--outline-variant, #2a2a2a);
    border-radius: 6px;
    background: var(--surface-container, #1f1f1f);
  }

  .palette-group[data-empty="true"] {
    opacity: 0.7;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    background: transparent;
    color: inherit;
    border: 0;
    text-align: left;
    cursor: pointer;
    font: inherit;
  }

  .group-caret {
    font-size: 10px;
    width: 12px;
    flex-shrink: 0;
  }

  .group-label {
    flex: 1;
    font-weight: 600;
    font-size: 13px;
    color: var(--on-surface, #e0e0e0);
  }

  .group-count {
    font-size: 11px;
    color: var(--on-surface-variant, #999);
    background: var(--surface-container-high, #2a2a2a);
    padding: 2px 6px;
    border-radius: 10px;
  }

  .group-description {
    margin: 0 10px 6px 30px;
    font-size: 11px;
    color: var(--on-surface-variant, #999);
    line-height: 1.4;
  }

  .group-entries {
    list-style: none;
    margin: 0;
    padding: 0 6px 6px 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .entry {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    background: var(--surface-container-low, #161616);
    color: var(--on-surface, #d0d0d0);
    border: 1px solid transparent;
    border-radius: 4px;
    text-align: left;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
  }

  .entry:hover:not(:disabled) {
    background: var(--surface-container-high, #2a2a2a);
    border-color: var(--primary, #4a9eff);
  }

  .entry:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .entry-label {
    flex: 1;
  }

  .entry-ai-badge {
    font-size: 10px;
    font-weight: 700;
    background: var(--tertiary-container, #2d1f4a);
    color: var(--on-tertiary-container, #d4c5ff);
    padding: 1px 5px;
    border-radius: 8px;
    letter-spacing: 0.5px;
  }

  .group-empty {
    padding: 4px 10px;
    font-size: 11px;
    color: var(--on-surface-variant, #777);
    font-style: italic;
  }
</style>