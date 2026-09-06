<!--
  CR-03 P4 — Enhance workspace content (§13).

  The Enhance workspace is where AI-as-intelligence meets the image:
  recommendations, accept/reject, change provenance. P4 ships the
  layout + §26 empty state. P5 wires the AI recommendations +
  provenance timeline.
-->
<script lang="ts">
  import WorkspaceScreen from "./WorkspaceScreen.svelte";
  import RecommendationCard from "./RecommendationCard.svelte";
  import { versionStore } from "../state/versions";
</script>

<WorkspaceScreen
  title="Enhance"
  icon="tune"
  description="AI-assisted recommendations that propose tweaks to the latest image version. Accept or reject each suggestion; every accepted change is recorded on the version timeline."
>
  {#if $versionStore.recommendations.length === 0}
    <div class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">
        auto_awesome
      </span>
      <p class="empty-title font-display">No recommendations yet</p>
      <p class="empty-body font-body">
        Once a pipeline run completes, AstroForge analyses the result and
        proposes a small set of targeted enhancements. Each recommendation
        explains what it would change and why.
      </p>
    </div>
  {:else}
    <ul class="rec-list" aria-label="AI recommendations">
      {#each $versionStore.recommendations as rec (rec.id)}
        <li>
          <RecommendationCard recommendation={rec} />
        </li>
      {/each}
    </ul>
  {/if}
</WorkspaceScreen>

<style>
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

  .rec-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }
</style>