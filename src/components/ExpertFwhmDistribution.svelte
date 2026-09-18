<!--
  CR-07 §23.2 — Expert FWHM distribution panel.

  Renders the per-star FWHM distribution for one
  Image Version: a histogram (SVG bar chart) plus a
  seven-number summary (count, mean, median, p25, p75,
  min, max). Data is loaded via
  `getVersionFwhmDistribution` from the Tauri backend,
  which returns a pre-binned histogram (Sturges' rule,
  capped to [1, 50] bins).

  When `count == 0` (no stars detected at the default
  3σ threshold), the histogram is replaced by a
  "No stars detected" message — the chart would
  otherwise be a degenerate empty SVG.

  Per §23 "progressive disclosure, consistent with
  CR-01": this panel only renders when the parent
  CompareWorkspace has its "Show expert details"
  toggle on.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    getVersionFwhmDistribution,
    type FwhmDistribution,
    type FwhmHistogram,
  } from "../lib/astroforge-api";

  type Props = {
    versionId: string;
    label?: string;
  };

  let { versionId, label }: Props = $props();

  let snapshot = $state<FwhmDistribution | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load(vid: string) {
    if (!vid) {
      loading = false;
      snapshot = null;
      return;
    }
    loading = true;
    error = null;
    try {
      snapshot = await getVersionFwhmDistribution(vid);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      snapshot = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void load(versionId);
  });

  onMount(() => {
    void load(versionId);
  });

  // SVG chart geometry. Width/height are viewBox units;
  // the SVG scales to the container width via CSS.
  const CHART_W = 320;
  const CHART_H = 160;
  const PAD_L = 36; // x-axis labels need ~30 px
  const PAD_R = 8;
  const PAD_T = 8;
  const PAD_B = 24; // x-axis tick labels
  const PLOT_W = CHART_W - PAD_L - PAD_R;
  const PLOT_H = CHART_H - PAD_T - PAD_B;

  // Derived: a per-bar geometry array for the SVG,
  // plus the x-axis tick labels (left edge, midpoint,
  // right edge of the FWHM range).
  type Bar = {
    x: number;
    y: number;
    w: number;
    h: number;
    label: string;
  };
  let bars = $derived.by(() => {
    const h = snapshot?.histogram;
    if (!h || h.counts.length === 0) return [] as Bar[];
    const max_count = Math.max(1, ...h.counts);
    const xMin = h.binEdges[0];
    const xMax = h.binEdges[h.binEdges.length - 1];
    const xRange = xMax - xMin || 1;
    return h.counts.map((c, i) => {
      const x0 = h.binEdges[i];
      const x1 = h.binEdges[i + 1];
      const x = PAD_L + ((x0 - xMin) / xRange) * PLOT_W;
      const w = Math.max(
        1,
        ((x1 - x0) / xRange) * PLOT_W,
      );
      const h_bar = (c / max_count) * PLOT_H;
      const y = PAD_T + (PLOT_H - h_bar);
      const label = x0.toFixed(2);
      return { x, y, w, h: h_bar, label };
    });
  });

  let xTicks = $derived.by(() => {
    const h = snapshot?.histogram;
    if (!h || h.binEdges.length === 0) return [] as { x: number; label: string }[];
    const xMin = h.binEdges[0];
    const xMax = h.binEdges[h.binEdges.length - 1];
    const xRange = xMax - xMin || 1;
    // Three ticks: left, mid, right of the FWHM range.
    return [
      { x: PAD_L, label: xMin.toFixed(2) },
      {
        x: PAD_L + PLOT_W / 2,
        label: ((xMin + xMax) / 2).toFixed(2),
      },
      {
        x: PAD_L + PLOT_W,
        label: xMax.toFixed(2),
      },
    ];
  });

  function fmt(v: number, digits = 2): string {
    if (!Number.isFinite(v)) return "—";
    return v.toFixed(digits);
  }
</script>

