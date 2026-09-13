# ADR-07.1 — Image Version Is the Comparison Primitive

**Status:** Accepted (2026-09-12, CR-07 audit + B1 slice)
**Source:** CR-07 §33, ADR-07.1

## Context

CR-07 §33 establishes that comparison must operate on meaningful Image
Versions rather than arbitrary filesystem files. The comparison workspace
needs a stable, durable reference to the candidate and the baseline so
that:

- The comparison can survive application restart.
- The comparison can be referenced from a `ComparisonSet` (§16) without
  re-importing source files.
- The comparison can record lineage back to the processing that
  produced each version.
- Decisions (§17, §18) can be applied to the version without
  re-deriving which file the user meant.

## Decision

The `ComparisonItem` struct (in `crates/astroforge-core/src/comparison.rs`)
stores `version_id: String` referencing an `ImageVersion` from the
project's domain model, **not** a filesystem path. The
`ComparisonSession` aggregates `ComparisonItem` slots (A/B/C/D) and
operates entirely on `ImageVersion` references.

The `ComparisonRegion` references a region within an `ImageVersion`, not
within a raw file. Shape geometry is stored in normalized image
coordinates (0..1) so the comparison survives image rescaling.

## Consequences

- **Positive:** Comparisons are portable across machines, filesystem
  reorganizations, and project migrations (provided ImageVersion IDs
  are stable).
- **Positive:** A `ComparisonSet` can be saved with `version_ids`
  alone; the workspace reloads the referenced versions on demand.
- **Positive:** The decision state machine (`ImageDecision`) operates
  on `version_id` directly, so promote/reject does not require
  resolving a file path first.
- **Negative:** All consumers must use the project's
  `ImageVersion::primary_artifact_id` to resolve the version to a
  renderable artifact. A missing artifact is an error, not a silent
  fallback to a stale file.
- **Negative:** Comparison data cannot be exchanged with tools that
  do not understand ImageVersion IDs (e.g. external file-based
  comparison tools).

## Alternatives considered

- **File path references.** Rejected: paths are not stable across
  project migration or filesystem reorganization. The CR-07 §33 ADR
  text is explicit on this point.
- **Content hash references.** Considered: hashes are stable but
  cannot differentiate between two versions with identical content but
  different processing provenance. ImageVersion IDs are stable and
  provenance-aware.
