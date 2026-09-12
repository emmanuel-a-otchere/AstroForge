# CR-08 Implementation Plan — Recipes, Reproducibility & Processing Provenance

**Source:** [`CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md`](CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md)
**Audit:** [`CR-08-AUDIT.md`](CR-08-AUDIT.md) — 41% shipped, 32% partial, 27% missing (116 sub-items)
**Status:** Proposed → 8 bundles staged
**Target:** AstroForge v1.0

The existing `Recipe` (recipe.rs, 566 LOC) + `RecipeStore` (recipe_store.rs, 452 LOC) + `AdaptiveParameterSet` (adaptive.rs, 393 LOC) + `PipelinePlan` + `AiOperation` already cover substantial ground.

The 7-bundle plan is informed by the audit's scorecard.

## Bundle order

| # | Bundle | Slices | PR scope | LOC est. |
|---|---|---|---|---|
| **R1** | **Recipe schema completion** | §4 + §21 | Extend `Recipe` with: author, applicability (target_types, acquisition, camera, filter, constraints), processing-style detail (noise/detail/star/color/naturalness prefs), required/optional stage flags, stage constraints, ordering constraints, resource policy, explicit content_hash (SHA-256) | 1,200 |
| **R2** | **Recipe types + system recipes** | §3 + §28 acceptance | System Recipes (read-only bundled catalog) + Project Recipes (project-scoped CRUD) + Imported Recipes (validation gate); protect system recipes from modification | 1,000 |
| **R3** | **Recipe editor (progressive disclosure)** | §10 + §15 Recipe Detail | `RecipeEditor.svelte` (Beginner: name/target/style/AI radio) → `RecipeEditorGuided.svelte` (objectives/stages/AI prefs/quality targets) → `RecipeEditorExpert.svelte` (current ProfileManager + stage constraints + model selection + reproducibility controls); `RecipeDetail.svelte` (title + version + description + target + AI + style + intent bullets + Apply/Duplicate/Edit) | 2,000 |
| **R4** | **Save-as-recipe + workspace integration** | §11 + §14 + §16 + §23 | "Save as Recipe" prompt post-pipeline; Recipe Library 5-tab nav (System / My / Project / Imported / Recent); processing workspace "Processing Complete → Save as Recipe / Export" CTA; RecipeAdaptationProposed/Accepted events | 1,500 |
| **R5** | **Provenance viewer + graph** | §7 + §8 + §17 + §18 | `ProvenanceViewer.svelte` (human-readable summary: source → recipe → processing → AI → result); `ProvenanceGraph.svelte` (DAG visualization); `ProvenanceRecord` + `ProvenanceEdge` data types; wire `AiOperation` → ImageVersion lineage | 1,800 |
| **R6** | **Import/export + security** | §19 + §20 + §22 partial + §28 acceptance | `.afrecipe` portable format (JSON + schema_version + content_hash); `import_recipe` / `export_recipe` IPC commands; validation pipeline (schema / stages / params / models / deps / fs-refs / executable payload rejection) | 1,000 |
| **R7** | **Reproducibility records + ADRs** | §6 + §21 partial + §30 | `ReproducibilityRecord` data type (exact vs material reproducibility); `RecipeApplication` aggregate; execution_environment aggregate; 9 ADRs (ADR-08.1..08.9); reproducibility tests | 1,200 |
| **R8** | **Test coverage** | §29 + §32 acceptance | Unit tests for validation, schema migration, versioning, hashing, applicability, adaptation, parameter constraints, provenance relationships, reproducibility records; integration test (Create Recipe → Apply → Generate Pipeline → Execute → Image Version → Persist Provenance → Restart → Read Provenance); security tests (invalid stages / params / fs refs / payloads) | 1,500 |

**Total: ~11,200 LOC over 8 PRs.**

## Implementation locations

Per the CR-08 §25 implementation map, the audit confirms: the
referenced `astroforge-persistence/`, `astroforge-pipeline/`,
`astroforge-runtime` crates **do not exist.** Recipe + provenance
modules live in `astroforge-core` (existing). The recommendation:

