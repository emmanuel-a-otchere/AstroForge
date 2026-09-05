<!--
  ProfileManager — modal for managing pipeline profiles.

  Phase 1.5 PR-C: complete CRUD + version history UI for the pipeline
  profile feature (docs/PROFILE_PIPELINES_PLAN.md).

  Layout:
    - Left rail: list of all profiles (name + version + targetType)
    - Right pane: selected profile detail (description + stages table)
                  + version history timeline
    - Footer: "Save current session as new version" action

  Scope notes:
    - Linear history only (D-1 = 2); no branches
    - Soft migration handled in Rust (D-3); this UI assumes v2 always
    - Read-only for stages (no in-place edit); users save a new version
-->
<script lang="ts">
  import {
    profileStore,
    loadProfiles,
    getProfile,
    listProfileVersions,
    saveProfile,
    type RecipeSummary,
    type Recipe,
    type RecipeVersion,
  } from "../lib/profile-store";
  import { sessionStore } from "../lib/pipeline-store";

  export let onClose: () => void = () => {};

  let selectedProfileId: string | null = null;
  let selectedVersion: number = 1; // latest by default
  let selectedRecipe: Recipe | null = null;
  let versionHistory: RecipeVersion[] = [];
  let saving = false;
  let saveError: string | null = null;
  let lastSavedVersion: number | null = null;

  $: if ($profileStore.length > 0 && selectedProfileId === null) {
    selectedProfileId = $profileStore[0].profileId;
    selectedVersion = $profileStore[0].version;
  }

  $: if (selectedProfileId !== null) {
    void loadSelected();
  }

  async function loadSelected() {
    if (selectedProfileId === null) return;
    try {
      const [recipe, versions] = await Promise.all([
        getProfile(selectedProfileId, selectedVersion),
        listProfileVersions(selectedProfileId),
      ]);
      selectedRecipe = recipe;
      versionHistory = versions;
    } catch (e) {
      saveError = (e as Error).message;
    }
  }

  async function handleRefresh() {
    await loadProfiles();
    if (selectedProfileId) {
      await loadSelected();
    }
  }

  function handleSelectProfile(summary: RecipeSummary) {
    selectedProfileId = summary.profileId;
    selectedVersion = summary.version;
  }

  function handleSelectVersion(v: number) {
    selectedVersion = v;
  }

  async function handleSaveAsNewVersion() {
    if (!selectedRecipe) return;
    saving = true;
    saveError = null;
    try {
      const graph = $sessionStore.pipelineGraph;
      // Build a Recipe from the live session pipeline + the selected
      // profile's metadata. Each PipelineNode becomes a RecipeStage.
      // Note: Recipe uses TS-shape (stageId), saveProfile converts to
      // snake_case for the Rust boundary.
      const stages = graph.nodes.map((node) => ({
        stageId: node.type,
        enabled: node.status !== "skipped",
        params: { ...(node.params as Record<string, unknown>) },
      }));
      const next: Recipe = {
        ...selectedRecipe,
        schemaVersion: "2.0",
        stages,
        version: 0, // Rust side computes next
        parentVersion: selectedRecipe.version,
      };
      const summary = await saveProfile(next);
      lastSavedVersion = summary.version;
      await handleRefresh();
      // Auto-select the new version in the timeline.
      selectedVersion = summary.version;
    } catch (e) {
      saveError = (e as Error).message;
    } finally {
      saving = false;
    }
  }

  function formatDate(iso: string): string {
    if (!iso) return "—";
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }

  function formatParam(v: unknown): string {
    if (v === null || v === undefined) return "—";
    if (typeof v === "number") {
      // Drop trailing zeros for cleaner display.
      return Number.isInteger(v) ? String(v) : v.toFixed(3).replace(/\.?0+$/, "");
    }
    if (typeof v === "string") return v;
    if (typeof v === "boolean") return v ? "true" : "false";
    if (Array.isArray(v)) return JSON.stringify(v);
    return JSON.stringify(v);
  }
</script>

<div
  class="modal-overlay"
  role="dialog"
  aria-modal="true"
  aria-labelledby="profile-manager-title"
  tabindex="-1"
  on:click|self={onClose}
  on:keydown={(e) => e.key === "Escape" && onClose()}
