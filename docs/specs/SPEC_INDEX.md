# AstroForge Specification Index

This document tracks all active and historical specifications for the AstroForge
project. Specifications follow semantic versioning and are the single source of
truth for the project's behavior, architecture, and feature set.

## Active Specification

| Version | File | Status | Date |
|---|---|---|---|
| **1.2.0** | [AstroForge_Spec_v1.2.0.md](./AstroForge_Spec_v1.2.0.md) | ✅ Active | 2026-09-11 |

> CR-06 landed in 1.2.0 (PRs #293–#299). The bump is backward-compatible:
> existing recipes remain valid. The 1.2.0 spec rolls CR-06 (AI Enhancement
> Studio) into the active spec; see the audit at [../M9_AUDIT.md](../M9_AUDIT.md)
> for the per-criterion acceptance walk-through.
>
> **CR-06 (resolved 2026-09-11):** AI Enhancement Studio & Intelligent Image
> Revamp. Status: Shipped (P1–P7). Target: AstroForge 1.2.0. Depends on
> CR-01–CR-05. Enables CR-07+ etc. See
> [../CR-06-AI-ENHANCEMENT-STUDIO.md](../CR-06-AI-ENHANCEMENT-STUDIO.md).
>
> **CR-06 P5.1 (in flight, 2026-09-12):** Real ONNX inference + tile execution +
> mask-aware apply. The dispatcher contract (P4 metadata shape) is unchanged;
> the engine inside the operation dispatch is swapped for `ort` (ONNX Runtime
> 1.28) running on bundled classical-kernel ONNX graphs. The §37
> `SegmentationLeakage` gate (P6 no-op) now compares real inside / outside
> region deltas. A new `ai_quality_reports` table persists every gate verdict.
> Bump target: 1.3.0 (spec file authored in this PR's M9 forward-look; see
> [../M9_AUDIT.md § Cross-cutting observations](../M9_AUDIT.md)).

## Historical Specifications

| Version | File | Status | Date | Notes |
|---|---|---|---|---|
| 1.1.0 | [AstroForge_Spec_v1.1.0.md](./AstroForge_Spec_v1.1.0.md) | 📦 Superseded | 2026-08-30 | Active until CR-06 landed; superseded by 1.2.0 |
| 1.0.0 | *(attachment, not on disk)* | 📦 Superseded | 2026-08-30 | Original draft; superseded by 1.1.0 |

## CR-05 follow-up audit

The current CR-05 UI and application workflow is audited in
[UI_WORKFLOW_AUDIT.md](../UI_WORKFLOW_AUDIT.md). That audit supersedes the
older 2026-09-05 tranche snapshot for evaluating current screen and menu
integration.


- **Status:** Implemented through P6; P7 verification and audit in progress
- **Location:** [CR-05](../CR-05-INTELLIGENT-PROCESSING.md)
- **Plan:** [CR-05 implementation plan](../plans/2026-09-07-cr05-intelligent-processing/PLAN.md)

| Document | Location | Purpose |
|---|---|---|
| **Living Project Plan** | [../PROJECT_PLAN.md](../PROJECT_PLAN.md) | Phased milestones, task breakdown, and progress tracking. Rebased frequently against actual work progress. |
| **Contributing Guide** | [../../CONTRIBUTING.md](../../CONTRIBUTING.md) | Spec-driven development workflow and governance rules. |

## Versioning Rules

Specifications follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (e.g., 1.0.0 → 2.0.0): Fundamental architecture, pipeline, or
  product-direction changes. Existing recipes or workflows may break.
- **MINOR** (e.g., 1.1.0 → 1.2.0): New stages, features, or capabilities added
  in a backward-compatible manner. Existing recipes remain valid.
- **PATCH** (e.g., 1.1.0 → 1.1.1): Clarifications, typo fixes, refinements to
  existing stages. No new features or behavioral changes.

## Spec-Driven Development Workflow

1. **Propose** a change by creating or updating a spec file in `docs/specs/`.
2. **Version** the spec according to the rules above. Update `SPEC_INDEX.md`.
3. **Update the project plan** in `docs/PROJECT_PLAN.md` to reflect any new or
   changed tasks resulting from the spec change.
4. **Review** the spec and plan changes before any code is written or modified.
5. **Implement** the codebase to match the approved spec — never the reverse.
6. **Verify** that the implementation matches the spec. If reality diverges,
   update the spec first, then the code.
7. **Rebase the project plan** after every completed milestone: mark tasks done,
   adjust remaining estimates, re-evaluate blocked items.
