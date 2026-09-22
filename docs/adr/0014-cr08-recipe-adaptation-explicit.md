# ADR-0014 — CR-08 ADR-08.3: Recipe Adaptation Must Be Explicit

- Status: Accepted (2026-09-22)
- Gates: `CR-08 §30`, `CR-08 §11`, `CR-08 §12`
- Closes: CR-08 §30 architectural decision records (sub-record 3/9)
- Source spec: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30

## Context

When AstroForge applies a Recipe to a new Session,
the dataset may not match the Recipe's original
context (different exposure, different gear,
different sky conditions). The apply round must
either: (a) refuse to apply, (b) silently adapt
the Recipe, or (c) explicitly adapt with the
user's consent. The §30 spec is firm: "AstroForge
may intelligently adapt a Recipe, but must
disclose meaningful changes."

Three options were on the table:

1. **Refuse to apply.** The Recipe must be
   byte-identical to the Session for an apply
   round to proceed.
2. **Silently adapt.** AstroForge adapts the
   Recipe behind the user's back; the user only
   sees the result.
3. **Explicitly adapt with disclosure.** The
   apply round produces a `reason: String` for
   each adaptation; the user reviews + accepts.

## Decision

**Adopt option 3: Explicitly adapt with
disclosure.** `Recipe.apply_recipe()` produces an
`AdaptiveParameters` record per derivation; the
UI surfaces the per-adaptation reason in the §13
Recipe Diff Panel + the §11 "Recipe adapted" prompt
(with [Accept Adaptation] / [Keep Recipe Order]
buttons per the §11 audit row).

### Why option 3

- **User-owned decisions.** Per the §30 spec
  ("AstroForge may intelligently adapt a Recipe,
  but must disclose meaningful changes"), silent
  adaptation violates the user-owned-decision
  principle (cross-reference ADR-0010).
- **Forward-compatible with the §13 Recipe Diff
  UI.** The `reason: String` per parameter feeds
  directly into the §13 Diff Panel's "Added /
  Removed / Modified" classification.
- **Honest constraints.** Option 1 would refuse
  to apply Recipes across the smallest context
  change (e.g. exposure time); the user would
  have to author a new Recipe for every minor
  variation.

### Why not option 1

- Breaks Recipe reusability. The §30 spec
  requires "a Recipe [that] can be applied to
  another dataset" (per §31 Definition of Done);
  refusing to apply would close that door.

### Why not option 2

- Removes the user from the decision loop.
  AstroForge users have varying tolerance for
  adaptation; some users (publication-grade
  pipelines) want zero adaptation, others
  (exploratory workflows) want maximal
  adaptation.

## Consequences

- `Recipe.apply_recipe()` returns an
  `AdaptiveParameters` record with a per-field
  `reason: String`. The §11 prompt surfaces these
  reasons.
- The §12 audit row (Adaptable / Partially
  Compatible / Compatible) ships the
  `validate_compatibility()` three-state
  distinction as part of this decision's
  enforcement.
- The §13 Recipe Diff Panel's "Modified"
  classification includes adaptive-parameter
  reasons verbatim.

## Related

- ADR-0012 (Recipe Represents Intent)
- ADR-0013 (Recipe and Pipeline Are Distinct)
- ADR-08.3 source: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30
- §11 + §12 audit rows: `docs/CR-08-AUDIT.md`