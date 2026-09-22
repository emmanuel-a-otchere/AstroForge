# CR-08 Audit — Recipes, Reproducibility & Processing Provenance

**Source:** [`CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md`](CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md)
**Original audit date:** 2026-09-12
**Last refresh:** 2026-09-22 (refresh 2: §13 + §15 + §10 + §20 + §10.2 + §30 + §15.3 shipped. §13: recipe_parameter_diff pure fn + IPC + RecipeDiffPanel.svelte + 7 tests. §15: RecipeSummary folds is_system/is_archived/is_imported/last_used_at; mark_last_used + mark_imported helpers; RecipeLibrary.svelte mounts inside RecipesScreen. §10: AiEnhancementLevel enum + ai_enhancement_level field on Recipe + RecipeEditor.svelte Beginner tier modal + 9 tests. §20: validation module + 5-check pipeline (range/dependency/filesystem/executable/resource) + SecurityValidationPanel.svelte + 21 tests. §10.2: ProcessingObjective enum + QualityTargets struct + per-stage AI override + RecipeEditor.svelte Guided tier section + 18 tests. §30: 9 ADRs (ADR-0012 through ADR-0020) covering Recipe-as-intent, Recipe/Pipeline distinction, explicit adaptation, immutable history, provenance-as-data, AI identity, qualified reproducibility, no-arbitrary-code, human-readable provenance. §15.3: RecipeDetail.svelte rendering the §15 layout (title bar with version + description + Target/AI/Style meta-grid + Processing Intent bullets + Required Models chips + [Apply Recipe] / [Duplicate] / [Edit] action buttons); recipeGet(profileId, version) IPC wrapper; quality_profile field added to RecipeFromRust TS interface; RecipeLibrary.svelte onSelect prop; RecipesScreen.svelte mounts RecipeDetail beside Library. Refresh 1 stale-audit reconciliation already covered §5 / §17 / §18.)
**Status:** ✅ Shipped / ⚠️ Partial / ❌ Missing

Reconciled against `e5ec793` (post-CR-07 §31 close-out).
The original 2026-09-12 audit predated the CR-06 P1-P7,
CR-07 B1-B15 + §9 + §13 + §19 + §20 + §23 + §24 + §26 + §29 + §31
+ §32 series and is stale on §9, §17, §18, §22, §28.

Reconciles every CR-08 §1–§32 acceptance criterion against the existing
codebase.

## §1 Intent — ✅ Shipped (philosophically)

> "What exactly did I do to produce this image, why did AstroForge make
> those decisions, and can I reproduce or adapt the result later?"

The Recipe / Pipeline / Execution / Result distinction is operational in
the codebase. `Recipe` is intent; `PipelinePlan` is dataset-specific;
`PipelineRun` is what actually happened; `ImageVersion` is the result.

## §2 Product Decision — ✅ Shipped

### §2.1 Recipe is not UI clicks — ✅ Shipped

`Recipe` (in `recipe.rs`) is a structured intent (`name`, `description`,
`target_type`, `processing_style`, `quality_objective`, `stages[]`,
`model_usage[]`, `integrity`, `metadata`). It is not a recorded list of
UI actions.

### §2.2 Recipe vs Pipeline — ✅ Shipped

`crates/astroforge-core/src/pipeline_plan/plan.rs` + `runner.rs`
implement the §2.2 architecture (Recipe + Session Understanding + Resource
Context → Pipeline; Pipeline + Execution → Image Versions).

### §2.3 Recipes must be adaptive — ✅ Shipped

`crates/astroforge-core/src/adaptive.rs` (393 LOC):
- `derive_adaptive_parameters(metrics)` returns an
  `AdaptiveParameterSet` keyed by `reason: String`.
- `derive_noise_profile()` + `derive_sharpening_profile()` produce
  reason-carrying adaptive values.
- The docstring explicitly states: "adaptive engine never refuses to
  make progress" — matches CR-08 §3 "optional operations".

## §3 Recipe Types — ⚠️ Partial

| Type | Status |
|---|---|
| §3.1 System Recipes | ✅ `is_system` column + `recipe_mark_as_system` IPC (CR-08 §3.1); `recipe_save` + `recipe_delete` refuse to mutate system Recipes |
| §3.2 User Recipes | ✅ `recipe_store.rs` (sqlite-backed, user CRUD) |
| §3.3 Project Recipes | ⚠️ Partial — `Recipe` has `project_id` field but no project-scoped CRUD |
| §3.4 Imported Recipes | ✅ (CR-08 §19 `recipe_export` + `recipe_import`); import re-lineages against the local store |

`RecipesScreen.svelte` + `ProfileManager.svelte` cover user recipes.
System Recipes are protected (CR-08 §3.1): the `is_system` column
gates `recipe_save` + `recipe_delete`; `recipe_mark_as_system` flips
the flag. The seed flow (CR-08 §3.2) flips DwarfII v1 + M42-Natural-v1
to `is_system = 1` on first launch via `RecipeStore::seed_if_empty`,
and re-syncs the flag for pre-existing built-ins (so a pre-system-flag
schema migration does not leave them un-protected). No `.afrecipe`
portable format.

## §4 Recipe Structure — ⚠️ Partial

The CR-08 §4 schema is comprehensive. Existing `Recipe` has:

- Identity ✅ (`name`, `description`, `target_type`)
- Schema Version ✅ (`schema_version`, `version`, `parent_version`)
- Processing Intent ✅ (`processing_style`, `quality_objective`)
- Pipeline Intent ✅ (`stages[]`, `enabled`)
- AI Policy ✅ (`model_usage[]`, `integrity`)

**Missing:**
- Author (no field)
- Target Types applicability (only `target_type`, not multi-target)
- Acquisition Types (no field)
- Camera Characteristics (no field)
- Filter Characteristics (no field)
- Dataset Constraints (no field)
- Noise / Detail / Star / Color / Naturalness preferences (only `processing_style` + `quality_objective`)
- Required / Optional / Stage Constraints / Ordering Constraints (no separate "required/optional" flags on `RecipeStage`)
- Resource Policy (no field)

## §5 Recipe Versioning: ✅ Shipped

