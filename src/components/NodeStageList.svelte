<!--
  CR-10 P1.6.3.2 — Node-mode stage list with the mutation context menu.

  Renders the active plan's stages as a vertical node stack (the
  constrained layout) and mutates them through the P1.6.3.1 IPCs:
  set_stage_enabled / reorder_stage / remove_stage. Remove on a
  destructive canonical type confirms through DestructiveConfirmDialog;
  required stages refuse removal (menu item disabled, backend also
  rejects). Every action's returned plan replaces the activePlan
  store value so the list, the run controls, and the header stay in
  sync with the durable state.

  "Re-run from here" (P6.1c) and "Reset to default params" (parameter
  panel, P1.6.3.4) have no IPC yet and are intentionally absent — see
  node-stage-list-logic.ts.
-->
<script lang="ts">
  import {
    removeStage,
    reorderStage,
    setStageEnabled,
    resetSessionToProjectDefault,
    type PipelinePlanDto,
  } from "../lib/astroforge-api";
  import { activePlan } from "../lib/pipeline-plan-store";
  import {
    menuActionsFor,
    stageNodesFromPlan,
    type StageMenuAction,
    type StageNodeView,
  } from "../lib/node-stage-list-logic";
  import DestructiveConfirmDialog from "./DestructiveConfirmDialog.svelte";

  let actionError: string | null = $state(null);
  let actionInFlight = $state(false);
  // Context menu: which row opened it + viewport position.
  let menuFor: StageNodeView | null = $state(null);
  let menuX = $state(0);
  let menuY = $state(0);
  // Destructive confirmation pending on this row.
  let confirmFor: StageNodeView | null = $state(null);

  let plan = $derived($activePlan);
  let nodes = $derived(plan ? stageNodesFromPlan(plan) : []);

  function errText(err: unknown): string {
    if (err instanceof Error) return err.message;
    if (typeof err === "object" && err !== null && "message" in err) {
      return String((err as { message: unknown }).message);
    }
    return JSON.stringify(err) ?? String(err);
  }

  function openMenu(event: MouseEvent, stage: StageNodeView) {
    event.preventDefault();
    event.stopPropagation();
    menuFor = stage;
    menuX = event.clientX;
    menuY = event.clientY;
  }

  function closeMenu() {
    menuFor = null;
  }

  function indexFor(stage: StageNodeView): number {
    return nodes.findIndex((n) => n.stage_id === stage.stage_id);
  }

  async function runAction(stage: StageNodeView, action: StageMenuAction) {
    if (actionInFlight) return;
    const current = $activePlan;
    if (!current) return;
    closeMenu();
    actionError = null;
    // Destructive removals confirm first (audit Q-3 convention).
    if (action.kind === "remove" && stage.destructive) {
      confirmFor = stage;
      return;
    }
    actionInFlight = true;
    try {
      await dispatch(current, stage, action);
    } catch (err) {
      actionError = errText(err);
    } finally {
      actionInFlight = false;
    }
  }

  async function dispatch(
    current: PipelinePlanDto,
    stage: StageNodeView,
    action: StageMenuAction,
  ): Promise<void> {
    const index = indexFor(stage);
    switch (action.kind) {
      case "toggle-enabled":
        activePlan.set(
          await setStageEnabled(current.plan_id, stage.stage_id, !stage.enabled),
        );
        break;
      case "move-up":
        activePlan.set(
          await reorderStage(current.plan_id, stage.stage_id, index - 1),
        );
        break;
      case "move-down":
        activePlan.set(
          await reorderStage(current.plan_id, stage.stage_id, index + 1),
        );
        break;
      case "remove":
        activePlan.set(await removeStage(current.plan_id, stage.stage_id));
        break;
    }
  }

  async function confirmRemove() {
    const stage = confirmFor;
    const current = $activePlan;
    if (!stage || !current) return;
    confirmFor = null;
    actionInFlight = true;
    actionError = null;
    try {
      activePlan.set(await removeStage(current.plan_id, stage.stage_id));
    } catch (err) {
      actionError = errText(err);
    } finally {
      actionInFlight = false;
    }
  }

  function cancelRemove() {
    confirmFor = null;
  }

  // CR-10 P1.6.4.3 — materialize the project's default graph for the
  // active session (round-trip re-bind, idempotent on an unchanged
  // default) and clear any override row.
  async function handleResetToDefault() {
    const current = $activePlan;
    if (!current || actionInFlight) return;
    actionInFlight = true;
    actionError = null;
    try {
      activePlan.set(
        await resetSessionToProjectDefault(
          current.project_id,
          current.session_id,
        ),
      );
    } catch (err) {
      actionError = errText(err);
    } finally {
      actionInFlight = false;
    }
  }
</script>

<svelte:window onclick={closeMenu} />

