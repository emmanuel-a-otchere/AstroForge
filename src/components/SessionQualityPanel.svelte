<!--
  SessionQualityPanel — RT-13 (#431).

  Surfaces the per-frame quality trends + robust outlier flags for a
  session's light frames: one trend row per metric (slope + direction),
  outlier chips a reviewer can act on (drop the frame), and the honest
  unreadable list. The outlier feed is the import recommendation
  engine's input.
-->
<script lang="ts">
  import type { SessionQualityReport } from "../lib/astroforge-api";

  export let report: SessionQualityReport;

  const METRIC_LABELS: Record<string, string> = {
    fwhm: "FWHM",
    stars: "Star count",
    estimated_snr: "SNR (est.)",
    local_snr: "SNR (local)",
  };

  function metricLabel(metric: string): string {
    return METRIC_LABELS[metric] ?? metric;
  }

  function formatValue(metric: string, value: number): string {
    if (metric === "stars") return value.toFixed(0);
    return value.toFixed(2);
  }
</script>

<section class="quality-panel" aria-label="Session quality">
  <h3 class="font-display">Frame quality</h3>

  {#if report.unreadable.length > 0}
    <p class="unreadable font-body" role="status">
      {report.unreadable.length} frame{report.unreadable.length === 1 ? "" : "s"}
      could not be decoded and {report.unreadable.length === 1 ? "is" : "are"} excluded:
      {report.unreadable.map((u) => u.path.split("/").pop() ?? u.path).join(", ")}
    </p>
  {/if}

  {#if report.frames.length === 0}
    <p class="empty font-body">No decodable frames to measure.</p>
  {:else}
    {#if report.outliers.length > 0}
      <ul class="outliers" aria-label="Outlier frames">
        {#each report.outliers as flag (flag.frameIndex)}
          <li class="outlier-chip" data-severity="high">
            <strong>{flag.frameName}</strong>
            {metricLabel(flag.metric)} {formatValue(flag.metric, flag.value)}
            vs typical {formatValue(flag.metric, flag.typical)}
            ({flag.robustSigma.toFixed(1)}σ)
          </li>
        {/each}
      </ul>
    {:else}
      <p class="no-outliers font-body">No outlier frames flagged.</p>
    {/if}

    {#if report.trends.length > 0}
      <table class="trends">
        <thead>
          <tr>
            <th scope="col">Metric</th>
            <th scope="col">First</th>
            <th scope="col">Last</th>
            <th scope="col">Per frame</th>
            <th scope="col">Direction</th>
          </tr>
        </thead>
        <tbody>
          {#each report.trends as trend (trend.metric)}
            <tr>
              <td>{metricLabel(trend.metric)}</td>
              <td>{formatValue(trend.metric, trend.first)}</td>
              <td>{formatValue(trend.metric, trend.last)}</td>
              <td>
                {trend.slopePerFrame >= 0 ? "+" : ""}
                {trend.slopePerFrame.toFixed(3)}
              </td>
              <td class:good={trend.improving} class:bad={!trend.improving}>
                {trend.improving ? "improving" : "worsening"}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <p class="empty font-body">Trends need at least two frames.</p>
    {/if}
  {/if}
</section>

<style>
  .quality-panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px;
    border: 1px solid var(--outline-variant, #cac4d0);
    border-radius: 8px;
    background: var(--surface-container-low, #f7f2fa);
  }

  .quality-panel h3 {
    margin: 0;
    font-size: 1rem;
  }

  .outliers {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .outlier-chip {
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--error, #b3261e);
    color: var(--error, #b3261e);
    background: var(--error-container, #f9dedc);
    font-size: 0.8rem;
  }

  .no-outliers,
  .empty,
  .unreadable {
    margin: 0;
    font-size: 0.85rem;
    color: var(--on-surface-variant, #49454f);
  }

  .unreadable {
    color: var(--error, #b3261e);
  }

  .trends {
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .trends th,
  .trends td {
    text-align: left;
    padding: 4px 10px 4px 0;
  }

  .trends .good {
    color: var(--primary, #6750a4);
  }

  .trends .bad {
    color: var(--error, #b3261e);
  }
</style>
