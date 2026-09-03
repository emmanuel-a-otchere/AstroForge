<script lang="ts">
  /**
   * ReceiptsPanel — read-only log of stage commits.
   *
   * Spec §4.1 + issue #143: each completed stage emits a human-readable
   * receipt with parameters, timing, and warnings. The panel reads from
   * two sources, in order of freshness:
   *
   *   1. The in-memory pipeline-store history (live, this session)
   *   2. The persisted stage_runs rows via fetchReceipts() (after a
   *      crash recovery or app restart)
   *
   * Collapsed by default; toggled via the "Show receipts" pill at the
   * top. Empty state hides itself so it doesn't clutter the canvas.
   */
  import { onMount } from "svelte";
  import { sessionStore } from "../lib/pipeline-store";
  import { fetchReceipts, type StageRunRecord } from "../lib/session";

  let open = false;
  let persistedRuns: StageRunRecord[] = [];
  let loading = false;

  // Live history entries that have a receipt. We filter on the fly so
  // the panel stays reactive even after undo/redo.
  $: liveEntries = $sessionStore.history.filter(
    (h) => h.receipt && h.action === "commit",
  );

  onMount(async () => {
    const sessionId = $sessionStore.sessionId;
    if (!sessionId) return;
    loading = true;
    try {
      persistedRuns = await fetchReceipts(sessionId);
    } catch (err) {
      console.warn("fetchReceipts failed:", err);
    } finally {
      loading = false;
    }
  });

  function fmtDuration(ms: number | undefined): string {
    if (ms == null) return "—";
    if (ms < 1000) return `${ms} ms`;
    return `${(ms / 1000).toFixed(1)} s`;
  }

  function fmtTimestamp(ts: string | undefined): string {
    if (!ts) return "—";
    try {
      return new Date(ts).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
    } catch {
      return ts;
    }
  }

  // Used when only persisted rows are present (post-crash). We rebuild
  // a receipt-shaped view by parsing params_json + metrics_json.
  function readReceipt(run: StageRunRecord) {
    let params: Record<string, unknown> = {};
    let metrics: Record<string, unknown> = {};
    let warnings: string[] = [];
    try {
      if (run.paramsJson) params = JSON.parse(run.paramsJson);
    } catch {
      // ignore malformed
    }
    try {
      if (run.metricsJson) {
        const parsed = JSON.parse(run.metricsJson);
        if (parsed && typeof parsed === "object") {
          metrics = parsed;
          if (Array.isArray(parsed.warnings)) {
            warnings = parsed.warnings;
          }
        }
      }
    } catch {
      // ignore
    }
    const durationMs =
      typeof metrics.durationMs === "number" ? (metrics.durationMs as number) : undefined;
    return { params, metrics, warnings, durationMs };
  }
</script>

<div class="receipts-panel">
  <button
    type="button"
    class="toggle-pill"
    on:click={() => (open = !open)}
    aria-expanded={open}
  >
    {open ? "▾ Hide receipts" : "▸ Show receipts"}
    {#if liveEntries.length > 0 || persistedRuns.length > 0}
      <span class="count">{liveEntries.length + persistedRuns.length}</span>
    {/if}
  </button>

  {#if open}
    <div class="panel-body">
      {#if liveEntries.length === 0 && persistedRuns.length === 0}
        <p class="empty">
          No receipts yet. Commit a stage to start the log.
        </p>
      {:else}
        {#each liveEntries as entry (entry.nodeId + "-" + entry.version)}
          <article class="receipt">
            <header>
              <span class="stage">{entry.nodeId}</span>
              <span class="ts">{fmtTimestamp(entry.receipt?.timestamp ?? entry.timestamp)}</span>
              <span class="dur">{fmtDuration(entry.receipt?.durationMs)}</span>
              <span class="status status-completed">{entry.status}</span>
            </header>
            {#if entry.receipt && entry.receipt.parameters && Object.keys(entry.receipt.parameters).length > 0}
              <pre class="params">{JSON.stringify(entry.receipt.parameters, null, 2)}</pre>
            {/if}
            {#if entry.receipt?.warnings && entry.receipt.warnings.length > 0}
              <ul class="warnings">
                {#each entry.receipt.warnings as w}
                  <li>{w}</li>
                {/each}
              </ul>
            {/if}
          </article>
        {/each}

        {#each persistedRuns as run (run.id)}
          {@const r = readReceipt(run)}
          <article class="receipt persisted">
            <header>
              <span class="stage">{run.stageId}</span>
              <span class="ts">{fmtTimestamp(run.startedAt ?? undefined)}</span>
              <span class="dur">{fmtDuration(r.durationMs)}</span>
              <span class="status status-{run.status}">{run.status}</span>
            </header>
            {#if Object.keys(r.params).length > 0}
              <pre class="params">{JSON.stringify(r.params, null, 2)}</pre>
            {/if}
            {#if r.warnings.length > 0}
              <ul class="warnings">
                {#each r.warnings as w}
                  <li>{w}</li>
                {/each}
              </ul>
            {/if}
            {#if run.error}
              <p class="error">⚠ {run.error}</p>
            {/if}
          </article>
        {/each}
      {/if}

      {#if loading}
        <p class="loading">Loading persisted receipts…</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .receipts-panel {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 0.5rem);
  }

  .toggle-pill {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.8rem;
    background: var(--overlay-glass);
    border: 1px solid var(--glow-primary-hairline);
    border-radius: 999px;
    color: var(--text-primary);
    font-size: 0.75rem;
    cursor: pointer;
    transition: background 0.15s ease;
  }
  .toggle-pill:hover {
    background: var(--overlay-card);
  }
  .toggle-pill .count {
    background: var(--cobalt-accent);
    color: var(--text-on-accent);
    padding: 0.05rem 0.45rem;
    border-radius: 999px;
    font-size: 0.7rem;
  }

  .panel-body {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 280px;
    overflow-y: auto;
    padding: 0.25rem;
  }

  .receipt {
    background: var(--overlay-card);
    border: 1px solid var(--glow-primary-hairline);
    border-radius: 0.5rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.78rem;
  }
  .receipt.persisted {
    opacity: 0.85;
    border-style: dashed;
  }
  .receipt header {
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
    margin-bottom: 0.25rem;
  }
  .receipt .stage {
    font-weight: 600;
    color: var(--cobalt-accent);
  }
  .receipt .ts {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .receipt .dur {
    margin-left: auto;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .receipt .status {
    padding: 0.05rem 0.45rem;
    border-radius: 0.25rem;
    font-size: 0.7rem;
    text-transform: lowercase;
  }
  .receipt .status-completed {
    background: var(--success-border-dim);
    color: var(--success-fg);
  }
  .receipt .status-failed {
    background: var(--glow-error);
    color: var(--text-on-accent);
  }
  .receipt .status-running {
    background: var(--glow-cobalt-dim);
    color: var(--text-primary);
  }

  .receipt .params {
    margin: 0.25rem 0 0;
    padding: 0.35rem 0.5rem;
    background: var(--overlay-glass);
    border-radius: 0.35rem;
    font-size: 0.7rem;
    color: var(--text-secondary);
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .receipt .warnings {
    margin: 0.25rem 0 0;
    padding-left: 1.1rem;
    color: var(--warning-fg);
  }

  .receipt .error {
    margin: 0.25rem 0 0;
    color: var(--error-fg);
    font-size: 0.7rem;
  }

  .empty,
  .loading {
    color: var(--text-secondary);
    font-size: 0.75rem;
    text-align: center;
    margin: 0.5rem 0;
  }
</style>
