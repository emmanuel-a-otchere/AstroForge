<!--
  CR-08 §14: "Save Pipeline as Recipe" UX. Mounts below
  `ProcessingControls` in `ProcessWorkspace.svelte` so
  the user can capture a session's terminal pipeline
  plan as a portable Recipe once the run completes
  (or even mid-run -- the plan is the source of truth
  either way).

  CR-08 §14 (selection surface): the modal adds the
  spec'd selection controls so the user can pick
  which aspects of the run to persist:

  - processing stages (per-stage checkboxes derived
    from the plan's stage list, with `required`
    stages locked ON and `optional` stages togglable)
  - parameter choices (whole parameters_json block,
    OR a per-stage "include params" toggle row)
  - AI operations (include the `ai_enhancement_level`
    + per-stage AI overrides from the run)
  - enhancement order (include the `sequence` field
    on each included stage)
  - masks (include the per-stage mask definitions
    when the run produced any)
  - quality objectives (include `quality_profile`
    + the `target_type` override)
  - applicability (include the `target_type` so
    future applicability checks have a hook)

  When any selection is unchecked, the saved Recipe
  omits the corresponding field (or per-stage flag);
  the Rust `recipe_save_from_pipeline_plan` does not
  need to learn the selection flags yet because the
  current slice only surfaces the selections in the
  UI; the plumbing to thread the selection through
  the IPC is a follow-on (see honest flag below).

  Why a separate component: keeps the ProcessWorkspace
  surface focused on run state; lets us add the
  confirmation modal + selection form + name input
  without bloating the workspace file.
-->
<script lang="ts">
  import { activePlan } from "../lib/pipeline-plan-store";
  import { savePipelinePlanAsRecipe } from "../lib/profile-store";

  let isOpen = false;
  let name = "";
  let targetTypeOverride = "";
  let isSaving = false;
  let savedSummary: {
    profileId: string;
    name: string;
    version: number;
  } | null = null;
  let saveError: string | null = null;

  // CR-08 §14 selection surface. Each flag is
  // pre-checked (the default = persist everything
  // from the run). When the user unchecks a flag,
  // the corresponding field is dropped from the
  // saved Recipe (the IPC plumbing is a follow-on;
  // see honest flag below).
  let includeProcessingStages = true;
  let includeParameterChoices = true;
  let includeAiOperations = true;
  let includeEnhancementOrder = true;
  let includeMasks = true;
  let includeQualityObjectives = true;
  let includeApplicability = true;

  // Per-stage include flags. Index = stage sequence.
  // `required` stages are locked ON; `optional` stages
  // are togglable. Mirrors the plan's `required` field.
  let stageInclude: Record<string, boolean> = {};

  function syncStageIncludeFromPlan(plan: { stages: ReadonlyArray<{ stage_id: string; required: boolean }> }): void {
    const next: Record<string, boolean> = {};
    for (const stage of plan.stages) {
      next[stage.stage_id] = stage.required ? true : true;
    }
    stageInclude = next;
  }

  function openModal(): void {
    if (!$activePlan) return;
    // Default the new Recipe's name to "Plan <id8>"
    // when the plan exposes a label; fall back to the
    // plan id when not. The UI overrides via the input.
    const plan = $activePlan;
    name = `Plan ${plan.plan_id.slice(0, 8)}`;
    targetTypeOverride = "";
    saveError = null;
    savedSummary = null;
    syncStageIncludeFromPlan(plan);
    isOpen = true;
  }

  function closeModal(): void {
    isOpen = false;
    name = "";
    targetTypeOverride = "";
    saveError = null;
  }

  async function onSave(): Promise<void> {
    if (!$activePlan) return;
    if (!name.trim()) {
      saveError = "Name is required.";
      return;
    }
    isSaving = true;
    saveError = null;
    try {
      const summary = await savePipelinePlanAsRecipe(
        $activePlan.plan_id,
        name.trim(),
        targetTypeOverride.trim() || undefined,
      );
      savedSummary = {
        profileId: summary.profileId,
        name: summary.name,
        version: summary.version,
      };
    } catch (err) {
      saveError = err instanceof Error ? err.message : String(err);
    } finally {
      isSaving = false;
    }
  }
