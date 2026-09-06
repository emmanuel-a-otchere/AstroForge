<!--
  CR-03 P4 — Import workspace content.

  §12 says: "Import is the moment the user brings new data into a
  project. The wizard drives the user through Source Selection,
  Calibration Selection, Plate Solve, and Stacking Result. P4
  ships the workspace layout + the §26 first-time empty state.
  The actual import controls land in P4b alongside the
  wizard deprecation."

  CR-04 (Intelligent Import + Session Understanding + Target
  Detection) extends this surface with the auto-detect flow.
-->
<script lang="ts">
  import WorkspaceScreen from "./WorkspaceScreen.svelte";
</script>

<WorkspaceScreen
  title="Import"
  icon="upload_file"
  description="Bring new data into the project. Drop a folder of captures, calibration frames, or stacked results and AstroForge handles the rest."
>
  <div class="empty-state">
    <span class="material-symbols-outlined empty-icon" aria-hidden="true">
      cloud_upload
    </span>
    <p class="empty-title font-display">Drop a folder of captures</p>
    <p class="empty-body font-body">
      Import walks you through plate-solving, calibration matching, and
      stacking. The first run is the one that takes the longest; every run
      after is faster.
    </p>
    <div class="empty-actions">
      <button type="button" class="primary-cta font-display" disabled>
        Select files…
      </button>
    </div>
    <p class="hint font-body">
      File selection lands in P4b. The intelligent ingest flow (drop folder
      → detect → classify → explain → confirm → build session) is the
      <a href="docs/CR-03-APPLICATION-SHELL-STUDIO.md">CR-04</a> scope.
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

  .empty-actions {
    display: flex;
    gap: var(--sp-sm);
  }

  .primary-cta {
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    padding: var(--sp-sm) var(--sp-lg);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.95rem;
    font-weight: 600;
  }

  .primary-cta:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .hint {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
    margin: 0;
  }

  .hint a {
    color: var(--primary);
  }
</style>