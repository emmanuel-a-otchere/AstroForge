# ADR-0017 — CR-08 ADR-08.6: AI Identity Is Part of Provenance

- Status: Accepted (2026-09-22)
- Gates: `CR-08 §30`, `CR-08 §9`, `CR-08 §21`
- Closes: CR-08 §30 architectural decision records (sub-record 6/9)
- Source spec: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30

## Context

When an AI model participates in processing
(denoising, sharpening, classification), the
provenance record must capture enough identity
information to reproduce or audit the result.
The §30 spec is firm: "AI-generated results must
retain model identity and execution
characteristics."

Three options were on the table:

1. **Model name only.** The provenance records
   "AI model X was used" without version,
   weights hash, or execution parameters.
2. **Model name + version + hash.** The
   provenance records `model_name`, `model_version`,
   `weights_hash` (SHA-256 of the weights file),
   plus execution params (seed, temperature,
   runtime).
3. **Full inference log.** The provenance records
   every token / every activation; effectively a
   checkpoint.

## Decision

**Adopt option 2: Model name + version + hash +
execution params.** The §9 AI provenance records
the `ModelUsage` struct (per §21) which carries
`model_type: Deterministic | Perceptual` +
`model_name` + `model_version` +
`weights_sha256`. The execution seed is recorded
on the Recipe's `integrity.seed_recorded` flag.
Per the §10.2 Beginner tier, the user can pick
`ai_enhancement_level: Off | Conservative |
Recommended | Advanced` which influences which
AI stages run; the actual stages + their params
are in the Pipeline + provenance record.

### Why option 2

- **Reproducibility.** Per ADR-0018, an execution
  is reproducible iff the exact model + weights
  + seed are produced. Option 1 omits the version
  + hash, breaking reproducibility.
- **Audit-grade trail.** The §9 AI provenance
  viewer shows model identity + hash; the user
  can verify the weights file matches the
  recorded hash.
- **Storage cost.** Option 3 would balloon the
  provenance record to MB-per-execution;
  option 2 stays in the KB-per-execution range.

### Why not option 1

- Inadequate for reproducibility. Two different
  versions of the same model produce different
  outputs; option 1 collapses them.

### Why not option 3

- Storage prohibitive. The §18 DAG would render
  slowly; the user can't navigate a graph with
  MB-sized nodes.

## Consequences

- `ModelUsage` is part of the §21 data model
  with `model_name` + `model_type` + (planned
  follow-on: `model_version` + `weights_sha256`).
- The §9 AI Provenance viewer surfaces the model
  identity; the §20 validation pipeline enforces
  the model-existence checks
  (`available_models`).
- The §10 Recipe (Beginner / Guided / Expert
  tiers) carries `ai_enhancement_level` which
  determines which AI stages run; the actual
  stage-level model identity is recorded in the
  Pipeline's provenance.

## Related

- ADR-0016 (Provenance Is First-Class Data)
- ADR-0018 (Reproducibility Is Qualified)
- ADR-08.6 source: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30
- §9 audit row: `docs/CR-08-AUDIT.md`