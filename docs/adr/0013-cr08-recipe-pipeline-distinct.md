# ADR-0013 — CR-08 ADR-08.2: Recipe and Pipeline Are Distinct

- Status: Accepted (2026-09-22)
- Gates: `CR-08 §30`, `CR-08 §11`, `CR-08 §21`
- Closes: CR-08 §30 architectural decision records (sub-record 2/9)
- Source spec: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30

## Context

AstroForge has two related concepts: the Recipe
(durable intent artifact) and the Pipeline
(concrete execution plan against a specific
dataset). Without an explicit decision record,
these terms can be conflated, leading to
design questions like "can a Pipeline reference a
Recipe?" or "does the apply round mutate the
Recipe?". The §30 spec calls out the
distinction: "A Recipe is reusable; a Pipeline is
dataset-specific."

Three options were on the table:

1. **Strict separation.** A Recipe never carries
   execution data; a Pipeline is a fresh object
   each apply round and references the Recipe by
   profile_id + version.
2. **Pipeline-as-Recipe-attachment.** A Recipe
   keeps a `Vec<Pipeline>` history of every
   pipeline derived from it, so re-applying is
   one-click.
3. **Pipeline inverts Recipe.** A Pipeline owns
   its stages; the Recipe is just metadata.

## Decision

**Adopt option 1: Strict separation.** The
Recipe is the durable artifact; the Pipeline is a
fresh object each apply round. The Pipeline
carries a `recipe_profile_id` + `recipe_version`
back-reference so provenance remains explicit
(per ADR-0017 on provenance-as-first-class-data).

### Why option 1

- **Recipe reusability.** Option 2 bakes the
  Recipe's per-dataset history into the Recipe
  itself, doubling its size on every apply and
  making the Recipe dataset-coupled. Option 1
  keeps the Recipe lean.
- **Clean undo / redo.** Option 3 reverses the
  ownership; the Pipeline becomes the artifact
  the user cares about, breaking the §10
  editor surface.
- **Provenance stays with the execution record.**
  The Pipeline is what gets persisted in the §17
  provenance viewer + the §18 DAG; Recipe-side
  history would duplicate this.

### Why not option 2

- Bloats the Recipe struct. Each Recipe would
  carry `Vec<PipelineSummary>` (or equivalent)
  which is implementation-detail leakage. The
  §20 validation pipeline would have to walk the
  history on every save, slowing validation.

### Why not option 3

- Breaks the §10 Recipe Editor. The user opens a
  Recipe (not a Pipeline) when they want to
  tweak processing. If the Pipeline owns the
  stages, the Recipe Editor has no surface to
  edit.

## Consequences

- The Recipe struct never carries execution
  history. Apply rounds produce fresh Pipeline
  records.
- The §11 application flow's
  `validate_compatibility` + `derive_adaptive_parameters`
  operate on a Recipe + a Session to produce a
  Pipeline. The Pipeline carries the
  `recipe_profile_id` + `recipe_version` reference.
- The §13 Recipe Diff UI compares two Recipes
  (intent diff), not two Pipelines (execution
  diff). Pipeline-vs-Pipeline execution diffs
  are a separate concern.

## Related

- ADR-0012 (Recipe Represents Intent)
- ADR-0014 (Recipe Adaptation Must Be Explicit)
- ADR-08.2 source: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30
- §11 audit row: `docs/CR-08-AUDIT.md`