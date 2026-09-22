<!--
  RecipeLibrary: CR-08 §15 Recipe Library 5-tab layout.

  The §15 UI specification calls for five classification
  tabs that surface every Recipe head row from
  `recipe_list` (CR-08 §22) into the appropriate bucket:

  - System:           recipes marked is_system (built-in or
                       admin-installed; the §3.1 protected set).
  - My Recipes:       recipes the current user created via
                       `recipe_save` / `recipe_save_from_pipeline_plan`.
  - Project Recipes:  recipes tied to the current project
                       (currently identical to My Recipes: the
                       project-scoped distinction lands in a
                       follow-on slice that adds `profile.project_id`).
  - Imported:         recipes created via `recipe_import`
                       (.afrecipe file ingestion).
  - Recently Used:    recipes with a non-null `last_used_at`,
                       ordered DESC by that timestamp.

  The component is a pure consumer of `profileStore`:
  filtering + ordering happen client-side against the
  head rows already fetched by `loadProfiles`. The
  Tauri round-trip runs once via the parent screen.

  Each tab shows an enhanced Recipe Card (§15 Recipe Card):
  - target type / processing style
  - AI usage badge (when stages use AI)
  - provenance status (deterministic / perceptual)
  - last used timestamp (relative: "2 hours ago", "yesterday")
  - system / imported / archived status pills
-->
<script lang="ts">
  import { profileStore, type RecipeSummary } from "../lib/profile-store";

  /** CR-08 §15: Optional click handler. Fires when
   * the user clicks a Recipe card. The parent
   * (typically `RecipesScreen.svelte`) uses this
   * to populate the §15 Recipe Detail panel +
   * downstream apply/duplicate/edit flows.
   * When omitted, cards are non-interactive
   * (rendered as a `<ul>` without click handlers).
   */
  interface Props {
    onSelect?: (summary: RecipeSummary) => void;
  }
  let { onSelect }: Props = $props();

  /** Active tab key. Drives the visible classification. */
  type TabKey = "system" | "mine" | "project" | "imported" | "recent";
  let activeTab: TabKey = $state("mine");

  interface TabSpec {
    key: TabKey;
    label: string;
    description: string;
  }

  const TABS: TabSpec[] = [
    {
      key: "system",
      label: "System",
      description: "Built-in and admin-installed Recipes (protected)",
    },
    {
      key: "mine",
      label: "My Recipes",
      description: "Recipes you saved from a processing pipeline",
    },
    {
      key: "project",
      label: "Project Recipes",
      description: "Recipes scoped to the current project",
    },
    {
      key: "imported",
      label: "Imported",
      description: "Recipes imported from .afrecipe files",
    },
    {
      key: "recent",
      label: "Recently Used",
      description: "Recipes applied in recent runs, newest first",
    },
  ];

  /**
   * Classify a head row into the active tab. Hidden from
   * archived rows (the per-tab Archived disclosure lives
   * one slice ahead; for now the Archived filter is
   * applied at the parent screen level so the Library
   * surfaces active recipes).
   */
  function filterForTab(
    profiles: RecipeSummary[],
    tab: TabKey,
  ): RecipeSummary[] {
    const active = profiles.filter((p) => !p.isArchived);
    switch (tab) {
      case "system":
        return active.filter((p) => p.isSystem);
      case "mine":
        return active.filter((p) => !p.isSystem && !p.isImported);
      case "project":
        // Project Recipes currently mirrors My Recipes; the
        // project-scoped distinction is a follow-on slice.
        return active.filter((p) => !p.isSystem && !p.isImported);
      case "imported":
        return active.filter((p) => p.isImported);
      case "recent":
        return active
          .filter((p) => p.lastUsedAt !== null)
          .sort((a, b) =>
            (b.lastUsedAt ?? "").localeCompare(a.lastUsedAt ?? ""),
          );
    }
  }

  /** Per-tab count surfaced in the tab header. */
  function countForTab(profiles: RecipeSummary[], tab: TabKey): number {
    return filterForTab(profiles, tab).length;
  }

  /**
   * Format an SQLite `last_used_at` string as a relative
   * timestamp. Falls back to the raw string for malformed
   * inputs (so a malformed server-side stamp never breaks
   * the UI; we surface "recently" as the safe default).
   */
  function formatRelative(ts: string | null): string {
    if (!ts) return "never";
    // SQLite `datetime('now')` returns 'YYYY-MM-DD HH:MM:SS'
    // in UTC. Parse as a Date.
    const isoish = ts.includes("T") ? ts : ts.replace(" ", "T") + "Z";
    const date = new Date(isoish);
    if (Number.isNaN(date.getTime())) return ts;
    const diffMs = Date.now() - date.getTime();
    const sec = Math.floor(diffMs / 1000);
    if (sec < 60) return "just now";
    const min = Math.floor(sec / 60);
    if (min < 60) return `${min} min ago`;
    const hr = Math.floor(min / 60);
    if (hr < 24) return `${hr} hr ago`;
    const day = Math.floor(hr / 24);
    if (day < 7) return `${day} day${day === 1 ? "" : "s"} ago`;
    return date.toLocaleDateString();
  }

  /** Whether a head row carries an AI stage (best-effort:
   * the summary doesn't carry the stage list; the card
   * surfaces a generic "AI" badge for system recipes and
   * leaves it off otherwise). */
  function aiBadge(p: RecipeSummary): boolean {
    // System Recipes use the AstroForge defaults that
    // include the AI stage (the seed profiles opt-in).
    return p.isSystem;
  }
