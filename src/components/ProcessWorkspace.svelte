<!--
  CR-03 P4 — Process workspace content (§12).

  P4 ships the layout + the workspace-state-driven pipeline-run
  list. P4b wires the actual run/control buttons (start, pause,
  resume, cancel).

  CR-05 P3 slice 2.6 — the workspace also hosts
  `IntelligencePanel` when there is an active plan, so the user
  can act on engine picks from the same screen that runs the
  pipeline. The panel is reactive to `activePlan`: switching
  plan (or generating a new one) re-fetches its recommendations.

  CR-05 P3 slice 2.7 — mounts the two pre-existing plan-context
  companions that were built but never wired:
    - `RecoveryBanner` (always mounted; renders only when
      `$resumablePlans.length > 0`)
    - `AutoPlanPanel` (rendered only when `!$activePlan`, since
      the plan summary it shows is redundant with the
      IntelligencePanel's plan-scoped view)

  CR-05 P3 slice 2.8 — mounts `ProcessingControls` so the user
  has Start / Pause / Resume / Cancel in the same surface as
  the recommendations + run history. The spec's "Zone C"
  right-hand panel (Intelligence & Controls) is a future
  layout refactor (P3 slice 2.10+); for now we stack the
  controls vertically above the run list, consistent with the
  recovery / auto-plan / intelligence zones already wired here.

  CR-10 P1.6.2.2 — when the session is in Node mode (read from
  the new `isNodeMode` store), mount `<NodePalette>` as a
  left-sidebar via the `sidebar` snippet on `WorkspaceScreen`.
  In Wizard mode (default), no sidebar is mounted and the
  workspace renders exactly as before. The toggle UI that
  writes to the store lands in P1.6.2.3.
-->
<script lang="ts">
  import { onDestroy } from "svelte";

  import WorkspaceScreen from "./WorkspaceScreen.svelte";
  import IntelligencePanel from "./IntelligencePanel.svelte";
  import RecoveryBanner from "./RecoveryBanner.svelte";
  import AutoPlanPanel from "./AutoPlanPanel.svelte";
  import ProcessingControls from "./ProcessingControls.svelte";
  import SaveAsRecipePanel from "./SaveAsRecipePanel.svelte";
  import NodePalette from "./NodePalette.svelte";
  import NodeStageList from "./NodeStageList.svelte";
  import { activeProject, projectContext } from "../state/project-context";
  import {
    activePlan,
    refreshRecommendations,
  } from "../lib/pipeline-plan-store";
  import { workspaceState } from "../state/workspace";
  import { isNodeMode } from "../lib/node-graph-store";
  import {
    insertStage,
    type PipelinePlanDto,
    sessionListForProject,
    sessionContinue,
  } from "../lib/astroforge-api";
  import type { NodeCatalogEntry } from "../lib/node-catalog";

  // CR-05 P3 slice 2.6 — keep the per-plan recommendation store
  // fresh whenever `activePlan` changes. We subscribe manually
  // (rather than reactively inside the panel) so the panel can
  // stay a pure renderer and we can dedupe refetches across
  // mount / unmount cycles.
  let currentPlanId: string | null = null;
  const unsubscribe = activePlan.subscribe(($plan) => {
    const next = $plan?.plan_id ?? null;
    if (next && next !== currentPlanId) {
      currentPlanId = next;
      void refreshRecommendations(next);
    } else if (!next) {
      currentPlanId = null;
    }
  });
  onDestroy(unsubscribe);

  // CR-05 P3 slice 2.7 — derive mount-time props for the recovery
  // banner (projectId) and the auto-plan generator
  // (projectId + sessionId). The latter is null when the project
  // has no active session yet; we suppress the panel in that
  // case so the user does not see a broken Generate button.
  let project = $derived($activeProject);
  let sessionId = $derived(project?.active_session_id ?? null);
  let canGenerateAutoPlan = $derived(Boolean(project && sessionId));

  // #465 W4 — the project's session history (newest first) with a
  // continue action; replaces the missing "previous sessions" UX.
  let sessionHistory = $state<Awaited<
    ReturnType<typeof sessionListForProject>
  >>([]);
  let historyLoading = $state(false);
  let historyError = $state<string | null>(null);

  async function loadHistory(): Promise<void> {
    if (!project) return;
    historyLoading = true;
    historyError = null;
    try {
      sessionHistory = await sessionListForProject(project.project_id);
    } catch (e) {
      historyError = e instanceof Error ? e.message : String(e);
    } finally {
      historyLoading = false;
    }
  }

  async function continueSession(entry: { sessionId: string }): Promise<void> {
    if (!project) return;
    historyError = null;
    try {
      await sessionContinue(project.project_id, entry.sessionId);
      // Refresh the frozen snapshot so the gates reopen in this sitting.
      const fresh = await import("../lib/astroforge-api").then((m) =>
        m.projectGet(project.project_id),
      );
      projectContext.replace(fresh);
      await loadHistory();
    } catch (e) {
      historyError = e instanceof Error ? e.message : String(e);
    }
  }

  // React to project changes (open/close/active flips).
  $effect(() => {
    void project;
    void loadHistory();
  });

  // CR-10 P1.6.2.4 / Slice P1.6.2.4: palette insert wiring.
  // When the user clicks an entry in the node palette, append
  // a new stage to the currently-active plan via the
  // `insert_stage` IPC, then refresh the local `activePlan`
  // store with the returned plan so the canvas re-renders.
  //
  // We keep the handler defensive: if no plan is active, the
  // click is a no-op (the parent of NodeSidebar is responsible
  // for surfacing "create a plan first" affordances).
  let insertError: string | null = null;
  let insertInFlight = false;

  async function handlePaletteSelect(entry: NodeCatalogEntry): Promise<void> {
    const current = getActivePlanSnapshot();
    if (current === null) {
      insertError = "Create or activate a plan before inserting stages.";
      return;
    }
    if (insertInFlight) return;
    insertInFlight = true;
    insertError = null;
    try {
      const updated: PipelinePlanDto = await insertStage({
        plan_id: current.plan_id,
        stage_type: entry.stage_type,
      });
      activePlan.set(updated);
    } catch (err) {
      insertError = err instanceof Error ? err.message : String(err);
    } finally {
      insertInFlight = false;
    }
  }

  function getActivePlanSnapshot(): PipelinePlanDto | null {
    let snapshot: PipelinePlanDto | null = null;
    const unsubscribe = activePlan.subscribe((value) => {
      snapshot = value;
    });
    unsubscribe();
    return snapshot;
  }

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
  {#snippet sidebar()}
    {#if $isNodeMode}
      <NodePalette onSelect={handlePaletteSelect} />
      <!-- CR-10 P1.6.3.2 — the constrained-layout stage stack with
           the mutation context menu (enable / reorder / remove). -->
      <NodeStageList />
    {/if}
  {/snippet}
  {#if $workspaceState.loading}
    <p class="status-line font-body" role="status">Loading pipeline runs…</p>
  {:else if $workspaceState.error}
    <div class="error-state" role="alert">
      <span class="material-symbols-outlined" aria-hidden="true">error</span>
      <p class="font-body">Failed to load runs: {$workspaceState.error}</p>
    </div>
  {:else}
    <!--
      CR-05 P3 slice 2.7 — RecoveryBanner is mounted first;
      it self-hides when there are no paused plans, so it is
      always safe to include here.
    -->

    {#if project}
      <section class="recovery-zone" aria-label="Recovery banner zone">
        <RecoveryBanner projectId={project.project_id} />
      </section>
    {/if}

    <!--
      #465 W4 — session history: previous sessions for this project,
      newest first, with continue-on-active. The pipeline reads
      SourceAssets from the active session, so "continue" points the
      plan generator + runner at that session's data.
    -->
    {#if project}
      <section class="session-history" aria-label="Session history">
        <h2 class="font-display">Sessions</h2>
        {#if historyLoading}
          <p class="history-line font-body" aria-live="polite">Loading sessions…</p>
        {:else if historyError}
          <p class="history-error font-body" role="alert">{historyError}</p>
        {:else if sessionHistory.length === 0}
          <p class="history-line font-body starfield">
            No sessions yet — import captures to create one.
          </p>
        {:else}
          <ul class="session-list" role="list">
            {#each sessionHistory as entry (entry.sessionId)}
              <li class="session-item" role="listitem" data-active={entry.isActive}>
                <div class="session-meta font-body">
                  <strong>{entry.name || entry.sessionId}</strong>
                  {#if entry.sourceLocation}
                    · {entry.sourceLocation}
                  {/if}
                  · {entry.processingStatus}
                  {#if entry.isActive}
                    · <span class="active-badge">active</span>
                  {/if}
                </div>
                {#if !entry.isActive}
                  <button
                    type="button"
                    class="continue-btn font-label"
                    onclick={() => continueSession(entry)}
                  >
                    Continue
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}

    <!--
      CR-05 P3 slice 2.7 — AutoPlanPanel only renders when
      there is no active plan (otherwise the plan summary it
      shows duplicates the IntelligencePanel's plan-scoped
      view). Suppressed when the project has no active
      session — the Generate button would otherwise call the
      backend with a null session id.
    -->
    {#if !$activePlan && canGenerateAutoPlan && project && sessionId}
      <section class="auto-plan-zone" aria-label="Auto-plan generator zone">
        <AutoPlanPanel projectId={project.project_id} {sessionId} />
      </section>
    {/if}

    <!--
      CR-05 P3 slice 2.6 — IntelligencePanel mounted above the
      run list when an active plan exists. Independent of
      `workspaceState` so the panel renders even while the run
      list is empty / loading.
    -->
    {#if $activePlan}
      <section class="intelligence-zone" aria-label="Active plan intelligence">
        <IntelligencePanel planId={$activePlan.plan_id} />
      </section>

      <!--
        CR-05 P3 slice 2.8 — ProcessingControls mounted
        directly below the intelligence zone so the read →
        act → history flow reads top-to-bottom. The component
        is self-contained: Start / Pause / Resume / Cancel
        + per-stage execution list, scoped to the plan id.
      -->
      <section class="processing-controls-zone" aria-label="Pipeline run controls">
        <ProcessingControls planId={$activePlan.plan_id} />
      </section>

      <!--
        CR-08 §14: "Save as Recipe" UX. Renders only when
        an active plan exists; the panel reads `$activePlan`
        to drive the IPC + modal. Mirrors the existing
        RecoveryBanner / AutoPlanPanel gating (mounted when
        the plan is live; absent in the empty-state path).
      -->
      <SaveAsRecipePanel />
    {/if}

    {#if sessionHistory.length > 0}
      <!--
        #465 W4 — run history sourced from the live plan store (the
        legacy pipeline_runs table was never written by the CR-05
        runner, so the old list stayed empty even with completed
        stage executions visible above).
      -->
      <ul class="run-list" aria-label="Pipeline runs">
        {#each sessionHistory as entry (entry.sessionId)}
          <li class="run-item" data-status={entry.processingStatus}>
            <div class="run-id font-label">{entry.name || entry.sessionId.slice(0, 12)}</div>
            <div class="run-meta font-body">
              Source: {entry.sourceLocation ?? "(not recorded)"} ·
              Updated: {entry.updatedAt}
            </div>
            <span class="status-pill" data-status={entry.processingStatus}>
              {entry.processingStatus}
            </span>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="empty-state">
        <span class="material-symbols-outlined empty-icon" aria-hidden="true">
          play_circle
        </span>
        <p class="empty-title font-display">No pipeline runs yet</p>
        <p class="empty-body font-body">
          Import captures, generate a plan, and start a run. Completed runs
          land here with per-stage history.
        </p>
      </div>
    {/if}
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

  /* CR-05 P3 slice 2.6 — intelligence zone sits above the run
     list with a deliberate vertical gap so the two zones read as
     distinct surfaces. */
  .intelligence-zone {
    margin-bottom: var(--sp-lg);
  }

  /* CR-05 P3 slice 2.7 — recovery banner sits at the top with
     the same vertical rhythm so it does not crowd the
     auto-plan / intelligence surfaces below. */
  .recovery-zone {
    margin-bottom: var(--sp-lg);
  }

  /* CR-05 P3 slice 2.7 — auto-plan generator sits below the
     recovery banner and above the intelligence zone. */
  .auto-plan-zone {
    margin-bottom: var(--sp-lg);
  }

  /* CR-05 P3 slice 2.8 — processing controls sit between the
     intelligence zone (read) and the run list (history) so
     the read → act → history flow reads top-to-bottom. */
  .processing-controls-zone {
    margin-bottom: var(--sp-lg);
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
  .session-history {
    margin: 0 0 var(--space-5, 24px);
  }

  .session-history h2 {
    margin: 0 0 var(--space-2, 8px);
    font-size: 1rem;
  }

  .session-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .session-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 12px;
    border: 1px solid var(--outline-variant, #cac4d0);
    border-radius: 8px;
    background: var(--surface-container-low, #f7f2fa);
  }

  .session-item[data-active="true"] {
    border-color: var(--primary, #6750a4);
  }

  .session-meta {
    font-size: 0.85rem;
    color: var(--on-surface-variant, #49454f);
  }

  .active-badge {
    color: var(--primary, #6750a4);
    font-weight: 600;
  }

  .continue-btn {
    padding: 4px 12px;
    border-radius: 999px;
    border: 1px solid var(--primary, #6750a4);
    background: transparent;
    color: var(--primary, #6750a4);
    cursor: pointer;
    font-size: 0.8rem;
  }

  .continue-btn:hover {
    background: var(--primary-container, #eaddff);
  }

  .history-line,
  .history-error {
    margin: 0;
    font-size: 0.85rem;
  }

  /* #460 W3 — the sessions empty line gets the sky treatment. */
  .history-line.starfield {
    border: 1px dashed var(--outline-variant);
    border-radius: var(--radius-md);
    padding: var(--sp-md);
    text-align: center;
    background-color: var(--surface-container-lowest);
    background-repeat: repeat;
    background-size: 220px 220px;
  }

  .history-error {
    color: var(--error, #b3261e);
  }
</style>