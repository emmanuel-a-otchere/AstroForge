<script lang="ts">
  /**
   * CR-05 P6 slice 3 — §25 Processing Timeline.
   *
   * Renders the §25 example list (HH:MM + label, click-to-reveal
   * the corresponding Image Version). Sourced from
   * `getProcessingTimeline` (single IPC, returns all events).
   *
   * Honest disclosures (per CR-05 §25 reproducibility invariant):
   *
   * - Events whose `started_at` doesn't parse are dropped server-side
   *   rather than fabricated. The timeline never invents a timestamp.
   * - Events without `output_version_id` (e.g. the `export` stage,
   *   which produces a file but no Image Version) render as
   *   "non-reveal" rows — the user sees them on the timeline but
   *   cannot click to reveal.
   * - The selection callback emits the `version_id` as a string;
   *   the parent decides how to reveal it (the existing
   *   `ImageViewer` integration is downstream of P6.3 and is not
   *   in scope for this slice).
   */
  import type { TimelineEventDto } from "../lib/astroforge-api";

  export let events: TimelineEventDto[] = [];

  /** Optional selection handler. The parent wires the reveal action. */
  export let onSelect: ((versionId: string) => void) | null = null;

  /** Optional currently-selected version_id (highlights the row). */
  export let selectedVersionId: string | null = null;

  /**
   * Format a unix-ms timestamp as HH:MM in the user's local
   * timezone. Returns `—` when the timestamp is missing or invalid
   * so the row still renders without crashing the whole panel.
   */
  function formatHHMM(unixMs: number | null): string {
    if (unixMs === null) return "—";
    const d = new Date(unixMs);
    if (Number.isNaN(d.getTime())) return "—";
    const hh = String(d.getHours()).padStart(2, "0");
    const mm = String(d.getMinutes()).padStart(2, "0");
    return `${hh}:${mm}`;
  }

  /**
   * Format a duration in milliseconds as a human-readable string.
   * Returns `—` for null or zero so the column doesn't render
   * `0 ms` for in-progress stages.
   */
  function formatDuration(ms: number | null): string {
    if (ms === null || ms === 0) return "—";
    if (ms < 1000) return `${ms} ms`;
    const s = ms / 1000;
    if (s < 60) return `${s.toFixed(1)} s`;
    const m = Math.floor(s / 60);
    const rem = Math.round(s % 60);
    return `${m}m ${rem}s`;
  }

  /**
   * Map status strings to a CSS class for the status dot. Stages
   * that aren't `completed` get a distinct color so the user
   * notices a failed/running row immediately.
   */
  function statusClass(status: string): string {
    if (status === "completed") return "ok";
    if (status === "failed") return "err";
    if (status === "running") return "active";
    return "muted";
  }
</script>

{#if events.length === 0}
  <p class="empty">No timeline events yet — start a run to populate the timeline.</p>
{:else}
  <ol class="processing-timeline" data-testid="processing-timeline">
    {#each events as event (event.stage_id)}
      {@const revealable = event.output_version_id !== null}
      {@const selected = revealable && event.output_version_id === selectedVersionId}
      <li
        class="event"
        class:revealable
        class:selected
        data-testid="timeline-event"
        data-stage-id={event.stage_id}
      >
        <span class="time">{formatHHMM(event.timestamp_unix_ms)}</span>
        <span class="dot {statusClass(event.status)}" title={event.status}></span>
        <button
          type="button"
          class="label"
          disabled={!revealable || !onSelect}
          on:click={() => revealable && event.output_version_id && onSelect?.(event.output_version_id)}
        >
          {event.stage_label}
        </button>
        <span class="duration">{formatDuration(event.duration_ms)}</span>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .processing-timeline {
    list-style: none;
    padding: 0;
    margin: 0.75rem 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .empty {
    color: var(--color-muted, #888);
    font-style: italic;
    padding: 0.5rem 0;
  }

  .event {
    display: grid;
    grid-template-columns: 4rem 0.6rem 1fr auto;
    column-gap: 0.5rem;
    align-items: center;
    padding: 0.25rem 0.4rem;
    border-radius: 4px;
  }

  .event.revealable {
    cursor: default;
  }

  .event.selected {
    background: var(--color-tl-selected-bg, #e6f0ff);
  }

  .time {
    font-variant-numeric: tabular-nums;
    color: var(--color-muted, #888);
    font-size: 0.9rem;
  }

  .dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: var(--color-dot-muted, #ccc);
  }

  .dot.ok { background: var(--color-dot-ok, #2e8b57); }
  .dot.err { background: var(--color-dot-err, #c0392b); }
  .dot.active { background: var(--color-dot-active, #d4a017); }
  .dot.muted { background: var(--color-dot-muted, #ccc); }

  .label {
    text-align: left;
    border: none;
    background: transparent;
    padding: 0.15rem 0.4rem;
    font: inherit;
    cursor: pointer;
    color: var(--color-text, #222);
    border-radius: 3px;
  }

  .label:hover:not(:disabled) {
    background: var(--color-tl-hover-bg, #f0f0f0);
  }

  .label:disabled {
    cursor: not-allowed;
    color: var(--color-muted, #888);
  }

  .duration {
    color: var(--color-muted, #888);
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
  }
</style>