<section class="node-stage-list" aria-label="Pipeline stages (node mode)">
  <div class="list-header">
    <h3 class="font-display">Stages</h3>
    {#if plan}
      <button
        type="button"
        class="reset-btn font-body"
        onclick={handleResetToDefault}
        disabled={actionInFlight}
        title="Materialize the project's default graph for this session"
      >
        Reset to project default
      </button>
    {/if}
  </div>
  {#if actionError}
    <p class="action-error font-body" role="alert">{actionError}</p>
  {/if}
  {#if !plan}
    <p class="empty font-body">No active plan — generate one to edit the graph.</p>
  {:else if nodes.length === 0}
    <p class="empty font-body">No stages yet — insert from the palette above.</p>
  {:else}
    <ol class="stack">
      {#each nodes as node, i (node.stage_id)}
        <li
          class="node"
          class:disabled-row={!node.enabled}
          class:destructive={node.destructive}
        >
          <button
            type="button"
            class="row"
            aria-haspopup="menu"
            aria-label="{node.label} — open stage actions"
            onclick={(e) => openMenu(e, node)}
            oncontextmenu={(e) => openMenu(e, node)}
            disabled={actionInFlight}
          >
            <span class="seq font-data">{node.sequence}</span>
            <span class="meta">
              <span class="label font-body">{node.label}</span>
              <span class="type font-data">{node.stage_type}</span>
            </span>
            <span class="badges">
              {#if node.required}
                <span class="badge required font-label">required</span>
              {/if}
              {#if node.destructive}
                <span class="badge destructive-badge font-label">destructive</span>
              {/if}
              <span
                class="badge state font-label"
                class:state-off={!node.enabled}
              >
                {node.enabled ? "enabled" : "disabled"}
              </span>
            </span>
            <span class="material-symbols-outlined kebab" aria-hidden="true"
              >more_vert</span
            >
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</section>

{#if menuFor}
  <div
    class="context-menu"
    role="menu"
    aria-label="Stage actions"
    style="left: {menuX}px; top: {menuY}px;"
  >
    {#each menuActionsFor(menuFor, indexFor(menuFor), nodes.length) as action (action.kind)}
      <button
        type="button"
        role="menuitem"
        class="menu-item font-body"
        class:menu-danger={action.kind === "remove"}
        disabled={action.disabled || actionInFlight}
        onclick={() => runAction(menuFor!, action)}
      >
        {action.label}
      </button>
    {/each}
  </div>
{/if}

<DestructiveConfirmDialog
  open={confirmFor !== null}
  title="Remove {confirmFor?.label ?? 'stage'}?"
  body="{confirmFor?.label ?? 'This stage'} is a destructive stage: removing it
    discards its parameters and its position in the pipeline. The stage can be
    re-inserted from the palette afterwards."
  confirmLabel="Remove stage"
  onConfirm={confirmRemove}
  onCancel={cancelRemove}
/>

<style>
  .node-stage-list {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    margin-top: var(--sp-md);
  }

  .list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-sm);
  }

  h3 {
    margin: 0;
    font-size: 1rem;
    color: var(--on-surface);
  }

  .reset-btn {
    appearance: none;
    border: 1px solid var(--outline-variant);
    background: var(--surface-container);
    color: var(--on-surface);
    border-radius: var(--radius-default);
    padding: 3px var(--sp-sm);
    font-size: 0.75rem;
    cursor: pointer;
    transition: border-color var(--transition-fast);
  }

  .reset-btn:hover:not(:disabled) {
    border-color: var(--cobalt-accent);
    box-shadow: 0 0 8px var(--glow-cobalt-dim);
  }

  .reset-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .empty {
    color: var(--on-surface-variant);
    font-size: 0.9rem;
    margin: 0;
  }

  .action-error {
    color: var(--error);
    font-size: 0.85rem;
    margin: 0;
  }

  .stack {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .node {
    position: relative;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    width: 100%;
    text-align: left;
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    padding: var(--sp-sm) var(--sp-md);
    cursor: context-menu;
    color: var(--on-surface);
    box-shadow: var(--elevation-1);
    transition: border-color var(--transition-fast), background var(--transition-fast);
  }

  .row:hover {
    background: var(--surface-container);
    border-color: var(--outline);
  }

  .row:disabled {
    opacity: 0.6;
    cursor: wait;
  }

  .disabled-row .row {
    opacity: 0.65;
  }

  .seq {
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    min-width: 1.2rem;
    text-align: right;
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .label {
    font-size: 0.9rem;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .type {
    font-size: 0.72rem;
    color: var(--on-surface-variant);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badges {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    flex-shrink: 0;
  }

  .badge {
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: var(--ls-label);
    padding: 2px 6px;
    border-radius: var(--radius-full);
    border: 1px solid var(--outline-variant);
    color: var(--on-surface-variant);
  }

  .badge.required {
    border-color: var(--tertiary-container);
    color: var(--tertiary);
  }

  .badge.destructive-badge {
    border-color: var(--error-container);
    color: var(--error);
  }

  .badge.state {
    border-color: var(--success-border);
    color: var(--success-fg);
  }

  .badge.state-off {
    border-color: var(--outline-variant);
    color: var(--on-surface-variant);
  }

  .kebab {
    font-size: 18px;
    color: var(--on-surface-variant);
  }

  .context-menu {
    position: fixed;
    z-index: 90;
    min-width: 160px;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    box-shadow: var(--elevation-3);
    padding: var(--sp-xs);
    display: flex;
    flex-direction: column;
  }

  .menu-item {
    appearance: none;
    border: none;
    background: transparent;
    color: var(--on-surface);
    text-align: left;
    padding: 6px var(--sp-sm);
    border-radius: var(--radius-default);
    cursor: pointer;
    font-size: 0.88rem;
  }

  .menu-item:hover:not(:disabled) {
    background: var(--accent-tint);
  }

  .menu-item:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .menu-danger {
    color: var(--error);
  }
</style>
