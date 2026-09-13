# ADR-07.4 — Comparison Is Non-Destructive

**Status:** Accepted (2026-09-12, CR-07 audit + B1 slice)
**Source:** CR-07 §33, ADR-07.4

## Context

CR-07 §33 establishes that reviewing or rejecting an image never
deletes or mutates its source version. The §34 Definition of Done is
explicit:

> "Preserve every alternative version."

The existing `AiOperation` + `StageRun` chain in `domain.rs` is
already non-destructive — `pipeline_run.rs` appends a new row per
stage and never mutates a prior row. The comparison data model must
preserve the same invariant.

## Decision

The comparison data model is **append-only with respect to the
referenced Image Versions.** No method on `ComparisonSession`,
`ComparisonItem`, `ComparisonRegion`, `ComparisonMetric`,
`ComparisonDelta`, `QualityAssessment`, `ImageDecision`, or
`ComparisonSet` mutates, deletes, or re-encodes the referenced
`ImageVersion` rows.

`ImageDecision::reject()` transitions a version's decision state from
Working / Candidate / Preferred to Rejected. **The Image Version row
itself is not deleted or modified** — only the decision record's
`state` field and `history` are updated. The version remains
referencable from any future comparison or processing pipeline.

`ImageDecision` records a full `history: Vec<DecisionHistoryEntry>`
that captures every state transition. The history is append-only —
no prior entry is rewritten (mirrors the ADR-08.4 principle from
CR-08 about historical execution immutability).

## Consequences

- **Positive:** A rejected version can be restored by transitioning
  back to Working (rejected is not a final terminal — it can be
  un-rejected via a new `transition_to(Working)` call, though this
  is not yet implemented in B1).
- **Positive:** The provenance graph (CR-08 §18) can reference any
  version, regardless of its current decision state.
- **Positive:** Comparison sets (§16) can include rejected versions,
  preserving the comparison context for posterity.
- **Negative:** The data model cannot enforce "deletion" semantics
  on a version. If a user truly wants to delete a version, they must
  do so at the project level (which itself is non-destructive — the
  Artifact row remains in the database).

## Alternatives considered

- **Soft-delete via `ImageDecision::Rejected` cascading to hide the
  version in lists.** Rejected: hidden versions would break the
  comparison set + provenance graph invariants.
- **Hard-delete on `ImageDecision::Rejected`.** Rejected: violates
  the CR-07 §34 DoD directly.
