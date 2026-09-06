<!--
  CR-03 P4 — Process workspace content (§12).

  P4 ships the layout + the workspace-state-driven pipeline-run
  list. P4b wires the actual run/control buttons (start, pause,
  resume, cancel).
-->
<script lang="ts">
  import WorkspaceScreen from "./WorkspaceScreen.svelte";
  import { workspaceState } from "../state/workspace";

  function formatTime(iso: string | null): string {
    if (!iso) return "—";
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }
</script>

<WorkspaceScreen
  title="Process"
  icon="auto_fix_high"
  description="Run image-processing pipelines against the project's source data. Every run is durable and recoverable; crash mid-run resumes from the last completed stage."
>
  {#if $workspaceState.loading}
    <p class="status-line font-body" role="status">Loading pipeline runs…</p>
  {:else if $workspaceState.error}
    <div class="error-state" role="alert">
      <span class="material-symbols-outlined" aria-hidden="true">error</span>
      <p class="font-body">Failed to load runs: {$workspaceState.error}</p>
    </div>
  {:else if $workspaceState.runs.length === 0}
    <div class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">
        play_circle
      </span>
      <p class="empty-title font-display">No pipeline runs yet</p>
      <p class="empty-body font-body">
        Pick a recipe and start a run. The pipeline driver persists progress
        so a crash or quit mid-run resumes from the last completed stage.
      </p>
      <p class="hint font-body">Run controls land in P4b.</p>
    </div>
  {:else}
    <ul class="run-list" aria-label="Pipeline runs">
      {#each $workspaceState.runs as run (run.run_id)}
        <li class="run-item" data-status={run.status}>
          <div class="run-id font-label">Run {run.run_id.slice(0, 8)}</div>
          <div class="run-meta font-body">
            Recipe: {run.recipe_id ?? "(default)"} ·
            Started: {formatTime(run.started_at)} ·
            Completed: {formatTime(run.completed_at)}
          </div>
          <span class="status-pill" data-status={run.status}>
            {run.status}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</WorkspaceScreen>

<style>
  .status-line {
    color: var(--on-surface-variant);
    font-size: 0.9rem;
  }

  .error-state {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid #e53935;
    border-radius: var(--radius-md);
    color: #ff8a80;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: var(--sp-xl);
    background: var(--surface-container-low);
    border: 1px dashed var(--outline-variant);
    border-radius: var(--radius-lg);
    gap: var(--sp-md);
  }

  .empty-icon {
    font-size: 48px;
    color: var(--on-surface-variant);
  }

  .empty-title {
    font-size: 1.25rem;
    margin: 0;
  }

  .empty-body {
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 60ch;
  }

  .hint {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
    margin: 0;
  }

  .run-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .run-item {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--sp-md);
    padding: var(--sp-md) var(--sp-lg);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
  }

  .run-id {
    font-weight: 600;
    color: var(--on-surface);
  }

  .run-meta {
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }

  .status-pill {
    font-size: 0.7rem;
    padding: 2px var(--sp-sm);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .status-pill[data-status="Completed"] {
    background: rgba(111, 191, 115, 0.2);
    color: #6fbf73;
  }
  .status-pill[data-status="Failed"],
  .status-pill[data-status="Cancelled"] {
    background: rgba(229, 57, 53, 0.2);
    color: #ff8a80;
  }
  .status-pill[data-status="Running"] {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
  }
  .status-pill[data-status="Queued"] {
    background: rgba(255, 179, 0, 0.2);
    color: #ffb300;
  }
</style>