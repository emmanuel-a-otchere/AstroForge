# ADR-07.5 — Provenance Is Always Available

**Status:** Accepted (2026-09-12, CR-07 audit + B1 slice)
**Source:** CR-07 §33, ADR-07.5

## Context

CR-07 §33 establishes that users can trace every candidate back
through its processing and AI history. The §14 Provenance Panel and
§17 Provenance Viewer require that provenance be inspectable without
entering Expert mode (CR-07 §31 acceptance criterion).

The existing `AiOperation` struct (`domain.rs:625`) already records
13/14 of the CR-08 §9 AI provenance fields per operation. The
`PipelineRun` + `StageRun` chain records the full processing
lineage.

## Decision

The comparison data model exposes provenance via two routes:

1. **Direct lineage** — `ComparisonItem::version_id` references an
   `ImageVersion`, which carries `source_artifact_id` + `pipeline_run_id`
   + `recipe_version_id`. The processing history is reachable by
   following these foreign keys.

2. **Aggregate** — `QualityAssessment::integrity_checks` carries the
   §12 astronomical integrity findings (faint-structure suppression,
   star disappearance, halos, ringing, etc.) aggregated at the
   comparison level rather than per-stage.

The `QualityAssessment::summary` field carries a human-readable
string (e.g. "Strong improvement with minor trade-offs."). This is
the §17 Provenance Viewer's user-facing summary.

B1 does not yet implement a `ProvenanceRecord` or `ProvenanceEdge`
aggregate type — those will ship in R5 (CR-08) which lands provenance
on top of the §25 data model established here.

## Consequences

- **Positive:** The comparison data model integrates with the
  existing `AiOperation` lineage without a parallel persistence
  layer.
- **Positive:** Provenance data is available even without a
  `ProvenanceRecord` aggregate — the foreign-key chain is sufficient.
- **Negative:** Cross-cutting queries (e.g. "all comparisons that
  referenced a perceptual AI operation") require joining through the
  foreign keys, which is more expensive than a denormalized
  `ProvenanceRecord` table.
- **Negative:** Until R5 ships, there is no DAG visualization of the
  provenance graph. The data is available; the UI is not.

## Alternatives considered

- **Standalone `ProvenanceRecord` table owned by comparison.** Rejected
  for B1: the foreign-key chain is sufficient and avoids introducing
  a parallel persistence crate (per the CR-07 audit recommendation).
  R5 will introduce `ProvenanceRecord` as a denormalized join for
  query efficiency.
- **No `QualityAssessment::summary` field.** Rejected: §17 requires
  a human-readable summary; encoding it on the data model makes the
  summary durable across sessions.
