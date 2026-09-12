<!--
  CR-03 P5 — Compare workspace (R3 update).

  R3 replaces the placeholder canvas with honest per-version
  metadata cards. The IPC returns a real timeline derived from
  the durable event log; the cards render what the durable
  state actually says. No fake images, no "Canvas A renders
  here" placeholders.

  Image rendering is intentionally absent: there is no real
  pixel artifact to display for any version yet (the
  canonical `image_versions` table is on the audit roadmap).
  When the table ships and a primary artifact is recorded, the
  card can call `read_preview_artifact` (or a new
  `read_image_artifact`) to surface the actual bytes.

  Side-by-side comparison is preserved: A and B are selected
  from the same real timeline, the cards render side by side,
  and a "swap" button lets the user flip them.
-->
<script lang="ts">
  import WorkspaceScreen from "./WorkspaceScreen.svelte";
  import ImageCanvas from "./ImageCanvas.svelte";
  import { versionStore, type ImageVersion } from "../state/versions";

  let aId = $state<string | null>(null);
  let bId = $state<string | null>(null);

  // R4: the project lifecycle module loads the version
  // timeline on project open. The remaining reactive
  // subscription below is a safety net for the case where
  // the active project changes without going through the
  // lifecycle (e.g. during the close transition while the
  // pipeline plan store resets).

  // R3: when the timeline arrives, default A to the earliest
  // and B to the latest. This is the only "magic" the picker
  // does; everything else is direct rendering of the IPC's
  // output.
  $effect(() => {
    const versions = $versionStore.versions;
    if (versions.length >= 2 && aId === null && bId === null) {
      aId = versions[0].version_id;
      bId = versions[versions.length - 1].version_id;
    }
  });

  const versionA = $derived<ImageVersion | null>(
    $versionStore.versions.find((v) => v.version_id === aId) ?? null,
  );
  const versionB = $derived<ImageVersion | null>(
    $versionStore.versions.find((v) => v.version_id === bId) ?? null,
  );

  function swap() {
    [aId, bId] = [bId, aId];
  }

  function formatDate(iso: string): string {
    if (!iso) return "—";
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }

  function statusFor(): "in_review" {
    // IPC does not carry a status. A future promote action
    // can move a version to "final"; the timeline card stays
    // honest with "in review" until then.
    return "in_review";
  }
</script>

<WorkspaceScreen
  title="Compare"
  icon="compare"
  description="Side-by-side comparison between any two image versions of this project. Versions are derived from the durable event log; cards show what the project actually has."