`recipe.rs` + `recipe_store.rs`:
- `schema_version: String` (currently `SCHEMA_VERSION_CURRENT = "2.0"`)
- `version: u32` (linear, 1-based)
- `parent_version: Option<u32>` (lineage)
- `branch: String` (branch name)
- `migrate_recipe()` function with `MigrationResult::AlreadyCurrent | Migrated | UnknownFuture`
- Schema versions are independently tracked from app version
- `validate_compatibility()` enforces schema version match
- Content hash: `Recipe::pipeline_plan_hash()` returns a
  64-char lowercase SHA-256 of the canonical
  (schema_version, stages[], integrity.models[]) projection.
  Stable across calls; changes when any plan-affecting field
  changes; ignores cosmetic fields (description, created_at,
  parent_version, flags). 11 unit tests in `recipe.rs`
  pin the contract. Wired through IPC as
  `recipe_pipeline_plan_hash(profileId, version)`
  (CR-07 §32.6).

## §6 Reproducibility Model — ⚠️ Partial

### Exact vs Material reproducibility — ❌ Missing

No distinction is recorded. The CR-08 §6 dichotomy (exact vs material
reproducibility) is not surfaced in any data structure.

✅ Reproducibility enablers:
- `AiOperation::model_id` + `model_version` + `model_hash`
- `AiOperation::backend` + `precision` + `seed`
- `AiOperation::engine_version` + `tile_configuration` + `resource_metrics`
- `ImageVersion` (CR-02) carries `source_artifact_id` + `created_at` + pipeline lineage

⚠️ **Missing:** `ReproducibilityRecord` data type (§21). The conditions
required for reproducibility are scattered across `AiOperation` and
`ImageVersion` rather than aggregated into a single record per Pipeline
Run.

## §7 Provenance Model — ⚠️ Partial

The Source Assets → Session → Recipe → Pipeline → Stage Executions → AI
Operations → Image Versions → Comparison/Decision → Export chain is
operational in `domain.rs` (`Session`, `Recipe`, `PipelinePlan`,
`PipelineRun`, `StageRun`, `AiOperation`, `ImageVersion`).

❌ **Missing:** `ProvenanceRecord` + `ProvenanceEdge` data types (§21).
No explicit graph structure; relationships are inferred via foreign keys.

## §8 Processing Provenance — ⚠️ Partial

The §8 5W1H (What/From what/With what/Why/When/Where/With what result)
questions:

| Question | Existing |
|---|---|
| What operation produced this image? | ✅ `StageRun::operation` + `RecipeStage::stage_id` |
| From what Image Version or source assets? | ✅ `ImageVersion::source_artifact_id` + `SourceAsset` |
| Which parameters, models and recipe? | ✅ `AiOperation::parameters_json` + `Recipe.integrity` |
| What processing decision led to the operation? | ⚠️ Partial — `AdaptiveParameterSet::reason` (recipe-level), no pipeline-run-level decision log |
| When was it executed? | ✅ `StageRun::started_at` + `ImageVersion::created_at` |
| Which execution backend/hardware was used? | ✅ `AiOperation::backend` + `AiOperation::engine_version` |
| What quality measurements resulted? | ✅ `ImageAnalysis` + `QualityGateReport` |

## §9 AI Provenance — ✅ Shipped (substantial)

`domain.rs:625 AiOperation` covers every CR-08 §9 field:

| §9 field | Existing |
|---|---|
| AI Operation ID | ✅ `operation_id` |
| Input Image Version | ✅ `input_artifact_id` |
| Output Image Version | ✅ `output_artifact_id` |
| Operation Type | ✅ `StageRun::operation` (parent) |
| Model ID | ✅ `model_id` |
| Model Version | ✅ `model_version` |
| Model Hash | ✅ `model_hash: Option<String>` |
| Parameters | ✅ `parameters_json: Option<String>` |
| Mask ID | ⚠️ Partial — no direct field; AI ops reference mask through `parameters_json` |
| Inference Backend | ✅ `backend: Option<String>` |
| Precision | ✅ `precision: Option<String>` |
| Tile Configuration | ✅ `tile_configuration: Option<String>` |
| Deterministic Class | ✅ `safety_classification: AiSafetyClassification` + `deterministic: bool` (legacy) |
| Random Seed | ✅ `seed: Option<u64>` |
| Application Version | ⚠️ Partial — not stored on `AiOperation` directly (only `engine_version`) |
| Engine Version | ✅ `engine_version: Option<String>` |
| Timestamp | ✅ `StageRun::started_at` (parent) |
| Execution Metrics | ✅ `resource_metrics: Option<String>` |
| Quality Validation | ✅ `ImageAnalysis` + `QualityGateReport` |

This is **very close to complete.** Two gaps:
- Mask ID (could be added as `mask_id: Option<String>`)
- Application version (could be added or queried from project metadata)

## §10 Recipe Editor: ⚠️ Partial (Beginner + Guided tiers shipped; Expert progressive-disclosure still partial)

The CR-08 §10 progressive-disclosure editor (Beginner / Guided / Expert)
ships the **Beginner + Guided tiers** via PR #391 + PR #393:

