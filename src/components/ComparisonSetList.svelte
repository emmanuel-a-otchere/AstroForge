<!--
  CR-07 B4 — comparison sets (§16).

  A comparison set is a named, reusable collection of candidate
  Image Versions ("M42 Final Candidates": A Natural, B AI
  Enhanced, ...). This panel lists the project's sets, saves the
  current A/B selection as a new set, applies a set back to the
  workspace pickers, and deletes sets.

  Deleting a set never touches the underlying Image Versions
  (ADR-07.4: comparison is non-destructive).
-->
<script lang="ts">
  import { comparisonState } from "../state/comparison";

  interface Props {
    projectId: string;
    currentA: string | null;
    currentB: string | null;
    labelA: string;
    labelB: string;
    /** Apply a set's first two versions back to the workspace
     *  A/B pickers. */
    onApply: (versionIdA: string, versionIdB: string) => void;
  }
  const { projectId, currentA, currentB, labelA, labelB, onApply }: Props =
    $props();

  let newName = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let confirmDeleteId = $state<string | null>(null);

  const sets = $derived($comparisonState.sets);
  const canSave = $derived(
    currentA !== null &&
      currentB !== null &&
      currentA !== currentB &&
      newName.trim().length > 0 &&
      !busy,
  );

  async function save() {
    if (!canSave || currentA === null || currentB === null) return;
    busy = true;
    error = null;
    try {
      await comparisonState.saveSet(
        projectId,
        newName.trim(),
        [currentA, currentB],
        [`A — ${labelA}`, `B — ${labelB}`],
      );
      newName = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function remove(setId: string) {
    busy = true;
    error = null;
    try {
      await comparisonState.removeSet(setId);
      confirmDeleteId = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function formatDate(iso: string): string {
    if (!iso) return "—";
    try {
      return new Date(iso).toLocaleDateString();
    } catch {
      return iso;
    }
  }
</script>

<section class="sets-panel" aria-label="Comparison sets">
  <header class="panel-header">
    <span class="panel-tag font-label">Comparison sets</span>
    <h3 class="panel-title font-display">Saved candidate groups</h3>
  </header>

  <div class="save-row">
    <input
      class="name-input font-body"
      type="text"
      placeholder="Name this A/B pair…"
      bind:value={newName}
      disabled={busy}
      aria-label="Comparison set name"
    />
    <button
      type="button"
      class="cta primary"
      disabled={!canSave}
      onclick={save}
      title={currentA === currentB
        ? "Select two different versions first"
        : "Save the current A/B selection as a set"}
    >
      <span class="material-symbols-outlined" aria-hidden="true">save</span>
      Save current pair
    </button>
  </div>

  {#if error}
    <p class="error font-body" role="alert">{error}</p>
  {/if}

  {#if sets.length === 0}
    <p class="note font-body">
      No saved sets yet. Pick versions A and B above, name the pair,
      and save it for later sessions.
    </p>
  {:else}
    <ul class="set-list">
      {#each sets as set (set.id)}
        <li class="set-row">
          <div class="set-info">
            <span class="set-name font-body">{set.name}</span>
            <span class="set-meta font-body">
              {set.version_ids.length} version{set.version_ids.length === 1
                ? ""
                : "s"} · {formatDate(set.created_at)}
            </span>
            {#if set.slot_labels.length > 0}
              <span class="set-slots font-body">
                {set.slot_labels.join(" · ")}
              </span>
            {/if}
          </div>
          <div class="set-actions">
            <button
              type="button"
              class="cta"
              disabled={busy || set.version_ids.length < 2}
              onclick={() => onApply(set.version_ids[0], set.version_ids[1])}
              title="Load this set's first two versions into A and B"
            >
              <span class="material-symbols-outlined" aria-hidden="true">
                input
              </span>
              Use
            </button>
            {#if confirmDeleteId === set.id}
              <button
                type="button"
                class="cta danger"
                disabled={busy}
                onclick={() => remove(set.id)}
              >
                Confirm delete
              </button>
              <button
                type="button"
                class="cta"
                disabled={busy}
                onclick={() => (confirmDeleteId = null)}
              >
                Cancel
              </button>
            {:else}
              <button
                type="button"
                class="cta danger"
                disabled={busy}
                onclick={() => (confirmDeleteId = set.id)}
                title="Delete this set (versions are not affected)"
              >
                <span class="material-symbols-outlined" aria-hidden="true">
                  delete
                </span>
                Delete
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .sets-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
  }

  .panel-header {
    display: flex;
    align-items: baseline;
    gap: var(--sp-sm);
  }

  .panel-tag {
    font-size: 0.7rem;
    color: var(--primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .panel-title {
    margin: 0;
    font-size: 1rem;
  }

  .save-row {
    display: flex;
    gap: var(--sp-sm);
  }

  .name-input {
    flex: 1;
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface);
    font-size: 0.85rem;
    font-family: inherit;
  }

  .name-input:focus {
    outline: none;
    border-color: var(--primary);
  }

  .cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--outline-variant);
    background: transparent;
    color: var(--on-surface);
    cursor: pointer;
    font-family: inherit;
    font-size: 0.85rem;
    white-space: nowrap;
  }

  .cta:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .cta.primary {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--on-primary);
  }

  .cta.danger {
    color: #e57373;
    border-color: rgba(229, 57, 53, 0.4);
  }

  .cta.danger:hover:not(:disabled) {
    background: rgba(229, 57, 53, 0.12);
  }

  .note {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }

  .error {
    margin: 0;
    color: #e57373;
    font-size: 0.85rem;
  }

  .set-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .set-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-md);
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
  }

  .set-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .set-name {
    color: var(--on-surface);
    font-size: 0.9rem;
  }

  .set-meta,
  .set-slots {
    color: var(--on-surface-variant);
    font-size: 0.75rem;
  }

  .set-actions {
    display: flex;
    gap: var(--sp-xs);
    flex-shrink: 0;
  }
</style>