>
  {#if $versionStore.loading}
    <p class="state-line font-body" aria-live="polite">
      Loading version timeline…
    </p>
  {:else if $versionStore.error}
    <div class="state-card font-body" role="alert">
      <span class="material-symbols-outlined" aria-hidden="true">
        error
      </span>
      <div>
        <h2 class="font-display">Timeline unavailable</h2>
        <p>{$versionStore.error}</p>
      </div>
    </div>
  {:else if $versionStore.versions.length < 2}
    <div class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">
        compare
      </span>
      <p class="empty-title font-display">Need two or more versions</p>
      <p class="empty-body font-body">
        Compare needs at least two image versions. The project records
        a version when a pipeline run completes and writes a
        <code>VersionCreated</code> event. Run a pipeline to build up
        the timeline.
      </p>
    </div>
  {:else}
    <div class="picker-row">
      <label class="picker">
        <span class="picker-label font-label">Version A</span>
        <select bind:value={aId} class="picker-select">
          {#each $versionStore.versions as v (v.version_id)}
            <option value={v.version_id}>
              {v.label} — sequence #{v.sequence}
            </option>
          {/each}
        </select>
      </label>
      <button
        type="button"
        class="swap-cta"
        onclick={swap}
        aria-label="Swap version A and version B"
      >
        <span class="material-symbols-outlined" aria-hidden="true">
          compare_arrows
        </span>
        <span class="font-body">Swap</span>
      </button>
      <label class="picker">
        <span class="picker-label font-label">Version B</span>
        <select bind:value={bId} class="picker-select">
          {#each $versionStore.versions as v (v.version_id)}
            <option value={v.version_id}>
              {v.label} — sequence #{v.sequence}
            </option>
          {/each}
        </select>
      </label>
    </div>

    <div class="canvas-area" aria-label="Side-by-side compare cards">
      <article class="canvas-pane" data-side="A">
        <header class="pane-header">
          <span class="pane-tag font-label">Version A</span>
          <h2 class="pane-title font-display">
            {versionA?.label ?? "—"}
          </h2>
        </header>
        {#if versionA}
          <dl class="meta">
            <dt>Sequence</dt>
            <dd>#{versionA.sequence}</dd>
            <dt>Created</dt>
            <dd>{formatDate(versionA.created_at)}</dd>
            <dt>Artifact</dt>
            <dd>
              {#if versionA.primary_artifact_id}
                <code>{versionA.primary_artifact_id}</code>
              {:else}
                <span class="muted">Not linked yet</span>
              {/if}
            </dd>
            <dt>Status</dt>
            <dd>
              <span class="status-pill" data-status={statusFor()}>
                {statusFor().replace("_", " ")}
              </span>
            </dd>
            {#if versionA.source_version_id}
              <dt>Branched from</dt>
              <dd><code>{versionA.source_version_id}</code></dd>
            {/if}
          </dl>
          {#if versionA.primary_artifact_id}
            <div class="canvas-mount" data-testid="compare-canvas-a">
              <ImageCanvas versionId={versionA.version_id} />
            </div>
            <p class="artifact-note font-body">
              <span class="material-symbols-outlined" aria-hidden="true">
                image
              </span>
              Rendering the applied artifact bytes. Drag to pan,
              scroll to zoom, shift-drag to inspect a region.
            </p>
          {:else}
            <p class="artifact-note font-body">
              <span class="material-symbols-outlined" aria-hidden="true">
                image_not_supported
              </span>
              This version has no primary artifact yet — pixel
              rendering needs the P5.1 apply round to write a TIFF.
            </p>
          {/if}
        {:else}
          <p class="muted font-body">Select a version for side A.</p>
        {/if}
      </article>
      <article class="canvas-pane" data-side="B">
        <header class="pane-header">
          <span class="pane-tag font-label">Version B</span>
          <h2 class="pane-title font-display">
            {versionB?.label ?? "—"}
          </h2>
        </header>
        {#if versionB}
          <dl class="meta">
            <dt>Sequence</dt>
            <dd>#{versionB.sequence}</dd>
            <dt>Created</dt>
            <dd>{formatDate(versionB.created_at)}</dd>
            <dt>Artifact</dt>
            <dd>
              {#if versionB.primary_artifact_id}
                <code>{versionB.primary_artifact_id}</code>
              {:else}
                <span class="muted">Not linked yet</span>
              {/if}
            </dd>
            <dt>Status</dt>
            <dd>
              <span class="status-pill" data-status={statusFor()}>
                {statusFor().replace("_", " ")}
              </span>
            </dd>
            {#if versionB.source_version_id}
              <dt>Branched from</dt>
              <dd><code>{versionB.source_version_id}</code></dd>
            {/if}
          </dl>
          {#if versionB.primary_artifact_id}
            <div class="canvas-mount" data-testid="compare-canvas-b">
              <ImageCanvas versionId={versionB.version_id} />
            </div>
            <p class="artifact-note font-body">
              <span class="material-symbols-outlined" aria-hidden="true">
                image
              </span>
              Rendering the applied artifact bytes. Drag to pan,
              scroll to zoom, shift-drag to inspect a region.
            </p>
          {:else}
            <p class="artifact-note font-body">
              <span class="material-symbols-outlined" aria-hidden="true">
                image_not_supported
              </span>
              This version has no primary artifact yet — pixel
              rendering needs the P5.1 apply round to write a TIFF.
            </p>
          {/if}
        {:else}
          <p class="muted font-body">Select a version for side B.</p>
        {/if}
      </article>
    </div>

    <p class="hint font-body">
      Both cards reflect the project's durable event log. The
      canvas renders the primary artifact TIFF written by the
      P5.1 apply round; pre-P5.1 versions show only metadata.
    </p>
  {/if}
</WorkspaceScreen>

<style>
  .state-line {
    color: var(--on-surface-variant);
  }

  .state-card {
    display: flex;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    color: var(--on-surface);
  }

  .state-card h2 {
    margin: 0 0 var(--sp-xs) 0;
    font-size: 1rem;
  }

  .state-card p {
    margin: 0;
    color: var(--on-surface-variant);
  }

  .state-card .material-symbols-outlined {
    color: #e53935;
    font-size: 28px;
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

  .empty-body code {
    font-family: var(--font-data, monospace);
    font-size: 0.9em;
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

  .swap-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: transparent;
    border: 1px solid var(--outline-variant);
    color: var(--on-surface);
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
    cursor: pointer;
    white-space: nowrap;
    align-self: end;
  }

  .swap-cta:hover {
    background: var(--surface-container);
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
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
  }

  .pane-header {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pane-tag {
    font-size: 0.7rem;
    color: var(--primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .pane-title {
    margin: 0;
    font-size: 1.1rem;
  }

  .meta {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--sp-xs) var(--sp-md);
    margin: 0;
  }

  .meta dt {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
  }

  .meta dd {
    margin: 0;
    font-size: 0.85rem;
  }

  .meta code {
    font-family: var(--font-data, monospace);
    font-size: 0.8em;
  }

  .muted {
    color: var(--on-surface-variant);
  }

  .status-pill {
    display: inline-block;
    padding: 2px var(--sp-xs);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .status-pill[data-status="in_review"] {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
  }

  .canvas-mount {
    width: 100%;
    height: 360px;
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: #000;
  }

  .artifact-note {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    margin: 0;
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border-radius: var(--radius-md);
    color: var(--on-surface-variant);
    font-size: 0.8rem;
  }

  .artifact-note .material-symbols-outlined {
    color: var(--primary);
  }

  .artifact-note code {
    font-family: var(--font-data, monospace);
    font-size: 0.9em;
  }

  .hint {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
    margin: 0;
  }

  .hint code {
    font-family: var(--font-data, monospace);
    font-size: 0.9em;
  }

  @media (max-width: 720px) {
    .picker-row,
    .canvas-area {
      grid-template-columns: 1fr;
    }
  }
</style>
