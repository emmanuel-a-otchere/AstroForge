# CR-08 ADR-002 — Implementation Map Route

**Status:** ✅ Accepted (2026-09-25)
**Driver:** CR-08 §25 Implementation Map + Slice G (`close_§21_partial_completion`)
**Supersedes:** §25's `astroforge-persistence/` / `astroforge-pipeline/` / `astroforge-runtime/` crate split

---

## Context

The CR-08 spec's §25 Implementation Map proposes a
crate split into:

- `astroforge-persistence/` (recipe + provenance + migrations)
- `astroforge-pipeline/` (`recipe_adapter.rs` + `recipe_generator.rs`)
- `astroforge-runtime/` (the DAG runner)
- `astroforge-ai/provenance/` (AI-side provenance)

The audit doc flagged this map as ⚠️ Partial because
**none of the three proposed crates exist** in the
repository. Slice A (`compare_recipe_versions` round 1)
rejected the persistence crate split and folded the
comparison + recipe + provenance modules into the
existing `astroforge-core/` crate. Slice G needs to
formalize the same rejection for the §25 map as a whole
so the §21 follow-on rows (`recipe_constraint`,
`recipe_resource_policy`, `provenance_record`) have a
canonical home.

## Decision

**The §25 crate split is rejected.** The AstroForge
codebase routes the §21 + §25 implementation map
through the existing `astroforge-core/` crate (with
the existing `db.rs` SQLite connection reused across
all of: recipes, provenance, reproducibility, recipe
events, recipe constraints, recipe resource policy,
provenance records).

The §25 routing table below replaces the spec's
crate-split map. Every §21 row + every §25 module
gets a concrete `astroforge-core/` file:

| §21 / §25 type | Module |
|---|---|
| `recipe` | `crates/astroforge-core/src/recipe.rs` (existing) |
| `recipe_version` | `crates/astroforge-core/src/recipe.rs` (existing) |
| `recipe_stage` | `crates/astroforge-core/src/recipe.rs` (existing) |
| `recipe_parameter` | `crates/astroforge-core/src/recipe.rs` (existing; `RecipeStage::params`) |
| `recipe_constraint` | `crates/astroforge-core/src/recipe.rs` (NEW in Slice G: `RecipeConstraint` struct + `Recipe.constraints: Vec<RecipeConstraint>` field) |
| `recipe_ai_policy` | `crates/astroforge-core/src/recipe.rs` (existing; partial: `model_usage[]` + `integrity` + `quality_objective`) |
| `recipe_quality_target` | `crates/astroforge-core/src/recipe.rs` (existing; partial: `quality_targets: QualityTargets`) |
| `recipe_resource_policy` | `crates/astroforge-core/src/recipe.rs` (NEW in Slice G: `RecipeResourcePolicy` struct + `Recipe.resource_policy: Option<RecipeResourcePolicy>` field) |
| `pipeline_plan` | `crates/astroforge-core/src/pipeline_plans_store.rs` (existing) |
| `pipeline_execution` | `crates/astroforge-core/src/domain_store.rs` (existing; `PipelineRun` rows) |
| `provenance_record` | `crates/astroforge-core/src/recipe.rs` (NEW in Slice G: `ProvenanceRecord` struct + pure `provenance_record(version_id, iv, recipe, run, stage_runs, ai_ops)` constructor) |
| `provenance_edge` | `crates/astroforge-core/src/domain_store.rs` (existing; foreign keys) |
| `reproducibility_record` | `crates/astroforge-core/src/reproducibility.rs` (existing; Slice D) |
| `execution_environment` | `crates/astroforge-core/src/resource.rs` (existing; `ResourceSnapshot::detect()`) + `crates/astroforge-core/src/reproducibility.rs` (existing; `HardwareSummary`) |
| `recipe_application` | `crates/astroforge-core/src/recipe.rs` (existing; partial: `apply_recipe()` function) |
| `recipe_adaptation` | `crates/astroforge-core/src/recipe.rs` (existing; `AdaptiveParameterSet::reason: String`) |
| `recipe_validation` | `crates/astroforge-core/src/validation.rs` (existing; `validate_compatibility` + `validate_recipe_security`) |
| `recipe_serialization` | `crates/astroforge-core/src/recipe.rs` (existing; `Recipe::to_json` + `from_json_migrated`) |
| `recipe_provenance` | `crates/astroforge-core/src/recipe.rs` (existing; `RecipeProvenance` + `recipe_provenance()`) |
| `recipe_events` | `crates/astroforge-core/src/recipe_events.rs` (existing; Slice F) |
| `recipe_comparison` | `crates/astroforge-core/src/recipe.rs` (existing; `RecipeComparison` + `compare_recipe_versions()`) |
| `recipe_applicability` | `crates/astroforge-core/src/recipe.rs` (existing; `ApplicabilityReport` + `check_recipe_applicability()`) |
| `recipe_reproducibility` | `crates/astroforge-core/src/reproducibility.rs` (existing; `ReproducibilityRecord` + `summarize_reproducibility()`) |

