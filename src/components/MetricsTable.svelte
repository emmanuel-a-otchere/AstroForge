<!--
  CR-07 B4 metrics table (§10 delta table + §11 summary +
  §9 contextual metric display).

  Consumes `compare_version_metrics`: the backend decodes both
  versions' applied artifacts, derives the §8 metric snapshots via
  the shipped `image_analysis::metrics` detectors, and returns
  B2's `MetricDeltaRow` table plus the §11 natural-language
  summary.

  Honesty rules:
  - Metrics without a shipped detector render "—" (the backend
    reports them as inconclusive, null-valued rows).
  - When either version has no applied artifact, the panel says so
    instead of rendering a fake table.
  - §9: every metric name is rendered with its contextual
    sentence visible inline (not just a hover tooltip). The
    `row.context` field is populated for every metric by
    `crates/astroforge-core/src/metric_registry.rs:228` (25
    metrics, lines 246-467). The sub-text line under each
    metric name surfaces the "what does this mean + why it
    matters" context that the spec requires.
-->
<script lang="ts">
  import {
    compareVersionMetrics,
    type DeltaDirection,
    type VersionMetricsComparison,
  } from "../lib/astroforge-api";

  interface Props {
    versionIdA: string;
    versionIdB: string;
    labelA: string;
    labelB: string;
    /** CR-07 B7: Comparison Scope. Surfaces in the panel
     *  header so the user knows whether the metrics come
     *  from the whole image, a drawn rectangle, or a named
     *  feature. Backend metrics are whole-image today; the
     *  scope is metadata, not a re-query (yet). */
    scope?: "whole" | "selected" | "feature";
    feature?: string | null;
    hasRegion?: boolean;
  }
  const {
    versionIdA,
    versionIdB,
    labelA,
    labelB,
    scope = "whole",
    feature = null,
    hasRegion = false,
  }: Props = $props();

  const SCOPE_LABEL: Record<NonNullable<Props["scope"]>, string> = {
    whole: "Whole image",
    selected: "Selected region",
    feature: "Specific feature",
  };

  let report = $state<VersionMetricsComparison | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    const a = versionIdA;
    const b = versionIdB;
    if (!a || !b || a === b) {
      report = null;
      error = null;
      return;
    }
    let cancelled = false;
    loading = true;
    error = null;
    compareVersionMetrics(a, b)
      .then((r) => {
        if (!cancelled) {
          report = r;
          loading = false;
        }
      })
      .catch((e: unknown) => {
        if (!cancelled) {
          report = null;
          error = e instanceof Error ? e.message : String(e);
          loading = false;
        }
      });
    return () => {
      cancelled = true;
    };
  });

  const DIRECTION_ICON: Record<DeltaDirection, string> = {
    improved: "check_circle",
    degraded: "warning",
    unchanged: "remove",
    inconclusive: "help",
  };

  function formatValue(value: number | null, unit: string): string {
    if (value === null) return "—";
    return unit ? `${value.toFixed(2)} ${unit}` : value.toFixed(2);
  }

  function formatPercent(row: {
    baseline_value: number | null;
    compared_value: number | null;
    percent_change: number;
  }): string {
    if (row.baseline_value === null || row.compared_value === null) return "—";
    const sign = row.percent_change > 0 ? "+" : "";
    return `${sign}${row.percent_change.toFixed(0)}%`;
  }
</script>

