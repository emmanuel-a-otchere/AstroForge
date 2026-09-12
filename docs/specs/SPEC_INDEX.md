# AstroForge Specification Index

This document tracks all active and historical specifications for the AstroForge
project. Specifications follow semantic versioning and are the single source of
truth for the project's behavior, architecture, and feature set.

## Active Specification

| Version | File | Status | Date |
|---|---|---|---|
| **1.3.0** | [AstroForge_Spec_v1.3.0.md](./AstroForge_Spec_v1.3.0.md) | 🟡 Pending bump (P5.1 + CR-07 + follow-ons landed; carrier authoring deferred — see [Open issue list](./SPEC_INDEX.md#open-issue-list)) | 2026-09-12 |
| **1.2.0** | [AstroForge_Spec_v1.2.0.md](./AstroForge_Spec_v1.2.0.md) | ✅ Active (becomes superseded on the 1.3.0 bump) | 2026-09-11 |

> **CR-06 (resolved 2026-09-11):** AI Enhancement Studio & Intelligent Image
> Revamp. Status: Shipped (P1–P7). Target: AstroForge 1.2.0. Depends on
> CR-01–CR-05. Enables CR-07+ etc. See
> [../CR-06-AI-ENHANCEMENT-STUDIO.md](../CR-06-AI-ENHANCEMENT-STUDIO.md).
>
> **CR-06 P5.1 (resolved, PR #307):** Real ONNX inference + tile execution +
> mask-aware apply. The dispatcher contract (P4 metadata shape) is
> unchanged; the engine inside the operation dispatch is swapped for `ort`
> (ONNX Runtime 1.28) running on bundled classical-kernel ONNX graphs.
> The §37 `SegmentationLeakage` gate (P6 no-op) now compares real
> inside / outside region deltas. A new `ai_quality_reports` table
> persists every gate verdict. Bump target: 1.3.0 (spec file authoring
> deferred; see Open issue list).
>
> **CR-07 + follow-ons (resolved, PRs #308 + #309 + #310):** Zone B canvas
> + compare surfaces + WebGL back-end. Adds `read_image_artifact` Tauri
> command, `ImageCanvas` rendering component, `CompareTools` split /
> blink / difference surfaces, plus a self-contained WebGL shader path
> with Canvas 2D fallback for > 4 MP renders. Bump target: 1.4.0
> (carrier authoring deferred — see Open issue list).
>
> **Forward-look slice (in flight, 2026-09-12):** DP#4 license verification
> (`OnnxEngine::open_catalog` is now digest-pinned against `CATALOG_MODELS`;
> unpinned entries carry the `UNVERIFIED_SHA256` sentinel and fail
> closed), `SessionCache` for session-reuse across apply rounds, plus
> two ADRs (`docs/adr/0001` plate-solve / `0002` smart-telescope SDK)
> that close decision-points #73 and #133 from the Open Decision Points
> table in PROJECT_PLAN.

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
| **ADRs** | [../adr/README.md](../adr/README.md) | Architecture Decision Records (plate-solve, smart-telescope SDK, …). |

## Open issue list

These items remain unresolved as of the last spec reconciliation.
Each lives as a follow-up slice on top of the closed tranche
programmes; none blocks a current tranche.

| # | Item | Origin | Slice |
|---|---|---|---|
| 1 | Author `AstroForge_Spec_v1.3.0.md` (carrier) | `SPEC_INDEX.md` 🟡 Pending bump entry | Forward-look (docs-only) |
| 2 | Author `AstroForge_Spec_v1.4.0.md` (carrier) | `SPEC_INDEX.md` 1.4.0 target | Forward-look (docs-only) |
| 3 | DP#4 catalog digest pinning for the 7 real-catalog models (SwinIR etc.) | `M9_AUDIT.md` forward-look; #135 | Slice per model as licenses + hashes land |
| 4 | GPU execution providers (CUDA / DirectML / Metal) | `M9_AUDIT.md` P5.2 | Per-platform compile-time features; CI matrix expansion |
| 5 | Plate-solve dependency (ADR-0001 — ASTAP bundled) | #73 | Phase 2.4 |
| 6 | Smart-telescope SDK decision (ADR-0002 — file-only v1.x; plugin Phase 4) | #133 | Phase 4 plugin API |

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
