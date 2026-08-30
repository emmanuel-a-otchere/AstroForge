# AstroForge Specification Index

This document tracks all active and historical specifications for the AstroForge
project. Specifications follow semantic versioning and are the single source of
truth for the project's behavior, architecture, and feature set.

## Active Specification

| Version | File | Status | Date |
|---|---|---|---|
| **1.1.0** | [AstroForge_Spec_v1.1.0.md](./AstroForge_Spec_v1.1.0.md) | ✅ Active | 2026-08-30 |

## Historical Specifications

| Version | File | Status | Date | Notes |
|---|---|---|---|---|
| 1.0.0 | *(attachment, not on disk)* | 📦 Superseded | 2026-08-30 | Original draft; superseded by 1.1.0 |

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
3. **Review** the spec change before any code is written or modified.
4. **Implement** the codebase to match the approved spec — never the reverse.
5. **Verify** that the implementation matches the spec. If reality diverges,
   update the spec first, then the code.
