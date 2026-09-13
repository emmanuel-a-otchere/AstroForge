# AstroForge Architecture Decision Records (ADRs)

This directory holds the canonical ADRs for AstroForge. Each ADR is
a short, dated document that captures a single architectural decision
along with its context, the options considered, the chosen path, and
the consequences (good and bad).

## Index

| ADR | Title | Date | Status |
|---|---|---|---|
| [0001](0001-plate-solve-dependency.md) | Plate-solve dependency (closes #73) | 2026-09-12 | Accepted |
| [0002](0002-smart-telescope-sdk.md) | Smart-telescope SDK integration (closes #133) | 2026-09-12 | Accepted |
| [0003](0003-dp4-catalog-audit-checklist.md) | DP#4 catalog license audit close-out checklist | 2026-09-12 | Accepted |
| [0004](0004-cr07-comparison-primitive.md) | CR-07 ADR-07.1: Image Version Is the Comparison Primitive | 2026-09-12 | Accepted |
| [0005](0005-cr07-visual-comparison-primary.md) | CR-07 ADR-07.2: Visual Comparison Is Primary | 2026-09-12 | Accepted |
| [0006](0006-cr07-metrics-contextual.md) | CR-07 ADR-07.3: Metrics Are Contextual | 2026-09-12 | Accepted |
| [0007](0007-cr07-comparison-non-destructive.md) | CR-07 ADR-07.4: Comparison Is Non-Destructive | 2026-09-12 | Accepted |
| [0008](0008-cr07-provenance-always-available.md) | CR-07 ADR-07.5: Provenance Is Always Available | 2026-09-12 | Accepted |
| [0009](0009-cr07-ai-explicit-in-comparison.md) | CR-07 ADR-07.6: AI Processing Is Explicit in Comparison | 2026-09-12 | Accepted |
| [0010](0010-cr07-decision-user-owned.md) | CR-07 ADR-07.7: Decision Is User-Owned | 2026-09-12 | Accepted |
| [0011](0011-cr07-comparison-feedback-loop.md) | CR-07 ADR-07.8: Comparison Is a Feedback Loop | 2026-09-12 | Accepted |

## Workflow

1. When an architectural decision needs to be made, file an ADR
   under this directory. Use the next four-digit sequence number.
2. ADRs ship in the same PR as the code or paperwork change they
   justify (or in a follow-up if the decision is purely forward-
   looking).
3. Status transitions: `Proposed` → `Accepted` → (optionally)
   `Superseded by ADR-NNNN`.
4. Decisions that change user-visible behaviour also bump the spec
   (see `docs/specs/SPEC_INDEX.md`).

## Related

- Spec-driven development workflow: `docs/specs/SPEC_INDEX.md`
- Open decision points: `docs/PROJECT_PLAN.md` § Open Decision Points
- CR documents: `docs/CR-*.md`
