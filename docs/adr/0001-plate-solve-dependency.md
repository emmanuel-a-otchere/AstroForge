# ADR-0001 — Plate-Solve Dependency

- Status: Accepted (2026-09-12)
- Gates: `P2-M4-T1`, `P2-M4-T2`, `P2-M4-T3`, `P2-M4-T4`, `P2-M4-T5`, `P2-M4-T6`,
  `P2-M4-T7` (PROJECT_PLAN § Phase 2.4)
- Closes: GitHub issue #73 ("Plate-solve dependency")

## Context

AstroForge's Phase 2 plan calls for plate solving as the bridge between
captured images and a sky-coordinate reference frame (WCS). Plate solving
unblocks:

- Auto-crop-to-subject using WCS (P2-M4-T4)
- Annotated star map overlay (P2-M4-T5)
- Photometric calibration anchoring against APASS / Gaia (P2-M4-T6)
- WCS output to FITS header (P2-M4-T3)

Three options were on the table:

1. **ASTAP** — a small bundled binary with offline star catalogs.
2. **astrometry.net** — an online service with a large reference catalog.
3. **Defer** — no plate solving in v1.x.

## Decision

**Adopt ASTAP, bundled, offline.** astrometry.net is not adopted in v1.x.

### Why ASTAP

- **Offline-first.** The AstroForge spec (§2) names offline-first as a
  design constraint; the catalog network download path would either
  require a paid API key or an unauthenticated rate-limited endpoint.
- **Predictable latency.** ASTAP solves in a few seconds on a typical
  laptop with the bundled H17 catalog; astrometry.net is unbounded.
- **No per-user API keys.** ASTAP ships with the binary; the user does
  not need to register an account.
- **Open license.** ASTAP is freely redistributable.

### Why not astrometry.net

- Requires a per-deployment API key (or rate-limited anonymous access).
- Adds a network dependency to a spec that promises offline-first.
- Solves with higher accuracy on very wide fields, but the AstroForge
  use cases (deep-sky capture sets already centered on a target) don't
  need that accuracy.

### Why not defer

- The narrowband composition + recipe export paths from CR-05 + CR-06
  benefit from WCS for provenance + reproducibility even if the
  per-pixel cropping is not yet wired.
- Deferring beyond Phase 2 starts to compete with Phase 3 work.

## Consequences

- **Bundle footprint increases.** ASTAP adds roughly 30 MB to the
  install (binary + H17 catalog). Larger catalogs (H18, H19) are
  optional and offered as a download rather than bundled.
- **Operating-system matrix widens.** ASTAP ships per-platform (Windows
  / macOS / Linux). CI matrix expands by one job per platform.
- **Star catalog licensing.** ASTAP uses the H17 catalog from
  astrometry.net; redistribution terms need to be confirmed before
  the bundle ships (DP#4-adjacent audit item; not blocking the ADR
  but tracked alongside the model-license audit).
- **Fallback for failures.** Per P2-M4-T7, a failed solve must not
  break registration; the pipeline continues with the existing
  star-detection + cross-correlation path.

## Follow-ups

- Confirm H17 redistribution terms (DP#4-adjacent audit).
- File follow-up issue for H18 / H19 catalog packaging as a
  post-Phase-2 enhancement.
- Wire WCS output into the FITS export path (P2-M4-T3) so a solved
  image carries provenance forward into recipes and exports.