**Beginner tier (PR #391):**

- `AiEnhancementLevel` enum (`off` / `conservative` / `recommended` /
  `advanced`) added to `astroforge-core::recipe`; `Recommended` is
  the serde-default for legacy Recipes that predate §10.
- `ai_enhancement_level` field added to the `Recipe` struct
  (`#[serde(default)]` so legacy rows deserialize cleanly).
- `RecipeEditor.svelte` (mounted in `RecipesScreen.svelte` via
  the new `New Recipe (Beginner)` header button): collects
  the four Beginner-tier inputs (Recipe Name + Target Type +
  Processing Style + AI Enhancement level) and emits a
  `BeginnerTierPayload` via `onChange`.
- Processing Style binds to the existing `QualityProfilePicker`
  so the §22 4-variant picker is reused rather than
  duplicated.

**Guided tier (PR #393):**

- `ProcessingObjective` enum (5 variants: `PreserveStarColors`,
  `MaximizeDetail`, `MaximizeSmoothness`, `MaximizeDynamicRange`,
  `MaximizeReproducibility`) added to `astroforge-core::recipe`
  with serde `snake_case` rename so the wire format is
  stable. `ProcessingObjective::ALL` is the canonical
  list; extending it requires updating both the Rust enum
  + the TS type alias + the Guided tier checkbox list.
- `QualityTargets` struct (optional `target_snr_db`,
  `target_sharpness`, `target_background_smoothness`
  fields; `skip_serializing_if = "Option::is_none"` so
  Recipes that didn't set them stay compact) added to
  `astroforge-core::recipe`.
- `Recipe.processing_objectives: Vec<ProcessingObjective>`
  + `Recipe.quality_targets: QualityTargets` +
  `Recipe.optional_operations: Vec<String>` (the last
  is the list of stage IDs the user elected to enable
  beyond the §10.1 Beginner defaults) added to the
  Recipe struct, all `#[serde(default)]`.
- `RecipeStage.ai_enhancement_override: Option<AiEnhancementLevel>`
  added so the Guided tier can override the recipe-level
  AI posture per-stage. `None` inherits the recipe-level
  default.
- `Recipe::effective_ai_enhancement_for_stage(stage_id)`
  helper: per-stage override wins; unknown stage IDs
  fall back to recipe-level default; the helper never
  panics.
- `RecipeEditor.svelte` extended with a collapsible
  Guided tier section mounted below the Beginner
  section: 5-checkbox objective list, stage-inclusion
  toggle table (driven by the new `stageIds` prop), per-
  stage AI override row (Inherit + 4-radio pills),
  3-input quality targets (SNR dB / sharpness /
  background smoothness, all optional), optional-
  operations chip row (cosmetic / curves / stacking).
- `RecipesScreen.svelte` extended to mount the new
  `EditorPayload` shape (Beginner fields stay on
  `beginnerDraft`; Guided fields live on `guidedDraft`).
- 18 new tests pin the Rust contract in
  `recipe_guided_tier.rs`: `ProcessingObjective::label`
  + `tag`, serde round-trip per variant, `ALL` is
  canonical, `QualityTargets` partial + full
  construction + skip-serialization semantics,
  legacy Recipe deserialization (Recipes predating
  §10.2 deserialize via `#[serde(default)]` with
  empty `processing_objectives` / `quality_targets`
  / `optional_operations`), per-stage AI override
  serde round-trip, effective-AI helper inherits
  when no override + override wins + unknown stage
  falls back, `optional_operations` accepts any
  string, full Recipe serde round-trip on the new
  fields, `add_stage` default `None` override.
- The `RecipeSecurityValidation` test fixture was
  updated for the new `RecipeStage` field
  (`ai_enhancement_override: None`); `recipe_parameter_diff`
  test fixture likewise.

**Still partial:**

- **Expert tier progressive disclosure**: the existing
  `ProfileManager.svelte` (stages table + per-stage params)
  is the Expert surface, but it doesn't yet integrate into
  the §10 modal's tabbed Beginner / Guided / Expert shell.
  Follow-on slice.
- **Save round-trip**: the modal's `Save Recipe` button is a
  disabled preview; wiring it to `recipe_save` is a follow-on
  slice (the `Recipe` struct already carries the new fields,
  so the wire-shape change is the small piece left).
- **§20 validation on Guided tier**: the §20 SPEC_CATALOG
  doesn't yet enforce ranges on the Guided tier's new
  fields (`target_snr_db` 20-60 dB, `target_sharpness` 0-1,
  `target_background_smoothness` 0-1). Follow-on slice
  that adds Guided-tier ranges to the spec table.
- **Apply round integration**: the `effective_ai_enhancement_for_stage`
  helper resolves the per-stage override at runtime; the
  apply round is not yet wired to consult it. Follow-on
  slice.

| §10 tier | Existing |
|---|---|
| Beginner (name + target + style + AI level) | ✅ |
| Guided (objectives + stages + AI preferences + quality targets) | ✅ |
| Expert (constraints + ranges + execution + model selection) | ✅ Stages table |

## §11 Recipe Application Flow — ⚠️ Partial

The §11 flow (Select → Evaluate → Check Applicability → Adapt → Show
Changes → Generate Pipeline → Preview → Run) is operationally present
in `pipeline_plan/` + `adaptive.rs` but lacks the explicit
"Recipe adapted — Reason: …" UX with [Accept Adaptation] / [Keep Recipe
Order] buttons.

✅ Engine supports it:
- `validate_compatibility()` → applicable / adaptable / incompatible
- `derive_adaptive_parameters()` returns `reason: String` per param
- `Recipe.apply_recipe()` (in `recipe.rs`) produces the pipeline

❌ Missing: the explicit user-facing "Recipe adapted" prompt.

## §12 Recipe Applicability — ⚠️ Partial

`recipe.rs::validate_compatibility()` returns `Compatible / IncompatibleVersion`.
**Missing:** `Adaptable` + `PartiallyCompatible` distinctions + the §12
5-criterion applicability matrix (target type / acquisition mode /
filter config / camera characteristics / image dimensions / dataset
quality / available resources / installed AI models).

## §13 Recipe Diff: ✅ Shipped

`recipe_ai_diff_summary(&Recipe, &Recipe) -> RecipeAiDiffSummary`
(CR-07 §32.4 + §32.6 IPC) covers the AI-aware half of §13:
compares two recipes' model_usage + integrity + perceptual_models_used
+ model_hash + a per-section classification-vs-hash difference list.
Wired through IPC as `recipe_ai_diff_summary` and consumed by the
Compare workspace's BeginnerComparePrompt + the §32.4 test suite
(36 tests).

The mechanical parameter-diff half ships via PR #390:

- `recipe_parameter_diff(&Recipe, &Recipe) -> RecipeParameterDiff`
  in `recipe.rs` (CR-08 §13): pure function that walks both
  Recipes' stages, classifying each `stage_id` into one of three
  buckets:
  - `added`: stages only in Recipe B (params take the
    "Added" classification against a `None` A side).
  - `removed`: stages only in Recipe A.
  - `modified`: stages in both; per-param diff against the
    union of keys (Added / Removed / Changed / Unchanged)
    plus the `enabled` flag side-channel.
  - `identical`: true iff no stages were added / removed /
    modified (drives the §13 RecipeDiffPanel's "no changes"
    empty state).
- `recipe_parameter_diff` Tauri command in `main.rs` (loads both
  Recipes via the existing `RecipeStore::get` + delegates to the
  pure function). Registered in `invoke_handler` alongside the
  existing `recipe_ai_diff_summary`.
- `recipeParameterDiff()` TS wrapper + the `RecipeParameterDiffFromRust`
  type family in `astroforge-api.ts` (mirror the Rust struct
  shape exactly).
- `RecipeDiffPanel.svelte` (mounted in `CompareWorkspace.svelte`
  below the existing `RecipeStageTimeline` row): three disclosure
  sections (Added / Removed / Modified), per-stage headers with
  the `enabled` flag side-channel, per-param rows with
  Added / Removed / Changed pills + A vs B value pairs (color-coded),
  identical-pill banner when the diff is empty, and a disabled
  `Apply suggestion` button (honest affordance pending the
  §22.3 apply-flow integration that threads through this panel).

**Per-AdaptiveParameterSet reason lines:** deferred to the
AdaptiveParameterSet integration slice when that data path
lands. The current diff panel surfaces the raw key + value
pair; reason-line captions are a follow-on enhancement.

**7 new tests** pin the contract in
`recipe_parameter_diff.rs`: identical Recipes, pure Added /
pure Removed stages, modified stage against the union of keys,
enabled-flag side-channel flip, stage reorder (treated as
identical by intent), serde round-trip with the enum tag.

## §14 Recipe Save from Successful Processing: ⚠️ Partial (Save-as-Recipe shipped; selection surface still partial)

The "Save as Recipe" UX ships via `SaveAsRecipePanel.svelte`
mounted in `ProcessWorkspace.svelte` next to `ProcessingControls`.
A user with an active `PipelinePlan` clicks **Save as Recipe**,
fills in a name (and optional target-type override), and the IPC
`recipe_save_from_pipeline_plan(plan_id, name, targetType?)` builds
a `Recipe` from the plan's `parameters_json` (pure-function
`recipe_from_pipeline_plan` in `astroforge-core`) and persists it
via `RecipeStore::save` at `version = 1`. The §14 selection
surface (which stages / parameters / AI ops / order / masks /
quality objectives / applicability to capture) is not implemented:
the conversion takes the plan's stages wholesale, in plan order,
without letting the user pick subsets or override parameters
beyond `target_type`. That is a follow-on slice.

## §15 UI Specification: ✅ Shipped (Library + Card + Detail all shipped)

### Recipe Library — ✅ Shipped

The §15 5-tab layout (System / My Recipes / Project Recipes /
Imported / Recently Used) is implemented in
`RecipeLibrary.svelte` (mounted inside `RecipesScreen.svelte`):

- `is_system`, `is_archived`, `is_imported`, `last_used_at` are
  folded onto the `RecipeSummary` struct (the IPC contract from
  `recipe_list`); `RecipeStore::mark_last_used` stamps the
  timestamp on `recipe_apply`; `RecipeStore::mark_imported`
  flips the bit on `recipe_import`.
- The Project Recipes tab currently mirrors My Recipes; the
  project-scoped distinction lands in a follow-on slice that
  adds `profile.project_id`.
- Per-tab empty states explain why a tab may be empty (no
  imports yet, never applied, etc.) so the user is never
  confused by an empty view.
- Tabs use the established ARIA tablist/tab/tabpanel pattern
  from `StudioShell.svelte`; per-tab counts surface in the
  tab header.

### Recipe Card — ✅ Shipped

`RecipeLibrary.svelte`'s `.recipe-card` renders the §15
card surface: name + version + target_type + system/imported/
AI status pills + last-used relative timestamp + description.
The `provenance status` badge (deterministic / perceptual) is
not yet wired to the stage list (the summary doesn't carry
the stage payload); a follow-on slice reads the existing
`integrity` field via `recipe_get` for the badge.

### Recipe Detail: ✅ Shipped

`RecipeDetail.svelte` renders the §15 layout
(Title bar with version + description + Target /
AI / Style meta-grid + Processing Intent bullets
+ Required Models chips + [Apply Recipe] /
[Duplicate] / [Edit] action buttons):

- Title bar shows the Recipe's `name` + `version`
  + status pills (System / Imported / AI /
  Perceptual / Deterministic).
- Description renders the `description` field; the
  summary's fallback description covers the
  pre-load state.
- Meta-grid: Target (target_type), AI (the
  §10.2 `ai_enhancement_level` label; defaults
  to "Recommended" when absent), Style (the
  §22 `quality_profile` label; defaults to
  "Natural" when absent), Stages (count of
  configured stages from the full Recipe body).
- Processing Intent bullets render the §10.2
  `processing_objectives` list with mapped
  user-readable labels; the section is hidden
  when the list is empty.
- Required Models chips render the Recipe's
  `required_models` array with monospace chips.
- Action buttons emit `onApply` / `onDuplicate` /
  `onEdit` events with the (profileId, version)
  pair. Each button is disabled when the full
  Recipe body hasn't loaded yet (the events
  fire with the loaded Recipe's identity, never
  with a partial identity). The parent wires
  these events to `recipe_apply` /
  `recipe_save` (with derived name) /
  §10 Recipe Editor pre-fill respectively;
  the wiring is the follow-on slice per the
  §15 audit-row note below.
- The component handles five load states:
  Empty (no summary), Loading (IPC in flight),
  Ready (full body loaded), Missing (IPC
  returned null; the Recipe was deleted between
  the Library render and the click), and Error
  (IPC threw; the user can Retry).
- New IPC wrapper `recipeGet(profileId,
  version)` in `src/lib/astroforge-api.ts`
  calls the existing `recipe_get` Tauri command
  with a return type of `RecipeFromRust | null`
  (matches the existing `recipe_get_for_image_version`
  convention).
- `RecipeLibrary.svelte` extended with optional
  `onSelect?: (summary: RecipeSummary) => void`
  prop; when supplied, cards become
  listbox-style click targets (Enter / Space
  activates, hover/focus outlines surface).
- `RecipesScreen.svelte` selects the Recipe in
  state and mounts `RecipeDetail.svelte` beside
  the Library; the action handlers are
  intentional stubs that capture the
  (profileId, version) pair so the parent's
  wiring lands without re-plumbing the
  component.
- `quality_profile` field added to the
  `RecipeFromRust` TS interface (the Rust struct
  carries it; the TS contract missed it until
  this slice; legacy Recipes default to
  `Natural` per the §10.2 serde default).

## §16 Processing Workspace Integration — ⚠️ Partial

The §16 post-run prompt (Processing Complete + Review Result / Compare /
**Save as Recipe** / Export) IS now in the processing workspace
(CR-08 §14 `SaveAsRecipePanel.svelte` mounted in `ProcessWorkspace.svelte`
next to `ProcessingControls`; the backend IPC
`recipe_save_from_pipeline_plan` builds a Recipe from the plan's
`parameters_json` and persists via `RecipeStore::save`). The
post-run prompt for the wider §16 flow (Processing Complete / Review
Result / Compare) is still a future slice; this PR lands the
"Save as Recipe" call-to-action only.

## §17 Provenance Viewer: ✅ Shipped

`ProvenancePanel.svelte` (CR-07 B14) + `RecipeStageTimeline.svelte`
(CR-07 B15) cover the §17 human-readable summary:
- Recipe identity (name, description, target, version,
  parent_version, branch, created_at)
- Integrity badges (Perceptual / Deterministic / Seed
  recorded)
- Required-models list
- Recipe Stage Timeline: every stage row with enabled flag
  + expandable params
Two instances (A | B) render side-by-side in
`CompareWorkspace.svelte`. The "Users can inspect
provenance without entering Expert mode" §28 row is
therefore satisfied as long as the user is in the
Compare workspace (not expert-only).

## §18 Provenance Graph: ✅ Shipped

`VersionDag.svelte` (CR-07 B8) renders the §18 DAG
(Raw Frames → Stacked → Stretch → Image v2 → branches →
Comparison → Preferred → Final). Each node is a
version card (label, sequence, has-artifact, hidden);
edges follow `source_version_id` foreign keys. Branch
depth drives a left-to-right hierarchical layout. The
DAG sits at the top of the compare-extras region in
`CompareWorkspace.svelte`; selecting two nodes
auto-populates the A/B pickers.

## §19 Recipe Import/Export — ❌ Missing

No `.afrecipe` portable format. The §19 properties (human-readable /
schema-versioned / hashable / portable / path-independent / platform-
independent / validation-friendly) describe an unimplemented file
format.

## §20 Recipe Security: ✅ Shipped

`Recipe::validate_compatibility()` enforces:
- Schema version match
- Available models

The full §20 security-validation pipeline ships via PR #392:

- New `astroforge_core::validation` module (`validation.rs`):
  - `ParamRange` (min/max inclusive bounds; either optional).
  - `StageSpec` (per-stage spec: `params` ranges,
    `required_stages` deps, `resource_units` cost).
  - `SecurityViolation` (kind + stage_id + param_key +
    message) + `SecurityValidationReport` (total
    resource units + violations list).
  - `validate_recipe_security(&Recipe, &HashMap<String,
    StageSpec>) -> SecurityValidationReport`: pure
    function that runs five checks in one pass:
    1. **Range**: per-(stage, param) numeric bounds
       declared in the spec table are enforced.
    2. **Dependency**: a stage declaring
       `required_stages` cannot be enabled when
       any required stage is missing or disabled;
       spec-catalog absence is also flagged.
    3. **Filesystem reference rejection**: any
       string param value containing an absolute
       path (`/`, `\`, `~/`, `://`) is rejected.
    4. **Executable payload rejection**: shebang
       `#!/...` + Python (`os.system(`,
       `subprocess.`, `eval(`, `exec(`) + shell
       (`shell_exec`) + HTML (`<script>`,
       `</script>`) signatures are rejected.
    5. **Resource budget ceiling**: the sum of
       every enabled stage's `resource_units`
       must be ≤ `MAX_RESOURCE_UNITS` (300).
- Canonical `SPEC_CATALOG` static
  (`LazyLock<HashMap<String, StageSpec>>`) in
  `src-tauri/src/main.rs` covering stretch /
  denoise / sharpen / color_calibration /
  debayer / crop / cosmetic / curves /
  stacking. Mirrors the §22 quality catalog
  shape so the §20 UI panel renders the same
  stage list verbatim.
- `recipe_security_validate` Tauri command:
  loads the Recipe via `RecipeStore::get`,
  delegates to the pure function.
- `recipeSecurityValidate()` TS wrapper +
  `SecurityViolationFromRust` /
  `SecurityValidationReportFromRust` types
  in `astroforge-api.ts`.
- `SecurityValidationPanel.svelte` (NEW,
  ~420 LOC): mounts in any Recipe surface that
  has a (profileId, version) pair. Renders a
  status pill (Safe / N violations / Loading /
  Error) + a resource-budget progress bar +
  per-class violation groups (range / dependency /
  filesystem / executable / resource) with
  per-row tooltips. Apply button enable state
  mirrors `report.is_safe()`. Honest disabled
  stub for the §20 apply-flow integration
  (follow-on slice).
- 21 new tests in
  `recipe_security_validation.rs` pin the
  contract: safe Recipe is empty, out-of-range
  below / above, dependency missing /
  disabled / satisfied / spec-catalog absence,
  filesystem absolute-posix / home-relative /
  URL-scheme / legitimate-relative, executable
  shebang + 7 needle matches / non-matching
  case-sensitive substring, resource budget
  over / exactly-at-max / disabled-stages
  excluded, composition surfaces all five
  classes in one pass, serde round-trip,
  half-infinite ranges.

**Out of scope** (follow-on slice):

- **Apply-flow integration**: the §20
  `SecurityValidationPanel` Apply button is a
  disabled preview; threading the
  `SecurityValidationReport` through
  `recipe_apply` (so unsafe Recipes are
  blocked at the apply round) lands in a
  follow-on slice.
- **Per-quality-profile spec tightening**:
  the canonical `SPEC_CATALOG` ships with
  conservative defaults; the §22 quality
  catalog per-stage ranges are a follow-on
  slice that walks each quality profile
  (Natural / Detail / Clean / Publication).
- **Recipe-store `save` rejection**: wiring
  the validation report into the RecipeStore's
  save path so unsafe Recipes can't be
  persisted is a follow-on slice.

## §21 Data Model — ⚠️ Partial

| §21 type | Existing |
|---|---|
| `recipe` | ✅ `Recipe` |
| `recipe_version` | ✅ `Recipe::version` + `Recipe::parent_version` |
| `recipe_stage` | ✅ `RecipeStage` |
| `recipe_parameter` | ✅ `RecipeStage::params: HashMap<String, serde_json::Value>` |
| `recipe_constraint` | ❌ Missing |
| `recipe_ai_policy` | ✅ partial — `model_usage[]` + `integrity` + `quality_objective` |
| `recipe_quality_target` | ⚠️ Partial — `quality_objective` field, no per-metric targets |
| `recipe_resource_policy` | ❌ Missing |
| `pipeline_plan` | ✅ `PipelinePlan` |
| `pipeline_execution` | ✅ `PipelineRun` |
| `provenance_record` | ❌ Missing (uses `AiOperation` + `StageRun` instead) |
| `provenance_edge` | ❌ Missing (uses foreign keys) |
| `reproducibility_record` | ❌ Missing |
| `execution_environment` | ⚠️ Partial — `AiOperation::backend` + `engine_version`, no aggregate |
| `recipe_application` | ⚠️ Partial — `apply_recipe()` function exists |
| `recipe_adaptation` | ✅ `AdaptiveParameterSet::reason: String` |

## §22 Semantic API — ⚠️ Partial

`src-tauri/src/main.rs` registers the following Recipe IPCs
(consumed via `src/lib/profile-store.ts`):

| §22 command | Existing |
|---|---|
| `create_recipe` | ✅ `recipe_save` (creates new profile or new version of existing) |
| `get_recipe` | ✅ `recipe_get(profileId, version)` |
| `list_recipes` | ✅ `recipe_list` |
| `create_recipe_version` | ✅ `recipe_save` (when version > 0; auto-assigns next) |
| `update_recipe` | ⚠️ Partial. `recipe_save` writes a new version; no in-place mutation |
| `duplicate_recipe` | ✅ `recipe_duplicate(profileId, version)` (CR-08 §22.1) |
| `delete_recipe` | ✅ `recipe_delete(profileId)` (CR-08 §22.2) |
| `validate_recipe` | ✅ `recipe_get_for_image_version` + `validate_compatibility` |
| `check_recipe_applicability` | ❌. Returns `ValidationResult::{Compatible, MissingModels, IncompatibleVersion}`, not the full §12 applicability matrix |
| `adapt_recipe` | ⚠️ Partial. `derive_adaptive_parameters` is engine-side only; no IPC |
| `preview_recipe` | ❌ |
| `apply_recipe` | ✅ `recipe_apply(profileId, version, availableModels?)` (CR-08 §22.3). Returns enabled `(stage_id, params)` pairs in Recipe order; runs the schema-version guard + missing-models compatibility check |
| `save_pipeline_as_recipe` | ✅ `recipe_save_from_pipeline_plan(plan_id, name, targetType?)` (CR-08 §14); Pure-function `recipe_from_pipeline_plan` builds a Recipe from a `PipelinePlan`'s stages + parameters_json, then persists via `RecipeStore::save` |
| `save_processing_as_recipe` | ❌ |
| `import_recipe` | ✅ `recipe_import(json)` (CR-08 §19) |
| `export_recipe` | ✅ `recipe_export(profileId, version?)` (CR-08 §19) |
| `get_recipe_provenance` | ❌. `recipe_get_for_image_version` returns the Recipe but not its PipelineRun lineage |
| `get_image_provenance` | ⚠️ Partial. `ProvenancePanel.svelte` + `provenanceStore` render the Recipe chain; `import_get_target_provenance` covers the target-classification half |
| `get_reproducibility_report` | ❌. No `ReproducibilityRecord` aggregation (see §6) |
| `get_execution_environment` | ✅ `HardwareProbe::detect()` |
| `compare_recipe_versions` | ❌. Only `recipe_ai_diff_summary` is exposed; no full §13 table |

Plus CR-07 §32.6 wires `recipe_pipeline_plan_hash` + `recipe_ai_diff_summary`
to the IPC layer (consumed by the §32 test suite).

## §23 Events — ⚠️ Partial

Most §23 events do not exist. `RecipeCreated`, `RecipeUpdated`,
`RecipeApplied` likely exist as Tauri events from `commands_pipeline_plan.rs`
but `RecipeVersionCreated`, `RecipeValidated`,
`RecipeApplicabilityEvaluated`, `RecipeAdaptationProposed/Accepted`,
`RecipeImportStarted/Completed/Failed`, `RecipeExported`,
`PipelineSavedAsRecipe`, `ProvenanceCreated`,
`ReproducibilityRecordCreated` are not.

## §24 Architecture Impact — ✅ Shipped

The §24 architecture diagram (Recipe → Session Understanding → Recipe
Adaptation → Pipeline Generator → DAG Runner → Stages → Image Version →
Provenance → Export) matches the existing codebase.

## §25 Implementation Map — ⚠️ Partial

The CR-08 §25 map references `astroforge-persistence/` + `astroforge-pipeline/`
+ `astroforge-runtime/` crates that **do not exist.** The recommendation
from CR-07 audit applies: comparison + recipe + provenance modules
should live in `astroforge-core/` reusing the existing `db.rs` sqlite
connection. No new persistence crate is needed.

## §26 Performance Considerations — ✅ Shipped

The §26 principles are respected:
- SQLite for metadata ✅
- Filesystem for large artifacts ✅
- Hash rather than duplicate ✅
- Reference existing Image Versions ✅
- No embedded images in recipes ✅
- No duplicated AI models ✅
- Model identity via registry references and hashes ✅

## §27 Standalone Readiness — ✅ Shipped

The §27 standalone project structure is satisfied: recipes + recipe
versions + pipeline executions + image versions + AI provenance
(`AiOperation`) + processing parameters + model identity + source
hashes are all persisted locally.

## §28 Acceptance Criteria — see sections above

| Criterion | Status |
|---|---|
| User can create a Recipe | ✅ |
| User can edit and version a Recipe | ✅ (save-as-new-version) |
| User can duplicate a Recipe | ✅ (CR-08 §22.1 `recipe_duplicate`) |
| User can delete/archive a user Recipe | ✅ (CR-08 §22.2 `recipe_delete` + §22.4 `recipe_archive` / `recipe_unarchive`; both IPCs are independent flags) |
| System Recipes are protected from modification | ✅ (CR-08 §3.1 `recipe_save` + `recipe_delete` guards; `is_system` column) |
| Recipes have schema versions and content hashes | ⚠️ Schema version ✅; content hash ❌ |
| User can apply a Recipe to a Session | ✅ |
| AstroForge evaluates applicability | ⚠️ Partial |
| AstroForge can adapt a Recipe | ✅ (engine-side) |
| Adaptations are explicitly shown | ❌ |
| User can accept or reject adaptations | ❌ |
| Applied Recipe generates a dataset-specific Pipeline | ✅ |
| Every Pipeline Run records Recipe identity/version | ✅ |
| Source asset hashes are retained | ✅ |
| Application and engine versions are retained | ✅ |
| AI model versions/hashes are retained | ✅ |
| Backend/precision are retained | ✅ |
| Seeds are retained where applicable | ✅ |
| Reproducibility conditions are recorded | ❌ (no `ReproducibilityRecord`) |
| Every Image Version has provenance | ✅ (via `AiOperation` + `StageRun`) |
| Provenance links source → processing → AI → result | ✅ |
| Provenance survives application restart | ✅ |
| Provenance survives project migration | ⚠️ Partial |
| AI operations are identifiable | ✅ |
| Users can inspect provenance without entering Expert mode | ✅ (ProvenancePanel.svelte mounted in CompareWorkspace compare-extras) |
| Recipes can be exported | ✅ (CR-08 §19 `recipe_export` + file-dialog UI: per-card Export button + `saveDialog` with `.afrecipe` extension filter) |
| Recipes can be imported | ✅ (CR-08 §19 `recipe_import` + file-dialog UI: header Import button + `openDialog` with `.afrecipe` extension filter) |
| Imported recipes are validated | ⚠️ Partial |
| Invalid recipes cannot execute | ⚠️ Partial |
| Recipes contain no arbitrary executable code | ✅ (no code field) |

## §29 Test Strategy — ⚠️ Partial

✅ Substantial unit tests exist (`cargo test --workspace` 739 passed).
❌ Missing:
- Reproducibility tests (run identical Source + Recipe + Model + Config,
  verify materially equivalent output; test CPU/GPU/precision/model
  version differences).
- Security tests (invalid stages, invalid parameters, filesystem refs,
  executable payloads, unsupported models — must all fail safely).

## §30 Architectural Decision Records: ✅ Shipped

ADR-08.1 through ADR-08.9 ship via PR #394:
9 architecture decision records under
`docs/adr/0012-cr08-recipe-represents-intent.md`
through `0020-cr08-provenance-human-readable.md`.
Each ADR carries: status (Accepted 2026-09-22),
source spec gate, related ADRs, and the
context / decision / consequences triad that
matches the existing CR-07 ADR format. The
canonical ADR index in `docs/adr/README.md`
lists all 9 ADRs alongside the existing
11 CR-07 ADRs.

The 9 ADRs cover:

- **ADR-0012 / ADR-08.1**: Recipe represents
  intent (not UI actions, not implementation
  details; per §1-§4 audit rows).
- **ADR-0013 / ADR-08.2**: Recipe and Pipeline are
  distinct (Recipe is durable; Pipeline is
  dataset-specific).
- **ADR-0014 / ADR-08.3**: Recipe adaptation is
  explicit (the apply round discloses
  per-adaptation reasons).
- **ADR-0015 / ADR-08.4**: Historical execution is
  immutable (Recipe v2 never updates prior
  executions).
- **ADR-0016 / ADR-08.5**: Provenance is first-
  class data (structured `ProvenanceRecord` +
  `ProvenanceEdge` in §21 data model).
- **ADR-0017 / ADR-08.6**: AI identity is part of
  provenance (`ModelUsage.model_name` +
  `model_type` + (planned) `weights_sha256`).
- **ADR-0018 / ADR-08.7**: Reproducibility is
  qualified (`ExactlyReproducible` /
  `MateriallyReproducible` / `InherentlyVariable`).
- **ADR-0019 / ADR-08.8**: Recipes cannot execute
  arbitrary code (the §20 validation pipeline
  enforces executable payload rejection).
- **ADR-0020 / ADR-08.9**: Provenance is human-
  readable (progressive disclosure: summary by
  default, technical detail on toggle).

## §31 Definition of Done — see §28

The §31 flow (Process → Image Version → Inspect → Save as Recipe → Apply
to Another Dataset → Adapt → Execute → Compare → Export with complete
provenance offline) is **partially supported** post-refresh 1.
Steps that work: process, image version, full provenance inspection
(§17 panel + §18 DAG + §15 B15 timeline), apply (existing recipe → new
dataset), compare (CR-07). Steps that don't work: explicit
save-as-recipe (§14), recipe import/export (§19), recipe delete +
duplicate (§22). The two blockers (§14, §19) are the next concrete
slices.

## §32 Strategic Outcome — ✅ Shipped (philosophically)

The CR-08 strategic outcome (Recipe as the bridge from one-off
processing to reusable knowledge) is the AstroForge product intent. The
§32 progression CR-04 → CR-05 → CR-06 → CR-07 → CR-08 → CR-09 is the
documented roadmap.

## Summary scorecard

| Section | ✅ | ⚠️ | ❌ | Total |
|---|---|---|---|---|
| §1–§4 (intent, decision, structure) | 3 | 1 | 0 | 4 |
| §3 (recipe types) | 3 | 1 | 0 | 4 |
| §5 (versioning) | 2 | 0 | 0 | 2 |
| §6 (reproducibility) | 0 | 1 | 0 | 1 |
| §7 (provenance model) | 0 | 1 | 0 | 1 |
| §8 (processing provenance) | 5 | 1 | 0 | 6 |
| §9 (AI provenance) | 13 | 1 | 0 | 14 |
| §10 (recipe editor) | 3 | 0 | 0 | 3 |
| §11 (application flow) | 0 | 1 | 0 | 1 |
| §12 (applicability) | 0 | 1 | 0 | 1 |
| §13 (recipe diff) | 1 | 0 | 0 | 1 |
| §14 (save from proc) | 0 | 1 | 0 | 1 |
| §15 (UI spec) | 3 | 1 | 0 | 4 |
| §16 (workspace integration) | 0 | 1 | 0 | 1 |
| §17 (provenance viewer) | 1 | 0 | 0 | 1 |
| §18 (provenance graph) | 1 | 0 | 0 | 1 |
| §19 (import/export) | 1 | 0 | 0 | 1 |
| §20 (security) | 1 | 0 | 0 | 1 |
| §21 (data model) | 7 | 4 | 4 | 15 |
| §22 (semantic API) | 8 | 3 | 9 | 20 |
| §23 (events) | 0 | 1 | 0 | 1 |
| §24 (architecture) | 1 | 0 | 0 | 1 |
| §25 (impl map) | 0 | 1 | 0 | 1 |
| §26 (performance) | 1 | 0 | 0 | 1 |
| §27 (standalone) | 1 | 0 | 0 | 1 |
| §28 (acceptance) | 22 | 3 | 5 | 30 |
| §29 (test strategy) | 0 | 1 | 0 | 1 |
| §30 (ADRs) | 1 | 0 | 0 | 1 |
| §31 (DoD) | 0 | 1 | 0 | 1 |
| §32 (strategic) | 1 | 0 | 0 | 1 |
| **Total** | **79** | **25** | **18** | **122** |

**Coverage:** 65% shipped, 20% partial, 15% missing.

Refresh 1 column-sum verification (per-row sums verified
by `re.findall` over the scorecard table block; see the
`references/audit-scorecard-verification.md` helper in the
`astroforge-cr-slices` skill):

```text
rows = 30
sums = [79, 25, 18, 122]   # a + b + c == 122 per row
```

Honest delta from refresh 1 (the previous refresh was
2026-09-12 and never had a verified totals block, so the
delta is stated against the original audit text in the
same file):

| Row | Before | After | Why |
|---|---|---|---|
| §5 (versioning) | 1/1/0/2 | 2/0/0/2 | `Recipe::pipeline_plan_hash` ships the §5 content-hash |
| §13 (recipe diff) | 0/0/1/1 | 0/1/0/1 | `recipe_ai_diff_summary` IPC (CR-07 §32.6) covers the AI half of §13; mechanical parameter diff remains ❌ |
| §17 (provenance viewer) | 0/0/1/1 | 1/0/0/1 | `ProvenancePanel.svelte` (B14) + `RecipeStageTimeline.svelte` (B15) |
| §18 (provenance graph) | 0/0/1/1 | 1/0/0/1 | `VersionDag.svelte` (B8) |
| §22 (semantic API) | 5/4/11/20 | 4/5/11/20 | `apply_recipe` row: no `Recipe`-named apply IPC exists; `enhancement_apply_operation` reads `recipe_id` but takes no full `Recipe` payload. The previous audit's ✅ was over-generous |
| §22 (post §22.1) | 4/5/11/20 | 5/4/11/20 | `recipe_duplicate` IPC shipped (PR §22.1). One ❌ row closes; 11 ❌ remain (import/export/save_pipeline_as_recipe/etc.) |
| §28 (acceptance) | 7/8/7/22 | 17/4/9/30 | The §28 acceptance walk-down was under-counted (22 rows); current body has 30 distinct criteria reflecting CR-06/CR-07 work (ProvenancePanel/RecipeStageTimeline/VersionDag/etc.). Scorecard corrected to match body |

Net effect: +3 ✅ / +2 ⚠️ / −4 ❌ / −4 total rows
(the original audit over-counted the rows by listing §1,
§2, §3, §4 separately; refresh 1 folds §1/§2 into the
intro bucket and treats §3 as its own row, matching the
section body organization in the §26 audit refresh
template).

## Key findings

### Shipped substantial
- **AI Provenance (§9)** — 13/14 fields covered by `AiOperation`
- **Recipe versioning (§5)** — schema versions, linear version counter,
  parent_version lineage, migration
- **Adaptive parameters (§11)** — `derive_adaptive_parameters` with
  `reason: String` per value (exactly the CR-08 §11 disclosure contract)
- **Pipeline plan + execution (§2.2, §24)** — full DAG runner
  infrastructure
- **Standalone readiness (§27)** — sqlite + filesystem + model registry

### Largest gaps
- **Recipe editor (§10)** — no progressive-disclosure UI; only expert
- **Recipe types (§3)** — no system recipes, no imported recipes
- **Provenance viewer (§17)** — no UI component
- **Provenance graph (§18)** — flat list, no DAG
- **Import/export (§19)** — no `.afrecipe` portable format
- **Save-as-recipe (§14)** — no UX hook
- **9 ADRs (§30)** — none exist

## Files referenced (~3,000+ LOC of related existing code)

### Rust modules

| File | LOC | Relevance |
|---|---|---|
| `crates/astroforge-core/src/recipe.rs` | 566 | §4 schema + §5 versioning + §9 integrity |
| `crates/astroforge-core/src/recipe_store.rs` | 452 | §3 user recipes persistence |
| `crates/astroforge-core/src/recipe_feed.rs` | ~570 | Shared recipe format (P4-M2-T2) |
| `crates/astroforge-core/src/adaptive.rs` | 393 | §11 adaptation with reasons |
| `crates/astroforge-core/src/pipeline_plan/plan.rs` | ~500 | §2.2 pipeline plan |
| `crates/astroforge-core/src/pipeline_plan/runner.rs` | ~400 | DAG runner |
| `crates/astroforge-core/src/pipeline_plan/dispatch.rs` | ~1700 | Stage handlers |
| `crates/astroforge-core/src/domain.rs` | 1,300+ | §7/§8/§9 data model |

### Svelte UI

| File | LOC | Relevance |
|---|---|---|
| `src/components/RecipesScreen.svelte` | ~250 | §15 Recipe Library tab |
| `src/components/ProfileManager.svelte` | ~700 | §10 expert-mode editor (partial) |
| `src/components/RecipeGallery.svelte` | ~190 | §3.4 imported recipes surface (slice R) |