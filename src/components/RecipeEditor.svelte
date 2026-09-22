<!--
  RecipeEditor: CR-08 §10 progressive-disclosure editor.

  The CR-08 §10 spec calls for a three-tier Recipe
  editor (Beginner / Guided / Expert). This slice
  ships the Beginner + Guided tier surfaces:

  Beginner (4 fields):
  - Recipe Name (text input)
  - Target Type (text input)
  - Processing Style (4-radio picker)
  - AI Enhancement (4-radio picker)

  Guided (4 collapsible fields, mounted below the
  Beginner section):
  - Processing Objectives (5-checkbox list)
  - Stage Inclusion (toggle table; lists every stage
    in the Recipe with its enabled flag; users can
    flip individual stages on / off)
  - Per-Stage AI Override (4-radio picker per
    enabled stage; "inherit" = recipe-level default)
  - Quality Targets (3 numeric inputs: target SNR
    dB, target sharpness, target background
    smoothness; each is optional)
  - Optional Operations (toggle chips for the
    cosmetic / curves / stacking stages)

  Scope:
  - Pure UI surface: collects the fields and
    exposes them via `onChange(payload)` so the
    parent can wire the change into `recipe_save`.
  - No persistence by itself.
  - The Expert tier is the existing ProfileManager
    (stages table + per-stage params + execution +
    masks + reproducibility controls). It is not
    wired into this component yet; that integration
    is a follow-on slice. Honest "Expert tier:
    ProfileManager" stub remains for the user to
    see the shape of the progressive-disclosure
    shell.

  The Guided tier fields are mutually independent
  (no cross-validation in this slice). The §20
  validation pipeline covers range / dependency /
  filesystem / executable / resource checks.
