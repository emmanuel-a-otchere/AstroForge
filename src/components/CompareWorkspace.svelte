<!--
  CR-03 P4 — Compare workspace content (§13).

  Side-by-side comparison between two image versions of the same
  project. P4 ships the layout; P5 wires the picker + the dual-canvas
  compare view.
-->
<script lang="ts">
  import WorkspaceScreen from "./WorkspaceScreen.svelte";
</script>

<WorkspaceScreen
  title="Compare"
  icon="compare"
  description="Side-by-side comparison between any two image versions of this project. Compare a stacked result against a stretched version, or before/after a recommended enhancement."
>
  <div class="empty-state">
    <span class="material-symbols-outlined empty-icon" aria-hidden="true">
      compare
    </span>
    <p class="empty-title font-display">No image versions yet</p>
    <p class="empty-body font-body">
      Once at least one pipeline run completes, the resulting image version
      shows up here. Compare any two versions side-by-side to see exactly
      what changed.
    </p>
    <p class="hint font-body">Dual-canvas compare lands in P5.</p>
  </div>
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

  .hint {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
    margin: 0;
  }
</style>