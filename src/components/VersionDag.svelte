<!--
  CR-07 B8: Version Tree visualization (§15).

  Renders a vertical DAG from a project's imageVersionList. Each
  node is a version card (label, sequence, has-artifact, hidden).
  Edges are CSS-drawn; the layout is recursive with a single
  generator per tree.

  Selecting two nodes automatically prepares them for
  comparison (per §15): the first click picks A, the second
  picks B. A third click resets and starts again. Escape clears
  the selection. The parent can also pre-select via the
  `presetA` / `presetB` props (used by the 'Compare' button on
  each version card).

  Out of scope for B8 (intentional):

    - Edge labels (Stretch / Natural AI / Aggressive AI).
      Reading the per-version Recipe is data-model adjacent;
      defer to a future slice.
    - Drag-to-rearrange.
    - Cycle repair (the schema forbids cycles; the builder
      surfaces cycle-affected versions via `excluded`).
-->
<script lang="ts">
  import { buildVersionDag, type DagNode } from "../state/version-dag";
  import type { ImageVersion } from "../lib/astroforge-api";

  interface Props {
    versions: ReadonlyArray<ImageVersion>;
    onSelectPair?: (a: string, b: string) => void;
    /** Pre-selected slot ids. When the user picks nodes, the
     *  picker treats them as the first selection so the next
     *  click fills the other slot. */
    presetA?: string | null;
    presetB?: string | null;
  }
  const { versions, onSelectPair, presetA = null, presetB = null }: Props =
    $props();

  const dag = $derived(buildVersionDag(versions));

  let pickA = $state<string | null>(null);
  let pickB = $state<string | null>(null);

  // Re-sync when presets change.
  $effect(() => {
    pickA = presetA;
    pickB = presetB;
  });

  function togglePick(version_id: string) {
    // First pick: clear any existing pair, set A.
    if (!pickA) {
      pickA = version_id;
      return;
    }
    // Same id: deselect.
    if (pickA === version_id) {
      pickA = null;
      pickB = null;
      return;
    }
    // Second pick: if it's already B, clear B (click order
    // matters here: A -> B -> A again would otherwise look
    // like a no-op).
    if (pickB === version_id) {
      pickB = null;
      return;
    }
    pickB = version_id;
    if (onSelectPair) onSelectPair(pickA, pickB);
  }

  function roleFor(version_id: string): "a" | "b" | null {
    if (pickA === version_id) return "a";
    if (pickB === version_id) return "b";
    return null;
  }

  function onKeydown(ev: KeyboardEvent) {
    if (ev.key === "Escape") {
      pickA = null;
      pickB = null;
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

<section class="version-dag" aria-label="Version tree">
  <header class="dag-header">
    <span class="dag-tag font-label">Version tree</span>
    <h3 class="dag-title font-display">
      {dag.total} version{dag.total === 1 ? "" : "s"}
    </h3>
    {#if dag.excluded.length > 0}
      <span class="dag-warn font-body" data-testid="dag-cycle-warning">
        {dag.excluded.length} excluded (cycle)
      </span>
    {/if}
    {#if pickA || pickB}
      <button
        type="button"
        class="dag-clear"
        onclick={() => {
          pickA = null;
          pickB = null;
        }}
      >
        Clear selection
      </button>
    {/if}
  </header>

  {#if dag.roots.length === 0}
    <p class="dag-empty font-body">
      No versions yet. Run an enhancement to populate the tree.
    </p>
  {:else}
    {#each dag.roots as root (root.version_id)}
      {@render branch(root)}
    {/each}
  {/if}
</section>

{#snippet branch(node: DagNode)}
  <div class="dag-branch" data-depth={node.depth}>
    <button
      type="button"
      class="dag-node"
      data-role={roleFor(node.version_id)}
      data-hidden={node.hidden}
      data-has-artifact={node.has_artifact}
      data-cycle={node.cycleDetected ? "true" : "false"}
      data-testid="dag-node"
      aria-pressed={roleFor(node.version_id) !== null}
      onclick={() => togglePick(node.version_id)}
    >
      <span class="dag-node-label font-body">{node.label}</span>
      <span class="dag-node-meta font-label">
        #{node.sequence}
        {#if !node.has_artifact}
          <span class="badge badge-empty" title="No artifact linked">
            empty
          </span>
        {/if}
        {#if node.hidden}
          <span class="badge badge-hidden" title="Hidden">hidden</span>
        {/if}
      </span>
    </button>
    {#if node.children.length > 0}
      <div class="dag-children">
        {#each node.children as child (child.version_id)}
          {@render branch(child)}
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<style>
  .version-dag {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    padding: var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
  }

  .dag-header {
    display: flex;
    align-items: baseline;
    gap: var(--sp-sm);
    flex-wrap: wrap;
  }

  .dag-tag {
    font-size: 0.7rem;
    color: var(--primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .dag-title {
    margin: 0;
    font-size: 1rem;
    flex: 1;
  }

  .dag-warn {
    margin: 0;
    color: #ff904a;
    font-size: 0.75rem;
  }

  .dag-clear {
    padding: 2px var(--sp-xs);
    border-radius: var(--radius-md);
    border: 1px solid var(--outline-variant);
    background: transparent;
    color: var(--on-surface);
    cursor: pointer;
    font-family: inherit;
    font-size: 0.75rem;
  }

  .dag-empty {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }

  .dag-branch {
    position: relative;
    padding-left: var(--sp-md);
  }

  /* Vertical edge from parent to this branch. */
  .dag-branch::before {
    content: "";
    position: absolute;
    left: 8px;
    top: 0;
    bottom: 0;
    border-left: 1px solid var(--outline-variant);
  }

  /* Hide the vertical edge when this branch has no children
     of its own (i.e. it's a leaf) so we don't draw a tail. */
  .dag-branch:not(:has(.dag-children))::before {
    bottom: calc(50% - 12px);
  }

  .dag-children {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
    padding-left: 0;
    /* The horizontal connector that runs from the parent's
       vertical edge to the first child node. */
    position: relative;
  }

  .dag-children::before {
    content: "";
    position: absolute;
    left: 8px;
    top: 0;
    width: 12px;
    height: 1px;
    background: var(--outline-variant);
  }

  .dag-node {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-sm);
    padding: var(--sp-xs) var(--sp-sm);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    background: var(--surface-container-low);
    color: var(--on-surface);
    cursor: pointer;
    font-family: inherit;
    text-align: left;
    width: 100%;
    transition: border-color 0.1s, background 0.1s;
  }

  .dag-node:hover {
    border-color: var(--primary);
  }

  .dag-node[data-hidden="true"] {
    opacity: 0.55;
  }

  .dag-node[data-cycle="true"] {
    border-color: #ff5060;
    border-style: dashed;
  }

  .dag-node[data-role="a"] {
    border-color: #4a90ff;
    background: rgba(74, 144, 255, 0.12);
  }

  .dag-node[data-role="b"] {
    border-color: #ff904a;
    background: rgba(255, 144, 74, 0.12);
  }

  .dag-node-label {
    font-size: 0.85rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dag-node-meta {
    font-size: 0.7rem;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .badge {
    padding: 1px 6px;
    border-radius: var(--radius-full);
    font-size: 0.6rem;
    color: var(--on-surface-variant);
    background: var(--surface-container-high);
  }

  .badge-empty {
    color: #ff904a;
  }

  .badge-hidden {
    color: #a0a6b3;
  }
</style>