<!--
  CR-07 §23.3 — Expert noise map panel.

  Renders the per-pixel local sigma field for one Image
  Version as an SVG heatmap. Each cell's fill colour is
  interpolated between blue (low sigma = quiet) and red
  (high sigma = noisy) on the [min, max] range of the
  field. Below the heatmap: three summary stats (min,
  mean, max) and a one-line legend.

  The data is loaded via `getVersionNoiseMap` from the
  Tauri backend, which downsamples the image to the
  preview budget and computes the per-pixel sigma
  field using the same MAD-on-residuals algorithm as the
  scalar `luminance_noise` metric, but applied per-pixel.

  Per §23 "progressive disclosure, consistent with
  CR-01": this panel only renders when the parent
  CompareWorkspace has its "Show expert details"
  toggle on.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    getVersionNoiseMap,
    type NoiseMapSnapshot,
    type NoiseMapData,
  } from "../lib/astroforge-api";

  type Props = {
    versionId: string;
    label?: string;
  };

  let { versionId, label }: Props = $props();

  let snapshot = $state<NoiseMapSnapshot | null>(null);
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
      snapshot = await getVersionNoiseMap(vid);
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

  // Heatmap geometry. We render one SVG <rect> per pixel
  // of the sigma field (max 256x256 = 65,536 rects, well
  // within reasonable SVG render budgets). The grid is
  // scaled to fit a 320x320 viewBox.
  const VIEW_W = 320;
  const VIEW_H = 320;

  // Color ramp: blue (low) -> green -> red (high).
  // Returns CSS rgb() string for a normalized t in [0, 1].
  function heatColor(t: number): string {
    const clamped = Math.max(0, Math.min(1, t));
    // Two-stop linear interpolation:
    //   t=0   -> rgb(28, 92, 196)   deep blue
    //   t=0.5 -> rgb(48, 160, 80)   mid green
    //   t=1   -> rgb(196, 64, 64)   deep red
    if (clamped < 0.5) {
      const u = clamped * 2;
      const r = Math.round(28 + (48 - 28) * u);
      const g = Math.round(92 + (160 - 92) * u);
      const b = Math.round(196 + (80 - 196) * u);
      return `rgb(${r}, ${g}, ${b})`;
    }
    const u = (clamped - 0.5) * 2;
    const r = Math.round(48 + (196 - 48) * u);
    const g = Math.round(160 + (64 - 160) * u);
    const b = Math.round(80 + (64 - 80) * u);
    return `rgb(${r}, ${g}, ${b})`;
  }

  // Derived: an array of {x, y, w, h, color, sigma} for
  // each inner pixel of the sigma field. The outer ring
  // (3 pixels on each side) is zeroed and not drawn.
  type Cell = {
    x: number;
    y: number;
    w: number;
    h: number;
    color: string;
    sigma: number;
  };
  let cells = $derived.by(() => {
    const m = snapshot?.map;
    if (!m || m.sigma.length === 0 || m.width === 0 || m.height === 0) {
      return [] as Cell[];
    }
    const cellW = VIEW_W / m.width;
    const cellH = VIEW_H / m.height;
    const range = m.max - m.min || 1;
    const out: Cell[] = [];
    for (let y = 0; y < m.height; y++) {
      for (let x = 0; x < m.width; x++) {
        const s = m.sigma[y * m.width + x];
        // Skip zeroed outer-ring pixels (the
        // half-window border where the local
        // estimator is not stable).
        if (s === 0 && x < 3 && y < 3) continue;
        if (s === 0 && x >= m.width - 3) continue;
        if (s === 0 && y >= m.height - 3) continue;
        const t = (s - m.min) / range;
        out.push({
          x: x * cellW,
          y: y * cellH,
          w: cellW + 0.5,
          h: cellH + 0.5,
          color: heatColor(t),
          sigma: s,
        });
      }
    }
    return out;
  });

  function fmt(v: number, digits = 4): string {
    if (!Number.isFinite(v)) return "-";
    return v.toFixed(digits);
  }
</script>

<section
  class="expert-noise"
  aria-labelledby="noise-heading-{versionId}"
>
  <h4 id="noise-heading-{versionId}" class="noise-heading">
    Noise map{label ? ` - ${label}` : ""}
  </h4>

  {#if loading}
    <p class="state">Loading noise map...</p>
  {:else if error}
    <p class="state state-error" role="alert">
      Failed to load noise map: {error}
    </p>
  {:else if !snapshot || snapshot.map.sigma.length === 0}
    <p class="state state-empty">
      Not enough pixels for a noise map. The image must
      be at least 7x7 after downsampling to compute a
      stable local sigma estimate.
    </p>
  {:else}
    {@const m = snapshot.map}
    <div class="heatmap-wrap">
      <svg
        class="heatmap"
        viewBox="0 0 {VIEW_W} {VIEW_H}"
        role="img"
        aria-label="Noise map heatmap for {label ?? versionId}"
        preserveAspectRatio="xMidYMid meet"
      >
        {#each cells as cell}
          <rect
            x={cell.x}
            y={cell.y}
            width={cell.w}
            height={cell.h}
            fill={cell.color}
            stroke="none"
          >
            <title>{fmt(cell.sigma)}</title>
          </rect>
        {/each}
      </svg>
    </div>
    <dl class="noise-summary">
      <div><dt>Min sigma</dt><dd>{fmt(m.min)}</dd></div>
      <div><dt>Mean sigma</dt><dd>{fmt(m.mean)}</dd></div>
      <div><dt>Max sigma</dt><dd>{fmt(m.max)}</dd></div>
      <div><dt>Resolution</dt><dd>{m.width}x{m.height}</dd></div>
    </dl>
    <p class="caption">
      {m.width}x{m.height} per-pixel local sigma field
      (7x7 MAD-on-residuals estimator). Blue = quiet
      (low sigma), red = noisy (high sigma); the colour
      ramp is normalized to [min, max] of the field, so
      the brightest red is the noisiest pixel and the
      deepest blue is the quietest.
    </p>
  {/if}
</section>

<style>
  .expert-noise {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    background: var(--surface-1, #1a1a1a);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: 4px;
    padding: 0.75rem 1rem;
    min-width: 0;
  }

  .noise-heading {
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

  .heatmap-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    background: #000;
    border-radius: 4px;
    overflow: hidden;
  }

  .heatmap {
    width: 100%;
    height: auto;
    display: block;
  }

  .noise-summary {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(80px, 1fr));
    gap: 0.25rem 0.75rem;
    margin: 0;
    font-family: var(--font-mono, monospace);
  }

  .noise-summary > div {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.1rem;
  }

  .noise-summary dt {
    font-size: 0.7rem;
    color: var(--text-dim, #888);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .noise-summary dd {
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