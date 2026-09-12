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
