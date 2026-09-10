<!--
  RecipesScreen — application-level Recipes destination.

  CR-05 R1: replaces the placeholder route for the "Recipes" nav
  target. Renders the saved pipeline profiles (CR-02 `RecipeStore`)
  as a list of cards with name, target type, and version. Selecting a
  card opens the existing `ProfileManager` modal so users get the
  full CRUD + version history UX from inside the application
  shell rather than only via a hidden menu entry.

  The component is read-only in this slice — it lists profiles and
  opens the editor; the editor's save/create behaviour is unchanged
  from Phase 1.5 PR-C (`src/components/ProfileManager.svelte`).
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    loadProfiles,
    type RecipeSummary,
  } from "../lib/profile-store";
  import ProfileManager from "./ProfileManager.svelte";

  let profiles: RecipeSummary[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let managerOpen = $state(false);
  let hasLoaded = $state(false);

  onMount(async () => {
    try {
      await loadProfiles();
      hasLoaded = true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });

  // profileStore is the writable source; subscribe to keep the list
  // fresh after ProfileManager saves a new version.
  $effect(() => {
    void loadProfiles().then((p) => {
      profiles = p;
    });
  });
</script>

<section class="recipes-screen" aria-label="Recipes">
  <header class="screen-header">
    <div>
      <h1 class="font-display">Recipes</h1>
      <p class="font-body">
        Reusable processing workflows. Open a recipe to view its stages and
        version history; the existing Profile Manager handles edits and
        new-version saves.
      </p>
    </div>
    <button
      type="button"
      class="manage-cta font-display"
      onclick={() => (managerOpen = true)}
    >
      <span class="material-symbols-outlined" aria-hidden="true">
        bookmark_manager
      </span>
      Manage recipes
    </button>
  </header>

  {#if loading}
    <p class="state-line font-body" aria-live="polite">Loading recipes…</p>
  {:else if error}
    <p class="state-line error font-body" role="alert">{error}</p>
  {:else if !hasLoaded}
    <p class="state-line font-body" aria-live="polite">
      Recipe store unavailable in this build.
    </p>
  {:else if profiles.length === 0}
    <div class="empty-card font-body" aria-live="polite">
      <span class="material-symbols-outlined" aria-hidden="true">bookmark</span>
      <h2 class="font-display">No recipes yet</h2>
      <p>
        Recipes are saved from a session's pipeline. Process a project once
        and use the Process controls to save the current pipeline as a
        reusable recipe.
      </p>
    </div>
  {:else}
    <ul class="recipe-grid" aria-label="Saved recipes">
      {#each profiles as summary (summary.profileId)}
        <li>
          <button
            type="button"
            class="recipe-card"
            onclick={() => (managerOpen = true)}
            aria-label="Open {summary.name}"
          >
            <span class="material-symbols-outlined card-icon" aria-hidden="true">
              bookmark
            </span>
            <span class="card-body">
              <span class="card-title font-display">{summary.name}</span>
              <span class="card-meta font-body">
                <span class="badge">v{summary.version}</span>
                <span class="target">{summary.targetType}</span>
              </span>
              {#if summary.description}
                <span class="card-description font-body">
                  {summary.description}
                </span>
              {/if}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if managerOpen}
    <ProfileManager onClose={() => (managerOpen = false)} />
  {/if}
</section>

<style>
  .recipes-screen {
    display: flex;
    flex-direction: column;
    gap: var(--sp-lg);
    padding: var(--sp-xl);
    overflow-y: auto;
    height: 100%;
  }

  .screen-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--sp-lg);
  }

  .screen-header h1 {
    margin: 0 0 var(--sp-xs) 0;
    font-size: 1.5rem;
  }

  .screen-header p {
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 60ch;
  }

  .manage-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.9rem;
    white-space: nowrap;
  }

  .manage-cta:hover {
    filter: brightness(0.92);
  }

  .state-line {
    color: var(--on-surface-variant);
  }

  .state-line.error {
    color: #e53935;
  }

  .empty-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-md);
    padding: var(--sp-xl);
    background: var(--surface-container-low);
    border: 1px dashed var(--outline-variant);
    border-radius: var(--radius-lg);
    text-align: center;
    color: var(--on-surface-variant);
  }

  .empty-card h2 {
    margin: 0;
    color: var(--on-surface);
    font-size: 1.1rem;
  }

  .empty-card p {
    margin: 0;
    max-width: 50ch;
  }

  .empty-card .material-symbols-outlined {
    font-size: 48px;
    color: var(--primary);
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
    align-items: flex-start;
    gap: var(--sp-md);
    width: 100%;
    padding: var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    cursor: pointer;
    text-align: left;
    color: var(--on-surface);
    transition: background 0.12s ease, border-color 0.12s ease;
  }

  .recipe-card:hover {
    background: var(--surface-container-high);
    border-color: var(--primary);
  }

  .card-icon {
    font-size: 28px;
    color: var(--primary);
    flex: 0 0 auto;
  }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
    min-width: 0;
  }

  .card-title {
    font-size: 1rem;
    font-weight: 600;
  }

  .card-meta {
    display: flex;
    gap: var(--sp-sm);
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

  .card-description {
    font-size: 0.85rem;
    color: var(--on-surface-variant);
    line-height: 1.4;
  }
</style>
