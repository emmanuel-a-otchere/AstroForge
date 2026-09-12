<!--
  RecipeGallery — P4-M2-T2 + T3 — browsable, filterable recipe feed.

  Spec reference: §11.3 "Sharing Mechanisms" — "In-app Recipe Gallery:
  browsable, filterable by target/equipment/palette."

  Distinct from the local "Recipes" screen (which shows the user's
  own profiles from RecipeStore). The gallery is a feed of shared
  recipes from a `RecipeSource` (Rust trait). Hosting is deferred to
  issue #119 (P4-M2-T1) — the UI works against an in-memory
  `galleryStore` populated by `loadGallery()`, which gracefully
  falls back to a fixture set when the Tauri IPC is unavailable
  (i.e. plain Vite dev on the LAN).

  Filter axes:
    * query       — name + author + target_type substring match
    * target      — exact target_type (e.g. "deep_sky", "planetary")
    * equipment   — exact camera (e.g. "Seestar S50")
    * palette     — Ha / OIII / SII / SHO / HOO / LRGB / Broadband
    * sort        — Relevance / DateDesc / DateAsc / Popularity
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    loadGallery,
    searchLocal,
    galleryStore,
    type SharedRecipe,
    type FilterCriteria,
    type FilterPalette,
    type SortOrder,
  } from "../lib/recipeFeed";

  let query = $state("");
  let target = $state("");
  let equipment = $state("");
  let palette: FilterPalette | "" = $state("");
  let sort: SortOrder = $state("Relevance");

  let loading = $state(true);
  let error: string | null = $state(null);
  let hasLoaded = $state(false);
  let allRecipes: SharedRecipe[] = [];

  // Performance: keep the search inline; the spec target is <500ms
  // and a 7-recipe fixture is sub-millisecond. For a real feed of
  // thousands of recipes this stays linear and stays sub-50ms.
  let visibleRecipes: SharedRecipe[] = $derived.by(() => {
    if (!hasLoaded) return [];
    const criteria: FilterCriteria = {
      query: query.trim() || undefined,
      target: target.trim() || undefined,
      equipment: equipment.trim() || undefined,
      palette: palette || undefined,
      sort,
    };
    return searchLocal(allRecipes, criteria);
  });

  onMount(async () => {
    try {
      allRecipes = await loadGallery();
      hasLoaded = true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });

  function clearFilters() {
    query = "";
    target = "";
    equipment = "";
    palette = "";
    sort = "Relevance";
  }

  const paletteOptions: FilterPalette[] = [
    "Ha",
    "OIII",
    "SII",
    "SHO",
    "HOO",
    "LRGB",
    "Broadband",
  ];
</script>

<section class="recipe-gallery" aria-label="Recipe gallery">
  <header class="screen-header">
    <div>
      <h1 class="font-display">Recipe gallery</h1>
      <p class="font-body">
        Browsable feed of shared recipes. Filter by target, equipment,
        palette, or search by name and author.
      </p>
    </div>
  </header>

  <div class="filter-bar" role="search">
    <label class="filter-field">
      <span class="font-body">Search</span>
      <input
        type="search"
        bind:value={query}
        placeholder="name, author, target…"
        aria-label="Search recipes"
      />
    </label>

    <label class="filter-field">
      <span class="font-body">Target</span>
      <input
        type="text"
        bind:value={target}
        placeholder="e.g. deep_sky"
        aria-label="Filter by target"
      />
    </label>

    <label class="filter-field">
      <span class="font-body">Equipment</span>
      <input
        type="text"
        bind:value={equipment}
        placeholder="e.g. Seestar S50"
        aria-label="Filter by equipment"
      />
    </label>

    <label class="filter-field">
      <span class="font-body">Palette</span>
      <select bind:value={palette} aria-label="Filter by palette">
        <option value="">Any</option>
        {#each paletteOptions as p (p)}
          <option value={p}>{p}</option>
        {/each}
      </select>
    </label>

    <label class="filter-field">
      <span class="font-body">Sort</span>
      <select bind:value={sort} aria-label="Sort order">
        <option value="Relevance">Relevance</option>
        <option value="DateDesc">Newest first</option>
        <option value="DateAsc">Oldest first</option>
        <option value="Popularity">Name A–Z</option>
      </select>
    </label>

    <button
      type="button"
      class="clear-cta font-body"
      onclick={clearFilters}
      aria-label="Clear all filters"
    >
      Clear
    </button>
  </div>

  {#if loading}
    <p class="state-line font-body" aria-live="polite">Loading gallery…</p>
  {:else if error}
    <p class="state-line error font-body" role="alert">{error}</p>
  {:else if !hasLoaded}
    <p class="state-line font-body" aria-live="polite">
      Gallery unavailable in this build.
    </p>
  {:else if allRecipes.length === 0}
    <div class="empty-card font-body" aria-live="polite">
      <span class="material-symbols-outlined" aria-hidden="true">bookmark</span>
      <h2 class="font-display">No recipes in this feed yet</h2>
      <p>
        The recipe feed is empty. Once the gallery host (issue #119)
        is wired, recipes authored by other users will appear here.
      </p>
    </div>
  {:else if visibleRecipes.length === 0}
    <p class="state-line font-body" aria-live="polite">
      No recipes match the current filters.
    </p>
  {:else}
    <p class="result-count font-body" aria-live="polite">
      Showing {visibleRecipes.length} of {allRecipes.length} recipes.
    </p>
    <ul class="gallery-grid" aria-label="Filtered recipes">
      {#each visibleRecipes as recipe (recipe.name + ":" + recipe.author)}
        <li>
          <article class="recipe-card">
            <header>
              <h2 class="card-title font-display">{recipe.name}</h2>
              <p class="card-meta font-body">
                <span class="badge">v{recipe.recipeVersion}</span>
                <span class="target">{recipe.targetType}</span>
              </p>
            </header>
            {#if recipe.author}
              <p class="card-author font-body">by {recipe.author}</p>
            {/if}
            <dl class="card-details font-body">
              {#if recipe.equipmentHints.camera}
                <div>
                  <dt>Camera</dt>
                  <dd>{recipe.equipmentHints.camera}</dd>
                </div>
              {/if}
              {#if recipe.equipmentHints.filters.length > 0}
                <div>
                  <dt>Filters</dt>
                  <dd>{recipe.equipmentHints.filters.join(", ")}</dd>
                </div>
              {/if}
              <div>
                <dt>Stages</dt>
                <dd>{recipe.pipeline.length}</dd>
              </div>
            </dl>
          </article>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .recipe-gallery {
    display: flex;
    flex-direction: column;
    gap: var(--sp-lg);
    padding: var(--sp-xl);
    overflow-y: auto;
    height: 100%;
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

  .filter-bar {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: var(--sp-md);
    align-items: end;
    padding: var(--sp-md);
    background: var(--surface-container);
    border-radius: var(--radius-md);
  }

  .filter-field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .filter-field span {
    font-size: 0.85rem;
    color: var(--on-surface-variant);
  }

  .filter-field input,
  .filter-field select {
    padding: var(--sp-sm);
    background: var(--surface);
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    font: inherit;
  }

  .clear-cta {
    padding: var(--sp-sm) var(--sp-md);
    background: transparent;
    color: var(--primary);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .state-line {
    margin: 0;
    padding: var(--sp-md);
    color: var(--on-surface-variant);
  }

  .state-line.error {
    color: var(--error, #b00020);
  }

  .empty-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-md);
    padding: var(--sp-xl);
    background: var(--surface-container);
    border-radius: var(--radius-md);
    text-align: center;
  }

  .empty-card h2 {
    margin: 0;
    font-size: 1.2rem;
  }

  .empty-card p {
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 50ch;
  }

  .result-count {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.9rem;
  }

  .gallery-grid {
    list-style: none;
    margin: 0;
    padding: 0;
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
    border-radius: var(--radius-md);
    border: 1px solid var(--outline-variant);
  }

  .card-title {
    margin: 0;
    font-size: 1.05rem;
  }

  .card-meta {
    display: flex;
    gap: var(--sp-sm);
    margin: var(--sp-xs) 0 0 0;
  }

  .badge {
    padding: 2px 8px;
    background: var(--primary-container);
    color: var(--on-primary-container);
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
  }

  .target {
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }

  .card-author {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }

  .card-details {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: var(--sp-sm);
    margin: 0;
    padding-top: var(--sp-sm);
    border-top: 1px solid var(--outline-variant);
  }

  .card-details div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .card-details dt {
    font-size: 0.75rem;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .card-details dd {
    margin: 0;
    font-size: 0.9rem;
  }
</style>