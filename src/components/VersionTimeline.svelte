<!--
  CR-03 P5 — version timeline strip.

  Horizontal timeline of image versions for the active project.
  Each card shows: label (v1/v2/v3), title, source pill, status
  pill, created_at. Selected version is highlighted.

  Layout:
    - Single-row horizontal scroll on desktop
    - Vertical stack on narrow viewports
-->
<script lang="ts">
  import {
    versionStore,
    type ImageVersion,
  } from "../state/versions";

  let {
    onSelect,
  }: { onSelect?: (version: ImageVersion) => void } = $props();

  let selected = $state<string | null>(null);

  function selectVersion(v: ImageVersion) {
    selected = v.id;
    onSelect?.(v);
  }
</script>

{#if $versionStore.versions.length === 0}
  <p class="empty font-body">No image versions yet.</p>
{:else}
  <ol class="timeline" aria-label="Image version timeline">
    {#each $versionStore.versions as version (version.id)}
      <li class="version-card" data-status={version.status}>
        <button
          type="button"
          class="version-button"
          data-active={selected === version.id}
          onclick={() => selectVersion(version)}
          aria-label="Select {version.label}: {version.title}"
        >
          <div class="version-header">
            <span class="version-label font-label">{version.label}</span>
            <span class="status-pill" data-status={version.status}>
              {version.status.replace("_", " ")}
            </span>
          </div>
          <h3 class="version-title font-display">{version.title}</h3>
          <p class="version-meta font-body">
            {new Date(version.created_at).toLocaleDateString(undefined, {
              year: "numeric",
              month: "short",
              day: "numeric",
            })}
          </p>
          <p class="version-source font-body">
            {#if version.source.kind === "pipeline_run"}
              From pipeline run
            {:else if version.source.kind === "ai_recommendation"}
              From AI recommendation
            {:else}
              Manual edit
            {/if}
          </p>
        </button>
        {#if version.parent_version_id}
          <span class="material-symbols-outlined lineage" aria-hidden="true">
            arrow_back
          </span>
        {/if}
      </li>
    {/each}
  </ol>
{/if}

<style>
  .timeline {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    gap: var(--sp-sm);
    overflow-x: auto;
    padding-bottom: var(--sp-xs);
  }

  .version-card {
    position: relative;
    flex: 0 0 220px;
  }

  .version-button {
    width: 100%;
    text-align: left;
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    padding: var(--sp-md);
    color: var(--on-surface);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .version-button:hover {
    border-color: var(--outline);
  }

  .version-button[data-active="true"] {
    border-color: var(--primary);
    background: var(--surface-container);
  }

  .version-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .version-label {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--primary);
  }

  .status-pill {
    font-size: 0.65rem;
    padding: 2px var(--sp-xs);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .status-pill[data-status="final"] {
    background: rgba(111, 191, 115, 0.2);
    color: #6fbf73;
  }

  .status-pill[data-status="in_review"] {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
  }

  .status-pill[data-status="exported"] {
    background: rgba(255, 179, 0, 0.2);
    color: #ffb300;
  }

  .version-title {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    line-height: 1.2;
  }

  .version-meta {
    font-size: 0.75rem;
    color: var(--on-surface-variant);
    margin: 0;
  }

  .version-source {
    font-size: 0.75rem;
    color: var(--on-surface-variant);
    margin: 0;
    font-style: italic;
  }

  .lineage {
    position: absolute;
    left: -12px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 18px;
    color: var(--on-surface-variant);
    background: var(--surface);
    border-radius: var(--radius-full);
  }

  .empty {
    color: var(--on-surface-variant);
    font-size: 0.9rem;
    text-align: center;
    padding: var(--sp-md);
  }

  @media (max-width: 720px) {
    .timeline {
      flex-direction: column;
      overflow-x: visible;
    }
    .version-card {
      flex: 0 0 auto;
    }
    .lineage {
      display: none;
    }
  }
</style>