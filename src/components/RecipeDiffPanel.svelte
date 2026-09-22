<!--
  RecipeDiffPanel — CR-08 §13 Recipe parameter-diff UI.

  Side-by-side per-stage parameter diff viewer for two
  Recipes (A and B, identified by `(profileId, version)`).
  Backed by the `recipe_parameter_diff` Tauri command
  (CR-08 §13) which returns a typed
  `RecipeParameterDiff` payload:

  - `added`: stages only in Recipe B (params take the
    "Added" classification against a None A side).
  - `removed`: stages only in Recipe A (params take the
    "Removed" classification against a None B side).
  - `modified`: stages in both Recipes; per-param diff
    against the union of keys (Added / Removed / Changed /
    Unchanged) plus the `enabled` flag side-channel.
  - `identical`: true iff no stages were added / removed /
    modified. The panel renders an empty-state card.

  The component is a pure consumer of `recipeParameterDiff`
  from `src/lib/astroforge-api.ts`. It mounts inside
  `CompareWorkspace.svelte` alongside the existing
  `ProvenancePanel` (CR-07 B14) + `RecipeStageTimeline`
  (CR-07 B15); the AI-aware half of §13 is already
  covered by `BeginnerComparePrompt` + the
  `recipe_ai_diff_summary` IPC.

  Acceptance shape (per §13 spec):
  - per-stage params table with Added / Removed / Changed
    pills.
  - per-AdaptiveParameterSet reason lines: surfaced
    inline as a "Why this differs" caption under each
    changed param (deferred to the AdaptiveParameterSet
    integration slice when the data path lands; current
    slice renders the raw key + value pair).
  - recommended "Apply suggestion" button: stubbed as a
    disabled button (honest affordance) until the §22.3
    apply-flow can be threaded through.
-->
<script lang="ts">
  import {
    recipeParameterDiff,
    recipeGetForImageVersion,
    type RecipeParameterDiffFromRust,
    type StageDiffEntryFromRust,
    type ParamDiffEntryFromRust,
    type ParamChangeFromRust,
  } from "../lib/astroforge-api";
  import { profileIdFor } from "../lib/profile-store";

  interface Props {
    /**
     * ImageVersion ID for Recipe A. The component
     * resolves the underlying recipe via
     * `recipeGetForImageVersion(versionIdA)` to get the
     * `(profileId, version)` pair the diff IPC requires.
     */
    versionIdA: string | null;
    /** ImageVersion ID for Recipe B. */
    versionIdB: string | null;
  }

  const { versionIdA, versionIdB }: Props = $props();

  let diff: RecipeParameterDiffFromRust | null = $state(null);
  let loading = $state(true);
  let error: string | null = $state(null);

  async function loadDiff(): Promise<void> {
    loading = true;
    error = null;
    diff = null;
    if (!versionIdA || !versionIdB) {
      error = "Both versions must be selected to compute a diff.";
      loading = false;
      return;
    }
    try {
      const [recipeA, recipeB] = await Promise.all([
        recipeGetForImageVersion(versionIdA),
        recipeGetForImageVersion(versionIdB),
      ]);
      if (!recipeA || !recipeB) {
        error =
          "One or both ImageVersions have no recorded Recipe; the diff requires Recipes linked to each version.";
        loading = false;
        return;
      }
      // `profile_id` is not on the Recipe IPC payload
      // (it's the storage key, derived from
      // `profile_id_for(name, target_type)`); derive it
      // client-side from the Recipe identity. The TS
      // helper mirrors `RecipeStore::profile_id_for` in
      // Rust exactly, so the wire shape stays unchanged.
      const profileIdA = profileIdFor(recipeA.name, recipeA.target_type);
      const profileIdB = profileIdFor(recipeB.name, recipeB.target_type);
      diff = await recipeParameterDiff(
        profileIdA,
        recipeA.version,
        profileIdB,
        recipeB.version,
      );
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void loadDiff();
  });

  function changeClass(change: ParamChangeFromRust): string {
    return `change-pill change-${change.toLowerCase()}`;
  }

  function formatValue(v: unknown): string {
    if (v === null || v === undefined) return "—";
    if (typeof v === "string") return v;
    if (typeof v === "number" || typeof v === "boolean") return String(v);
    try {
      return JSON.stringify(v);
    } catch {
      return String(v);
    }
  }