-->
<script lang="ts">
  import {
    DEFAULT_QUALITY_PROFILE,
    type AiEnhancementLevelFromRust,
    type ProcessingObjectiveFromRust,
    type QualityProfile,
    type QualityTargetsFromRust,
  } from "../lib/astroforge-api";
  import QualityProfilePicker from "./QualityProfilePicker.svelte";
  import { untrack } from "svelte";

  /** Combined Beginner + Guided payload. The
   * parent persists via `recipe_save` which
   * round-trips through the Recipe struct's
   * `quality_profile`, `ai_enhancement_level`,
   * `processing_objectives`, `quality_targets`,
   * `optional_operations`, and per-stage
   * `ai_enhancement_override` fields. */
  export interface EditorPayload {
    // Beginner
    name: string;
    targetType: string;
    qualityProfile: QualityProfile;
    aiEnhancementLevel: AiEnhancementLevelFromRust;
    // Guided
    processingObjectives: ProcessingObjectiveFromRust[];
    qualityTargets: QualityTargetsFromRust;
    optionalOperations: string[];
    /** Per-stage AI override map. Keyed by stage
     * ID; the value is `null` when the stage
     * inherits the recipe-level default. */
    stageOverrides: Record<
      string,
      AiEnhancementLevelFromRust | null
    >;
    /** Per-stage enabled flag map. Drives stage
     * inclusion (the Guided tier's stage
     * inclusion toggle). */
    stageEnabled: Record<string, boolean>;
  }

  /** Backward-compat alias used by the §10.1
   * Beginner-tier call sites. The Beginner payload
   * is a strict subset of the EditorPayload. */
  export type BeginnerTierPayload = Pick<
    EditorPayload,
    "name" | "targetType" | "qualityProfile" | "aiEnhancementLevel"
  >;

  interface Props {
    /** Current values to seed the form (initial render). */
    initial: EditorPayload;
    /** Stage IDs the Recipe currently includes.
     * The Guided tier's stage inclusion + per-stage
     * AI override lists are driven by this list
     * (which is the set of stages the apply round
     * actually runs). */
    stageIds: string[];
    /** Fired on every field change (debounced by parent
     * if the parent chooses). */
    onChange: (next: EditorPayload) => void;
    /** Optional disabled state (e.g. system Recipe
     * protection flips this on). */
    disabled?: boolean;
  }

  let { initial, stageIds, onChange, disabled = false }: Props = $props();

  // §10 Beginner + Guided tier local form state.
  // The parent owns the durable copy; the
  // component owns the in-flight edits so the
  // parent's `recipe_save` round-trip doesn't
  // fight live keystrokes.
  // The component initializes from `initial` once
  // (the parent owns the open / close lifecycle);
  // subsequent changes to `initial` are intentionally
  // NOT reflected so the user's in-flight edits are
  // preserved across prop re-renders.
  // The `untrack(...)` wrappers tell Svelte not to
  // treat these references as reactive dependencies.
  let name = $state(untrack(() => initial.name));
  let targetType = $state(untrack(() => initial.targetType));
  let qualityProfile: QualityProfile = $state(
    untrack(() => initial.qualityProfile),
  );
  let aiEnhancementLevel: AiEnhancementLevelFromRust = $state(
    untrack(() => initial.aiEnhancementLevel),
  );

  // Guided tier state.
  let processingObjectives: ProcessingObjectiveFromRust[] = $state(
    untrack(() => initial.processingObjectives),
  );
  let qualityTargets: QualityTargetsFromRust = $state(
    untrack(() => initial.qualityTargets),
  );
  let optionalOperations: string[] = $state(
    untrack(() => initial.optionalOperations),
  );
  let stageOverrides: Record<
    string,
    AiEnhancementLevelFromRust | null
  > = $state(untrack(() => ({ ...initial.stageOverrides })));
  let stageEnabled: Record<string, boolean> = $state(
    untrack(() => ({ ...initial.stageEnabled })),
  );

  // Collapsible state for the Guided tier section.
  // Open by default so the user sees the new fields
  // immediately after the slice lands; the parent
  // can collapse it by passing `initial` with
  // collapsed = true in a follow-on slice.
  let guidedOpen = $state(true);

  // Fire `onChange` whenever any field changes.
  // The parent's callback owns debouncing +
  // persistence; the component just emits every edit.
  $effect(() => {
    onChange({
      name,
      targetType,
      qualityProfile,
      aiEnhancementLevel,
      processingObjectives,
      qualityTargets,
      optionalOperations,
      stageOverrides,
      stageEnabled,
    });
  });

  // Four-variant picker labels mirror the Rust
  // `AiEnhancementLevel::label()` exactly so the
  // tier's radio captions match the wire format
  // surfaced in the audit doc + pipeline_plan_hash
  // contract.
  const AI_OPTIONS: ReadonlyArray<{
    value: AiEnhancementLevelFromRust;
    label: string;
    description: string;
  }> = [
    {
      value: "off",
      label: "Off",
      description:
        "No AI enhancement. Pure deterministic pipeline.",
    },
    {
      value: "conservative",
      label: "Conservative",
      description:
        "Minimal AI. Apply skips advanced AI stages.",
    },
    {
      value: "recommended",
      label: "Recommended",
      description:
        "Project default. Honors the Recipe's required_models.",
    },
    {
      value: "advanced",
      label: "Advanced",
      description:
        "All AI stages run; warnings for tight resource budgets.",
    },
  ];

  // The five canonical processing objectives
  // (mirrors `ProcessingObjective::ALL` in Rust).
  const OBJECTIVE_OPTIONS: ReadonlyArray<{
    value: ProcessingObjectiveFromRust;
    label: string;
  }> = [
    { value: "preserve_star_colors", label: "Preserve star colors" },
    { value: "maximize_detail", label: "Maximize detail" },
    { value: "maximize_smoothness", label: "Maximize smoothness" },
    { value: "maximize_dynamic_range", label: "Maximize dynamic range" },
    { value: "maximize_reproducibility", label: "Maximize reproducibility" },
  ];

  // The set of stages the user can mark as
  // "optional operations". Mirrors the
  // optional-ops chips shipped in the §22
  // quality-catalog recipes (cosmetic / curves /
  // stacking).
  const OPTIONAL_OP_CHIPS: ReadonlyArray<{ id: string; label: string }> = [
    { id: "cosmetic", label: "Cosmetic cleanup" },
    { id: "curves", label: "Curves" },
    { id: "stacking", label: "Stacking" },
  ];

  function toggleObjective(
    value: ProcessingObjectiveFromRust,
    checked: boolean,
  ): void {
    const has = processingObjectives.includes(value);
    if (checked && !has) {
      processingObjectives = [...processingObjectives, value];
    } else if (!checked && has) {
      processingObjectives = processingObjectives.filter(
        (v) => v !== value,
      );
    }
  }

  function setStageOverride(
    stageId: string,
    next: AiEnhancementLevelFromRust | null,
  ): void {
    stageOverrides = { ...stageOverrides, [stageId]: next };
  }

  function toggleStageEnabled(stageId: string, checked: boolean): void {
    stageEnabled = { ...stageEnabled, [stageId]: checked };
  }

  function toggleOptionalOp(stageId: string, checked: boolean): void {
    if (checked && !optionalOperations.includes(stageId)) {
      optionalOperations = [...optionalOperations, stageId];
    } else if (!checked) {
      optionalOperations = optionalOperations.filter((v) => v !== stageId);
    }
  }
