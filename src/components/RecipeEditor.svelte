<!--
  RecipeEditor: CR-08 §10 Beginner tier editor.

  The CR-08 §10 spec calls for a progressive-disclosure
  Recipe editor with three tiers (Beginner / Guided /
  Expert). This slice ships the Beginner tier surface:

  - Recipe Name (text input)
  - Target Type (text input, matches the existing
    ProfileManager semantics)
  - Processing Style (4-radio picker; binds to
    `quality_profile` via the existing
    `QualityProfilePicker`)
  - AI Enhancement (4-radio picker; binds to the
    new `ai_enhancement_level` field)

  Scope:
  - Pure UI surface: collects the four Beginner-tier
    fields and exposes them via `onChange(payload)`
    so the parent (ProfileManager, or the future
    RecipeEditor shell) can wire the change into
    `recipe_save`.
  - No persistence by itself. The Beginner tier
    payload is a TS DTO; the parent's recipe_save
    round-trip persists it.
  - Guided + Expert tiers land in follow-on slices
    (per the audit's sub-slice cadence); the
    component renders "Coming soon" disabled
    controls for them as honest affordances.

  This component is the Beginner-tier surface of a
  larger progressive-disclosure shell. It does not
  replace the existing ProfileManager (the Expert
  tier); it augments it with the Beginner tier
  surface per §10.
-->
<script lang="ts">
  import {
    DEFAULT_QUALITY_PROFILE,
    type AiEnhancementLevelFromRust,
    type QualityProfile,
  } from "../lib/astroforge-api";
  import QualityProfilePicker from "./QualityProfilePicker.svelte";
  import { untrack } from "svelte";

  /** Beginner-tier payload. The parent persists via
   * `recipe_save` (which round-trips through the
   * Recipe struct's `quality_profile` +
   * `ai_enhancement_level` fields). */
  export interface BeginnerTierPayload {
    name: string;
    targetType: string;
    qualityProfile: QualityProfile;
    aiEnhancementLevel: AiEnhancementLevelFromRust;
  }

  interface Props {
    /** Current values to seed the form (initial render). */
    initial: BeginnerTierPayload;
    /** Fired on every field change (debounced by parent
     * if the parent chooses). */
    onChange: (next: BeginnerTierPayload) => void;
    /** Optional disabled state (e.g. system Recipe
     * protection flips this on). */
    disabled?: boolean;
  }

  let { initial, onChange, disabled = false }: Props = $props();

  // §10 Beginner-tier local form state. The parent
  // owns the durable copy; the component owns the
  // in-flight edits so the parent's `recipe_save`
  // round-trip doesn't fight live keystrokes.
  // The component initializes from `initial` once
  // (the parent owns the open / close lifecycle);
  // subsequent changes to `initial` are intentionally
  // NOT reflected so the user's in-flight edits are
  // preserved across prop re-renders.
  // The `untrack(...)` wrappers tell Svelte not to
  // treat these references as reactive dependencies;
  // without them svelte-check warns that the
  // `$state(...)` initializer only captures the
  // initial value (which is exactly the intent here).
  let name = $state(untrack(() => initial.name));
  let targetType = $state(untrack(() => initial.targetType));
  let qualityProfile: QualityProfile = $state(
    untrack(() => initial.qualityProfile),
  );
  let aiEnhancementLevel: AiEnhancementLevelFromRust = $state(
    untrack(() => initial.aiEnhancementLevel),
  );

  // Fire `onChange` whenever any of the four fields
  // change. The parent's callback owns debouncing +
  // persistence; the component just emits every edit.
  $effect(() => {
    onChange({
      name,
      targetType,
      qualityProfile,
      aiEnhancementLevel,
    });
  });

  // Four-variant picker labels mirror the Rust
  // `AiEnhancementLevel::label()` exactly so the
  // Beginner tier's radio captions match the wire
  // format surfaced in the audit doc + pipeline_plan_hash
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
</script>

<section class="beginner-tier" aria-label="Beginner tier editor">
  <header class="tier-header">
    <span class="material-symbols-outlined" aria-hidden="true">tune</span>
    <h3 class="font-display">Beginner</h3>
    <p class="font-body tier-subtitle">
      Pick a name, target, style, and AI posture. The
      Guided and Expert tiers add finer-grained controls.
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
          <span class="radio-title font-label">{opt.label}</span>
          <span class="radio-desc font-body">{opt.description}</span>
        </label>
      {/each}
    </div>
  </fieldset>

  <!--
    Guided + Expert tier affordances: honest
    "Coming in a follow-on slice" disabled buttons
    so the user reads the shape of the larger
    progressive-disclosure shell rather than
    missing the controls. The Expert tier
    (constraints + ranges + execution + model
    selection) is the existing ProfileManager; the
    Guided tier (objectives + stage inclusion + AI
    preferences + quality targets) lands in a
    follow-on slice.
  -->
  <div class="tier-future">
    <button
      type="button"
      class="tier-future-cta font-label"
      disabled
      title="Guided tier lands in a follow-on slice (objectives + stage inclusion + AI preferences + quality targets)"
      data-testid="beginner-guided-stub"
    >
      <span class="material-symbols-outlined" aria-hidden="true">
        school
      </span>
      Guided tier: coming soon
    </button>
    <button
      type="button"
      class="tier-future-cta font-label"
      disabled
      title="Expert tier is the existing ProfileManager (stages table + per-stage params); wired in a follow-on slice"
      data-testid="beginner-expert-stub"
    >
      <span class="material-symbols-outlined" aria-hidden="true">
        code
      </span>
      Expert tier: see Profile Manager
    </button>
  </div>
</section>

<style>
  .beginner-tier {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
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

  .field-label {
    color: var(--on-surface);
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .field input[type="text"] {
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface);
    font-size: 0.95rem;
  }

  .field input[type="text"]:focus {
    border-color: var(--primary);
    outline: 2px solid var(--primary-container);
    outline-offset: 1px;
  }

  .field input[type="text"]:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .ai-field {
    border: 0;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .radio-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: var(--sp-sm);
  }

  .radio-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: border-color 0.12s ease, background 0.12s ease;
  }

  .radio-card:hover {
    border-color: var(--primary);
  }

  .radio-card.selected {
    border-color: var(--primary);
    background: var(--primary-container);
    color: var(--on-primary-container);
  }

  .radio-card input[type="radio"] {
    margin: 0;
    accent-color: var(--primary);
  }

  .radio-title {
    font-weight: 600;
    font-size: 0.9rem;
  }

  .radio-desc {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
    line-height: 1.4;
  }

  .radio-card.selected .radio-desc {
    color: var(--on-primary-container);
  }

  .tier-future {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-sm);
  }

  .tier-future-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    border: 1px dashed var(--outline-variant);
    border-radius: var(--radius-md);
    padding: var(--sp-sm) var(--sp-md);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .tier-future-cta:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
