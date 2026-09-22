# ADR-0016 — CR-08 ADR-08.5: Provenance Is First-Class Data

- Status: Accepted (2026-09-22)
- Gates: `CR-08 §30`, `CR-08 §7`, `CR-08 §8`, `CR-08 §17`, `CR-08 §18`
- Closes: CR-08 §30 architectural decision records (sub-record 5/9)
- Source spec: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30

## Context

AstroForge's processing history (which Recipe was
applied to which Session, with what parameters,
yielding which Image Version, with which AI
models, what seeds, what timing) is either a
structured data model or a log file. The §30 spec
is firm: "Processing history is structured
application data, not merely a log."

Three options were on the table:

1. **Log-file provenance.** Append every
   processing event to a flat text log; the user
   reads the log when debugging.
2. **Structured provenance data model.**
   `ProvenanceRecord` + `ProvenanceEdge` types in
   the §21 data model; the §17 viewer + §18 DAG
   UI surface this data model directly.
3. **Hybrid.** Structured data model for the
   recent window; text log for the long tail.

## Decision

**Adopt option 2: Structured provenance data
model.** Provenance is part of the §21 data
model: `ProvenanceRecord` (events) +
`ProvenanceEdge` (relationships). The §17 viewer
+ §18 DAG UI query this data model directly. The
data model is durable (persisted alongside
Pipelines + Image Versions) and offline-readable.

### Why option 2

- **Offline-readable per §2 spec.** The user can
  open the §17 viewer + §18 DAG without any
  network; both query the structured data model
  directly.
- **User-facing surface.** The §17 viewer + §18
  DAG UI are first-class application surfaces;
  they need structured queries (filter by stage,
  filter by model, traverse DAG), which a flat
  log can't serve.
- **Forward-compatible with the §22 semantic API.**
  The structured data model can be exposed via
  the §22 semantic API as queryable Records +
  Edges.

### Why not option 1

- Limits the §17 + §18 surfaces to text search,
  breaking the user-facing graph navigation.

### Why not option 3

- Splits the provenance surface across two
  representations; debugging becomes ambiguous
  ("is this event in the structured half or the
  log half?").

## Consequences

- `ProvenanceRecord` + `ProvenanceEdge` types are
  part of the §21 data model; they live in
  `astroforge-core` alongside `Recipe` +
  `Pipeline`.
- The §17 viewer + §18 DAG UI query the
  structured data model via dedicated IPC
  commands; the log file is not consumed by
  these surfaces.
- The §30 spec line "Processing history is
  structured application data, not merely a log"
  is enforced by the absence of any UI surface
  that consumes a flat log.

## Related

- ADR-0015 (Historical Execution Is Immutable)
- ADR-0017 (AI Identity Is Part of Provenance)
- ADR-0019 (Provenance Is Human-Readable)
- ADR-08.5 source: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30
- §7 + §8 + §17 + §18 audit rows: `docs/CR-08-AUDIT.md`