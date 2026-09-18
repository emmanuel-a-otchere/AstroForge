<!--
  CR-07 §23.4 — Expert clipping masks panel.

  Renders the per-pixel highlight + shadow clipping
  masks for one Image Version as two stacked SVG
  visualizations. Each preview pixel is rendered as a
  small rect: clipped pixels are highlighted (red for
  highlight-clipped, deep blue for shadow-clipped);
  non-clipped pixels are rendered at very low opacity
  so the user can see the image shape without the
  clipping dominating the visual.

  The data is loaded via `getVersionClippingMasks` from
  the Tauri backend, which downsamples the image to the
  preview budget (≤256 px on the long axis) and
  computes the masks using the established codebase
  thresholds: highlight `v >= 0.99`, shadow
  `v <= 0.01`.

  Per §23 "progressive disclosure, consistent with
  CR-01": this panel only renders when the parent
  CompareWorkspace has its "Show expert details"
  toggle on.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    getVersionClippingMasks,
    type ClippingMasksSnapshot,
    type ClippingMasksData,
  } from "../lib/astroforge-api";

  type Props = {
    versionId: string;
    label?: string;
  };

  let { versionId, label }: Props = $props();

  let snapshot = $state<ClippingMasksSnapshot | null>(null);
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
      snapshot = await getVersionClippingMasks(vid);
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

  // Two stacked SVGs, each 320x160 viewBox.
  const VIEW_W = 320;
  const VIEW_H = 160;

  type Cell = {
    x: number;
    y: number;
    w: number;
    h: number;
    fill: string;
  };

  // Render a single mask as a list of cells. Clipped
  // pixels get a saturated fill; non-clipped pixels
  // get a very dim background so the user can see
  // the image shape.
  function buildCells(
    mask: number[],
    width: number,
    height: number,
    highlight: boolean,
  ): Cell[] {
    if (width === 0 || height === 0) return [];
    const cellW = VIEW_W / width;
    const cellH = VIEW_H / height;
    const out: Cell[] = [];
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const clipped = mask[y * width + x] === 1;
        let fill: string;
        if (clipped) {
          // Saturated fill for clipped pixels.
          fill = highlight ? "rgb(220, 64, 64)" : "rgb(64, 96, 220)";
        } else {
          // Dim background for non-clipped pixels so
          // the image shape is visible without
          // competing with the clipped-pixel signal.
          fill = "rgba(120, 120, 120, 0.15)";
        }
        out.push({
          x: x * cellW,
          y: y * cellH,
          w: cellW + 0.5,
          h: cellH + 0.5,
          fill,
        });
      }
    }
    return out;
  }

  let highlightCells = $derived.by(() => {
    const m = snapshot?.masks;
    if (!m) return [] as Cell[];
    return buildCells(
      m.highlightMask,
      m.width,
      m.height,
      true,
    );
  });

  let shadowCells = $derived.by(() => {
    const m = snapshot?.masks;
    if (!m) return [] as Cell[];
    return buildCells(
      m.shadowMask,
      m.width,
      m.height,
      false,
    );
  });

  function fmtPct(v: number, digits = 3): string {
    if (!Number.isFinite(v)) return "-";
    return (v * 100).toFixed(digits) + "%";
  }

  function fmtCount(v: number): string {
    return v.toString();
  }
</script>

<section
  class="expert-clipping"
  aria-labelledby="clipping-heading-{versionId}"
>
  <h4 id="clipping-heading-{versionId}" class="clipping-heading">
    Clipping masks{label ? ` - ${label}` : ""}
  </h4>

  {#if loading}
    <p class="state">Loading clipping masks...</p>
  {:else if error}
    <p class="state state-error" role="alert">
      Failed to load clipping masks: {error}
    </p>
  {:else if !snapshot || snapshot.masks.width === 0}
    <p class="state state-empty">
      No clipping data available. The image may be
      empty or the downsampling produced a zero-pixel
      preview.
    </p>
  {:else}
    {@const m = snapshot.masks}
    <div class="mask-section">
      <div class="mask-label">Highlight (`v &gt;= 0.99`)</div>
      <svg
        class="mask-svg"
        viewBox="0 0 {VIEW_W} {VIEW_H}"
        preserveAspectRatio="xMidYMid meet"
        role="img"
        aria-label="Highlight clipping mask for {label ?? versionId}"
      >
        {#each highlightCells as cell}
          <rect
            x={cell.x}
            y={cell.y}
            width={cell.w}
            height={cell.h}
            fill={cell.fill}
            stroke="none"
          />
        {/each}
      </svg>
    </div>
    <div class="mask-section">
      <div class="mask-label">Shadow (`v &lt;= 0.01`)</div>
      <svg
        class="mask-svg"
        viewBox="0 0 {VIEW_W} {VIEW_H}"
        preserveAspectRatio="xMidYMid meet"
        role="img"
        aria-label="Shadow clipping mask for {label ?? versionId}"
      >
        {#each shadowCells as cell}
          <rect
            x={cell.x}
            y={cell.y}
            width={cell.w}
            height={cell.h}
            fill={cell.fill}
            stroke="none"
          />
        {/each}
      </svg>
    </div>
    <dl class="clipping-summary">
      <div>
        <dt>Highlight count</dt>
        <dd>{fmtCount(m.highlightCount)}</dd>
      </div>
      <div>
        <dt>Highlight fraction</dt>
        <dd>{fmtPct(m.highlightFraction)}</dd>
      </div>
      <div>
        <dt>Shadow count</dt>
        <dd>{fmtCount(m.shadowCount)}</dd>
      </div>
      <div>
        <dt>Shadow fraction</dt>
        <dd>{fmtPct(m.shadowFraction)}</dd>
      </div>
    </dl>
    <p class="caption">
      Red = highlight-clipped pixels (v &gt;= 0.99),
      blue = shadow-clipped pixels (v &lt;= 0.01).
      Dim grey = non-clipped pixels (kept visible so the
      image shape is clear). Thresholds match
      `image_analysis::metrics::highlight_clipping` and
      `quality_gates::clipping`. The downsampled preview
      resolution is {m.width}x{m.height}.
    </p>
  {/if}
</section>

<style>
  .expert-clipping {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    background: var(--surface-1, #1a1a1a);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: 4px;
    padding: 0.75rem 1rem;
    min-width: 0;
  }

  .clipping-heading {
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

  .mask-section {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .mask-label {
    font-size: 0.75rem;
    color: var(--text-dim, #888);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-family: var(--font-mono, monospace);
  }

  .mask-svg {
    width: 100%;
    height: auto;
    display: block;
    background: #000;
    border-radius: 4px;
  }

  .clipping-summary {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: 0.25rem 0.75rem;
    margin: 0;
    font-family: var(--font-mono, monospace);
  }

  .clipping-summary > div {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.1rem;
  }

  .clipping-summary dt {
    font-size: 0.7rem;
    color: var(--text-dim, #888);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .clipping-summary dd {
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