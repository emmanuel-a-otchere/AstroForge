<!--
  CR-03 P5 — Export workspace.

  Format picker + version picker + disabled Export button. The
  actual export IPC lands in a future Rust migration (CR-04+ or
  P5a). P5 ships the picker UX so the visitor can see what the
  final workflow will look like.
-->
<script lang="ts">
  import WorkspaceScreen from "./WorkspaceScreen.svelte";
  import { latestVersion, versionStore } from "../state/versions";

  type ExportFormat = "png16" | "tiff32" | "fits" | "jpeg";

  interface FormatDef {
    id: ExportFormat;
    label: string;
    description: string;
  }

  const FORMATS: readonly FormatDef[] = [
    {
      id: "png16",
      label: "PNG (16-bit)",
      description: "Lossless, widely supported. Best for web sharing.",
    },
    {
      id: "tiff32",
      label: "TIFF (32-bit float)",
      description: "Lossless, archival. Preserves linear data.",
    },
    {
      id: "fits",
      label: "FITS",
      description: "Astronomy standard. Preserves metadata + WCS.",
    },
    {
      id: "jpeg",
      label: "JPEG (8-bit sRGB)",
      description: "Small, lossy. For previews and social media.",
    },
  ];

  let selectedFormat = $state<ExportFormat>("png16");
  let selectedVersionId = $state<string | null>(null);

  // Default to the latest version once the store loads.
  $effect(() => {
    if (selectedVersionId === null && $latestVersion) {
      selectedVersionId = $latestVersion.version_id;
    }
  });
</script>

<WorkspaceScreen
  title="Export"
  icon="download"
  description="Export a final image version to the format of your choice. AstroForge keeps a full export history so you can re-export any past version at any time."
>
  {#if $versionStore.versions.length === 0}
    <div class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">
        download
      </span>
      <p class="empty-title font-display">Nothing to export yet</p>
      <p class="empty-body font-body">
        Run a pipeline, accept any AI recommendations, and the final
        image version will be available to export here.
      </p>
    </div>
  {:else}
    <div class="export-grid">
      <section class="version-picker" aria-labelledby="version-pick-title">
        <h2 id="version-pick-title" class="font-display picker-title">
          Version
        </h2>
        <ul class="version-list">
          {#each $versionStore.versions as v (v.version_id)}
            <li>
              <label class="version-row" data-active={selectedVersionId === v.version_id}>
                <input
                  type="radio"
                  name="export-version"
                  value={v.version_id}
                  bind:group={selectedVersionId}
                  class="version-radio"
                />
                <div class="version-info">
                  <span class="version-label font-label">{v.label}</span>
                  <span class="version-title font-display">
                    Sequence #{v.sequence}
                  </span>
                  <span class="version-meta font-body">
                    in review ·
                    {new Date(v.created_at).toLocaleDateString()}
                    {#if v.primary_artifact_id}
                      · {v.primary_artifact_id}
                    {:else}
                      · no artifact yet
                    {/if}
                  </span>
                </div>
              </label>
            </li>
          {/each}
        </ul>
      </section>

      <section class="format-picker" aria-labelledby="format-pick-title">
        <h2 id="format-pick-title" class="font-display picker-title">
          Format
        </h2>
        <ul class="format-list">
          {#each FORMATS as fmt (fmt.id)}
            <li>
              <label class="format-row" data-active={selectedFormat === fmt.id}>
                <input
                  type="radio"
                  name="export-format"
                  value={fmt.id}
                  bind:group={selectedFormat}
                  class="format-radio"
                />
                <div class="format-info">
                  <span class="format-label font-display">{fmt.label}</span>
                  <span class="format-desc font-body">{fmt.description}</span>
                </div>
              </label>
            </li>
          {/each}
        </ul>
      </section>
    </div>

    <footer class="export-footer">
      <button type="button" class="primary-cta font-display" disabled>
        Export…
      </button>
      <p class="hint font-body">
        Export IPC lands in P5a once the Rust side ships image_versions
        + exports tables.
      </p>
    </footer>
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

  .export-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-lg);
  }

  .picker-title {
    font-size: 1.05rem;
    margin: 0 0 var(--sp-sm);
    color: var(--on-surface);
  }

  .version-list,
  .format-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .version-row,
  .format-row {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-sm);
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    cursor: pointer;
  }

  .version-row:hover,
  .format-row:hover {
    border-color: var(--outline);
  }

  .version-row[data-active="true"],
  .format-row[data-active="true"] {
    border-color: var(--primary);
    background: var(--surface-container);
  }

  .version-radio,
  .format-radio {
    margin-top: 4px;
    flex-shrink: 0;
  }

  .version-info,
  .format-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .version-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--primary);
  }

  .version-title {
    font-size: 0.95rem;
    font-weight: 600;
  }

  .version-meta {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
  }

  .format-label {
    font-size: 0.95rem;
    font-weight: 600;
  }

  .format-desc {
    font-size: 0.85rem;
    color: var(--on-surface-variant);
  }

  .export-footer {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: var(--sp-sm);
  }

  .primary-cta {
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    padding: var(--sp-sm) var(--sp-xl);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 1rem;
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

  @media (max-width: 720px) {
    .export-grid {
      grid-template-columns: 1fr;
    }
  }
</style>