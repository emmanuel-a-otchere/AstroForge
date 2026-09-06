<!--
  CR-03 P4 — Export workspace content (§13).

  Select an image version and an export format. P4 ships the layout
  + the format-picker stub. P5/P6 wire the actual export IPC.
-->
<script lang="ts">
  import WorkspaceScreen from "./WorkspaceScreen.svelte";
</script>

<WorkspaceScreen
  title="Export"
  icon="download"
  description="Export a final image version to the format of your choice. AstroForge keeps a full export history so you can re-export any past version at any time."
>
  <div class="empty-state">
    <span class="material-symbols-outlined empty-icon" aria-hidden="true">
      download
    </span>
    <p class="empty-title font-display">Nothing to export yet</p>
    <p class="empty-body font-body">
      Run a pipeline, accept any AI recommendations, and the final image
      version will be available to export here.
    </p>
    <p class="hint font-body">
      Export format picker + actual export IPC land in P5/P6.
    </p>
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