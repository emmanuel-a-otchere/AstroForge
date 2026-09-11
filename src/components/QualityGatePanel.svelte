<!--
  CR-06 P6 — Quality Gate Panel.

  The Quality Gate Panel lives in the Enhancement
  Studio's Zone C, below the Mask Editor. Per CR-06
  §37, every applied AI operation is validated
  against a battery of post-operation checks. The
  panel surfaces the verdict + the per-gate findings
  so the user can accept, reduce strength, or re-run
  before committing the result.

  The panel reads its data from `aiEnhancementStore`
  (the `qualityReport` field, populated by
  `runQualityReport`). The user can also click
  "Run gate on current image" to invoke the gate
  manually against the latest analysis pixels.

  The panel shows:

    - Headline verdict (Ok / Info / Warning / Failure)
      with the per-severity counts.
    - Per-gate findings list with severity badge +
      message + score. Warning / Failure findings
      get a left accent bar.
    - "Run gate" button that calls
      `runAiQualityReport` against the latest
      analysis pixels + the same pixel set as the
      result (since the P4 dispatcher is a
      passthrough). When the apply round (P5.1)
      produces a real result, the apply round will
      populate `qualityReport` automatically with
      the gate run.
-->
<script lang="ts">
  import { aiEnhancementStore } from "../state/ai-enhancement";
  import {
    runAiQualityReport,
    type GateFindingJson,
    type QualityGateReportJson,
    type Severity,
  } from "../lib/astroforge-api";

  let feedback: string | null = null;

  $: analysis =
    ($aiEnhancementStore.analysis as {
      image_version_id?: string;
      project_id?: string;
      width?: number;
      height?: number;
      channels?: number;
      pixels?: number[];
    } | null) ?? null;
  $: qualityReport =
    ($aiEnhancementStore.qualityReport as QualityGateReportJson | null) ??
    null;

  async function runGate(): Promise<void> {
    if (!analysis) {
      feedback = "No analysis yet — run the analyzer first.";
      return;
    }
    try {
      const response = await runAiQualityReport({
        source_image_version_id: analysis.image_version_id ?? "src",
        result_image_version_id: analysis.image_version_id ?? "res",
        operation_id: "manual_review",
        width: analysis.width ?? 0,
        height: analysis.height ?? 0,
        channels: analysis.channels ?? 3,
        source_pixels: analysis.pixels ?? [],
        result_pixels: analysis.pixels ?? [],
      });
      aiEnhancementStore.update((s) => ({
        ...s,
        qualityReport: response.report,
      }));
      feedback = `Gate verdict: ${response.verdict}`;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      feedback = `Gate run failed: ${msg}`;
    }
  }

  function severityIcon(s: Severity): string {
    switch (s) {
      case "ok":
        return "✓";
      case "info":
        return "ℹ";
      case "warning":
        return "⚠";
      case "failure":
        return "✗";
    }
  }

  function count(findings: GateFindingJson[], target: Severity): number {
    return findings.filter((f) => f.severity === target).length;
  }
</script>

<section class="quality-panel" aria-label="Quality gate panel">
  <header>
    <h4>Quality Gate</h4>
    <button type="button" class="primary" on:click={runGate}>
      Run gate on current image
    </button>
  </header>

  {#if !qualityReport}
    <p class="empty">
      No gate report yet. Apply an operation, or click "Run gate" above.
    </p>
  {:else}
    <div class="verdict verdict-{qualityReport.verdict}">
      <span class="verdict-icon">{severityIcon(qualityReport.verdict)}</span>
      <span class="verdict-text">{qualityReport.verdict.toUpperCase()}</span>
      <span class="verdict-counts">
        ✓ {count(qualityReport.findings, "ok")} ·
        ℹ {count(qualityReport.findings, "info")} ·
        ⚠ {count(qualityReport.findings, "warning")} ·
        ✗ {count(qualityReport.findings, "failure")}
      </span>
    </div>

    <ol class="finding-list">
      {#each qualityReport.findings as f}
        <li class="finding finding-{f.severity}">
          <span class="finding-icon">{severityIcon(f.severity)}</span>
          <span class="finding-gate">{f.gate}</span>
          <span class="finding-message">{f.message}</span>
          <span class="finding-score">{f.score.toFixed(2)}</span>
        </li>
      {/each}
    </ol>
  {/if}

  {#if feedback}
    <p class="feedback">{feedback}</p>
  {/if}
</section>

<style>
  .quality-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm, 0.5rem);
    padding: var(--sp-sm, 0.75rem);
    border: 1px solid var(--color-border, #2a2f3a);
    border-radius: 6px;
    background: var(--color-surface-muted, #1f242d);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  header h4 {
    margin: 0;
    font-size: 0.95rem;
  }
  .primary {
    background: var(--color-accent, #4f9cff);
    color: var(--color-on-accent, #fff);
    border: none;
    border-radius: 4px;
    padding: 0.3rem 0.6rem;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .empty {
    margin: 0;
    padding: 0.4rem;
    border: 1px dashed var(--color-border, #2a2f3a);
    border-radius: 3px;
    color: var(--color-text-muted, #9aa3b2);
    font-size: 0.8rem;
  }
  .verdict {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.6rem;
    border-radius: 4px;
    font-size: 0.85rem;
  }
  .verdict-icon {
    font-size: 1.2rem;
    font-weight: 600;
  }
  .verdict-text {
    font-weight: 600;
    letter-spacing: 0.04em;
  }
  .verdict-counts {
    margin-left: auto;
    font-size: 0.75rem;
    color: var(--color-text-muted, #9aa3b2);
  }
  .verdict-ok {
    background: rgba(80, 200, 120, 0.15);
    color: #50c878;
  }
  .verdict-info {
    background: rgba(79, 156, 255, 0.15);
    color: #4f9cff;
  }
  .verdict-warning {
    background: rgba(255, 195, 0, 0.18);
    color: #ffc300;
  }
  .verdict-failure {
    background: rgba(255, 100, 100, 0.18);
    color: #ff6464;
  }
  .finding-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .finding {
    display: grid;
    grid-template-columns: 1.2rem 9rem 1fr 3rem;
    align-items: center;
    gap: 0.4rem;
    padding: 0.25rem 0.5rem;
    background: var(--color-surface, #15181f);
    border-radius: 3px;
    font-size: 0.75rem;
  }
  .finding-warning {
    border-left: 2px solid #ffc300;
  }
  .finding-failure {
    border-left: 2px solid #ff6464;
  }
  .finding-info {
    border-left: 2px solid #4f9cff;
  }
  .finding-ok {
    border-left: 2px solid #50c878;
  }
  .finding-icon {
    text-align: center;
    font-weight: 600;
  }
  .finding-gate {
    font-family: ui-monospace, monospace;
    color: var(--color-text-muted, #c0c5d0);
  }
  .finding-message {
    color: var(--color-text, #e6e9ef);
  }
  .finding-score {
    text-align: right;
    color: var(--color-text-muted, #9aa3b2);
    font-family: ui-monospace, monospace;
  }
  .feedback {
    margin: 0;
    padding: 0.3rem 0.5rem;
    border-left: 3px solid var(--color-accent, #4f9cff);
    background: var(--color-surface, #15181f);
    border-radius: 3px;
    font-size: 0.8rem;
  }
</style>