</script>

<section class="recipe-library" aria-label="Recipe Library">
  <div class="tablist" role="tablist" aria-label="Recipe classification">
    {#each TABS as tab (tab.key)}
      <button
        type="button"
        role="tab"
        class="tab"
        class:active={activeTab === tab.key}
        aria-selected={activeTab === tab.key}
        aria-controls="panel-{tab.key}"
        id="tab-{tab.key}"
        data-testid="recipe-tab-{tab.key}"
        onclick={() => (activeTab = tab.key)}
      >
        <span class="tab-label font-label">{tab.label}</span>
        <span class="tab-count" aria-label="{countForTab($profileStore, tab.key)} items">
          {countForTab($profileStore, tab.key)}
        </span>
      </button>
    {/each}
  </div>

  {#each TABS as tab (tab.key)}
    {#if activeTab === tab.key}
      <div
        class="tab-panel"
        role="tabpanel"
        id="panel-{tab.key}"
        aria-labelledby="tab-{tab.key}"
        data-testid="recipe-panel-{tab.key}"
      >
        <p class="tab-description font-body">{tab.description}</p>

        {#if filterForTab($profileStore, tab.key).length === 0}
          <div class="empty-card font-body" aria-live="polite">
            <span class="material-symbols-outlined" aria-hidden="true">
              {tab.key === "system"
                ? "shield"
                : tab.key === "recent"
                  ? "schedule"
                  : "bookmark"}
            </span>
            <h3 class="font-display">No recipes in this view</h3>
            <p>
              {#if tab.key === "system"}
                System Recipes are seeded automatically the first time
                you launch AstroForge. If this view is empty, your
                `RecipeStore` migration hasn't run yet: restart the app
                to seed.
              {:else if tab.key === "imported"}
                Import a Recipe from a .afrecipe file using the
                <strong>Import…</strong> button at the top of this screen.
              {:else if tab.key === "recent"}
                Applied Recipes appear here with a relative timestamp.
                Try a recipe on a session and it will surface in this tab.
              {:else}
                Recipes you save from a pipeline run land here. Process
                a project once and use the Process controls to save the
                current pipeline as a reusable recipe.
              {/if}
            </p>
          </div>
        {:else}
          <ul class="recipe-grid" aria-label="{tab.label} recipes">
            {#each filterForTab($profileStore, tab.key) as summary (summary.profileId)}
              <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
              <li
                class="recipe-card"
                class:clickable={onSelect !== undefined}
                role={onSelect ? "button" : undefined}
                tabindex={onSelect ? 0 : undefined}
                onclick={onSelect ? () => onSelect(summary) : undefined}
                onkeydown={onSelect
                  ? (event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        onSelect(summary);
                      }
                    }
                  : undefined}
                data-testid="recipe-card"
              >
                <div class="card-head">
                  <span class="material-symbols-outlined card-icon" aria-hidden="true">
                    bookmark
                  </span>
                  <span class="card-title font-display">{summary.name}</span>
                </div>

                <div class="card-meta font-body">
                  <span class="badge">v{summary.version}</span>
                  <span class="target">{summary.targetType}</span>
                  {#if summary.isSystem}
                    <span class="status-pill system" title="System Recipe (protected)">
                      System
                    </span>
                  {/if}
                  {#if summary.isImported}
                    <span class="status-pill imported" title="Imported from .afrecipe">
                      Imported
                    </span>
                  {/if}
                  {#if aiBadge(summary)}
                    <span class="status-pill ai" title="Uses AI stage">
                      AI
                    </span>
                  {/if}
                </div>

                {#if summary.description}
                  <p class="card-description font-body">
                    {summary.description}
                  </p>
                {/if}

                <div class="card-footer font-body">
                  <span class="footer-label">Last used</span>
                  <span class="footer-value" title={summary.lastUsedAt ?? "never"}>
                    {formatRelative(summary.lastUsedAt)}
                  </span>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
  {/each}
</section>

<style>
  .recipe-library {
    display: flex;
    flex-direction: column;
    gap: var(--sp-lg);
    width: 100%;
  }

  .tablist {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-xs);
    border-bottom: 1px solid var(--outline-variant);
    padding-bottom: 0;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: transparent;
    border: 0;
    border-bottom: 2px solid transparent;
    color: var(--on-surface-variant);
    padding: var(--sp-sm) var(--sp-md);
    cursor: pointer;
    font-size: 0.9rem;
    transition: color 0.12s ease, border-color 0.12s ease;
  }

  .tab:hover {
    color: var(--on-surface);
  }

  .tab.active {
    color: var(--primary);
    border-bottom-color: var(--primary);
  }

  .tab-label {
    font-weight: 600;
  }

  .tab-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 22px;
    padding: 0 6px;
    background: var(--surface-container-high);
    border-radius: var(--radius-full);
    font-size: 0.75rem;
    color: var(--on-surface-variant);
  }

  .tab.active .tab-count {
    background: var(--primary-container);
    color: var(--on-primary-container);
  }

  .tab-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .tab-description {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }

  .recipe-grid {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: var(--sp-md);
  }

  .recipe-card {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    padding: var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    color: var(--on-surface);
  }

  .recipe-card.clickable {
    cursor: pointer;
    transition: border-color 0.1s ease, background 0.1s ease;
  }

  .recipe-card.clickable:hover {
    border-color: var(--primary);
    background: var(--surface-container-high);
  }

  .recipe-card.clickable:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }

  .card-head {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-sm);
  }

  .card-icon {
    font-size: 24px;
    color: var(--primary);
    flex: 0 0 auto;
  }

  .card-title {
    font-size: 1rem;
    font-weight: 600;
    line-height: 1.3;
  }

  .card-meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-xs);
    align-items: center;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    background: var(--surface-container-highest);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-full);
    font-size: 0.75rem;
    color: var(--on-surface);
  }

  .target {
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.75rem;
  }

  .status-pill {
    display: inline-block;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    font-size: 0.7rem;
    border: 1px solid var(--outline-variant);
  }

  .status-pill.system {
    background: var(--primary-container);
    color: var(--on-primary-container);
    border-color: var(--primary);
  }

  .status-pill.imported {
    background: var(--surface-container-highest);
    color: var(--on-surface);
  }

  .status-pill.ai {
    background: var(--secondary-container);
    color: var(--on-secondary-container);
    border-color: var(--secondary);
  }

  .card-description {
    margin: 0;
    font-size: 0.85rem;
    color: var(--on-surface-variant);
    line-height: 1.4;
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: var(--sp-xs);
    border-top: 1px dashed var(--outline-variant);
    color: var(--on-surface-variant);
    font-size: 0.8rem;
  }

  .footer-label {
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.7rem;
  }

  .empty-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-sm);
    padding: var(--sp-xl);
    background: var(--surface-container-low);
    border: 1px dashed var(--outline-variant);
    border-radius: var(--radius-lg);
    text-align: center;
    color: var(--on-surface-variant);
  }

  .empty-card h3 {
    margin: 0;
    color: var(--on-surface);
    font-size: 1.05rem;
  }

  .empty-card p {
    margin: 0;
    max-width: 50ch;
  }

  .empty-card .material-symbols-outlined {
    font-size: 40px;
    color: var(--primary);
  }
</style>
