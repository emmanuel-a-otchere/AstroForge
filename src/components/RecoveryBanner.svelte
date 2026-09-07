<!--
  CR-05 P2.5 — RecoveryBanner component.

  Shown on Project open when at least one PipelinePlan is in `Paused`
  status for the current project. Per CR-05 §10 + the P2.5 UX contract:
  the user should be able to resume the paused run without restarting
  from scratch.

  Props:
    projectId — required to call refreshResumablePlans on mount.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    lastError,
    loadPlan,
    refreshResumablePlans,
    refreshStageExecutions,
    resumablePlans,
    resumePipelineRun,
    runStatus,
  } from "../lib/pipeline-plan-store";

  export let projectId: string;

  let resumingId: string | null = null;

  onMount(() => {
    refreshResumablePlans(projectId);
  });

  async function onResume(planId: string) {
    resumingId = planId;
    try {
      await resumePipelineRun(planId);
      await loadPlan(planId);
      await refreshStageExecutions(planId);
      await refreshResumablePlans(projectId);
    } finally {
      resumingId = null;
    }
  }

  async function onDismiss(planId: string) {
    // CR-05 P2.5 — dismissing a recovery banner is a no-op for the
    // store; the user can still see the plan via PlanList. The banner
    // reappears on next project open. Future P3+ may add a "mark
    // as not-recoverable" affordance; slice 2.5 keeps it minimal.
    resumablePlans.update((list) => list.filter((p) => p.plan_id !== planId));
  }
</script>

{#if $resumablePlans.length > 0}
  <section class="recovery-banner" aria-label="Recoverable pipeline plans">
    <header class="banner-header">
      <span class="material-symbols-outlined" aria-hidden="true">history</span>
      <h2 class="font-display">Recover previous processing</h2>
    </header>
    <p class="font-body banner-description">
      These plans were paused mid-run. Resume to continue from the
      last completed stage.
    </p>

    <ul class="plan-list" data-testid="recovery-plan-list">
      {#each $resumablePlans as plan (plan.plan_id)}
        <li class="plan-row" data-testid="recovery-plan-row">
          <div class="plan-meta">
            <span class="plan-target font-body">
              {plan.target_type.replace(/_/g, " ")}
            </span>
            <span class="plan-mode font-label">{plan.mode}</span>
            <span class="plan-created font-label">
              Created {plan.created_at}
            </span>
          </div>
          <div class="plan-actions">
            <button
              type="button"
              class="resume font-label"
              on:click={() => onResume(plan.plan_id)}
              disabled={$runStatus === "running" || resumingId === plan.plan_id}
              data-testid="resume-plan"
            >
              {resumingId === plan.plan_id ? "Resuming…" : "Resume"}
            </button>
            <button
              type="button"
              class="dismiss font-label"
              on:click={() => onDismiss(plan.plan_id)}
              data-testid="dismiss-recovery"
            >
              Dismiss
            </button>
          </div>
        </li>
      {/each}
    </ul>

    {#if $lastError}
      <p class="error font-body" role="alert">{$lastError}</p>
    {/if}
  </section>
{/if}

<style>
  .recovery-banner {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-lg);
    background: var(--surface-container);
    border-left: 4px solid #ffb300;
    border-radius: var(--radius-md);
  }
  .banner-header {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
  }
  .banner-header h2 {
    margin: 0;
    font-size: 1.1rem;
  }
  .banner-description {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }
  .plan-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }
  .plan-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--surface-container-low);
    border-radius: var(--radius-sm);
    padding: var(--sp-sm) var(--sp-md);
  }
  .plan-meta {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }
  .plan-target {
    color: var(--on-surface);
    font-size: 0.95rem;
  }
  .plan-mode,
  .plan-created {
    color: var(--on-surface-variant);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .plan-actions {
    display: flex;
    gap: var(--sp-sm);
  }
  .resume {
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    border-radius: var(--radius-full);
    padding: var(--sp-xs) var(--sp-md);
    cursor: pointer;
  }
  .resume:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .dismiss {
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline);
    border-radius: var(--radius-full);
    padding: var(--sp-xs) var(--sp-md);
    cursor: pointer;
  }
  .error {
    color: #ff8a80;
    background: rgba(229, 57, 53, 0.12);
    padding: var(--sp-xs) var(--sp-sm);
    border-radius: var(--radius-sm);
  }
</style>