<script lang="ts">
  import {
    pipelineGraph,
    activeStepIndex,
    goToStep,
    type PipelineNode,
    type NodeStatus,
  } from "../lib/pipeline-store";

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

  function handleNodeClick(index: number) {
    goToStep(index);
  }

  function nodeY(index: number): number {
    return index * 72 + 16;
  }
</script>

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
          on:click={() => handleNodeClick(i)}
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
</style>
