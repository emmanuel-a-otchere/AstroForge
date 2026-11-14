<!--
  CR-10 P1.6.3.3 — the free-form node canvas (ADR-0018: hand-rolled
  SVG helper, zero graph-library deps; the pipeline graph is a
  linear sequence of ~20 stages, well under the spike's 200-node
  budget).

  Renders the active plan's stages as draggable cards wired by
  bezier connectors in sequence order. Drag moves nodes (positions
  persist to localStorage keyed by plan id); wheel zooms (clamped
  0.5-2x); background drag pans. Mutations stay in the list view —
  the canvas is visualization + arrangement, so one editing surface
  owns state.
-->
<script lang="ts">
  import {
    bezierPath,
    clampNodePos,
    clampZoom,
    defaultLayout,
    NODE_HEIGHT,
    NODE_WIDTH,
    screenToWorld,
    viewTransform,
    type CanvasNode,
    type Point,
  } from "../lib/node-canvas-logic";
  import { activePlan } from "../lib/pipeline-plan-store";

  let pan = $state<Point>({ x: 0, y: 0 });
  let zoom = $state(1);
  let arranged = $state<Record<string, Point>>({});
  let arrangedPlanId = $state<string | null>(null);
  // Drag state: which node, and the grab offset in world coords.
  let draggingId = $state<string | null>(null);
  let dragOffset = $state<Point>({ x: 0, y: 0 });
  let panning = $state(false);
  let panStart = $state<Point>({ x: 0, y: 0 });
  let svgEl: SVGSVGElement | null = $state(null);

  let plan = $derived($activePlan);
  let stages = $derived(
    plan
      ? [...plan.stages].sort((a, b) => a.sequence - b.sequence)
      : [],
  );

  // Load the saved arrangement when the plan changes.
  $effect(() => {
    const pid = plan?.plan_id ?? null;
    if (pid === arrangedPlanId) return;
    arrangedPlanId = pid;
    arranged = pid ? loadArrangement(pid) : {};
    // Reset the view for the new plan so the default column shows.
    pan = { x: 0, y: 0 };
    zoom = 1;
  });

  function loadArrangement(planId: string): Record<string, Point> {
    try {
      const raw = localStorage.getItem(`node-canvas-${planId}`);
      return raw ? (JSON.parse(raw) as Record<string, Point>) : {};
    } catch {
      return {};
    }
  }

  function saveArrangement(planId: string) {
    try {
      localStorage.setItem(
        `node-canvas-${planId}`,
        JSON.stringify(arranged),
      );
    } catch {
      /* storage full or unavailable — arrangement is best-effort */
    }
  }

  let nodes = $derived(defaultLayout(stages, arranged));
  let nodeById = $derived(new Map(nodes.map((n) => [n.stage_id, n])));
  let wires = $derived(
    nodes.slice(0, -1).map((n, i) => ({
      d: bezierPath(n, nodes[i + 1]),
      key: `${n.stage_id}->${nodes[i + 1].stage_id}`,
    })),
  );

  function onNodePointerDown(event: PointerEvent, node: CanvasNode) {
    if (!svgEl) return;
    event.stopPropagation();
    (event.target as Element).setPointerCapture?.(event.pointerId);
    const world = screenToWorld(event.clientX, event.clientY, pan, zoom);
    draggingId = node.stage_id;
    dragOffset = { x: world.x - node.x, y: world.y - node.y };
  }

  function onBackgroundPointerDown(event: PointerEvent) {
    panning = true;
    panStart = { x: event.clientX - pan.x, y: event.clientY - pan.y };
  }

  function onPointerMove(event: PointerEvent) {
    if (draggingId && plan) {
      const world = screenToWorld(event.clientX, event.clientY, pan, zoom);
      const next = clampNodePos(
        world.x - dragOffset.x,
        world.y - dragOffset.y,
        2000,
        4000,
      );
      arranged = { ...arranged, [draggingId]: next };
    } else if (panning) {
      pan = {
        x: event.clientX - panStart.x,
        y: event.clientY - panStart.y,
      };
    }
  }

  function onPointerUp() {
    if (draggingId && plan) saveArrangement(plan.plan_id);
    draggingId = null;
    panning = false;
  }

  function onWheel(event: WheelEvent) {
    event.preventDefault();
    const factor = event.deltaY < 0 ? 1.1 : 1 / 1.1;
    zoom = clampZoom(zoom * factor);
  }

  function resetView() {
    arranged = {};
    if (plan) saveArrangement(plan.plan_id);
    pan = { x: 0, y: 0 };
    zoom = 1;
  }

  function nodeFill(node: CanvasNode): string {
    if (!node.enabled) return "var(--surface-container)";
    return "var(--surface-container-low)";
  }