<section class="expert-fwhm" aria-labelledby="fwhm-heading-{versionId}">
  <h4 id="fwhm-heading-{versionId}" class="fwhm-heading">
    FWHM distribution{label ? ` — ${label}` : ""}
  </h4>

  {#if loading}
    <p class="state">Loading FWHM distribution…</p>
  {:else if error}
    <p class="state state-error" role="alert">
      Failed to load FWHM distribution: {error}
    </p>
  {:else if !snapshot || snapshot.histogram.count === 0}
    <p class="state state-empty">
      No stars detected (count: 0). The histogram is empty;
      FWHM is a per-star measurement, so a starless image
      has no distribution to show.
    </p>
  {:else}
    {@const h = snapshot.histogram}
    <svg
      class="fwhm-chart"
      viewBox="0 0 {CHART_W} {CHART_H}"
      role="img"
      aria-label="FWHM histogram for {label ?? versionId}"
    >
      <!-- Y-axis baseline + horizontal grid (top, mid, bottom). -->
      <line
        x1={PAD_L}
        y1={PAD_T + PLOT_H}
        x2={PAD_L + PLOT_W}
        y2={PAD_T + PLOT_H}
        class="axis"
      />
      {#each [0, 0.5, 1] as frac}
        {@const y = PAD_T + PLOT_H * (1 - frac)}
        <line
          x1={PAD_L}
          y1={y}
          x2={PAD_L + PLOT_W}
          y2={y}
          class="grid"
        />
      {/each}
      <!-- Y-axis label -->
      <text
        x={PAD_L - 4}
        y={PAD_T + PLOT_H}
        text-anchor="end"
        dominant-baseline="hanging"
        class="axis-label"
      >
        0
      </text>
      <text
        x={PAD_L - 4}
        y={PAD_T}
        text-anchor="end"
        dominant-baseline="hanging"
        class="axis-label"
      >
        {Math.max(...h.counts)}
      </text>
      <!-- Bars -->
      {#each bars as bar}
        <rect
          x={bar.x}
          y={bar.y}
          width={bar.w}
          height={bar.h}
          class="bar"
        >
          <title>{bar.label}: {bar.h === 0 ? 0 : Math.round((bar.h / PLOT_H) * Math.max(...h.counts))} stars</title>
        </rect>
      {/each}
      <!-- X-axis ticks -->
      {#each xTicks as tick}
        <text
          x={tick.x}
          y={PAD_T + PLOT_H + 14}
          text-anchor="middle"
          class="axis-label"
        >
          {tick.label}
        </text>
      {/each}
      <text
        x={PAD_L + PLOT_W / 2}
        y={CHART_H - 2}
        text-anchor="middle"
        class="axis-label"
      >
        FWHM (pixels)
      </text>
    </svg>
    <dl class="fwhm-summary">
      <div><dt>Count</dt><dd>{h.count}</dd></div>
      <div><dt>Mean</dt><dd>{fmt(h.mean)}</dd></div>
      <div><dt>Median</dt><dd>{fmt(h.median)}</dd></div>
      <div><dt>P25</dt><dd>{fmt(h.p25)}</dd></div>
      <div><dt>P75</dt><dd>{fmt(h.p75)}</dd></div>
      <div><dt>Min</dt><dd>{fmt(h.min)}</dd></div>
      <div><dt>Max</dt><dd>{fmt(h.max)}</dd></div>
    </dl>
    <p class="caption">
      {h.binEdges.length - 1} histogram bins
      (Sturges' rule, capped to [1, 50]). Lower FWHM
      generally indicates tighter stars; the median and
      P25/P75 are the most useful single-number
      summaries.
    </p>
  {/if}
</section>

<style>
  .expert-fwhm {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    background: var(--surface-1, #1a1a1a);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: 4px;
    padding: 0.75rem 1rem;
    min-width: 0;
  }

  .fwhm-heading {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-dim, #aaa);
    font-weight: 600;
  }

  .state {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-dim, #888);
    line-height: 1.4;
  }

  .state-error {
    color: #f87171;
  }

  .state-empty {
    font-style: italic;
  }

  .fwhm-chart {
    width: 100%;
    height: auto;
    display: block;
  }

  .axis {
    stroke: var(--text-dim, #888);
    stroke-width: 1;
  }

  .grid {
    stroke: var(--border-subtle, #2a2a2a);
    stroke-width: 1;
    stroke-dasharray: 2 2;
  }

  .axis-label {
    fill: var(--text-dim, #888);
    font-size: 9px;
    font-family: var(--font-mono, monospace);
  }

  .bar {
    fill: var(--accent, #4a8eff);
    stroke: var(--accent, #4a8eff);
    stroke-width: 0.5;
  }

  .fwhm-summary {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(70px, 1fr));
    gap: 0.25rem 0.75rem;
    margin: 0;
    font-family: var(--font-mono, monospace);
  }

  .fwhm-summary > div {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.1rem;
  }

  .fwhm-summary dt {
    font-size: 0.7rem;
    color: var(--text-dim, #888);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .fwhm-summary dd {
    margin: 0;
    font-size: 0.9rem;
    color: var(--text, #ddd);
    font-variant-numeric: tabular-nums;
  }

  .caption {
    margin: 0;
    font-size: 0.75rem;
    color: var(--text-dim, #888);
    line-height: 1.4;
  }
</style>