## Consequences

**Positive.** Single-crate routing means:

- No new persistence crate to maintain. The existing
  `db.rs` SQLite connection handles every §21 + §25
  module.
- No migration churn. `crates/astroforge-core/src/`
  is the canonical home for the §21 + §25 surface;
  every downstream crate (e.g. `astroforge-ai`,
  `astroforge-app`) imports from there.
- Slice G's §21 partial-completion can land in a
  single PR because every new struct is in
  `crates/astroforge-core/src/recipe.rs`.

**Negative.**

- `crates/astroforge-core/src/recipe.rs` will grow.
  Slice G adds ~200 lines (3 new structs +
  1 new pure constructor + 2 new Recipe fields).
  The file is currently ~2400 lines; the post-Slice G
  size is ~2600 lines, still under the 5000-line
  soft-limit the codebase enforces for individual
  Rust files.
- The §25 crate-split map in the spec doc is
  superseded by this ADR. Any external readers of
  the spec that quote the crate-split map will see
  the spec text but no matching crates in the repo;
  this ADR is the canonical explanation.

## Alternatives considered

**Alternative A — split per the spec (rejected).** Add
the three crates proposed in §25 + port the existing
recipe + provenance modules. Cost: ~3 weeks of pure
crate-extraction work for zero functional gain; the
audit doc had already flagged the spec's map as
inapplicable. Rejected because Slice A's persistence
rejection already settled the design question.

**Alternative B — keep everything in `recipe.rs`
without an ADR (rejected).** Skip the ADR and just
land Slice G inline. Cost: the §25 Implementation Map
audit row stays ⚠️ Partial with no explanation; the
next reviewer who asks "where does the
`recipe_constraint` type live?" has to grep the
codebase. Rejected because the audit doc's §25 row
deserves a one-paragraph ADR to make the routing
explicit.

## Compliance with the existing codebase

The §22 round 1 Slice A rejection of the
persistence crate split (see
`docs/CR-08-AUDIT.md` §22 row) is the precedent for
this ADR. The §25 module split is the same design
question at a larger scope: "do we need new crates
for the §21 + §25 surface?" The answer is the same:
no; fold into `astroforge-core/` and reuse the
existing `db.rs` SQLite connection.

## Audit doc flips expected from Slice G

- §25 Implementation Map: ⚠️ Partial -> ✅ Shipped.
- §21 Data Model: ⚠️ Partial -> ✅ Shipped (every ❌
  row flipped).

After Slice G lands, the only remaining ⚠️ Partial
rows in §21 + §22 + §25 are partial-by-design rows
(`recipe_ai_policy` is partial because the spec only
demands a subset; `recipe_quality_target` is partial
because per-metric targets are out of scope; `execution_environment`
is partial because the aggregate is not surfaced).