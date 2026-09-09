<script lang="ts">
  /**
   * CR-05 P5 slice 4 — §24 Pipeline Visualization (Expert mode).
   *
   * Renders the pipeline as an SVG DAG. Honest about the data shape:
   * the current `PipelinePlan` schema stores stages in a linear
   * `sequence` order with no parent/dependency field, so the DAG is
   * *derived* from `stage_type` membership (Stack → Background/Color
   * parallel branches → Stretch, with optional Denoise/Detail after).
   * Any future schema change to add explicit parent ids would slot
   * into the `branchFor()` switch.
   */
  import type {
    PipelineStageDto,
    PipelinePlanDto,
    ProcessingMetricsDto,
  } from "../lib/astroforge-api";

  export let plan: PipelinePlanDto | null = null;
  export let metrics: ProcessingMetricsDto | null = null;
  /** Map of `stage_id` → status, populated by ProcessingControls. */
  export let stageStatus: Record<string, string> = {};

  type LayoutNode = {
    stage_id: string;
    label: string;
    type: string;
    lane: number;
    seq: number;
    status: string;
  };

  // ── DAG layout — derived from stage_type, not stored ──────────────
  // Lane 0 is the spine. Background/Color branch into lane -1 / +1
  // after Stack and rejoin into lane 0 at Stretch. Denoise/Detail
  // float to lane -1/+1 after Stretch.
  function laneFor(stage: PipelineStageDto, hasBranches: boolean): number {
    if (!hasBranches) return 0;
    switch (stage.stage_type) {
      case "background":
        return -1;
      case "color":
        return 1;
      case "denoise":
        return -1;
      case "detail":
        return 1;
      default:
        return 0;
    }
  }

  function hasBranchableStages(stages: PipelineStageDto[]): boolean {
    return stages.some(
      (s) =>
        s.stage_type === "background" ||
        s.stage_type === "color" ||
        s.stage_type === "denoise" ||
        s.stage_type === "detail",
    );
  }

  $: stages = plan?.stages ?? [];
  $: branchable = hasBranchableStages(stages);
  $: nodes = stages.map<LayoutNode>((s) => ({
    stage_id: s.stage_id,
    label: s.label || s.stage_type,
    type: s.stage_type,
    lane: laneFor(s, branchable),
    seq: s.sequence,
    status: stageStatus[s.stage_id] ?? "pending",
  }));

  // ── Geometry constants (px) — keep generous so labels are readable.
  const NODE_W = 160;
  const NODE_H = 56;
  const LANE_SPACING = 110;
  const LANE_X = 40;
  const X_STEP = 220;
  const Y_BASE = 80;
  const SVG_W = (n: number) => LANE_X * 2 + X_STEP * Math.max(n, 1);
  const SVG_H = 220;

  function xOf(seq: number): number {
    return LANE_X * 2 + seq * X_STEP - NODE_W / 2;
  }
  function yOf(lane: number): number {
    return Y_BASE + lane * LANE_SPACING;
  }

  function statusGlyph(s: string): string {
    switch (s) {
      case "completed":
        return "✓";
      case "running":
        return "▶";
      case "failed":
        return "✗";
      case "skipped":
        return "—";
      default:
        return "○";
    }
  }

  function statusColor(s: string): string {
    switch (s) {
      case "completed":
        return "var(--accent-success, #7fb069)";
      case "running":
        return "var(--accent-primary, #82a0bc)";
      case "failed":
        return "var(--accent-error, #d96c5c)";
      default:
        return "var(--on-surface-muted, #8a8a8a)";
    }
  }

  // ── Edge computation — connect each node to the next one in the
  // same lane, and from the spine into Background/Color after Stack,
  // back from both into Stretch, then from Stretch into Denoise/Detail.
  $: edges = (() => {
    const out: Array<{
      d: string;
      key: string;
    }> = [];
    if (nodes.length < 2) return out;
    for (let i = 0; i < nodes.length - 1; i++) {
      const a = nodes[i];
      const b = nodes[i + 1];
      // Skip pairs where b is a branch start (handled separately).
      if (b.lane !== 0 && b.lane !== a.lane) {
        // Continue — fan-out handled below.
      }
      // Default forward edge along the same lane.
      const ax = xOf(a.seq) + NODE_W;
      const ay = yOf(a.lane) + NODE_H / 2;
      const bx = xOf(b.seq);
      const by = yOf(b.lane) + NODE_H / 2;
      const midX = (ax + bx) / 2;
      out.push({
        key: `${a.stage_id}->${b.stage_id}`,
        d: `M ${ax} ${ay} C ${midX} ${ay}, ${midX} ${by}, ${bx} ${by}`,
      });
    }
    return out;
  })();

  $: completionPct = metrics
    ? Math.round(metrics.completion_ratio * 100)
    : 0;
</script>