</script>

<section class="recipe-diff" aria-label="Recipe parameter diff">
  <header class="diff-header">
    <h3 class="font-display">
      Parameter Diff
    </h3>
    {#if diff?.identical}
      <span class="identical-pill" data-testid="diff-identical-pill">
        Identical
      </span>
    {/if}
  </header>

  {#if loading}
    <p class="state-line font-body" aria-live="polite">Loading diff…</p>
  {:else if error}
    <p class="state-line error font-body" role="alert">{error}</p>
  {:else if diff && diff.identical}
    <p class="state-line font-body" aria-live="polite">
      Both Recipes have the same stages in the same order with the
      same enabled flags and the same params. Nothing to compare.
    </p>
  {:else if diff}
    <div class="diff-body">
      {#if diff.added.length > 0}
        <details class="diff-section" open data-testid="diff-added-section">
          <summary class="font-label">
            Added stages ({diff.added.length})
            <span class="section-hint">in Recipe B only</span>
          </summary>
          {#each diff.added as stage (stage.stage_id)}
            <article class="stage-block added" data-testid="diff-stage-added">
              <header class="stage-header">
                <span class="material-symbols-outlined" aria-hidden="true">
                  add_circle
                </span>
                <span class="stage-id font-display">{stage.stage_id}</span>
                <span class="enabled-pill">
                  enabled: {String(stage.enabled_b)}
                </span>
              </header>
              <ul class="param-list">
                {#each stage.params as param (param.key)}
                  <li class="param-row">
                    <span class={changeClass(param.change)}>
                      {param.change}
                    </span>
                    <span class="param-key font-body">{param.key}</span>
                    <span class="param-value font-body">
                      {formatValue(param.b)}
                    </span>
                  </li>
                {/each}
              </ul>
            </article>
          {/each}
        </details>
      {/if}

      {#if diff.removed.length > 0}
        <details class="diff-section" open data-testid="diff-removed-section">
          <summary class="font-label">
            Removed stages ({diff.removed.length})
            <span class="section-hint">in Recipe A only</span>
          </summary>
          {#each diff.removed as stage (stage.stage_id)}
            <article class="stage-block removed" data-testid="diff-stage-removed">
              <header class="stage-header">
                <span class="material-symbols-outlined" aria-hidden="true">
                  remove_circle
                </span>
                <span class="stage-id font-display">{stage.stage_id}</span>
                <span class="enabled-pill">
                  enabled: {String(stage.enabled_a)}
                </span>
              </header>
              <ul class="param-list">
                {#each stage.params as param (param.key)}
                  <li class="param-row">
                    <span class={changeClass(param.change)}>
                      {param.change}
                    </span>
                    <span class="param-key font-body">{param.key}</span>
                    <span class="param-value font-body">
                      {formatValue(param.a)}
                    </span>
                  </li>
                {/each}
              </ul>
            </article>
          {/each}
        </details>
      {/if}

      {#if diff.modified.length > 0}
        <details class="diff-section" open data-testid="diff-modified-section">
          <summary class="font-label">
            Modified stages ({diff.modified.length})
            <span class="section-hint">in both Recipes</span>
          </summary>
          {#each diff.modified as stage (stage.stage_id)}
            <article class="stage-block modified" data-testid="diff-stage-modified">
              <header class="stage-header">
                <span class="material-symbols-outlined" aria-hidden="true">
                  edit
                </span>
                <span class="stage-id font-display">{stage.stage_id}</span>
                {#if stage.enabled_differs}
                  <span class="enabled-pill differs">
                    enabled: {String(stage.enabled_a)} → {String(stage.enabled_b)}
                  </span>
                {/if}
              </header>
              {#if stage.params.length === 0}
                <p class="font-body empty-params">No params on this stage.</p>
              {:else}
                <ul class="param-list">
                  {#each stage.params as param (param.key)}
                    {#if param.change !== "Unchanged"}
                      <li class="param-row">
                        <span class={changeClass(param.change)}>
                          {param.change}
                        </span>
                        <span class="param-key font-body">{param.key}</span>
                        <span class="param-pair font-body">
                          <span class="param-side a">{formatValue(param.a)}</span>
                          <span class="param-arrow" aria-hidden="true">→</span>
                          <span class="param-side b">{formatValue(param.b)}</span>
                        </span>
                      </li>
                    {/if}
                  {/each}
                </ul>
              {/if}
            </article>
          {/each}
        </details>
      {/if}

      <!--
        Honest stub: the §13 spec calls for an
        "Apply suggestion" button on the diff panel
        (synthesizes a new Recipe from B's settings on
        Recipe A's image). The §22.3 apply-flow is in
        place but threading it through the diff panel
        needs a follow-on slice that wires the
        recipe_apply IPC into a per-row action handler.
        Disabled with a tooltip so the user reads the
        affordance rather than missing the button.
      -->
      <button
        type="button"
        class="apply-suggestion font-label"
        disabled
        title="Apply B's params to A's image — wired in a follow-on slice"
        data-testid="diff-apply-suggestion-btn"
      >
        <span class="material-symbols-outlined" aria-hidden="true">
          arrow_forward
        </span>
        Apply suggestion
      </button>
    </div>
  {/if}
</section>

<style>
  .recipe-diff {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
  }

  .diff-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-sm);
  }

  .diff-header h3 {
    margin: 0;
    font-size: 1rem;
  }

  .vs {
    color: var(--on-surface-variant);
    font-weight: 400;
  }

  .identical-pill {
    display: inline-block;
    padding: 2px 10px;
    background: var(--primary-container);
    color: var(--on-primary-container);
    border: 1px solid var(--primary);
    border-radius: var(--radius-full);
    font-size: 0.75rem;
    font-weight: 600;
  }

  .state-line {
    color: var(--on-surface-variant);
  }

  .state-line.error {
    color: #e53935;
  }

  .diff-body {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .diff-section {
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    background: var(--surface-container);
    padding: var(--sp-sm);
  }

  .diff-section > summary {
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
    list-style: none;
  }

  .section-hint {
    color: var(--on-surface-variant);
    font-weight: 400;
    font-size: 0.8rem;
    margin-left: var(--sp-xs);
  }

  .stage-block {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
    padding: var(--sp-sm);
    margin-top: var(--sp-sm);
    background: var(--surface-container-low);
    border-left: 3px solid var(--outline-variant);
    border-radius: var(--radius-sm);
  }

  .stage-block.added {
    border-left-color: var(--primary);
  }

  .stage-block.removed {
    border-left-color: #e53935;
  }

  .stage-block.modified {
    border-left-color: var(--secondary);
  }

  .stage-header {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    flex-wrap: wrap;
  }

  .stage-id {
    font-size: 0.95rem;
    font-weight: 600;
  }

  .enabled-pill {
    display: inline-block;
    padding: 1px 8px;
    background: var(--surface-container-highest);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-full);
    font-size: 0.7rem;
    color: var(--on-surface-variant);
  }

  .enabled-pill.differs {
    border-color: var(--secondary);
    color: var(--on-surface);
  }

  .param-list {
    list-style: none;
    padding: 0;
    margin: var(--sp-xs) 0 0 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .param-row {
    display: grid;
    grid-template-columns: 90px 1fr 2fr;
    gap: var(--sp-sm);
    align-items: center;
    padding: var(--sp-xs) var(--sp-sm);
    background: var(--surface-container);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
  }

  .param-key {
    color: var(--on-surface);
    font-family: var(--font-data, monospace);
  }

  .param-value {
    color: var(--on-surface-variant);
    font-family: var(--font-data, monospace);
    word-break: break-word;
  }

  .param-pair {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    font-family: var(--font-data, monospace);
  }

  .param-side.a {
    color: #e53935;
  }

  .param-side.b {
    color: var(--primary);
  }

  .param-arrow {
    color: var(--on-surface-variant);
  }

  .change-pill {
    display: inline-block;
    padding: 1px 8px;
    border-radius: var(--radius-full);
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    text-align: center;
  }

  .change-added {
    background: var(--primary-container);
    color: var(--on-primary-container);
  }

  .change-removed {
    background: #e53935;
    color: white;
  }

  .change-changed {
    background: var(--secondary-container);
    color: var(--on-secondary-container);
  }

  .change-unchanged {
    background: var(--surface-container-highest);
    color: var(--on-surface-variant);
  }

  .empty-params {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    font-style: italic;
  }

  .apply-suggestion {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    align-self: flex-start;
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .apply-suggestion:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