</script>

<section class="beginner-tier" aria-label="Recipe editor: Beginner tier">
  <header class="tier-header">
    <span class="material-symbols-outlined" aria-hidden="true">tune</span>
    <h3 class="font-display">Beginner</h3>
    <p class="font-body tier-subtitle">
      Pick a name, target, style, and AI posture.
    </p>
  </header>

  <div class="field-row">
    <label class="field font-label" for="beginner-name">
      Recipe Name
      <input
        id="beginner-name"
        type="text"
        bind:value={name}
        {disabled}
        placeholder="e.g. M42-Natural-v1"
        data-testid="beginner-name-input"
      />
    </label>

    <label class="field font-label" for="beginner-target">
      Target Type
      <input
        id="beginner-target"
        type="text"
        bind:value={targetType}
        {disabled}
        placeholder="e.g. narrowband"
        data-testid="beginner-target-input"
      />
    </label>
  </div>

  <div class="field font-label">
    <span class="field-label">Processing Style</span>
    <QualityProfilePicker
      value={qualityProfile}
      disabled={disabled}
      onChange={(next) => (qualityProfile = next ?? DEFAULT_QUALITY_PROFILE)}
      label=""
    />
  </div>

  <fieldset class="ai-field" disabled={disabled}>
    <legend class="font-label field-label">AI Enhancement</legend>
    <div class="radio-grid" role="radiogroup" aria-label="AI Enhancement Level">
      {#each AI_OPTIONS as opt (opt.value)}
        <label class="radio-card" class:selected={aiEnhancementLevel === opt.value}>
          <input
            type="radio"
            name="ai-enhancement-level"
            value={opt.value}
            checked={aiEnhancementLevel === opt.value}
            onchange={() => (aiEnhancementLevel = opt.value)}
            data-testid="beginner-ai-radio-{opt.value}"
          />
          <span class="radio-label font-label">{opt.label}</span>
          <span class="radio-desc font-body">{opt.description}</span>
        </label>
      {/each}
    </div>
  </fieldset>
</section>

<section
  class="guided-tier"
  aria-label="Recipe editor: Guided tier"
  data-testid="guided-tier-section"
>
  <header class="tier-header">
    <button
      type="button"
      class="tier-toggle font-label"
      aria-expanded={guidedOpen}
      aria-controls="guided-tier-body"
      onclick={() => (guidedOpen = !guidedOpen)}
      data-testid="guided-tier-toggle"
    >
      <span class="material-symbols-outlined" aria-hidden="true">
        {guidedOpen ? "expand_less" : "expand_more"}
      </span>
      Guided
      <span class="tier-hint font-body">
        {guidedOpen ? "click to collapse" : "objectives + stages + targets"}
      </span>
    </button>
    <p class="font-body tier-subtitle">
      Add processing goals, stage inclusion, per-stage AI
      overrides, quality targets, and optional operations.
    </p>
  </header>

  {#if guidedOpen}
    <div id="guided-tier-body" class="guided-body">
      <!-- Processing objectives: 5-checkbox list -->
      <fieldset class="guided-field" disabled={disabled}>
        <legend class="font-label field-label">Processing Objectives</legend>
        <div class="checkbox-list" role="group" aria-label="Processing objectives">
          {#each OBJECTIVE_OPTIONS as opt (opt.value)}
            {@const checked = processingObjectives.includes(opt.value)}
            <label class="checkbox-row font-label" class:selected={checked}>
              <input
                type="checkbox"
                {checked}
                onchange={(event) =>
                  toggleObjective(opt.value, event.currentTarget.checked)}
                data-testid="guided-objective-{opt.value}"
              />
              {opt.label}
            </label>
          {/each}
        </div>
      </fieldset>

      <!-- Stage inclusion: per-stage enabled toggle -->
      <fieldset class="guided-field" disabled={disabled}>
        <legend class="font-label field-label">Stage Inclusion</legend>
        {#if stageIds.length === 0}
          <p class="empty-hint font-body">
            No stages yet. Add stages in the Expert tier (ProfileManager).
          </p>
        {:else}
          <table class="stage-table">
            <thead>
              <tr>
                <th class="font-label">Stage</th>
                <th class="font-label">Enabled</th>
              </tr>
            </thead>
            <tbody>
              {#each stageIds as stageId (stageId)}
                {@const enabled = stageEnabled[stageId] ?? true}
                <tr>
                  <td class="stage-name font-body">{stageId}</td>
                  <td>
                    <label class="switch font-label">
                      <input
                        type="checkbox"
                        checked={enabled}
                        onchange={(event) =>
                          toggleStageEnabled(stageId, event.currentTarget.checked)}
                        data-testid="guided-stage-enabled-{stageId}"
                      />
                      <span class="switch-track" aria-hidden="true"></span>
                    </label>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </fieldset>

      <!-- Per-stage AI override -->
      <fieldset class="guided-field" disabled={disabled}>
        <legend class="font-label field-label">Per-Stage AI Override</legend>
        {#if stageIds.length === 0}
          <p class="empty-hint font-body">
            Add stages first; per-stage overrides attach to
            existing stages.
          </p>
        {:else}
          <div class="stage-overrides" role="group" aria-label="Per-stage AI override">
            {#each stageIds as stageId (stageId)}
              {@const override = stageOverrides[stageId] ?? null}
              <div class="stage-override-row">
                <span class="stage-name font-label">{stageId}</span>
                <div class="radio-pill-row" role="radiogroup" aria-label="AI override for {stageId}">
                  <label class="radio-pill" class:selected={override === null}>
                    <input
                      type="radio"
                      name="override-{stageId}"
                      value="inherit"
                      checked={override === null}
                      onchange={() => setStageOverride(stageId, null)}
                      data-testid="guided-override-{stageId}-inherit"
                    />
                    <span class="font-label">Inherit</span>
                  </label>
                  {#each AI_OPTIONS as opt (opt.value)}
                    <label class="radio-pill" class:selected={override === opt.value}>
                      <input
                        type="radio"
                        name="override-{stageId}"
                        value={opt.value}
                        checked={override === opt.value}
                        onchange={() => setStageOverride(stageId, opt.value)}
                        data-testid="guided-override-{stageId}-{opt.value}"
                      />
                      <span class="font-label">{opt.label}</span>
                    </label>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </fieldset>

      <!-- Quality targets: 3 optional numeric inputs -->
      <fieldset class="guided-field" disabled={disabled}>
        <legend class="font-label field-label">Quality Targets</legend>
        <p class="field-hint font-body">
          All three targets are optional. Leave blank to
          skip the target. The §20 validation pipeline
          enforces the ranges (SNR 20-60 dB, sharpness
          0-1, smoothness 0-1).
        </p>
        <div class="field-row">
          <label class="field font-label" for="qt-snr">
            Target SNR (dB)
            <input
              id="qt-snr"
              type="number"
              step="0.1"
              min="20"
              max="60"
              value={qualityTargets.target_snr_db ?? ""}
              oninput={(e) =>
                (qualityTargets = {
                  ...qualityTargets,
                  target_snr_db:
                    e.currentTarget.value === ""
                      ? null
                      : Number(e.currentTarget.value),
                })}
              data-testid="guided-target-snr"
            />
          </label>
          <label class="field font-label" for="qt-shar">
            Target Sharpness (0-1)
            <input
              id="qt-shar"
              type="number"
              step="0.01"
              min="0"
              max="1"
              value={qualityTargets.target_sharpness ?? ""}
              oninput={(e) =>
                (qualityTargets = {
                  ...qualityTargets,
                  target_sharpness:
                    e.currentTarget.value === ""
                      ? null
                      : Number(e.currentTarget.value),
                })}
              data-testid="guided-target-sharpness"
            />
          </label>
          <label class="field font-label" for="qt-smooth">
            Target Background Smoothness (0-1)
            <input
              id="qt-smooth"
              type="number"
              step="0.01"
              min="0"
              max="1"
              value={qualityTargets.target_background_smoothness ?? ""}
              oninput={(e) =>
                (qualityTargets = {
                  ...qualityTargets,
                  target_background_smoothness:
                    e.currentTarget.value === ""
                      ? null
                      : Number(e.currentTarget.value),
                })}
              data-testid="guided-target-smoothness"
            />
          </label>
        </div>
      </fieldset>

      <!-- Optional operations: chip toggles -->
      <fieldset class="guided-field" disabled={disabled}>
        <legend class="font-label field-label">Optional Operations</legend>
        <div class="chip-row" role="group" aria-label="Optional operations">
          {#each OPTIONAL_OP_CHIPS as chip (chip.id)}
            {@const checked = optionalOperations.includes(chip.id)}
            <button
              type="button"
              class="chip font-label"
              class:selected={checked}
              aria-pressed={checked}
              onclick={() => toggleOptionalOp(chip.id, !checked)}
              data-testid="guided-op-{chip.id}"
            >
              <span class="material-symbols-outlined" aria-hidden="true">
                {checked ? "check_circle" : "radio_button_unchecked"}
              </span>
              {chip.label}
            </button>
          {/each}
        </div>
      </fieldset>
    </div>
  {/if}
</section>

<section class="tier-future" aria-label="Recipe editor: future tiers">
  <button
    type="button"
    class="tier-future-cta font-label"
    disabled
    title="Expert tier is the existing ProfileManager (stages table + per-stage params); wired in a follow-on slice"
    data-testid="beginner-expert-stub"
  >
    <span class="material-symbols-outlined" aria-hidden="true">code</span>
    Expert tier: see Profile Manager
  </button>
</section>

<style>
  .beginner-tier,
  .guided-tier,
  .tier-future {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
  }

  .guided-tier {
    margin-top: 0;
  }

  .tier-header {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--sp-sm);
  }

  .tier-header h3 {
    margin: 0;
    font-size: 1rem;
  }

  .tier-toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: transparent;
    border: 0;
    padding: 0;
    cursor: pointer;
    color: inherit;
    font-size: 1rem;
  }

  .tier-toggle:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
  }

  .tier-hint {
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    margin-left: var(--sp-xs);
  }

  .tier-subtitle {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
    flex: 1 1 100%;
  }

  .field-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-md);
  }

  @media (max-width: 640px) {
    .field-row {
      grid-template-columns: 1fr;
    }
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .field input {
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    padding: var(--sp-sm);
    font-size: 0.95rem;
    color: var(--on-surface);
  }

  .field-label {
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .ai-field,
  .guided-field {
    border: 0;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .field-hint {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.8rem;
  }

  .empty-hint {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
    font-style: italic;
  }

  .radio-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: var(--sp-sm);
  }

  .radio-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: var(--sp-sm);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    cursor: pointer;
    background: var(--surface-container);
    transition: border-color 0.1s ease;
  }

  .radio-card.selected {
    border-color: var(--primary);
    background: var(--primary-container);
  }

  .radio-card input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .radio-label {
    font-weight: 600;
    font-size: 0.9rem;
  }

  .radio-desc {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
  }

  .checkbox-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .checkbox-row {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: 6px var(--sp-sm);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    cursor: pointer;
    background: var(--surface-container);
    font-size: 0.9rem;
  }

  .checkbox-row.selected {
    border-color: var(--primary);
    background: var(--primary-container);
  }

  .stage-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .stage-table th,
  .stage-table td {
    padding: 6px var(--sp-sm);
    border-bottom: 1px solid var(--outline-variant);
    text-align: left;
  }

  .stage-name {
    font-family: var(--font-mono, monospace);
  }

  .switch {
    position: relative;
    display: inline-block;
    width: 36px;
    height: 20px;
    cursor: pointer;
  }

  .switch input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .switch-track {
    position: absolute;
    inset: 0;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: 999px;
    transition: background 0.1s ease;
  }

  .switch-track::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    background: var(--on-surface);
    border-radius: 50%;
    transition: transform 0.1s ease;
  }

  .switch input:checked + .switch-track {
    background: var(--primary);
  }

  .switch input:checked + .switch-track::after {
    transform: translateX(16px);
    background: var(--on-primary);
  }

  .stage-overrides {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .stage-override-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--sp-sm);
    align-items: center;
    padding: var(--sp-xs) var(--sp-sm);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    background: var(--surface-container);
  }

  .radio-pill-row {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .radio-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border: 1px solid var(--outline-variant);
    border-radius: 999px;
    cursor: pointer;
    font-size: 0.8rem;
    background: var(--surface-container);
  }

  .radio-pill.selected {
    border-color: var(--primary);
    background: var(--primary-container);
  }

  .radio-pill input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .chip-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-xs);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 6px 12px;
    border: 1px solid var(--outline-variant);
    border-radius: 999px;
    background: var(--surface-container);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .chip.selected {
    border-color: var(--primary);
    background: var(--primary-container);
  }

  .chip:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }

  .tier-future {
    flex-direction: row;
    flex-wrap: wrap;
    gap: var(--sp-sm);
  }

  .tier-future-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: var(--surface-container);
    border: 1px dashed var(--outline-variant);
    color: var(--on-surface-variant);
    border-radius: var(--radius-md);
    padding: var(--sp-xs) var(--sp-sm);
    cursor: not-allowed;
    font-size: 0.85rem;
  }
</style>