<section class="metrics-panel" aria-label="Metric comparison">
  <header class="panel-header">
    <span class="panel-tag font-label">Metrics</span>
    <h3 class="panel-title font-display">
      {labelA} vs {labelB}
    </h3>
    <!-- CR-07 B7: active scope chip. Shows the user what
         the metric table actually covers. -->
    <span class="scope-chip" data-scope={scope}>
      {SCOPE_LABEL[scope]}
      {#if scope === "feature" && feature}
        · {feature}
      {:else if scope === "selected" && !hasRegion}
        · (no region drawn)
      {/if}
    </span>
  </header>

  {#if versionIdA === versionIdB}
    <p class="note font-body">
      Select two different versions to compute the delta table.
    </p>
  {:else if loading}
    <p class="note font-body" aria-live="polite">
      Computing metric deltas…
    </p>
  {:else if error}
    <p class="note font-body" role="alert">
      <span class="material-symbols-outlined" aria-hidden="true">info</span>
      Metrics need both versions to have applied pixel artifacts.
      The backend reported: {error}
    </p>
  {:else if report}
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th class="font-label">Metric</th>
            <th class="font-label num">A</th>
            <th class="font-label num">B</th>
            <th class="font-label num">Δ</th>
            <th class="font-label">Verdict</th>
          </tr>
        </thead>
        <tbody>
          {#each report.rows as row (row.kind)}
            <tr data-direction={row.direction} title={row.context}>
              <td class="metric-name font-body">
                <span class="metric-label">{row.label}</span>
                <!-- §9 contextual metric display: every metric
                     carries a one-sentence explanation in
                     `row.context` (populated for all 25 metrics
                     by `crates/astroforge-core/src/metric_registry.rs`).
                     Render it inline beneath the label so the user
                     never sees a bare metric name without the
                     "what does this mean + why it matters"
                     sentence the spec requires. The browser
                     tooltip (title=) above remains for users who
                     want the full context on hover. -->
                <span class="metric-context font-body">{row.context}</span>
              </td>
              <td class="num font-data">
                {formatValue(row.baseline_value, row.unit)}
              </td>
              <td class="num font-data">
                {formatValue(row.compared_value, row.unit)}
              </td>
              <td class="num font-data">{formatPercent(row)}</td>
              <td class="verdict">
                <span
                  class="material-symbols-outlined verdict-icon"
                  aria-hidden="true"
                >
                  {DIRECTION_ICON[row.direction]}
                </span>
                <span class="verdict-label font-body">{row.direction}</span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <pre class="summary font-body">{report.summary}</pre>
  {/if}
</section>

<style>
  .metrics-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
  }

  .panel-header {
    display: flex;
    align-items: baseline;
    gap: var(--sp-sm);
  }

  .panel-tag {
    font-size: 0.7rem;
    color: var(--primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .panel-title {
    margin: 0;
    font-size: 1rem;
    flex: 1;
  }

  .scope-chip {
    display: inline-block;
    padding: 2px var(--sp-xs);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .scope-chip[data-scope="selected"] {
    background: rgba(74, 144, 255, 0.18);
    color: #4a90ff;
  }

  .scope-chip[data-scope="feature"] {
    background: rgba(255, 144, 74, 0.18);
    color: #ff904a;
  }

  .note {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
  }

  .table-wrap {
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    text-align: left;
    font-size: 0.7rem;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: var(--sp-xs) var(--sp-sm);
    border-bottom: 1px solid var(--outline-variant);
  }

  td {
    padding: var(--sp-xs) var(--sp-sm);
    border-bottom: 1px solid var(--outline-variant);
    font-size: 0.85rem;
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .metric-name {
    color: var(--on-surface);
    /* §9 contextual metric display: the cell now stacks
       `.metric-label` (the metric name, e.g. "FWHM") on
       top of `.metric-context` (the contextual sentence).
       Vertically center the row contents via flex so the
       numeric cells still align with the visual top of the
       stacked label. */
    display: flex;
    flex-direction: column;
    gap: 2px;
    vertical-align: top;
  }

  .metric-label {
    /* No styling override needed: inherits .metric-name's
       on-surface color + font-body sizing. The separation
       is structural (column flex), not visual. */
    font-weight: 500;
  }

  .metric-context {
    /* §9: muted sub-text under the metric name. Visually
       subordinate so it doesn't compete with the value
       cells, but high enough contrast (var(--on-surface-variant)
       is the standard subdued-text token) to remain
       legible. Line-clamp at 2 keeps tall rows bounded;
       the title= attribute on the <tr> carries the full
       text for hover-reveal. The standard `line-clamp`
       is paired with the `-webkit-line-clamp` vendor
       prefix for cross-browser compatibility; the
       `svelte-check` warning "Also define the standard
       property 'line-clamp' for compatibility" requires
       both. */
    color: var(--on-surface-variant);
    font-size: 0.78rem;
    line-height: 1.35;
    max-width: 38ch;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .verdict {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
  }

  .verdict-icon {
    font-size: 16px;
  }

  .verdict-label {
    text-transform: capitalize;
    color: var(--on-surface-variant);
    font-size: 0.8rem;
  }

  tr[data-direction="improved"] .verdict-icon {
    color: #81c784;
  }
  tr[data-direction="degraded"] .verdict-icon {
    color: #ffb300;
  }
  tr[data-direction="unchanged"] .verdict-icon {
    color: var(--on-surface-variant);
  }
  tr[data-direction="inconclusive"] .verdict-icon {
    color: var(--on-surface-variant);
    opacity: 0.6;
  }

  tr[data-direction="inconclusive"] td {
    opacity: 0.65;
  }

  .summary {
    margin: 0;
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface);
    font-size: 0.85rem;
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
