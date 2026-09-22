# ADR-0020 — CR-08 ADR-08.9: Provenance Is Human-Readable

- Status: Accepted (2026-09-22)
- Gates: `CR-08 §30`, `CR-08 §17`, `CR-08 §18`
- Closes: CR-08 §30 architectural decision records (sub-record 9/9)
- Source spec: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30

## Context

Provenance records are technical: model hashes,
parameter ranges, pipeline plan hashes, timing.
The §30 spec is firm: "Technical provenance must
have both user and Expert representations."

Three options were on the table:

1. **Single technical representation.** The user
   reads raw `ProvenanceRecord` + `ProvenanceEdge`
   JSON.
2. **Dual representation.** The user gets a
   human-readable summary; the Expert user gets
   the raw technical data.
3. **Progressive disclosure.** The user sees a
   summary; clicking "Show technical" reveals the
   raw data inline.

## Decision

**Adopt option 3: Progressive disclosure.** The
§17 viewer + §18 DAG surface a human-readable
summary by default. A "Show technical" toggle
reveals the raw `ProvenanceRecord` + edge
metadata inline. Both representations read from
the same structured data model (per ADR-0016).

### Why option 3

- **User onboarding.** A new user sees "Denoise
   stage with M42-Denoise-v3 model" rather than
   the raw JSON; the §17 viewer + §18 DAG are
   approachable from day one.
- **Expert user power.** An Expert user can flip
   the toggle and inspect the raw provenance;
   the Expert workflow is supported without
   forcing every user through the raw view.
- **Single source of truth.** Both
   representations read from the same
   `ProvenanceRecord` + `ProvenanceEdge` data
   model (per ADR-0016); there's no parallel
   representation to drift.

### Why not option 1

- Hostile to non-Expert users. The §17 + §18
  surfaces are first-class user-facing; raw
  JSON is not a user experience.

### Why not option 2

- Splits the surface. The Expert and user
  representations would diverge over time as
  one gets features the other doesn't.

## Consequences

- The §17 viewer + §18 DAG surface a
  human-readable summary by default with a
  "Show technical" toggle. Both views query the
  same data model.
- The §22 semantic API exposes both
  representations: a `summary()` method that
  returns the user-readable form, and direct
  field access for the technical form.

## Related

- ADR-0016 (Provenance Is First-Class Data)
- ADR-0019 (Recipes Cannot Execute Arbitrary Code)
- ADR-08.9 source: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30
- §17 + §18 audit rows: `docs/CR-08-AUDIT.md`