</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} />

<section class="node-canvas" aria-label="Pipeline node canvas (free-form)">
  <div class="canvas-toolbar">
    <span class="hint font-body">drag cards · wheel zooms · background pans</span>
    <button type="button" class="tool-btn font-body" onclick={resetView}>
      Reset layout
    </button>
  </div>
  {#if !plan || stages.length === 0}
    <p class="empty font-body">
      No stages yet — switch to the list to insert from the palette.
    </p>
  {:else}
    <!-- viewBox mirrors the component size; the g carries pan/zoom. -->
    <svg
      bind:this={svgEl}
      class="canvas"
      viewBox="0 0 1000 700"
      role="img"
      aria-label="Pipeline graph"
      onpointerdown={onBackgroundPointerDown}
      onwheel={onWheel}
    >
      <g transform={viewTransform(pan, zoom)}>
        {#each wires as w (w.key)}
          <path class="wire" d={w.d} />
        {/each}
        {#each nodes as node (node.stage_id)}
          <g
            class="node"
            class:node-disabled={!node.enabled}
            transform="translate({node.x} {node.y})"
            onpointerdown={(e) => onNodePointerDown(e, node)}
          >
            <rect
              width={NODE_WIDTH}
              height={NODE_HEIGHT}
              rx="8"
              fill={nodeFill(node)}
              stroke={draggingId === node.stage_id
                ? "var(--cobalt-accent)"
                : "var(--outline-variant)"}
            />
            <text class="node-label" x="12" y="21">{node.label}</text>
            <text class="node-type" x="12" y="38">{node.stage_type}</text>
            {#if !node.enabled}
              <text class="node-off" x={NODE_WIDTH - 12} y="21" text-anchor="end"
                >off</text
              >
            {/if}
          </g>
        {/each}
      </g>
    </svg>
  {/if}
</section>

<style>
  .node-canvas {
    margin-top: var(--sp-md);
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .canvas-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-sm);
  }

  .hint {
    color: var(--on-surface-variant);
    font-size: 0.75rem;
  }

  .tool-btn {
    appearance: none;
    border: 1px solid var(--outline-variant);
    background: var(--surface-container);
    color: var(--on-surface);
    border-radius: var(--radius-default);
    padding: 3px var(--sp-sm);
    font-size: 0.75rem;
    cursor: pointer;
  }

  .tool-btn:hover {
    border-color: var(--cobalt-accent);
  }

  .empty {
    color: var(--on-surface-variant);
    font-size: 0.9rem;
  }

  .canvas {
    width: 100%;
    height: 420px;
    background-color: var(--surface-container-lowest);
    background-image:
      radial-gradient(1px 1px at 12% 22%, var(--star-color-58) 50%, transparent 51%),
      radial-gradient(1px 1px at 55% 68%, var(--star-color-67) 50%, transparent 51%),
      radial-gradient(1px 1px at 84% 30%, var(--star-color-58) 50%, transparent 51%),
      radial-gradient(1px 1px at 30% 85%, var(--star-color-83) 50%, transparent 51%);
    background-size: 260px 260px;
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    touch-action: none;
    cursor: grab;
    user-select: none;
  }

  .canvas:active {
    cursor: grabbing;
  }

  .wire {
    fill: none;
    stroke: var(--outline);
    stroke-width: 2;
  }

  .node {
    cursor: grab;
  }

  .node-disabled {
    opacity: 0.55;
  }

  .node-label {
    fill: var(--on-surface);
    font-size: 14px;
    font-family: var(--font-body);
    font-weight: 600;
    pointer-events: none;
  }

  .node-type {
    fill: var(--on-surface-variant);
    font-size: 11px;
    font-family: var(--font-data);
    pointer-events: none;
  }

  .node-off {
    fill: var(--on-surface-variant);
    font-size: 11px;
    font-family: var(--font-label);
    text-transform: uppercase;
    pointer-events: none;
  }
</style>
