<script lang="ts">
  import {
    pipelineGraph,
    activeStepIndex,
    goToStep,
    reapplyStage,
    isDestructiveStage,
    type PipelineNode,
    type NodeStatus,
    type PipelineStageType,
  } from "../lib/pipeline-store";
  import DestructiveConfirmDialog from "./DestructiveConfirmDialog.svelte";

  $: nodes = $pipelineGraph.nodes;
  $: edges = $pipelineGraph.edges;
  $: activeIdx = $activeStepIndex;

  const statusColors: Record<NodeStatus, string> = {
    pending: "var(--on-surface-variant)",
    running: "var(--cobalt-accent)",
    completed: "var(--tertiary-container)",
    failed: "var(--error)",
    skipped: "var(--outline-variant)",
    active: "var(--cobalt-accent)",
  };

  // Context menu state. Set by right-click / long-press on a node card;
  // cleared on outside-click or Escape. Only one menu open at a time.
  let menuNodeId: string | null = null;
  let menuX = 0;
  let menuY = 0;

  // M7 T4: pending re-apply awaiting destructive-stage confirmation.
  let pendingReapplyNodeId: string | null = null;
  $: pendingReapplyNode = pendingReapplyNodeId
    ? nodes.find((n) => n.id === pendingReapplyNodeId) ?? null
    : null;
  $: pendingReapplyType = pendingReapplyNode?.type ?? null;
  $: pendingReapplyLabel = pendingReapplyNode?.label ?? "";

  function openMenu(nodeId: string, event: MouseEvent) {
    event.preventDefault();
    const idx = nodes.findIndex((n) => n.id === nodeId);
    if (idx < 0) return;
    // Anchor the menu near the clicked card so it appears adjacent.
    const card = (event.currentTarget as HTMLElement).getBoundingClientRect();
    menuNodeId = nodeId;
    menuX = card.right + 4;
    menuY = card.top;
  }

  function closeMenu() {
    menuNodeId = null;
  }

  function handleReapply(nodeId: string) {
    const idx = nodes.findIndex((n) => n.id === nodeId);
    if (idx < 0) return;
    // M7 T4: gate destructive stages behind the confirmation modal.
    if (isDestructiveStage(nodes[idx].type)) {
      pendingReapplyNodeId = nodeId;
      closeMenu();
      return;
    }
    if (reapplyStage(nodeId)) {
      closeMenu();
    }
  }

  function handleReapplyConfirm() {
    if (!pendingReapplyNodeId) return;
    const id = pendingReapplyNodeId;
    pendingReapplyNodeId = null;
    reapplyStage(id);
  }

  function handleReapplyCancel() {
    pendingReapplyNodeId = null;
  }

  function handleNodeClick(index: number) {
    goToStep(index);
  }

  function nodeY(index: number): number {
    return index * 72 + 16;
  }
</script>

<svelte:window
  onclick={(e) => {
    // Close the context menu on any window click. The menu's own
    // onclick handlers stopPropagation so they don't trip this.
    if (menuNodeId !== null) {
      const target = e.target as HTMLElement | null;
      if (target?.closest(".node-context-menu")) return;
      closeMenu();
    }
  }}
  onkeydown={(e) => {
    if (e.key === "Escape" && menuNodeId !== null) {
      closeMenu();
    }
  }}
/>

<div class="node-sidebar">
  <div class="sidebar-header">
    <span class="text-label-caps">Pipeline Graph</span>
  </div>
  <div class="graph-container">
    <svg class="edges-layer" viewBox="0 0 200 {nodes.length * 72 + 32}">
      {#each edges as edge}
        {@const fromIdx = nodes.findIndex(n => n.id === edge.from)}
        {@const toIdx = nodes.findIndex(n => n.id === edge.to)}
        {#if fromIdx >= 0 && toIdx >= 0}
          <line
            x1="100"
            y1={nodeY(fromIdx) + 24}
            x2="100"
            y2={nodeY(toIdx) + 24}
            stroke={nodes[fromIdx].status === "completed" ? "var(--tertiary-container)" : "var(--outline-variant)"}
            stroke-width="1.5"
          />
        {/if}
      {/each}
    </svg>
    <div class="nodes-layer">
      {#each nodes as node, i}
        <button
          class="node-card"
          class:active={i === activeIdx}
          class:completed={node.status === "completed"}
          style="--node-color: {statusColors[node.status]}"
          onclick={() => handleNodeClick(i)}
          oncontextmenu={(e) => openMenu(node.id, e)}
          type="button"
        >
          <span class="node-led" style="background: {statusColors[node.status]}"></span>
          <div class="node-info">
            <span class="node-num text-label-caps">{i + 1}</span>
            <span class="node-label text-body">{node.label}</span>
          </div>
          {#if node.status === "completed"}
            <span class="material-symbols-outlined node-check">check_circle</span>
          {/if}
        </button>
      {/each}
    </div>
  </div>
</div>

{#if menuNodeId !== null}
  <div
    class="node-context-menu"
    role="menu"
    style="left: {menuX}px; top: {menuY}px"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <button
      type="button"
      class="menu-item"
      role="menuitem"
      onclick={() => handleReapply(menuNodeId!)}
    >
      <span class="material-symbols-outlined">restart_alt</span>
      Re-run from here
    </button>
  </div>
{/if}

{#if pendingReapplyNodeId !== null && pendingReapplyType !== null}
  <DestructiveConfirmDialog
    stageType={pendingReapplyType}
    stageLabel={pendingReapplyLabel}
    action="reapply"
    onConfirm={handleReapplyConfirm}
    onCancel={handleReapplyCancel}
  />
{/if}

<style>
  .node-sidebar {
    width: 240px;
    height: 100%;
    background: var(--surface-container);
    border-right: 1px solid var(--outline-variant);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .sidebar-header {
    padding: var(--sp-md);
    border-bottom: 1px solid var(--outline-variant);
  }

  .graph-container {
    flex: 1;
    position: relative;
    overflow-y: auto;
    padding: var(--sp-sm);
  }

  .edges-layer {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 0;
  }

  .nodes-layer {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 8px;
  }

  .node-card {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: 10px 12px;
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: left;
    width: 100%;
  }

  .node-card:hover {
    background: var(--surface-container-high);
    border-color: var(--node-color);
  }

  .node-card.active {
    border-color: var(--cobalt-accent);
    background: var(--surface-container-high);
    box-shadow: 0 0 8px var(--glow-cobalt-mid);
  }

  .node-card.completed {
    border-color: var(--tertiary-container);
  }

  .node-led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 4px 1px var(--node-color);
  }

  .node-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .node-num {
    color: var(--on-surface-variant);
    font-size: 10px;
  }

  .node-label {
    color: var(--on-surface);
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .node-check {
    font-size: 16px;
    color: var(--tertiary-container);
    flex-shrink: 0;
  }

  .node-context-menu {
    position: fixed;
    z-index: 300;
    min-width: 180px;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default, 8px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    padding: 4px;
    display: flex;
    flex-direction: column;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: none;
    border: none;
    color: var(--on-surface);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    border-radius: var(--radius-default, 6px);
  }

  .menu-item:hover {
    background: var(--surface-container-highest, var(--cobalt-accent));
    color: var(--on-surface);
  }

  .menu-item .material-symbols-outlined {
    font-size: 16px;
  }
</style>
