<!--
  CR-03 P5 — version timeline strip.

  R3 update: the timeline now reads from the durable
  `image_version_list` IPC (via `versionStore`) instead of the
  removed placeholder data layer. The IPC returns
  `ImageVersion` records with `label`, `sequence`,
  `primary_artifact_id`, `created_at`, and `hidden`. The card
  renders these fields directly — no fake titles, no fake
  source pills. Status is always "in review" (a future
  promote-to-final action ships when the AI ops Rust migration
  lands).

  Layout:
    - Single-row horizontal scroll on desktop
    - Vertical stack on narrow viewports
-->
<script lang="ts">
  import { versionStore, type ImageVersion } from "../state/versions";

  let {
    onSelect,
  }: { onSelect?: (version: ImageVersion) => void } = $props();

  let selected = $state<string | null>(null);

  function selectVersion(v: ImageVersion) {
    selected = v.version_id;
    onSelect?.(v);
  }

  function formatDate(iso: string): string {
    if (!iso) return "—";
    try {
      return new Date(iso).toLocaleDateString(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
      });
    } catch {
      return iso;
    }
  }
</script>

{#if $versionStore.versions.length === 0}
  <p class="empty font-body">
    No image versions yet. A version appears in the timeline
    when the project records a `VersionCreated` event.
  </p>
{:else}
  <ol class="timeline" aria-label="Image version timeline">
    {#each $versionStore.versions as version (version.version_id)}
      <li class="version-card" data-status="in_review">
        <button
          type="button"
          class="version-button"
          data-active={selected === version.version_id}
          onclick={() => selectVersion(version)}
          aria-label="Select {version.label}"
        >
          <div class="version-header">
            <span class="version-label font-label">{version.label}</span>
            <span class="status-pill" data-status="in_review">
              in review
            </span>
          </div>
          <p class="version-meta font-body">
            {formatDate(version.created_at)}
          </p>
          <p class="version-source font-body">
            Sequence #{version.sequence}
            {#if version.primary_artifact_id}
              · artifact {version.primary_artifact_id}
            {:else}
              · no artifact yet
            {/if}
          </p>
        </button>
        {#if version.source_version_id}
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
    flex: 0 0 240px;
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

  .status-pill[data-status="in_review"] {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
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