<section class="expert-dag" aria-label="Pipeline DAG (Expert mode)">
  <header class="dag-header">
    <h3>Pipeline DAG</h3>
    {#if metrics}
      <span class="completion" data-testid="dag-completion">
        {metrics.completed_count} / {metrics.stage_count} stages
        ({completionPct}%)
      </span>
    {/if}
  </header>

  {#if nodes.length === 0}
    <p class="empty font-body">No pipeline loaded yet.</p>
  {:else}
    <div class="dag-canvas">
      <svg
        role="img"
        aria-label="Pipeline DAG showing {nodes.length} stages"
        width={SVG_W(nodes.length)}
        height={SVG_H}
        viewBox={`0 0 ${SVG_W(nodes.length)} ${SVG_H}`}
      >
        <!-- Lane guides (subtle horizontal lines for each lane) -->
        {#each Array.from({ length: 3 }, (_, i) => i - 1) as lane}
          {#if branchable}
            <line
              x1={LANE_X}
              x2={SVG_W(nodes.length) - LANE_X}
              y1={yOf(lane)}
              y2={yOf(lane)}
              class="lane-guide"
            />
          {/if}
        {/each}

        <!-- Edges -->
        {#each edges as edge}
          <path d={edge.d} class="dag-edge" />
        {/each}

        <!-- Nodes -->
        {#each nodes as node (node.stage_id)}
          <g
            class="dag-node"
            data-stage-id={node.stage_id}
            data-stage-type={node.type}
            transform={`translate(${xOf(node.seq)}, ${yOf(node.lane)})`}
          >
            <rect
              width={NODE_W}
              height={NODE_H}
              rx="8"
              ry="8"
              class="dag-node-bg"
              style={`stroke: ${statusColor(node.status)};`}
            />
            <text x="12" y="20" class="dag-node-label">
              {node.label}
            </text>
            <text x="12" y="38" class="dag-node-type">
              {node.type}
            </text>
            <text
              x={NODE_W - 14}
              y={NODE_H - 10}
              text-anchor="end"
              class="dag-node-status"
              style={`fill: ${statusColor(node.status)};`}
            >
              {statusGlyph(node.status)}
            </text>
          </g>
        {/each}
      </svg>
    </div>

    {#if metrics?.adaptive_parameters?.noise}
      <aside class="recommendation font-body" data-testid="dag-recommendation">
        <strong>Recommendation:</strong>
        {metrics.adaptive_parameters.noise.label} noise reduction
        (strength {metrics.adaptive_parameters.noise.recommended_strength.toFixed(2)}).
        <span class="reason">{metrics.adaptive_parameters.noise.reason}</span>
      </aside>
    {/if}
  {/if}
</section>

<style>
  .expert-dag {
    border: 1px solid var(--border-muted, #2e2e2e);
    border-radius: 8px;
    padding: 1rem;
    margin-top: 1rem;
    background: var(--surface-elevated, #1a1a1a);
  }
  .dag-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }
  .dag-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--on-surface, #e6e6e6);
  }
  .completion {
    font-size: 0.85rem;
    color: var(--on-surface-muted, #8a8a8a);
    font-variant-numeric: tabular-nums;
  }
  .empty {
    color: var(--on-surface-muted, #8a8a8a);
    font-style: italic;
  }
  .dag-canvas {
    overflow-x: auto;
    padding: 0.25rem 0 0.5rem;
  }
  svg {
    display: block;
  }
  .lane-guide {
    stroke: var(--border-muted, #2e2e2e);
    stroke-width: 1;
    stroke-dasharray: 2 4;
    opacity: 0.5;
  }
  .dag-edge {
    fill: none;
    stroke: var(--accent-primary, #82a0bc);
    stroke-width: 1.5;
    opacity: 0.7;
  }
  .dag-node-bg {
    fill: var(--surface-base, #141414);
    stroke-width: 2;
  }
  .dag-node-label {
    fill: var(--on-surface, #e6e6e6);
    font-size: 0.85rem;
    font-weight: 600;
  }
  .dag-node-type {
    fill: var(--on-surface-muted, #8a8a8a);
    font-size: 0.7rem;
    font-family: ui-monospace, monospace;
  }
  .dag-node-status {
    font-size: 1.1rem;
    font-weight: 700;
  }
  .recommendation {
    margin-top: 0.75rem;
    padding: 0.6rem 0.8rem;
    background: var(--accent-info-bg, #1a2a3a);
    border-left: 3px solid var(--accent-primary, #82a0bc);
    border-radius: 4px;
    color: var(--on-surface, #e6e6e6);
    font-size: 0.85rem;
    line-height: 1.4;
  }
  .reason {
    display: block;
    margin-top: 0.25rem;
    color: var(--on-surface-muted, #8a8a8a);
    font-size: 0.8rem;
  }
</style>
