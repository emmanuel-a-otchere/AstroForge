# AstroForge Specification Index

This document tracks all active and historical specifications for the AstroForge
project. Specifications follow semantic versioning and are the single source of
truth for the project's behavior, architecture, and feature set.

## Active Specification

| Version | File | Status | Date |
|---|---|---|---|
| **1.4.0** | [AstroForge_Spec_v1.4.0.md](./AstroForge_Spec_v1.4.0.md) | ✅ Active | 2026-09-12 |
| **1.3.0** | [AstroForge_Spec_v1.3.0.md](./AstroForge_Spec_v1.3.0.md) | 📦 Superseded | 2026-09-12 |

> **CR-06 (resolved 2026-09-11):** AI Enhancement Studio & Intelligent Image
> Revamp. Status: Shipped (P1–P7). Target: AstroForge 1.2.0. Depends on
> CR-01–CR-05. Enables CR-07+ etc. See
> [../CR-06-AI-ENHANCEMENT-STUDIO.md](../CR-06-AI-ENHANCEMENT-STUDIO.md).
>
> **CR-06 P5.1 (resolved, PR #307, 2026-09-12):** Real ONNX inference + tile execution +
> mask-aware apply. The dispatcher contract (P4 metadata shape) is unchanged;
> the engine inside the operation dispatch is swapped for `ort` (ONNX Runtime
> 1.28) running on bundled classical-kernel ONNX graphs. The §37
> `SegmentationLeakage` gate (P6 no-op) now compares real inside / outside
> region deltas. A new `ai_quality_reports` table persists every gate verdict.
> Carried into [AstroForge_Spec_v1.3.0.md § Delta from 1.2.0](./AstroForge_Spec_v1.3.0.md#delta-from-120).
>
> **CR-07 + follow-ons (resolved, PRs #308 + #309 + #310, 2026-09-12):** Zone B canvas
> + compare surfaces + WebGL back-end. Adds `read_image_artifact` Tauri
> command, `ImageCanvas` rendering component, `CompareTools` split /
> blink / difference surfaces, plus a self-contained WebGL shader path
> with Canvas 2D fallback for > 4 MP renders. Carried into
> [AstroForge_Spec_v1.3.0.md § Delta from 1.2.0](./AstroForge_Spec_v1.3.0.md#delta-from-120).
>
> **Forward-look slice (resolved, PR #311, 2026-09-12):** DP#4 license verification
> (`OnnxEngine::open_catalog` is digest-pinned against `CATALOG_MODELS`;
> unpinned entries carry the `UNVERIFIED_SHA256` sentinel and fail
> closed), `SessionCache` for session-reuse across apply rounds, plus
> two ADRs (`docs/adr/0001` plate-solve / `0002` smart-telescope SDK)
> that close decision-points #73 and #133 from the Open Decision Points
> table in PROJECT_PLAN. Carried into [AstroForge_Spec_v1.4.0.md § Delta from 1.3.0](./AstroForge_Spec_v1.4.0.md#delta-from-130).
>
> **Spec carrier authoring (resolved, 2026-09-12):**
> [AstroForge_Spec_v1.3.0.md](./AstroForge_Spec_v1.3.0.md) (carrier from
> 1.1.0 with a **Delta from 1.2.0** section linking to canonical CR
> documents) + [AstroForge_Spec_v1.4.0.md](./AstroForge_Spec_v1.4.0.md)
> (carrier from 1.3.0 with a **Delta from 1.3.0** section). Each carrier
> preserves the previous carrier's full content unchanged; the delta
> section links to canonical CR documents rather than re-authoring prose.

## Historical Specifications

| Version | File | Status | Date | Notes |
|---|---|---|---|---|
| 1.3.0 | [AstroForge_Spec_v1.3.0.md](./AstroForge_Spec_v1.3.0.md) | 📦 Superseded | 2026-09-12 | First formal delta carrier (CR-06 P5.1 + CR-07 + ADRs + DP#4) |
| 1.2.0 | *(delta commit, not on disk)* | 📦 Superseded | 2026-09-11 | CR-06 (AI Enhancement Studio) delta; bumped in 1.3.0 carrier |
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
| **ADRs** | [../adr/README.md](../adr/README.md) | Architecture Decision Records (plate-solve, smart-telescope SDK, …). |

## Open issue list

These items remain unresolved as of the last spec reconciliation.
Each lives as a follow-up slice on top of the closed tranche
programmes; none blocks a current tranche.

| # | Item | Origin | Slice |
|---|---|---|---|
| 1 | DP#4 catalog digest pinning for the 7 real-catalog models (SwinIR etc.) — each one-line table update when upstream publishes hash + license | M9_AUDIT forward-look; #135 | Per-model close-out (see `docs/adr/0003-dp4-catalog-audit-checklist.md`) |
| 2 | GPU execution provider runtime validation — per-platform CI runners (one job per `gpu-*` feature × OS) | M9_AUDIT P5.2; PR #315 | Per-platform CI matrix expansion |
| 3 | Plate-solve integration on top of ADR-0001 (ASTAP bundled + offline) | #73 (ADR-0001) | Phase 2.4 |
| 4 | Smart-telescope SDK plugin on top of ADR-0002 (file-only v1.x contract) | #133 (ADR-0002) | Phase 4 plugin API |

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
