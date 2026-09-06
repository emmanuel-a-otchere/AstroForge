<!--
  CR-03 P5 — Compare workspace.

  Two dropdowns (Version A / Version B) plus a placeholder for the
  dual-canvas view. The actual canvas needs gl-renderer.ts which
  was deleted in P4b; until P5a salvages it or a new renderer
  lands, the canvas area shows an info card explaining what
  Compare will show.
-->
<script lang="ts">
  import WorkspaceScreen from "./WorkspaceScreen.svelte";
  import { versionStore, type ImageVersion } from "../state/versions";

  let aId = $state<string | null>(null);
  let bId = $state<string | null>(null);

  // Default A to the oldest, B to the latest.
  $effect(() => {
    const versions = $versionStore.versions;
    if (versions.length >= 2 && aId === null && bId === null) {
      aId = versions[0].id;
      bId = versions[versions.length - 1].id;
    }
  });

  const versionA = $derived<ImageVersion | null>(
    $versionStore.versions.find((v) => v.id === aId) ?? null,
  );
  const versionB = $derived<ImageVersion | null>(
    $versionStore.versions.find((v) => v.id === bId) ?? null,
  );
</script>

<WorkspaceScreen
  title="Compare"
  icon="compare"
  description="Side-by-side comparison between any two image versions of this project. Compare a stacked result against a stretched version, or before/after a recommended enhancement."
>
  {#if $versionStore.versions.length < 2}
    <div class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">
        compare
      </span>
      <p class="empty-title font-display">Need two or more versions</p>
      <p class="empty-body font-body">
        Compare needs at least two image versions. Run a pipeline and pick
        a recommendation to build up the timeline.
      </p>
    </div>
  {:else}
    <div class="picker-row">
      <label class="picker">
        <span class="picker-label font-label">Version A</span>
        <select bind:value={aId} class="picker-select">
          {#each $versionStore.versions as v (v.id)}
            <option value={v.id}>{v.label} — {v.title}</option>
          {/each}
        </select>
      </label>
      <span class="material-symbols-outlined vs-icon" aria-hidden="true">
        compare_arrows
      </span>
      <label class="picker">
        <span class="picker-label font-label">Version B</span>
        <select bind:value={bId} class="picker-select">
          {#each $versionStore.versions as v (v.id)}
            <option value={v.id}>{v.label} — {v.title}</option>
          {/each}
        </select>
      </label>
    </div>

    <div class="canvas-area" aria-label="Side-by-side compare canvas">
      <div class="canvas-pane">
        <div class="canvas-label">
          <span class="font-label">{versionA?.label ?? "—"}</span>
          <span class="font-body">{versionA?.title ?? ""}</span>
        </div>
        <div class="canvas-placeholder" data-side="A">
          <span class="material-symbols-outlined" aria-hidden="true">
            image
          </span>
          <p class="font-body">Canvas A renders here</p>
        </div>
      </div>
      <div class="canvas-pane">
        <div class="canvas-label">
          <span class="font-label">{versionB?.label ?? "—"}</span>
          <span class="font-body">{versionB?.title ?? ""}</span>
        </div>
        <div class="canvas-placeholder" data-side="B">
          <span class="material-symbols-outlined" aria-hidden="true">
            image
          </span>
          <p class="font-body">Canvas B renders here</p>
        </div>
      </div>
    </div>

    <p class="hint font-body">
      Dual-canvas rendering lands once the renderer is salvaged from the
      wizard code (or replaced in P5a).
    </p>
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

  .picker-row {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: end;
    gap: var(--sp-md);
  }

  .picker {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .picker-label {
    font-size: 0.75rem;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .picker-select {
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface);
    font-size: 0.95rem;
    font-family: inherit;
  }

  .picker-select:focus {
    outline: none;
    border-color: var(--primary);
  }

  .vs-icon {
    font-size: 32px;
    color: var(--primary);
    align-self: center;
    padding-bottom: var(--sp-xs);
  }

  .canvas-area {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-md);
    margin-top: var(--sp-md);
  }

  .canvas-pane {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .canvas-label {
    display: flex;
    align-items: baseline;
    gap: var(--sp-sm);
    font-size: 0.85rem;
  }

  .canvas-label .font-label {
    font-weight: 600;
    color: var(--primary);
  }

  .canvas-label .font-body {
    color: var(--on-surface-variant);
  }

  .canvas-placeholder {
    aspect-ratio: 4 / 3;
    background: var(--surface-container-low);
    border: 1px dashed var(--outline-variant);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sp-xs);
    color: var(--on-surface-variant);
  }

  .canvas-placeholder .material-symbols-outlined {
    font-size: 48px;
    opacity: 0.5;
  }

  .hint {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
    margin: 0;
  }

  @media (max-width: 720px) {
    .picker-row,
    .canvas-area {
      grid-template-columns: 1fr;
    }
    .vs-icon {
      transform: rotate(90deg);
    }
  }
</style>