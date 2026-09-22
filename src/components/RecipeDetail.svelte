<!--
  RecipeDetail: CR-08 §15 Recipe Detail surface.

  Renders the §15 spec'd layout (per
  `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md`
  §15):

  ┌──────────────────────────────────────────────┐
  │ Galaxy — Natural                    v1.3     │
  │                                              │
  │ Natural galaxy processing with preserved     │
  │ star profiles and controlled noise.          │
  │                                              │
  │ Target       Galaxy                          │
  │ AI           Conservative                    │
  │ Style        Natural                         │
  │                                              │
  │ Processing Intent                            │
  │ ● Preserve faint structures                  │
  │ ● Control noise                              │
  │ ● Preserve stars                             │
  │                                              │
  │ [Apply Recipe] [Duplicate] [Edit]            │
  └──────────────────────────────────────────────┘

  The component takes a `summary: RecipeSummary`
  prop (the same shape that `RecipeLibrary.svelte`
  uses) and loads the full `Recipe` body via the
  `recipeGet(profileId, version)` IPC. The summary
  is the immediately-available surface (already
  in the Library's list); the full Recipe body
  fills in once the IPC round-trip lands.

  Three action buttons:
  - **Apply Recipe** — emits `apply` event with
    the (profileId, version) pair. The parent
    wires this to `recipe_apply` IPC + the
    Session picker (out of scope for this slice;
    the action is a no-op stub that emits the
    event so the parent's wiring can land in a
    follow-on slice).
  - **Duplicate** — emits `duplicate` event with
    the (profileId, version) pair. The parent
    wires this to the existing `recipe_save` IPC
    with a derived name (e.g. "Galaxy — Natural
    Copy"). Out of scope for this slice; the
    action emits the event.
  - **Edit** — emits `edit` event with the
    (profileId, version) pair. The parent wires
    this to the §10 Recipe Editor modal. Out of
    scope for this slice; the action emits the
    event.

  The component handles three load states:
  - **Empty**: no `summary` passed; renders a
    "Select a Recipe" empty state.
  - **Loading**: the IPC round-trip is in flight;
    renders the summary's lightweight fields
    immediately + a Loading banner for the full
    body fields.
  - **Loaded**: the full Recipe body is available;
    renders the §15 surface verbatim.
  - **Missing**: the IPC returned null (Recipe
    was deleted between the Library render and
    the click); renders a "Recipe not found"
    empty state.
  - **Error**: the IPC round-trip threw; renders
    the error message inline with a retry button.
-->
<script lang="ts">
  import {
    recipeGet,
    type RecipeFromRust,
  } from "../lib/astroforge-api";
  import type { RecipeSummary } from "../lib/profile-store";
  import { untrack } from "svelte";

  interface Props {
    /** The Recipe summary the user clicked. Drives
     * the immediate summary fields (name + version +
     * description fallback + status pills). */
    summary: RecipeSummary | null;
    /** Fired when the user clicks "Apply Recipe". */
    onApply?: (profileId: string, version: number) => void;
    /** Fired when the user clicks "Duplicate". */
    onDuplicate?: (profileId: string, version: number) => void;
    /** Fired when the user clicks "Edit". */
    onEdit?: (profileId: string, version: number) => void;
  }

  let { summary, onApply, onDuplicate, onEdit }: Props = $props();

  // Full Recipe body load state machine.
  let recipe = $state<RecipeFromRust | null>(null);
  let loadState = $state<"idle" | "loading" | "ready" | "missing" | "error">(
    "idle",
  );
  let loadError = $state<string | null>(null);

  // When `summary` changes, refetch the full body.
  // The component initializes from `summary` once
  // (parent owns the open / close lifecycle); the
  // `untrack(...)` wrappers tell Svelte not to
  // treat these references as reactive dependencies.
  $effect(() => {
    const next = untrack(() => summary);
    if (!next) {
      recipe = null;
      loadState = "idle";
      loadError = null;
      return;
    }
    loadState = "loading";
    loadError = null;
    recipeGet(next.profileId, next.version)
      .then((result) => {
        if (result === null) {
          recipe = null;
          loadState = "missing";
        } else {
          recipe = result;
          loadState = "ready";
        }
      })
      .catch((err: unknown) => {
        recipe = null;
        loadState = "error";
        loadError =
          err instanceof Error
            ? err.message
            : "Failed to load Recipe detail.";
      });
  });

  function retry(): void {
    // Re-fire the effect by reassigning summary (same
    // identity, same effect dependency, but Svelte
    // re-runs the effect on assignment to the prop
    // through the parent's re-render pass). The simpler
    // path: directly call recipeGet.
    if (!summary) return;
    loadState = "loading";
    loadError = null;
    recipeGet(summary.profileId, summary.version)
      .then((result) => {
        if (result === null) {
          recipe = null;
          loadState = "missing";
        } else {
          recipe = result;
          loadState = "ready";
        }
      })
      .catch((err: unknown) => {
        recipe = null;
        loadState = "error";
        loadError =
          err instanceof Error
            ? err.message
            : "Failed to load Recipe detail.";
      });
  }

  function handleApply(): void {
    if (!summary || !onApply) return;
    onApply(summary.profileId, summary.version);
  }

  function handleDuplicate(): void {
    if (!summary || !onDuplicate) return;
    onDuplicate(summary.profileId, summary.version);
  }

  function handleEdit(): void {
    if (!summary || !onEdit) return;
    onEdit(summary.profileId, summary.version);
  }

  // Quality Profile label helper. Falls back to the
  // raw wire value when unknown; "Natural" when absent.
  function qualityProfileLabelFor(
    wire: string | undefined,
  ): string {
    if (!wire) return "Natural";
    const label: Record<string, string> = {
      natural: "Natural",
      detail: "Detail",
      clean: "Clean",
      publication: "Publication",
    };
    return label[wire] ?? wire;
  }

  // AI Enhancement Level label helper. Falls back to
  // the raw wire value when unknown; "Recommended"
  // when absent (the §10 default).
  function aiLabelFor(wire: string | undefined): string {
    if (!wire) return "Recommended";
    const label: Record<string, string> = {
      off: "Off",
      conservative: "Conservative",
      recommended: "Recommended",
      advanced: "Advanced",
    };
    return label[wire] ?? wire;
  }

  // Processing Objective label helper. Falls back to
  // the raw wire value when unknown.
  function objectiveLabelFor(wire: string): string {
    const label: Record<string, string> = {
      preserve_star_colors: "Preserve star colors",
      maximize_detail: "Maximize detail",
      maximize_smoothness: "Maximize smoothness",
      maximize_dynamic_range: "Maximize dynamic range",
      maximize_reproducibility: "Maximize reproducibility",
    };
    return label[wire] ?? wire;
  }
</script>

<section
  class="recipe-detail"
  aria-label="Recipe detail"
  data-testid="recipe-detail"
>
  {#if !summary}
    <div class="empty-state" data-testid="recipe-detail-empty">
      <span class="material-symbols-outlined" aria-hidden="true">
        bookmark
      </span>
      <h3 class="font-display">Select a Recipe</h3>
      <p class="font-body">
        Pick a card from the Library to inspect its
        title bar, description, target, AI posture,
        style, processing intent, and action buttons.
      </p>
    </div>
  {:else if loadState === "missing"}
    <div class="empty-state" data-testid="recipe-detail-missing">
      <span class="material-symbols-outlined" aria-hidden="true">
        error
      </span>
      <h3 class="font-display">Recipe not found</h3>
      <p class="font-body">
        The Recipe <strong>{summary.name}</strong> v{summary.version}
        no longer exists in the store. It may have been
        deleted or archived. Pick a different card.
      </p>
    </div>
  {:else if loadState === "error"}
    <div class="empty-state error" data-testid="recipe-detail-error">
      <span class="material-symbols-outlined" aria-hidden="true">
        warning
      </span>
      <h3 class="font-display">Failed to load Recipe</h3>
      <p class="font-body">{loadError ?? "Unknown error."}</p>
      <button
        type="button"
        class="retry font-label"
        onclick={retry}
        data-testid="recipe-detail-retry"
      >
        Retry
      </button>
    </div>
  {:else}
    <header class="detail-head">
      <div class="title-row">
        <h2 class="detail-title font-display" data-testid="recipe-detail-title">
          {recipe?.name ?? summary.name}
        </h2>
        <span class="version-pill font-label">
          v{recipe?.version ?? summary.version}
        </span>
      </div>

      <div class="status-row">
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
        {#if recipe?.ai_enhancement_level && recipe.ai_enhancement_level !== "off"}
          <span
            class="status-pill ai"
            title="Uses AI stage"
            data-testid="recipe-detail-ai-pill"
          >
            AI: {aiLabelFor(recipe.ai_enhancement_level)}
          </span>
        {/if}
        {#if recipe?.integrity?.perceptual_models_used}
          <span class="status-pill perceptual" title="Uses perceptual AI model">
            Perceptual
          </span>
        {/if}
        {#if recipe?.integrity?.seed_recorded}
          <span
            class="status-pill deterministic"
            title="Deterministic seed recorded"
          >
            Deterministic
          </span>
        {/if}
      </div>
    </header>

    <p
      class="detail-description font-body"
      data-testid="recipe-detail-description"
    >
      {recipe?.description ?? summary.description ?? ""}
    </p>

    <div class="meta-grid" data-testid="recipe-detail-meta">
      <div class="meta-row">
        <span class="meta-label font-label">Target</span>
        <span class="meta-value font-body">
          {recipe?.target_type ?? summary.targetType}
        </span>
      </div>
      <div class="meta-row">
        <span class="meta-label font-label">AI</span>
        <span class="meta-value font-body">
          {aiLabelFor(recipe?.ai_enhancement_level)}
        </span>
      </div>
      <div class="meta-row">
        <span class="meta-label font-label">Style</span>
        <span class="meta-value font-body">
          {qualityProfileLabelFor(recipe?.quality_profile)}
        </span>
      </div>
      <div class="meta-row">
        <span class="meta-label font-label">Stages</span>
        <span class="meta-value font-body">
          {recipe ? `${recipe.stages.length} configured` : "—"}
        </span>
      </div>
    </div>

    {#if loadState === "loading"}
      <p
        class="loading-banner font-body"
        role="status"
        aria-live="polite"
        data-testid="recipe-detail-loading"
      >
        Loading full Recipe details...
      </p>
    {/if}

    {#if recipe && recipe.processing_objectives && recipe.processing_objectives.length > 0}
      <section class="intent-section">
        <h3 class="intent-title font-label">Processing Intent</h3>
        <ul class="intent-list" data-testid="recipe-detail-intent">
          {#each recipe.processing_objectives as obj (obj)}
            <li class="intent-bullet font-body">
              <span class="bullet" aria-hidden="true">●</span>
              {objectiveLabelFor(obj)}
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if recipe && recipe.required_models && recipe.required_models.length > 0}
      <section class="models-section">
        <h3 class="models-title font-label">Required Models</h3>
        <ul class="models-list" data-testid="recipe-detail-models">
          {#each recipe.required_models as model (model)}
            <li class="model-chip font-body" title={model}>
              <span class="material-symbols-outlined" aria-hidden="true">
                memory
              </span>
              {model}
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <footer class="detail-actions" data-testid="recipe-detail-actions">
      <button
        type="button"
        class="action-apply font-label"
        onclick={handleApply}
        disabled={!onApply || loadState !== "ready"}
        data-testid="recipe-detail-apply"
      >
        <span class="material-symbols-outlined" aria-hidden="true">play_arrow</span>
        Apply Recipe
      </button>
      <button
        type="button"
        class="action-duplicate font-label"
        onclick={handleDuplicate}
        disabled={!onDuplicate || loadState !== "ready"}
        data-testid="recipe-detail-duplicate"
      >
        <span class="material-symbols-outlined" aria-hidden="true">content_copy</span>
        Duplicate
      </button>
      <button
        type="button"
        class="action-edit font-label"
        onclick={handleEdit}
        disabled={!onEdit || loadState !== "ready"}
        data-testid="recipe-detail-edit"
      >
        <span class="material-symbols-outlined" aria-hidden="true">edit</span>
        Edit
      </button>
    </footer>
  {/if}
</section>

<style>
  .recipe-detail {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    min-height: 280px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sp-sm);
    text-align: center;
    padding: var(--sp-lg);
    color: var(--on-surface-variant);
  }

  .empty-state.error {
    color: var(--error, #c62828);
  }

  .empty-state .material-symbols-outlined {
    font-size: 48px;
  }

  .empty-state h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--on-surface);
  }

  .empty-state p {
    margin: 0;
    max-width: 480px;
  }

  .retry {
    margin-top: var(--sp-xs);
    padding: var(--sp-xs) var(--sp-md);
    border: 1px solid var(--primary);
    border-radius: var(--radius-md);
    background: var(--surface-container);
    color: var(--primary);
    cursor: pointer;
  }

  .retry:hover {
    background: var(--primary-container);
  }

  .detail-head {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .title-row {
    display: flex;
    align-items: baseline;
    gap: var(--sp-sm);
    flex-wrap: wrap;
  }

  .detail-title {
    margin: 0;
    font-size: 1.4rem;
  }

  .version-pill {
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    padding: 2px 8px;
    font-size: 0.85rem;
    color: var(--on-surface-variant);
  }

  .status-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-xs);
  }

  .status-pill {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 999px;
    font-size: 0.75rem;
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    color: var(--on-surface-variant);
  }

  .status-pill.system {
    border-color: var(--primary);
    color: var(--primary);
  }

  .status-pill.imported {
    border-color: var(--tertiary, #7b1fa2);
    color: var(--tertiary, #7b1fa2);
  }

  .status-pill.ai {
    border-color: var(--secondary, #00838f);
    color: var(--secondary, #00838f);
  }

  .status-pill.perceptual {
    border-color: var(--warning, #f9a825);
    color: var(--warning, #f9a825);
  }

  .status-pill.deterministic {
    border-color: var(--success, #2e7d32);
    color: var(--success, #2e7d32);
  }

  .detail-description {
    margin: 0;
    color: var(--on-surface-variant);
    line-height: 1.5;
    font-size: 0.95rem;
  }

  .meta-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: var(--sp-sm);
    padding: var(--sp-sm);
    background: var(--surface-container);
    border-radius: var(--radius-md);
  }

  .meta-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta-label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--on-surface-variant);
  }

  .meta-value {
    font-size: 0.95rem;
    color: var(--on-surface);
  }

  .loading-banner {
    margin: 0;
    padding: var(--sp-xs) var(--sp-sm);
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }

  .intent-section,
  .models-section {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .intent-title,
  .models-title {
    margin: 0;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--on-surface-variant);
  }

  .intent-list,
  .models-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .intent-bullet {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: 4px var(--sp-sm);
    background: var(--surface-container);
    border-radius: var(--radius-md);
    font-size: 0.9rem;
  }

  .bullet {
    color: var(--primary);
    font-size: 0.7rem;
  }

  .model-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    font-family: var(--font-mono, monospace);
    font-size: 0.8rem;
    color: var(--on-surface);
    align-self: flex-start;
  }

  .detail-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-sm);
    margin-top: auto;
    padding-top: var(--sp-md);
    border-top: 1px solid var(--outline-variant);
  }

  .action-apply,
  .action-duplicate,
  .action-edit {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: var(--sp-xs) var(--sp-md);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.9rem;
    border: 1px solid var(--outline-variant);
  }

  .action-apply {
    background: var(--primary);
    color: var(--on-primary);
    border-color: var(--primary);
  }

  .action-apply:hover:not(:disabled) {
    background: var(--primary-container);
    color: var(--primary);
  }

  .action-duplicate,
  .action-edit {
    background: var(--surface-container);
    color: var(--on-surface);
  }

  .action-duplicate:hover:not(:disabled),
  .action-edit:hover:not(:disabled) {
    background: var(--surface-container-high);
  }

  .action-apply:disabled,
  .action-duplicate:disabled,
  .action-edit:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>