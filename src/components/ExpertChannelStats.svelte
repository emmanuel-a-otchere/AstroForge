<!--
  CR-07 §23.1 — Expert channel statistics panel.

  Renders the per-channel (R, G, B, ...) statistics for one
  Image Version: mean, stddev, min, max, and clip count. The
  data is loaded via `getVersionMetricSnapshot(versionId)` and
  the component filters the returned `metrics[]` array for
  keys starting with `channel.`. The remaining (detector-backed)
  keys are ignored here — they're consumed by `MetricsTable`.

  The panel is read-only: it surfaces a finding, it does not
  recommend an action. The §21 "no AI Winner" invariant applies
  (the panel does not call any side "best" or "preferred"; the
  user makes the comparison call).

  Svelte 5 syntax: `let { versionId } = $props()` for the
  version id (string). The component takes no other inputs and
  has no emit outputs.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { getVersionMetricSnapshot } from "../lib/astroforge-api";
  import type { VersionMetricSnapshot, VersionMetricEntry } from "../lib/astroforge-api";

  interface ChannelRow {
    /** R, G, B, or c{n} for the 4th-and-beyond channels. */
    label: string;
    mean: number;
    stddev: number;
    min: number;
    max: number;
    clip_count: number;
  }

  let { versionId, label = "Version" } = $props<{
    versionId: string;
    label?: string;
  }>();

  let loading = $state(false);
  let error = $state<string | null>(null);
  let width = $state(0);
  let height = $state(0);
  let channels = $state(0);
  let rows = $state<ChannelRow[]>([]);

  /**
   * Filter the snapshot's `metrics[]` down to the per-channel
   * stats. The keys are `channel.<r|g|b|c{n}>.<stat>`; the
   * mapping builds a row per channel label, defaulting missing
   * stat keys to 0 (so a missing column surfaces as "—" rather
   * than crashing the render).
   */
  function buildRows(metrics: VersionMetricEntry[]): ChannelRow[] {
    // Group the entries by channel label.
    const byChannel = new Map<string, Partial<ChannelRow>>();
    for (const entry of metrics) {
      if (!entry.key.startsWith("channel.")) continue;
      // key shape: "channel.<label>.<stat>"
      const parts = entry.key.split(".");
      if (parts.length !== 3) continue;
      const chLabel = parts[1];
      const stat = parts[2];
      const existing = byChannel.get(chLabel) ?? { label: chLabel };
      const value = entry.value;
      switch (stat) {
        case "mean":
          existing.mean = value;
          break;
        case "stddev":
          existing.stddev = value;
          break;
        case "min":
          existing.min = value;
          break;
        case "max":
          existing.max = value;
          break;
        case "clip_count":
          existing.clip_count = value;
          break;
      }
      byChannel.set(chLabel, existing);
    }
    // Order: R, G, B, then c3, c4, ... in numeric order. The
    // Rust side emits keys alphabetically (BTreeMap), so a
    // plain "r, g, b, c3, c4" sort is correct for the common
    // 3-channel case. For > 3 channels the c{n} numeric sort
    // requires a custom comparator.
    const labels = Array.from(byChannel.keys());
    labels.sort((a, b) => {
      const order = ["r", "g", "b"];
      const ia = order.indexOf(a);
      const ib = order.indexOf(b);
      if (ia !== -1 && ib !== -1) return ia - ib;
      if (ia !== -1) return -1;
      if (ib !== -1) return 1;
      // Both c-prefixed: extract the numeric part.
      const na = parseInt(a.slice(1), 10);
      const nb = parseInt(b.slice(1), 10);
      return na - nb;
    });
    return labels.map((label) => {
      const row = byChannel.get(label)!;
      return {
        label: label.toUpperCase(),
        mean: row.mean ?? 0,
        stddev: row.stddev ?? 0,
        min: row.min ?? 0,
        max: row.max ?? 0,
        clip_count: row.clip_count ?? 0,
      };
    });
  }

  async function load() {
    if (!versionId) return;
    loading = true;
    error = null;
    try {
      const snap: VersionMetricSnapshot = await getVersionMetricSnapshot(
        versionId,
      );
      rows = buildRows(snap.metrics);
      width = snap.width;
      height = snap.height;
      channels = snap.channels;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      rows = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Re-load when the version id changes.
    void versionId;
    load();
  });

  onMount(load);

  /** Format a 0..1 normalized value as a 3-decimal fraction. */
  function fmt(v: number): string {
    return v.toFixed(3);
  }

  /** Format the clip count as a percent of total pixels. */
  function fmtClip(count: number): string {
    const total = width * height;
    if (total === 0) return "—";
    const pct = (count / total) * 100;
    return `${count.toLocaleString()} (${pct.toFixed(2)}%)`;
  }
</script>

<section class="expert-channel-stats" aria-label="Expert channel statistics">
  <header>
    <h4>Channel statistics ({label})</h4>
    {#if width > 0 && channels > 0}
      <p class="meta">
        {width}×{height} · {channels} channel{channels === 1 ? "" : "s"}
      </p>
    {/if}
  </header>

  {#if loading}
    <p class="empty">Loading channel statistics…</p>
  {:else if error}
    <p class="error" role="alert">Failed to load: {error}</p>
  {:else if rows.length === 0}
    <p class="empty">
      No channel statistics available for this version.
    </p>
  {:else}
    <table>
      <thead>
        <tr>
          <th scope="col">Channel</th>
          <th scope="col">Mean</th>
          <th scope="col">Stddev</th>
          <th scope="col">Min</th>
          <th scope="col">Max</th>
          <th scope="col">Clipped pixels</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row}
          <tr>
            <th scope="row">{row.label}</th>
            <td>{fmt(row.mean)}</td>
            <td>{fmt(row.stddev)}</td>
            <td>{fmt(row.min)}</td>
            <td>{fmt(row.max)}</td>
            <td>{fmtClip(row.clip_count)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="footnote">
      Mean, stddev, min, and max are normalized to the 0.0–1.0
      range. "Clipped pixels" are the count of pixels at or above
      1.0 (highlight clipping threshold) in that channel.
    </p>
  {/if}
</section>

<style>
  .expert-channel-stats {
    background: var(--surface-1, #1a1a1a);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: 6px;
    padding: 1rem 1.25rem;
    margin-top: 1rem;
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }

  h4 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }

  .meta {
    margin: 0;
    font-size: 0.8rem;
    color: var(--text-dim, #888);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  th,
  td {
    text-align: left;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--border-subtle, #2a2a2a);
  }

  thead th {
    font-weight: 600;
    color: var(--text-dim, #888);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  tbody th[scope="row"] {
    font-weight: 600;
    width: 4rem;
  }

  .empty,
  .error {
    margin: 0;
    color: var(--text-dim, #888);
    font-size: 0.85rem;
  }

  .error {
    color: var(--error, #d05050);
  }

  .footnote {
    margin: 0.75rem 0 0;
    color: var(--text-dim, #888);
    font-size: 0.75rem;
    line-height: 1.4;
  }
</style>
