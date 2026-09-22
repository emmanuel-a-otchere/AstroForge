# ADR-0015 — CR-08 ADR-08.4: Historical Execution Is Immutable

- Status: Accepted (2026-09-22)
- Gates: `CR-08 §30`, `CR-08 §5`, `CR-08 §8`, `CR-08 §15`
- Closes: CR-08 §30 architectural decision records (sub-record 4/9)
- Source spec: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30

## Context

When a Recipe is updated (a new version is saved
that overrides a previous version), what happens
to historical executions that used the previous
version? The §30 spec is firm: "Changing a Recipe
must never rewrite historical processing."

Three options were on the table:

1. **Cascade-update.** When Recipe v2 supersedes
   v1, every historical execution that referenced
   v1 has its `recipe_version` updated to v2.
2. **Immutable historical references.** Each
   execution pins to the (profile_id, version)
   pair it was derived from. Recipe v2 supersedes
   v1 in the Recipe store but never updates past
   executions.
3. **Loose reference.** Each execution pins to
   `profile_id` only; the version is taken at
   query time.

## Decision

**Adopt option 2: Immutable historical
references.** Each Pipeline (and its associated
Image Version + provenance record) pins to the
exact (profile_id, version) pair it was derived
from. Saving a new Recipe version creates a new
row in the Recipe store; it never updates prior
rows or their referencing executions.

### Why option 2

- **Provenance integrity.** The §18 provenance
  graph + §17 provenance viewer rely on stable
  (profile_id, version) references. Cascade-
  updating would invalidate the graph every time
  a Recipe is edited.
- **Reproducibility.** Historical executions
  remain byte-reproducible (per ADR-0018). The
  exact Recipe version that produced each output
  is recoverable.
- **Audit-grade trail.** AstroForge's offline
  + audit posture (§2 spec) demands stable
  historical pointers; option 3's loose reference
  breaks this.

### Why not option 1

- Invalidates the §18 provenance DAG on every
  Recipe save. The DAG is the user-facing
  debugging surface; mutating it on every Recipe
  change is hostile to the user.

### Why not option 3

- Decouples execution from Recipe version. The
  user can't reproduce a specific output by
  referencing the Recipe version that produced
  it.

## Consequences

- `RecipeStore::save` writes a new row (new
  `version: u32`) without updating prior rows.
  `Recipe.parent_version: Option<u32>` records
  the derivation chain for the lineage view.
- The §15 Recipe Library shows every version of
  every Recipe in the Library list; the user can
  pin / archive / restore prior versions.
- `recipe_pipeline_plan_hash` (per §32.4) hashes
  the (profile_id, version) pair, so any version
  change yields a fresh pipeline plan.

## Related

- ADR-0017 (Provenance Is First-Class Data)
- ADR-0018 (Reproducibility Is Qualified)
- ADR-08.4 source: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30
- §5 + §15 audit rows: `docs/CR-08-AUDIT.md`