```
crates/
├── astroforge-core/
│   ├── recipe.rs             (existing, R1 extends)
│   ├── recipe_store.rs       (existing)
│   ├── recipe_feed.rs        (existing, P4-M2-T2)
│   ├── recipe_format.rs      (NEW, R6 .afrecipe)
│   ├── recipe_validation.rs  (NEW, R2 + R6 validation pipeline)
│   ├── recipe_adaptation.rs  (NEW, R4 — extract from adaptive.rs)
│   ├── recipe_application.rs (NEW, R7 — RecipeApplication aggregate)
│   ├── provenance/
│   │   ├── mod.rs
│   │   ├── record.rs         (NEW, R5)
│   │   ├── graph.rs          (NEW, R5)
│   │   ├── viewer.rs         (NEW, R5)
│   │   └── reproducibility.rs(NEW, R7)
│   ├── adaptive.rs           (existing, R4 extracts public surface)
│   └── pipeline_plan/        (existing)
└── astroforge-ai/
    └── (no new modules — provenance records stay in core)

src/
├── components/
│   ├── RecipesScreen.svelte          (existing, R4 extends)
│   ├── RecipeEditor.svelte           (NEW, R3)
│   ├── RecipeEditorGuided.svelte     (NEW, R3)
│   ├── RecipeEditorExpert.svelte     (NEW, R3)
│   ├── RecipeDetail.svelte           (NEW, R3)
│   ├── RecipeLibraryNav.svelte       (NEW, R4 — 5 tabs)
│   ├── SaveAsRecipePrompt.svelte     (NEW, R4)
│   ├── ProvenanceViewer.svelte       (NEW, R5)
│   └── ProvenanceGraph.svelte        (NEW, R5)
└── lib/
    ├── recipe-format.ts              (NEW, R6)
    ├── recipe-validation.ts          (NEW, R6)
    └── provenance-store.ts           (NEW, R5)

docs/adr/
├── 0012-cr08-recipe-intent.md        (R7)
├── 0013-cr08-recipe-pipeline-distinct.md
├── 0014-cr08-adaptation-explicit.md
├── 0015-cr08-immutability.md
├── 0016-cr08-provenance-first-class.md
├── 0017-cr08-ai-identity-provenance.md
├── 0018-cr08-reproducibility-qualified.md
├── 0019-cr08-no-executable-code.md
└── 0020-cr08-provenance-human-readable.md
```

## Key audit-driven decisions

1. **System recipes live in `recipes/system/` as JSON** — read-only at
   runtime; `RecipeStore::insert_system_recipe` is the only write path,
   and it accepts a `&Recipe` from a compile-time catalog.
2. **Recipe authoring is versionless; persistence creates versioned
   `RecipeVersion` records.** Authoring → save → version bump.
3. **`.afrecipe` is JSON** (matches existing serde shape), with a
   `schema_version` header + a `content_hash` SHA-256 of the canonical
   payload.
4. **Recipe security validation rejects:**
   - Filesystem refs (anything with `path:` or `file:` keys)
   - Executable payloads (`code:`, `script:`, `command:` keys)
   - Unknown stage IDs
   - Parameter values outside per-stage declared ranges
   - Model references not in `CATALOG_MODELS`
5. **ProvenanceRecord is a denormalized join** of `AiOperation` +
   `StageRun` + `ImageVersion` + `PipelineRun` keyed by `(run_id,
   image_version_id)`. Materialized at pipeline completion; persisted
   as JSON in `image_versions.provenance_record_json` column.
6. **ProvenanceGraph uses the existing ImageVersion lineage** (parent
   artifact IDs + Recipe `parent_version`) to construct the DAG.
7. **9 ADRs ship in R7** to align with §30.

## §28 acceptance gap-fill (most-impactful)

The audit surfaced 7 missing acceptance criteria. Per bundle:
- Duplicate/delete recipe → R2 (User Recipe CRUD)
- System Recipes protected → R2
- Content hash → R1
- Duplicate recipe versions → R2 + R6 export
- Adaptations explicitly shown → R4 (recipe_adaptation.rs + UX)
- User accept/reject adaptations → R4 (RecipeAdaptationAccepted event)
- Reproducibility conditions recorded → R7 (ReproducibilityRecord)
- Provenance visible without expert mode → R5 (ProvenanceViewer)
- Recipe export/import → R6 (.afrecipe)
- Invalid recipes can't execute → R6 (validation pipeline)
- Recipes contain no arbitrary code → R6 (validation rejects payloads)

## First concrete slice

**R1 — Recipe schema completion.** Paperwork + Rust types only:

1. Extend `Recipe` struct with: `author`, `applicability` (target_types,
   acquisition, camera, filter, constraints), `processing_style_detail`
   (noise / detail / star / color / naturalness prefs),
   `stage_constraints` (required/optional + ordering), `resource_policy`.
2. Add explicit `content_hash: String` (SHA-256 of canonical JSON).
3. Migrate existing recipes via `migrate_recipe()` to fill new fields
   with defaults.
4. Tests: hash determinism (same input → same hash), hash
   sensitivity (different input → different hash), migration
   backward-compat.

No behavior change; just richer data + tests. Unblocks R2 + R3 + R4.