# ADR-0018 — CR-08 ADR-08.7: Reproducibility Is Qualified

- Status: Accepted (2026-09-22)
- Gates: `CR-08 §30`, `CR-08 §6`, `CR-08 §8`
- Closes: CR-08 §30 architectural decision records (sub-record 7/9)
- Source spec: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30

## Context

AstroForge processes astronomical images with
multiple sources of non-determinism: AI models
with stochastic inference, floating-point
non-associativity across hardware, OS-level
filesystem rounding. Reproducibility is therefore a
spectrum, not a binary. The §30 spec is firm:
"AstroForge records whether an operation is
exactly reproducible, materially reproducible,
or inherently variable."

Three options were on the table:

1. **Binary reproducible / not.** Each execution
   is either bit-identical to a prior run or not.
2. **Three-state qualified.** Each execution
   carries a `ReproducibilityClass`:
   `ExactlyReproducible` / `MateriallyReproducible`
   / `InherentlyVariable`.
3. **Continuous gradient.** Each execution
   carries a `reproducibility_score: f64` from
   0.0 to 1.0.

## Decision

**Adopt option 2: Three-state qualified.** The
`ReproducibilityClass` enum (in the §21 data
model) is part of the §8 processing provenance
record. The classification is determined at
apply time: an execution is `ExactlyReproducible`
iff every participating stage is deterministic +
no AI model is used; `MateriallyReproducible` if
AI models are used but with recorded seeds +
deterministic ordering; `InherentlyVariable`
otherwise.

### Why option 2

- **User-facing clarity.** The user sees
  "Materially reproducible (seed 12345)" rather
  than a fuzzy score; the decision is clear.
- **Actionable.** Each class has a clear
  consequence: `ExactlyReproducible` runs can be
  cached + skipped; `MateriallyReproducible`
  runs re-execute with the recorded seed;
  `InherentlyVariable` runs always re-execute.
- **Audit-grade.** The §6 reproducibility audit
  row categorizes each Recipe version by its
  worst-case `ReproducibilityClass`.

### Why not option 1

- Overpromises. Many legitimate workflows
  (AI-denoised) are not bit-reproducible; calling
  them "not reproducible" hides the fact that
  they are reproducible-to-within-tolerance.

### Why not option 3

- Ambiguous thresholds. What counts as
  "0.85 reproducible"? The user can't act on a
  gradient; the three states are actionable.

## Consequences

- `ReproducibilityClass` enum is part of the §21
  data model: `ExactlyReproducible` /
  `MateriallyReproducible` / `InherentlyVariable`.
- Each execution's provenance record carries a
  `ReproducibilityClass`; the §8 processing
  provenance viewer surfaces this.
- The §6 reproducibility audit row derives its
  scoring from the worst-case class across the
  Recipe's stages.

## Related

- ADR-0015 (Historical Execution Is Immutable)
- ADR-0016 (Provenance Is First-Class Data)
- ADR-0017 (AI Identity Is Part of Provenance)
- ADR-08.7 source: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30
- §6 + §8 audit rows: `docs/CR-08-AUDIT.md`