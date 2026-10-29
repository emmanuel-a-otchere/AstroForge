# AstroForge — Recon Tranche 2026-10: Ship-Readiness Plan

**Date:** 2026-10 (post-recon)
**Status:** Proposed
**Milestone:** "Recon Tranche 2026-10 — Ship Readiness" (milestone #4)
**Scope:** Concrete plan for the gaps, improvements, and enrichment ideas identified
in the 2026-10 repository recon. Each workstream below is tracked by a GitHub
issue carrying the `roadmap-recon` label and placed in the milestone.

This tranche is intentionally **not** a new CR against the processing engine. It
is a ship-readiness tranche: everything below already traces to existing spec
material (Phase 3 M3 packaging, §constraints 4–8 GB RAM, community/contribution
goals in the README and spec §community) or is pure engineering hygiene
(CI honesty, dead code, doc sprawl). Per the spec-driven rule in
`PROJECT_PLAN.md`, no task here introduces new product behavior without a spec
carrier; the enrichment ideas (§5) are pre-gated on spec acceptance.

---

## 1. Where the project stands (recon summary)

Shipped: CR-02 through CR-09 complete; CR-10 (node-based editor) mid-flight
(P0 + P1.6.1.x + P1.6.2.1–4 landed; free-form canvas, Graph↔Recipe round-trip,
mutation IPCs, and the `crop` / `star_handling` / `creative_polish` stage
handlers pending). Engine is deep (~65k LOC Rust core, 45+ integration test
files), tests run in CI, but the project cannot yet be installed by a user:
no packaging/release pipeline, unresolved license posture, empty Getting
Started, and CI that silently ignores frontend type errors.

## 2. Workstreams (tracked issues)

| # | Workstream | Label | Priority | Est. | Issue |
|---|---|---|---|---|---|
| W1 | CI hardening (honest checks, vitest, OS matrix) | ci | P0 | S | [RT-1](https://github.com/emmanuel-a-otchere/AstroForge/issues/419) |
| W2 | Release engineering slice (tauri-action, installers, v0.1.0) | release-engineering | P0 | M | [RT-2](https://github.com/emmanuel-a-otchere/AstroForge/issues/420) |
| W3 | License close-out (project + per-model, resolves #135/#316) | decision, license, legal | P0 | S | [RT-3](https://github.com/emmanuel-a-otchere/AstroForge/issues/421) |
| W4 | README Getting Started + onboarding docs | documentation | P0 | S | [RT-4](https://github.com/emmanuel-a-otchere/AstroForge/issues/422) |
| W5 | HOUSE-1 wizard dead-code removal | housekeeping | P1 | S | [RT-5](https://github.com/emmanuel-a-otchere/AstroForge/issues/424) |
| W6 | HOUSE-2 ingest/import_scan consolidation | housekeeping | P1 | M | [RT-6](https://github.com/emmanuel-a-otchere/AstroForge/issues/425) |
| W7 | Real AI model integration (SwinIR denoise + super-res ONNX) | ai, onnx | P1 | L | [RT-7](https://github.com/emmanuel-a-otchere/AstroForge/issues/426) |
| W8 | GPU execution providers for `ort` (P5.2) | ai, hardware | P1 | M | [RT-8](https://github.com/emmanuel-a-otchere/AstroForge/issues/427) |
| W9 | Issue-tracker reconciliation (stale P1.5 issues) | triage | P1 | S | [RT-9](https://github.com/emmanuel-a-otchere/AstroForge/issues/423) |
| W10 | Recipe gallery static index (#119 decision) | recipe | P2 | S | [RT-10](https://github.com/emmanuel-a-otchere/AstroForge/issues/428) |
| W11 | Sample dataset bundle for contributors | community, testing | P2 | M | [RT-11](https://github.com/emmanuel-a-otchere/AstroForge/issues/429) |
| W12 | Plate-solve dependency decision + annotation feature spike | plate-solving, decision | P2 | M | [RT-12](https://github.com/emmanuel-a-otchere/AstroForge/issues/430) |
| W13 | Session quality dashboard (metrics surface) | quality, ui | P2 | M | [RT-13](https://github.com/emmanuel-a-otchere/AstroForge/issues/431) |
| W14 | i18n + accessibility scoping (#130–132) | accessibility, i18n | P3 | M | [RT-14](https://github.com/emmanuel-a-otchere/AstroForge/issues/432) |

Priorities: **P0** = blocks any user-facing ship; **P1** = blocks a *credible*
v0.1 ship or engineering integrity; **P2** = enriches the product, spec-gated;
**P3** = community/global reach, can slip past v0.1.

## 3. Execution order

```
Wave 1 (hygiene + unblock):  W1 → W2, W3, W4, W9
Wave 2 (integrity):          W5, W6
Wave 3 (AI credibility):     W3 gates W7; W8 after W7
Wave 4 (enrichment, spec-gated): W10, W11, W12, W13
Wave 5 (reach):              W14
```

Rules:

- CR-10's remaining slices continue on their own track and are **not** part of
  this tranche; this plan defers to `docs/plans/2026-09-28-cr10-node-based-editor/PLAN.md`.
  The tranche issues must not touch CR-10 in-flight files.
- One workstream = one PR-sized slice series, following the repo's established
  one-slice-per-PR cadence (as used by CR-08/CR-10).
- Each issue links its spec carrier where one exists; enrichment ideas (W10–W13)
  require a spec delta note before implementation starts.

## 4. Wave detail

### Wave 1 — Hygiene & unblock (P0)

**W1 CI hardening.** Remove `|| true` from the frontend type-check step;
add `npm run test:coverage` (vitest) as a required CI job; add macOS + Windows
jobs running at least `cargo test -p astroforge-core` + `cargo clippy` so
per-platform image-math and Tauri breakage surfaces early. Acceptance: red CI
on a deliberately broken type; green on main across all three OSes.

**W2 Release engineering.** `release.yml` using `tauri-action` producing
installers (Windows NSIS/MSI, macOS app+dmg, Linux AppImage/deb) on tag push;
draft a `v0.1.0` tag once W3 lands. Acceptance: a `v0.1.0-rc.1` tag produces
downloadable artifacts on all three platforms.

**W3 License close-out.** Decide project license (README says TBD while
`Cargo.toml` declares `MIT OR Apache-2.0` — reconcile), write `LICENSE` files,
and complete the per-model catalog license + digest pinning audit (closes #135,
supersedes #316's remaining scope). Gates W2's non-rc release and W7.

**W4 Getting Started.** Fill README's Getting Started section: prerequisites
per OS, `npm install`, `npm run tauri dev`, test commands, link to
`docs/ORIENTATION.md`. Acceptance: a clean-machine contributor can build.

**W9 Issue-tracker reconciliation.** Audit the 36 open issues against the
shipped CR-02…CR-10 state; close/comment issues superseded by shipped CR
machinery (notably the P1.5 M2–M8 series); retag remainder into this plan's
labels. Acceptance: issue count reflects reality; each open issue maps to a
live plan or this tranche.

### Wave 2 — Engineering integrity (P1)

**W5 HOUSE-1.** Land the dead-code removal per the disposal rule in
`docs/HOUSEKEEPING.md` (~2,000 LOC: wizard sidebars, `gl-renderer.ts`, wizard
stores). Single PR. Acceptance: grep-based zero-consumer proof documented in the
PR; CI green.

**W6 HOUSE-2.** Consolidate `ingest.rs` into the canonical `import_scan.rs`
path per HOUSEKEEPING.md, with the parity migration test (identical
SourceAsset records through both paths) as the gating check.

### Wave 3 — AI credibility (P1, W3-gated)

**W7 Real AI models.** Bundle a quantized SwinIR-class denoise model and a
lightweight super-res model behind the existing P5.1 dispatcher contract
(tile execution, mask-aware apply, §37 SegmentationLeakage gate). Requires the
W3 per-model license manifest. Acceptance: denoise stage produces real output
on a sample dataset (W11) within the 4–8 GB RAM budget.

**W8 GPU providers.** Extend `gpu_providers.rs` with platform execution
providers (CoreML/DirectML/CUDA) behind runtime capability detection, keeping
CPU fallback. Spec carrier: CR-06 P5.2.

### Wave 4 — Enrichment (P2, spec-gated before implementation)

**W10 Recipe index.** Resolve #119 with the static in-repo JSON index option
(`recipes/index.json` + schema + validation in CI), unblocking community recipe
sharing without hosting infrastructure.

**W11 Sample dataset bundle.** A small, licensed sample dataset (Seestar/Dwarf
session) shipped via release assets or Git LFS so contributors can run the
pipeline end-to-end; also the W7 acceptance vehicle.

**W12 Plate-solve decision + spike.** Close ADR-0001's open decision, then
implement plate-solved annotation overlay (target name, FOV, auto-crop-to-target
spike) on the existing `plate_solve.rs` + CR-07 canvas.

**W13 Session quality dashboard.** Surface the existing
`metric_registry.rs` / `streaming_metrics.rs` outputs (FWHM, eccentricity,
SNR per frame) as a per-session trends view with outlier flags feeding the
import recommendation engine.

### Wave 5 — Reach (P3)

**W14 i18n/a11y.** Close #130 (WCAG scope decision), then scaffold i18n and
keyboard/screen-reader support per #131–132. Only after v0.1 ships.

## 5. Explicitly deferred

- CLI/headless mode (Phase 3+), plugin architecture (Phase 4), model
  fine-tuning datasets (#129) — unchanged in `PROJECT_PLAN.md` phases; this
  tranche does not re-plan them.
- CR-10 completion and CR-11 (recipe application UX) — remain on the CR-10
  track; this plan only requires tranche issues to not collide with it.

## 6. Kanban / project-board mapping

Each tracked issue carries `roadmap-recon` + a priority label mapping to the
kanban columns: `status:planned` → Backlog; assignee + in-progress label →
In Progress; linked PR → In Review; closed → Done. The GitHub Projects board
("Roadmap") can be created with Status = Todo/In Progress/In Review/Done and
Priority = P0/P1/P2/P3 fields; `gh`-based automation is blocked for the current
token scope (Projects v2 GraphQL not accessible), so issues are structured for
one-command board ingestion once a project exists.

## 7. Definition of done (tranche)

1. All P0 issues closed: CI honest on 3 OSes, installers build on tag, license
   files present with per-model manifest, Getting Started complete.
2. P1 issues closed or explicitly re-planned with reasons recorded.
3. Issue tracker reconciled (W9) with no orphaned pre-CR-02 milestone issues.
4. `PROJECT_PLAN.md` and `CHANGELOG.md` updated to reflect the tranche.