>
  <div class="modal" role="document">
    <header class="modal-header">
      <div>
        <div class="kicker">Pipeline Profiles</div>
        <h2 id="profile-manager-title">Manage saved processing pipelines</h2>
      </div>
      <button class="btn-close" on:click={onClose} aria-label="Close profile manager">
        <span class="material-symbols-outlined">close</span>
      </button>
    </header>

    <div class="modal-body">
      <aside class="profile-list" aria-label="Saved profiles">
        <div class="list-header">
          <span class="list-title">Profiles</span>
          <button
            class="btn-refresh"
            on:click={handleRefresh}
            title="Reload profiles"
            aria-label="Reload profiles"
          >
            <span class="material-symbols-outlined">refresh</span>
          </button>
        </div>
        {#if $profileStore.length === 0}
          <p class="empty">No profiles saved yet.</p>
        {:else}
          <ul>
            {#each $profileStore as summary (summary.profileId)}
              <li>
                <button
                  class="profile-item"
                  class:selected={selectedProfileId === summary.profileId}
                  on:click={() => handleSelectProfile(summary)}
                >
                  <div class="profile-name">{summary.name}</div>
                  <div class="profile-meta">
                    <span class="badge">v{summary.version}</span>
                    <span class="target">{summary.targetType}</span>
                  </div>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </aside>

      <section class="profile-detail" aria-label="Profile detail">
        {#if selectedRecipe}
          <div class="detail-header">
            <h3>{selectedRecipe.name}</h3>
            <span class="badge">v{selectedRecipe.version}</span>
            <span class="badge branch-badge">{selectedRecipe.branch}</span>
            <span class="schema">schema {selectedRecipe.schemaVersion}</span>
          </div>

          {#if selectedRecipe.description}
            <p class="description">{selectedRecipe.description}</p>
          {/if}

          <div class="versions">
            <div class="section-label">Version history</div>
            {#if versionHistory.length === 0}
              <p class="empty">No versions recorded.</p>
            {:else}
              <ol class="version-list">
                {#each versionHistory as v (v.version)}
                  <li>
                    <button
                      class="version-item"
                      class:selected={selectedVersion === v.version}
                      on:click={() => handleSelectVersion(v.version)}
                    >
                      <span class="version-num">v{v.version}</span>
                      {#if v.parentVersion !== null}
                        <span class="version-parent">← v{v.parentVersion}</span>
                      {:else}
                        <span class="version-parent">initial</span>
                      {/if}
                      <span class="version-date">{formatDate(v.createdAt)}</span>
                    </button>
                  </li>
                {/each}
              </ol>
            {/if}
          </div>

          <div class="stages">
            <div class="section-label">
              Stages ({selectedRecipe.stages.length})
            </div>
            <table class="stages-table">
              <thead>
                <tr>
                  <th>Stage</th>
                  <th>Enabled</th>
                  <th>Params</th>
                </tr>
              </thead>
              <tbody>
                {#each selectedRecipe.stages as stage (stage.stageId)}
                  <tr>
                    <td class="stage-id">{stage.stageId}</td>
                    <td class="stage-enabled">
                      {stage.enabled ? "✓" : "—"}
                    </td>
                    <td class="stage-params">
                      {#if Object.keys(stage.params).length === 0}
                        <span class="muted">(none)</span>
                      {:else}
                        <dl>
                          {#each Object.entries(stage.params) as [k, v] (k)}
                            <div class="param-row">
                              <dt>{k}</dt>
                              <dd>{formatParam(v)}</dd>
                            </div>
                          {/each}
                        </dl>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          {#if saveError}
            <div class="save-error">Save failed: {saveError}</div>
          {/if}
          {#if lastSavedVersion !== null}
            <div class="save-success">
              Saved as v{lastSavedVersion}.
              <button
                class="btn-link"
                on:click={() => (lastSavedVersion = null)}
                aria-label="Dismiss save confirmation"
              >Dismiss</button>
            </div>
          {/if}
        {:else}
          <p class="empty">Select a profile to inspect.</p>
        {/if}
      </section>
    </div>

    <footer class="modal-footer">
      <button class="btn-secondary" on:click={onClose}>Close</button>
      <button
        class="btn-primary"
        disabled={!selectedRecipe || saving}
        on:click={handleSaveAsNewVersion}
      >
        {#if saving}
          Saving…
        {:else}
          Save current session as new version
        {/if}
      </button>
    </footer>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-scrim);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
    backdrop-filter: blur(4px);
  }

  .modal {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    width: 900px;
    max-width: 95vw;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border);
  }

  .kicker {
    font-family: var(--font-data);
    font-size: 0.6875rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--accent);
    margin-bottom: 0.25rem;
  }

  .modal-header h2 {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .btn-close {
    background: none;
    border: 1px solid transparent;
    color: var(--text-secondary);
    border-radius: 0.375rem;
    padding: 0.375rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .btn-close:hover {
    color: var(--text-primary);
    border-color: var(--border);
  }

  .modal-body {
    display: grid;
    grid-template-columns: 280px 1fr;
    flex: 1;
    min-height: 0;
  }

  .profile-list {
    border-right: 1px solid var(--border);
    overflow-y: auto;
    padding: 0.75rem;
    background: var(--bg-tertiary);
  }

  .list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.25rem 0.5rem 0.5rem;
  }

  .list-title {
    font-family: var(--font-data);
    font-size: 0.6875rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
  }

  .btn-refresh {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: 0.375rem;
    padding: 0.25rem 0.5rem;
    cursor: pointer;
    display: flex;
    align-items: center;
  }
  .btn-refresh:hover {
    color: var(--text-primary);
    border-color: var(--accent);
  }

  .profile-list ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .profile-item {
    width: 100%;
    text-align: left;
    background: none;
    border: 1px solid transparent;
    padding: 0.625rem 0.75rem;
    border-radius: 0.5rem;
    cursor: pointer;
    color: var(--text-primary);
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .profile-item:hover {
    background: var(--bg-secondary);
    border-color: var(--border);
  }

  .profile-item.selected {
    background: var(--bg-secondary);
    border-color: var(--accent);
  }

  .profile-name {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .profile-meta {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .badge {
    font-family: var(--font-data);
    font-size: 0.6875rem;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    background: var(--surface-container-high);
    color: var(--accent);
    font-weight: 700;
  }

  .branch-badge {
    background: var(--bg-primary);
    color: var(--text-secondary);
  }

  .profile-detail {
    overflow-y: auto;
    padding: 1.25rem 1.5rem;
  }

  .detail-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .detail-header h3 {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .schema {
    margin-left: auto;
    font-family: var(--font-data);
    font-size: 0.6875rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .description {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin: 0 0 1.25rem;
    line-height: 1.5;
  }

  .section-label {
    font-family: var(--font-data);
    font-size: 0.6875rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .versions {
    margin-bottom: 1.5rem;
  }

  .version-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .version-item {
    width: 100%;
    text-align: left;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    padding: 0.5rem 0.75rem;
    border-radius: 0.375rem;
    cursor: pointer;
    color: var(--text-primary);
    display: flex;
    gap: 0.75rem;
    align-items: center;
    font-size: 0.8125rem;
  }

  .version-item.selected {
    border-color: var(--accent);
    background: var(--bg-secondary);
  }

  .version-num {
    font-family: var(--font-data);
    font-weight: 700;
    color: var(--accent);
  }

  .version-parent {
    font-family: var(--font-data);
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .version-date {
    margin-left: auto;
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .stages-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8125rem;
  }

  .stages-table th {
    text-align: left;
    padding: 0.5rem 0.75rem;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    font-family: var(--font-data);
    font-size: 0.6875rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-bottom: 1px solid var(--border);
  }

  .stages-table td {
    padding: 0.625rem 0.75rem;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }

  .stage-id {
    font-family: var(--font-data);
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
  }

  .stage-enabled {
    text-align: center;
    color: var(--success);
    font-weight: 700;
  }

  .stage-params dl {
    margin: 0;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.125rem 0.75rem;
  }

  .param-row {
    display: contents;
  }

  .param-row dt {
    font-family: var(--font-data);
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .param-row dd {
    margin: 0;
    color: var(--text-primary);
    font-size: 0.8125rem;
  }

  .muted {
    color: var(--text-muted);
  }

  .empty {
    color: var(--text-muted);
    font-size: 0.875rem;
    font-style: italic;
  }

  .save-error {
    margin-top: 1rem;
    padding: 0.75rem;
    border-radius: 0.375rem;
    background: rgba(255, 80, 80, 0.1);
    border: 1px solid var(--error, #ff5050);
    color: var(--error, #ff5050);
    font-size: 0.8125rem;
  }

  .save-success {
    margin-top: 1rem;
    padding: 0.75rem;
    border-radius: 0.375rem;
    background: rgba(80, 200, 120, 0.1);
    border: 1px solid var(--success, #50c878);
    color: var(--success, #50c878);
    font-size: 0.8125rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    border-top: 1px solid var(--border);
    background: var(--bg-tertiary);
  }

  .btn-primary,
  .btn-secondary {
    padding: 0.5rem 1.25rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn-primary {
    background: var(--accent);
    color: var(--bg-primary);
  }
  .btn-primary:hover:not(:disabled) {
    background: var(--accent-dim);
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }
  .btn-secondary:hover {
    color: var(--text-primary);
  }

  .btn-link {
    background: none;
    border: none;
    color: inherit;
    text-decoration: underline;
    cursor: pointer;
    font-size: 0.8125rem;
    margin-left: auto;
  }
</style>