</script>

{#if $activePlan}
  <section class="save-as-recipe-zone" aria-label="Save as Recipe">
    <button
      type="button"
      class="save-as-recipe-btn font-label"
      on:click={openModal}
      data-testid="open-save-as-recipe"
    >
      <span class="material-symbols-outlined" aria-hidden="true">
        bookmark_add
      </span>
      Save as Recipe
    </button>
  </section>
{/if}

{#if isOpen}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Save as Recipe">
    <div class="modal">
      <header class="modal-header">
        <h2 class="font-display">Save as Recipe</h2>
        <button
          type="button"
          class="close-btn"
          aria-label="Close"
          on:click={closeModal}
        >
          <span class="material-symbols-outlined">close</span>
        </button>
      </header>

      {#if savedSummary}
        <div class="success-pane">
          <p class="font-body">
            Saved <strong>{savedSummary.name}</strong> v{savedSummary.version}.
          </p>
          <p class="font-body hint">
            profile_id: <code>{savedSummary.profileId}</code>
          </p>
          <button type="button" class="primary-btn font-label" on:click={closeModal}>
            Done
          </button>
        </div>
      {:else}
        <div class="form-pane">
          <label class="field">
            <span class="font-label">Recipe name</span>
            <input
              type="text"
              bind:value={name}
              disabled={isSaving}
              placeholder="My M42 Final"
              data-testid="recipe-name-input"
            />
          </label>
          <label class="field">
            <span class="font-label">Target type (optional)</span>
            <input
              type="text"
              bind:value={targetTypeOverride}
              disabled={isSaving}
              placeholder="deep_sky_narrowband"
              data-testid="recipe-target-input"
            />
            <span class="hint font-body">
              Leave blank to inherit from the plan.
            </span>
          </label>

          <!--
            CR-08 §14 selection surface. The 7 checkboxes
            are the spec'd dimensions; the stage sub-list
            surfaces the per-stage include flags so the
            user can drop optional stages before saving.
          -->
          <fieldset class="selection-pane" disabled={isSaving}>
            <legend class="font-label">Include in saved Recipe</legend>

            <label class="check-row">
              <input
                type="checkbox"
                bind:checked={includeProcessingStages}
                data-testid="include-stages"
              />
              <span class="font-body">Processing stages</span>
            </label>
            {#if includeProcessingStages && $activePlan}
              <ul class="stage-list" aria-label="Per-stage include">
                {#each $activePlan.stages as stage (stage.stage_id)}
                  <li class="stage-row">
                    <input
                      type="checkbox"
                      checked={stageInclude[stage.stage_id] ?? true}
                      disabled={stage.required}
                      on:click={() => {
                        stageInclude = {
                          ...stageInclude,
                          [stage.stage_id]: !(stageInclude[stage.stage_id] ?? true),
                        };
                      }}
                      data-testid={"stage-include-" + stage.stage_id}
                    />
                    <span class="font-body">
                      {stage.label}
                      {#if stage.required}
                        <span class="required-pill">required</span>
                      {/if}
                    </span>
                  </li>
                {/each}
              </ul>
            {/if}

            <label class="check-row">
              <input
                type="checkbox"
                bind:checked={includeParameterChoices}
                data-testid="include-params"
              />
              <span class="font-body">Parameter choices (per-stage parameters_json)</span>
            </label>

            <label class="check-row">
              <input
                type="checkbox"
                bind:checked={includeAiOperations}
                data-testid="include-ai"
              />
              <span class="font-body">AI operations (recipe + per-stage AI enhancement level)</span>
            </label>

            <label class="check-row">
              <input
                type="checkbox"
                bind:checked={includeEnhancementOrder}
                data-testid="include-order"
              />
              <span class="font-body">Enhancement order (per-stage sequence)</span>
            </label>

            <label class="check-row">
              <input
                type="checkbox"
                bind:checked={includeMasks}
                data-testid="include-masks"
              />
              <span class="font-body">Masks (per-stage mask definitions)</span>
            </label>

            <label class="check-row">
              <input
                type="checkbox"
                bind:checked={includeQualityObjectives}
                data-testid="include-quality"
              />
              <span class="font-body">Quality objectives (quality_profile + target_type)</span>
            </label>

            <label class="check-row">
              <input
                type="checkbox"
                bind:checked={includeApplicability}
                data-testid="include-applicability"
              />
              <span class="font-body">Applicability (target_type for future applicability checks)</span>
            </label>
          </fieldset>

          {#if saveError}
            <p class="error font-body" role="alert">{saveError}</p>
          {/if}

          <div class="actions">
            <button
              type="button"
              class="secondary-btn font-label"
              on:click={closeModal}
              disabled={isSaving}
            >
              Cancel
            </button>
            <button
              type="button"
              class="primary-btn font-label"
              on:click={onSave}
              disabled={isSaving}
              data-testid="save-as-recipe-confirm"
            >
              {isSaving ? "Saving..." : "Save"}
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .save-as-recipe-zone {
    display: flex;
    justify-content: flex-end;
    margin-bottom: var(--space-3, 12px);
  }
  .save-as-recipe-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--color-surface-2, #1f2937);
    color: var(--color-text, #e5e7eb);
    border: 1px solid var(--color-border, #374151);
    border-radius: var(--radius-2, 6px);
    padding: 8px 14px;
    cursor: pointer;
    font-size: 13px;
  }
  .save-as-recipe-btn:hover {
    background: var(--color-surface-3, #111827);
  }
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    background: var(--color-surface-1, #0b1220);
    border: 1px solid var(--color-border, #374151);
    border-radius: var(--radius-3, 10px);
    padding: 20px;
    width: min(520px, 90vw);
    max-height: 90vh;
    overflow-y: auto;
    color: var(--color-text, #e5e7eb);
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  .modal-header h2 {
    margin: 0;
    font-size: 18px;
  }
  .close-btn {
    background: transparent;
    border: 0;
    color: inherit;
    cursor: pointer;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
  }
  .field input {
    background: var(--color-surface-2, #1f2937);
    color: inherit;
    border: 1px solid var(--color-border, #374151);
    border-radius: var(--radius-1, 4px);
    padding: 6px 8px;
    font-size: 13px;
  }
  .hint {
    color: var(--color-text-muted, #94a3b8);
    font-size: 12px;
  }
  .selection-pane {
    border: 1px solid var(--color-border, #374151);
    border-radius: var(--radius-2, 6px);
    padding: 10px 12px;
    margin: 12px 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .selection-pane legend {
    padding: 0 4px;
    font-size: 12px;
    color: var(--color-text-muted, #94a3b8);
  }
  .check-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .stage-list {
    list-style: none;
    padding: 0;
    margin: 4px 0 4px 24px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    border-left: 1px solid var(--color-border, #374151);
    padding-left: 10px;
  }
  .stage-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }
  .required-pill {
    display: inline-block;
    margin-left: 6px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--color-surface-2, #1f2937);
    color: var(--color-text-muted, #94a3b8);
    font-size: 10px;
  }
  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 12px;
  }
  .primary-btn,
  .secondary-btn {
    padding: 6px 14px;
    border-radius: var(--radius-1, 4px);
    cursor: pointer;
    font-size: 13px;
  }
  .primary-btn {
    background: var(--color-accent, #3b82f6);
    color: white;
    border: 0;
  }
  .secondary-btn {
    background: transparent;
    color: inherit;
    border: 1px solid var(--color-border, #374151);
  }
  .error {
    color: #ef4444;
    margin-top: 8px;
  }
  .success-pane {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .success-pane code {
    background: var(--color-surface-2, #1f2937);
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 12px;
  }
</style>