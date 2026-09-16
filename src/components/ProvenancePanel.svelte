<!--
  CR-07 B14 — Provenance Panel (§13 / §14 of
  docs/CR-07-IMAGE-REVIEW-COMPARISON-DECISION.md).

  Renders the B13c live `Recipe` (or honest "not
  recorded" state) for one Image Version. Anchors the
  §14 acceptance criterion: "the data is in
  `Recipe` + `RecipeStage` + `IntegrityBadge` +
  `ModelUsage`, but no `ProvenancePanel` component
  exists."

  Five render states:
    1. status === 'loading' -> spinner + "Loading
       provenance".
    2. status === 'error' -> error notice + retry.
    3. status === 'loaded' && recipe === null ->
       "Profile not recorded" empty state (the
       version's `recipe_id` was null in
       `image_versions`; B13a column is null, B13b
       apply round never populated it).
    4. status === 'loaded' && recipe !== null ->
       the full panel: identity, integrity badges,
       model list, stage count + first three names.
    5. status === 'idle' -> "Select a version" or
       simply nothing (default; happens before the
       first $effect fires).

  The "stages" section is a preview only: B15 ships
  the full vertical timeline. The "models" list
  mirrors Recipe.integrity.models.
-->
<script lang="ts">
  import { provenanceStore, type ProvenanceRecipe } from "../state/provenance-store";

  interface Props {
    versionId: string;
    versionLabel: string;
  }
  const { versionId, versionLabel }: Props = $props();

  // CR-07 B14: when `versionId` changes (or first
  // arrives), kick the store to fetch. The store
  // handles the in-flight stale-load guard itself,
  // so two ProvenancePanels pointing at different
  // versions don't fight each other.
  $effect(() => {
    void provenanceStore.load(versionId);
  });

  // Local view into the store. Svelte 5 runes: a
  // derived read via $provenanceStore.x keeps the
  // template reactive without subscribing to the
  // full store object.
  const recipe = $derived<ProvenanceRecipe | null>(
    $provenanceStore.versionId === versionId
      ? $provenanceStore.recipe
      : null,
  );
  const status = $derived(
    $provenanceStore.versionId === versionId
      ? $provenanceStore.status
      : "idle" as const,
  );
  const error = $derived<string | null>(
    $provenanceStore.versionId === versionId
      ? $provenanceStore.error
      : null,
  );

  function formatDate(iso: string): string {
    if (!iso) return "—";
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }

  function modelTypeLabel(t: "deterministic" | "perceptual"): string {
    return t === "perceptual" ? "Perceptual" : "Deterministic";
  }
</script>

<section class="provenance-panel" aria-label={`Provenance for ${versionLabel}`}>
  <header class="panel-header">
    <span class="panel-tag font-label">Provenance</span>
    <h3 class="panel-title font-display">{versionLabel}</h3>
  </header>

  {#if status === "loading"}
    <p class="status-note font-body" aria-live="polite">
      <span class="material-symbols-outlined" aria-hidden="true">progress_activity</span>
      Loading provenance…
    </p>
  {:else if status === "error"}
    <div class="error" role="alert">
      <p class="error-text font-body">
        <span class="material-symbols-outlined" aria-hidden="true">error</span>
        {error ?? "Unknown error"}
      </p>
      <button
        type="button"
        class="retry-cta"
        onclick={() => provenanceStore.load(versionId)}
        aria-label="Retry loading provenance"
      >
        Retry
      </button>
    </div>
  {:else if status === "loaded" && recipe === null}
    <div class="empty">
      <p class="empty-title font-body">
        <span class="material-symbols-outlined" aria-hidden="true">info</span>
        Profile not recorded
      </p>
      <p class="empty-body font-body">
        This image version was not produced from a named
        Recipe profile. The apply round that wrote it
        did not carry a <code>recipe_id</code> (B13a
        added the column; B13b wired the producer).
        Legacy rows + AI-applied versions all show
        this state.
      </p>
    </div>
  {:else if status === "loaded" && recipe !== null}
    <div class="recipe">
      <div class="recipe-identity">
        <h4 class="recipe-name font-display">{recipe.name}</h4>
        {#if recipe.description}
          <p class="recipe-description font-body">{recipe.description}</p>
        {/if}
        <dl class="recipe-meta font-body">
          <div class="meta-row">
            <dt>Target</dt>
            <dd>{recipe.targetType || "—"}</dd>
          </div>
          <div class="meta-row">
            <dt>Version</dt>
            <dd>v{recipe.version}{recipe.parentVersion !== null ? ` (← v${recipe.parentVersion})` : ""}</dd>
          </div>
          <div class="meta-row">
            <dt>Branch</dt>
            <dd>{recipe.branch || "—"}</dd>
          </div>
          <div class="meta-row">
            <dt>Created</dt>
            <dd>{formatDate(recipe.createdAt)}</dd>
          </div>
        </dl>
      </div>

      <div class="recipe-integrity">
        <h5 class="section-title font-label">Integrity</h5>
        <div class="integrity-badges">
          <span
            class="integrity-badge"
            data-active={recipe.integrity.perceptualModelsUsed}
          >
            <span class="material-symbols-outlined" aria-hidden="true">psychology</span>
            Perceptual
          </span>
          <span
            class="integrity-badge"
            data-active={recipe.integrity.deterministicModelsUsed}
          >
            <span class="material-symbols-outlined" aria-hidden="true">calculate</span>
            Deterministic
          </span>
          <span
            class="integrity-badge"
            data-active={recipe.integrity.seedRecorded}
          >
            <span class="material-symbols-outlined" aria-hidden="true">tag</span>
            Seed recorded
          </span>
        </div>
        {#if recipe.integrity.models.length > 0}
          <ul class="model-list font-body">
            {#each recipe.integrity.models as m, i (i)}
              <li class="model-row">
                <span class="model-name">{m.modelName}</span>
                <span class="model-type" data-type={m.modelType}>
                  {modelTypeLabel(m.modelType)}
                </span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty-body font-body">No models recorded.</p>
        {/if}
      </div>

      <div class="recipe-stages">
        <h5 class="section-title font-label">
          Stages ({recipe.stages.length})
        </h5>
        {#if recipe.stages.length === 0}
          <p class="empty-body font-body">No stages recorded.</p>
        {:else}
          <ol class="stage-list font-body">
            {#each recipe.stages.slice(0, 3) as s, i (i)}
              <li class="stage-row">
                <span class="stage-num">{i + 1}</span>
                <span class="stage-name">{s.stageId}</span>
                {#if !s.enabled}
                  <span class="stage-disabled">disabled</span>
                {/if}
              </li>
            {/each}
            {#if recipe.stages.length > 3}
              <li class="stage-overflow font-body">
                + {recipe.stages.length - 3} more (see Stage Timeline in B15)
              </li>
            {/if}
          </ol>
        {/if}
      </div>
    </div>
  {:else}
    <p class="status-note font-body">Select a version to view provenance.</p>
  {/if}
</section>

<style>
  .provenance-panel {
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
    align-items: center;
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
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-note {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    color: var(--on-surface-variant);
    margin: 0;
  }

  .error {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    padding: var(--sp-sm);
    background: rgba(244, 67, 54, 0.1);
    border-left: 3px solid #f44336;
    border-radius: var(--radius-sm);
  }

  .error-text {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    color: #ef9a9a;
    margin: 0;
  }

  .retry-cta {
    align-self: flex-start;
    padding: var(--sp-xs) var(--sp-sm);
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }

  .empty {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
    padding: var(--sp-sm);
    background: var(--surface-container-high);
    border-left: 3px solid var(--outline-variant);
    border-radius: var(--radius-sm);
  }

  .empty-title {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    color: var(--on-surface);
    margin: 0;
    font-weight: 500;
  }

  .empty-body {
    color: var(--on-surface-variant);
    margin: 0;
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .recipe {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .recipe-name {
    margin: 0 0 var(--sp-xs) 0;
    font-size: 1.05rem;
  }

  .recipe-description {
    color: var(--on-surface-variant);
    margin: 0 0 var(--sp-sm) 0;
    font-size: 0.9rem;
    line-height: 1.4;
  }

  .recipe-meta {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--sp-xs) var(--sp-sm);
    margin: 0;
    font-size: 0.85rem;
  }

  .meta-row {
    display: contents;
  }

  .meta-row dt {
    color: var(--on-surface-variant);
    text-transform: uppercase;
    font-size: 0.7rem;
    letter-spacing: 0.05em;
  }

  .meta-row dd {
    margin: 0;
    color: var(--on-surface);
  }

  .section-title {
    margin: 0 0 var(--sp-xs) 0;
    font-size: 0.7rem;
    color: var(--primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .integrity-badges {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-xs);
    margin-bottom: var(--sp-sm);
  }

  .integrity-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: 2px var(--sp-xs);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    font-size: 0.75rem;
    border: 1px solid var(--outline-variant);
  }

  .integrity-badge[data-active="true"] {
    background: rgba(76, 175, 80, 0.15);
    color: #81c784;
    border-color: rgba(76, 175, 80, 0.3);
  }

  .model-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .model-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--sp-xs);
    background: var(--surface-container-high);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
  }

  .model-name {
    color: var(--on-surface);
  }

  .model-type {
    font-size: 0.7rem;
    text-transform: uppercase;
    padding: 2px var(--sp-xs);
    border-radius: var(--radius-full);
    color: var(--on-surface-variant);
    background: var(--surface-container);
  }

  .model-type[data-type="perceptual"] {
    color: #ce93d8;
    background: rgba(206, 147, 216, 0.1);
  }

  .model-type[data-type="deterministic"] {
    color: #90caf9;
    background: rgba(144, 202, 249, 0.1);
  }

  .stage-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .stage-row {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: var(--sp-xs);
    background: var(--surface-container-high);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
  }

  .stage-num {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: 50%;
    background: var(--surface-container);
    color: var(--on-surface-variant);
    font-size: 0.7rem;
    flex-shrink: 0;
  }

  .stage-name {
    flex: 1;
    color: var(--on-surface);
  }

  .stage-disabled {
    font-size: 0.7rem;
    color: var(--on-surface-variant);
    text-transform: uppercase;
  }

  .stage-overflow {
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    text-align: center;
    padding: var(--sp-xs);
  }
</style>
