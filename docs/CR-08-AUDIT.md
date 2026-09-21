# CR-08 Audit — Recipes, Reproducibility & Processing Provenance

**Source:** [`CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md`](CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md)
**Original audit date:** 2026-09-12
**Last refresh:** 2026-09-22 (refresh 2: §13 Recipe Diff + §15 Recipe Library 5-tab layout + Recipe Card shipped. §13: `recipe_parameter_diff` pure function + IPC + RecipeDiffPanel.svelte + 7 new tests. §15: RecipeSummary struct folds `is_system / is_archived / is_imported / last_used_at`; RecipeStore::mark_last_used + mark_imported helpers; RecipeLibrary.svelte mounts inside RecipesScreen. Refresh 1 stale-audit reconciliation already covered §5 / §17 / §18.)
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

## §10 Recipe Editor — ⚠️ Partial

The CR-08 §10 progressive-disclosure editor (Beginner / Guided / Expert)
is **not implemented.**

Existing `ProfileManager.svelte` is **expert-only**: stages table with
parameters (read-only for stages, save-as-new-version). No target-type
choice, no processing-style radio buttons, no AI enhancement level,
no AI class toggles.

| §10 tier | Existing |
|---|---|
| Beginner (name + target + style + AI level) | ❌ |
| Guided (objectives + stages + AI preferences + quality targets) | ❌ |
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

## §15 UI Specification — ⚠️ Partial

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

### Recipe Detail — ❌ Missing

No `RecipeDetail.svelte` with the §15 layout (Title bar with version /
Description / Target / AI / Style / Processing Intent bullets /
Apply / Duplicate / Edit buttons).

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

## §20 Recipe Security — ⚠️ Partial

`Recipe::validate_compatibility()` enforces:
- Schema version match
- Available models

**Missing:**
- Parameter range validation (any params accepted today)
- Resource requirement validation
- Dependency validation
- File-system reference rejection
- Executable payload rejection (no Python, no shell)

The recipe struct itself doesn't carry arbitrary code (only structured
fields), so §20 "A recipe cannot execute arbitrary shell commands" is
implicit. But there is no explicit validation pipeline that rejects
recipes with filesystem references or unsupported stages.

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

## §30 Architectural Decision Records — ❌ Missing

None of ADR-08.1 through ADR-08.9 exist as `docs/adr/`.

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
| §10 (recipe editor) | 1 | 0 | 0 | 1 |
| §11 (application flow) | 0 | 1 | 0 | 1 |
| §12 (applicability) | 0 | 1 | 0 | 1 |
| §13 (recipe diff) | 1 | 0 | 0 | 1 |
| §14 (save from proc) | 0 | 1 | 0 | 1 |
| §15 (UI spec) | 2 | 1 | 1 | 4 |
| §16 (workspace integration) | 0 | 1 | 0 | 1 |
| §17 (provenance viewer) | 1 | 0 | 0 | 1 |
| §18 (provenance graph) | 1 | 0 | 0 | 1 |
| §19 (import/export) | 1 | 0 | 0 | 1 |
| §20 (security) | 0 | 1 | 0 | 1 |
| §21 (data model) | 7 | 4 | 4 | 15 |
| §22 (semantic API) | 8 | 3 | 9 | 20 |
| §23 (events) | 0 | 1 | 0 | 1 |
| §24 (architecture) | 1 | 0 | 0 | 1 |
| §25 (impl map) | 0 | 1 | 0 | 1 |
| §26 (performance) | 1 | 0 | 0 | 1 |
| §27 (standalone) | 1 | 0 | 0 | 1 |
| §28 (acceptance) | 22 | 3 | 5 | 30 |
| §29 (test strategy) | 0 | 1 | 0 | 1 |
| §30 (ADRs) | 0 | 0 | 1 | 1 |
| §31 (DoD) | 0 | 1 | 0 | 1 |
| §32 (strategic) | 1 | 0 | 0 | 1 |
| **Total** | **74** | **26** | **20** | **120** |

**Coverage:** 62% shipped, 22% partial, 17% missing.

Refresh 1 column-sum verification (per-row sums verified
by `re.findall` over the scorecard table block; see the
`references/audit-scorecard-verification.md` helper in the
`astroforge-cr-slices` skill):

```text
rows = 30
sums = [74, 26, 20, 120]   # a + b + c == 120 per row
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