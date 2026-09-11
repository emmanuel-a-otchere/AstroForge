# CR-06 Implementation Plan — AI Enhancement Studio & Intelligent Image Revamp

**Date:** 2026-09-10
**Source CR:** [CR-06 — AI Enhancement Studio & Intelligent Image Revamp](../../CR-06-AI-ENHANCEMENT-STUDIO.md) (Status: Shipped P1–P7 (PRs #292–#299); spec bump 1.1.0 → 1.2.0, Priority: Critical)
**Strategy:** Phase-decomposed into 8 PRs (P0–P7), following the CR-02 / CR-03 / CR-04 / CR-05 conventions. Each phase is one PR-sized tranche; each tranche is independently reviewable and CI-green before the next starts. P0 is documentation-only (this PR).

## Why a phased rollout

CR-06 is the largest CR in the AstroForge programme: 41 sections, 9 ADRs, 35 acceptance criteria, 8 new persistent entities (`image_analysis`, `image_region`, `ai_recommendation`, `ai_operation`, `ai_model_reference`, `ai_mask`, `enhancement_stack`, `enhancement_preview`), a new workspace UI layered on top of CR-03/CR-05, an image-analysis engine, a recommendation engine, a region/mask system, a quality-gate loop, and the safety classification (Deterministic / Perceptual / Generative) that must be visible everywhere. A big-bang landing would be unreviewable and would block CR-07 (Compare), CR-09 (Resource Intelligence), CR-16 (Model Hub UX), CR-21 (Export) for the duration.

The phased structure keeps each PR small enough to reason about end-to-end and keeps the catalogue of in-flight PRs honest. The eight phases follow the same skeleton that worked for CR-05:

- **P0** — documentation + scope freeze (this PR)
- **P1** — data model + provenance + AI safety classification
- **P2** — image-analysis engine
- **P3** — recommendation engine + intelligent ordering
- **P4** — AI enhancement operations + enhancement stack
- **P5** — region-aware masks (auto, parametric, user, composite)
- **P6** — quality gates + branching
- **P7** — audit + DoD test + CR-06 acceptance

## Current state (as of 1a7cd3c, post-audit R1–R7 landing)

What already exists in `crates/astroforge-core`:

- `domain.rs` — Project / Target / Session / SourceAsset / Artifact / ImageVersion / PipelineRun / StageRunRecord / AiOperation / Export / ProjectEvent (CR-02 substrate).
- `domain_store.rs` — SQLite persistence with migrations.
- `artifact_store` / `artifact.rs` — content-addressed filesystem storage.
- `pipeline.rs`, `pipeline_run.rs`, `session.rs`, `export.rs`, `mvp_pipeline.rs` — CR-02 wizard substrate.
- `db.rs` — `SESSION_SCHEMA_SQL` and the migration runner.

What already exists in `crates/astroforge-ai`:

- `hub.rs` — `model_catalog()` returns the canonical 7-model registry (`astroforge-ai/src/hub.rs`).
- `registry.rs`, `hardware.rs` — model registry primitives and hardware probing.

What already exists in `src/` (Svelte):

- `CompareWorkspace.svelte` (R3) — metadata cards per Image Version, no pixel rendering yet.
- `VersionTimeline.svelte` (R3) — sequence labels derived from the real IPC.
- `RecipesScreen.svelte`, `AiModelsScreen.svelte`, `SettingsScreen.svelte`, `HelpScreen.svelte` (R1) — application-level surfaces.
- `OverviewWorkspace.svelte` (R2) — truthful §8 checklist driven by durable events.
- `ProcessingControls.svelte` — recovery panel with `Adjust processing` / `Contact support` placeholders (R7 notice surfaced).
- `project-lifecycle.ts` (R4) — single source of truth for project open / close.

What does NOT yet exist:

- A canonical `image_analysis` / `image_region` / `ai_recommendation` / `ai_operation` / `ai_mask` / `enhancement_stack` / `enhancement_preview` schema (CR-06 §26).
- An image-analysis engine that produces structured observations with evidence + confidence (CR-06 §5, §6).
- A recommendation engine that maps observations → recommendations with rationale and resource estimates (CR-06 §7, §27).
- Region-aware masks (auto / parametric / user / composite) (CR-06 §12).
- A safety classification (Deterministic / Perceptual / Generative) (CR-06 §16) — note that the `AiOperation` row in `domain.rs` already has a `safety_classification` field, but no UI surfaces it.
- An AI enhancement workspace with the three-zone Studio layout (CR-06 §13).
- AI operation provenance with model hash, seed, deterministic class, tile configuration (CR-06 §21).
- An AI quality-gate loop that validates outcomes against noise / halos / clipping (CR-06 §37).

## Phase breakdown

### P0 — Documentation + scope freeze (this PR)

Deliverables:

- [x] `docs/CR-06-AI-ENHANCEMENT-STUDIO.md` — the spec itself.
- [x] `docs/specs/SPEC_INDEX.md` — cross-link to the proposed CR.
- [x] `docs/PROJECT_PLAN.md` — add Phase 5 with milestone preview + changelog entry.
- [x] This plan document.
- [x] An M9 audit doc stub at `docs/M9_AUDIT.md` (filled at P7) so the cross-ref pattern from CR-05 is in place from day one.

Verify:

- Specs / plans / changelog all reference each other.
- No code change yet.

### P1 — Data model + provenance + AI safety classification

Goal: extend `astroforge-core` with the new entities from CR-06 §26 and the provenance fields from §21, and surface the safety classification in the existing `AiOperation` row.

Tasks:

1. Add `image_analysis`, `image_region`, `ai_recommendation`, `ai_operation` (CR-06 §21 enriched — keep existing `AiOperation` row shape, add `model_hash`, `seed`, `deterministic_class`, `tile_configuration`, `engine_version`, `resource_metrics`), `ai_model_reference`, `ai_mask`, `enhancement_stack`, `enhancement_preview` to `domain.rs`.
2. Add migrations to `db.rs` (idempotent — additive only).
3. Add CRUD + query helpers to `domain_store.rs` for the new entities.
4. Extend `AiOperation` row with the new fields; existing reads tolerate missing columns for backwards compatibility (the column-add migration runs on next open).
5. New Tauri commands in `src-tauri/src/commands_ai_analysis.rs`, `commands_ai_recommendations.rs`, `commands_ai_operations.rs`, `commands_ai_masks.rs`. Register in `main.rs`.
6. TS wrapper types in `src/lib/astroforge-api.ts`.
7. State stores in `src/state/ai-analysis.ts`, `src/state/ai-recommendations.ts`, `src/state/ai-masks.ts`, `src/state/enhancement-stack.ts`.
8. Add new project-scoped writables to `project-lifecycle.ts` reset path.
9. Tests in `crates/astroforge-core/tests/image_analysis.rs` (CRUD round-trip), `ai_operations.rs` (provenance shape), `ai_masks.rs` (mask provenance).

Verify: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test -p astroforge-core`, `npm run check`, `npm run build`, `bash scripts/mvp_smoke.sh tests/fixtures/sample-session` all green.

### P2 — Image analysis engine

Goal: produce structured observations for an Image Version with evidence + confidence.

Tasks:

1. `crates/astroforge-core/src/image_analysis/mod.rs` — analysis primitives: noise (sigma-clipping), background (per-tile median), stars (segmentation / FWHM), clipping (luminance histogram tails), chromatic noise (channel difference), local contrast (Sobel / Laplacian).
2. `crates/astroforge-core/src/image_analysis/structures.rs` — astronomical structure detection: star fields, nebula, galaxy, core, faint structures. Heuristic first; ML hooks in P5.
3. `crates/astroforge-core/src/image_analysis/defects.rs` — hot pixels, satellite trails, registration residuals, edge artifacts, walking noise.
4. `crates/astroforge-core/src/image_analysis/report.rs` — `ImageAnalysisReport` aggregation with per-observation `evidence` (the WCS coordinates or pixel ranges that support the observation) and `confidence` (0.0–1.0).
5. Tauri command `analyze_image` / `get_image_analysis` (CR-06 §28).
6. Frontend surface: an `ImageAnalysisPanel.svelte` component in CR-06's Zone C, with observation rows (Background / Noise / Stars / Nebula / Clipping / Detail) and confidence bars.
7. Tests: deterministic analysis on the bundled `tests/fixtures/sample-session` (no GPU), pinned outputs (assert SNR > 0, background gradient slope in expected range, etc.).

Verify: same gates as P1, plus a visual-corpus check on 3 reference datasets if/when we have them (P2 records any corpus gaps; P7 fills them).

### P3 — Recommendation engine + intelligent ordering

Goal: turn observations into structured recommendations with rationale + resource estimates.

Tasks:

1. `crates/astroforge-core/src/recommendations/mod.rs` — `RecommendationEngine` that walks the analysis report and emits a list of `Recommendation` rows (CR-06 §27).
2. `crates/astroforge-core/src/recommendations/ordering.rs` — sequence-detection (e.g., denoise-before-detail ordering; flag "detail before denoise" as sub-optimal).
3. `crates/astroforge-core/src/recommendations/resource_estimate.rs` — read model memory floor + tile config; emit `estimated_runtime`, `estimated_memory`.
4. Tauri command `get_ai_recommendations` (CR-06 §28).
5. Frontend surface: `RecommendationList.svelte` with confidence badges, expected-effect, risk-level, ordering rationale on hover.
6. Tests: a known analysis report produces a known recommendation ordering (no GPU needed).

Verify: same gates as P1, plus the recommendation-engine unit test against synthetic observations.

### P4 — AI enhancement operations + enhancement stack

Goal: the actual AI operations that produce new Image Versions, plus the stack that orders them.

Tasks:

1. `crates/astroforge-ai/src/operations/denoise.rs` — generic ONNX denoise dispatch (model-loaded lazily).
2. `crates/astroforge-ai/src/operations/deconv.rs` — deconvolution hooks (CPU default; tiled inference with overlap blending).
3. `crates/astroforge-ai/src/operations/star.rs` — star sharpening / size normalization / star reduction.
4. `crates/astroforge-ai/src/operations/sr.rs` — 2× super-resolution (perceptual class; explicit warning carried in provenance).
5. `crates/astroforge-ai/src/operations/inpaint.rs` — region-restricted inpainting (no default-unrestricted mode).
6. `crates/astroforge-core/src/enhancement/stack.rs` — enhancement stack: ordered list of operations; supports `reorder`, `disable`, `remove`, `re-preview`, `branch`.
7. `crates/astroforge-core/src/enhancement/preview.rs` — preview artifacts (temporary, not surfaced as Image Versions).
8. Tauri commands: `preview_ai_operation`, `apply_ai_operation`, `create_enhancement_branch`, `compare_ai_result`, `reorder_ai_operations` (CR-06 §28).
9. Frontend: `EnhancementStudio.svelte` (Zone A/B/C per CR-06 §13), `EnhancementOperationCard.svelte` (per-operation card per §14), `BeforeAfterCompare.svelte` (preview canvas).
10. Frontend: `Apply` always creates a new `ImageVersion` (non-destructive — CR-06 §4, §22).
11. Tests: deterministic stack reorder, branch creation, preview-vs-final artifact difference.

Verify: same gates as P1, plus new tests; visual regression deferred to P2 (any visual corpus gaps are recorded).

### P5 — Region-aware masks

Goal: AI operations can target semantic regions.

Tasks:

1. `crates/astroforge-core/src/masks/auto.rs` — auto-generated masks from AI segmentation (stars / nebula / galaxy / background / core).
2. `crates/astroforge-core/src/masks/parametric.rs` — parametric masks from astrophotography properties (star brightness, size, luminance, saturation).
3. `crates/astroforge-core/src/masks/user.rs` — user-defined masks (brush / gradient / polygon / radial).
4. `crates/astroforge-core/src/masks/composite.rs` — boolean composition of multiple masks (`Nebula + Exclude Stars + Protect Core`).
5. Mask renderer for the canvas (overlay with adjustable opacity; CR-06 §13 Zone B).
6. Tauri commands: `create_ai_mask`, `update_ai_mask`, `list_ai_masks`.
7. Frontend: `MaskEditor.svelte`, `MaskOverlay.svelte` (canvas overlay).
8. Tests: composition semantics (`A ∪ B`, `A ∩ B`, `A − B`); auto-mask determinism on the bundled fixture.

Verify: same gates as P1; mask composition is pure-Rust so no GPU gating.

### P6 — Quality gates + branching

Goal: the post-operation validation loop from CR-06 §37, and the branching UX from §24.

Tasks:

1. `crates/astroforge-core/src/quality/gates.rs` — validate outcome against clipping / noise amplification / star artifacts / halos / ringing / color shifts / segmentation leakage.
2. `crates/astroforge-core/src/quality/report.rs` — `QualityGateReport` returned alongside each applied operation.
3. Tauri command `get_ai_quality_report`.
4. Frontend: `QualityGatePanel.svelte` shown after each `apply_ai_operation`. Warnings are visible; the user accepts / reduces strength / re-runs.
5. Branching UX: `create_enhancement_branch` returns a new active Image Version; the UI surfaces both branches in CompareWorkspace.
6. Tests: gate fires on a deliberately bad operation result; gate passes on a neutral result.

Verify: same gates as P1; the gate loop runs only on test fixtures (no GPU required for a deterministic synthetic bad outcome).

### P7 — Audit, DoD test, CR-06 acceptance

Goal: the standing audit pattern (M8 / M9 audit docs) and a DoD integration test that pins the full CR-06 workflow.

Tasks:

1. `docs/M9_AUDIT.md` — audit of CR-06 P1–P6, with named gaps (DoD evidence, visual corpus, model integrity, etc.).
2. `crates/astroforge-core/tests/dod_enhancement.rs` — integration test that walks the full CR-06 §40 DoD: analyze → recommend → preview → apply → branch → quality gate → image version with provenance. Mirrors `dod_workflow.rs`.
3. UI workflow audit for the new Enhancement Studio surface (likely a follow-up to R1–R7; called out in M9 audit).
4. CR-06 acceptance walk-through — every box in §35 checked, with PR link or recorded gap.
5. CHANGELOG entry; SPEC_INDEX bump candidate (CR-06 will warrant a 1.2.0 minor or 2.0.0 major; the bump lands in this PR or in a sibling).

Verify: same gates as P1, plus the DoD integration test, plus visual regression if any corpus is available.

## Cross-cutting decisions

### Trust boundary (CR-06 §3, §16, §17, §21, §22)

CR-06's central principle — "AI may enhance an astronomical image, but it must never silently redefine the astronomical evidence represented by that image" — is enforced in code at three layers:

- **Data model:** every `AiOperation` row carries `safety_classification` (Deterministic / Perceptual / Generative), `model_id`, `model_version`, `model_hash`, `seed`, `parameters`, `tile_configuration`. P1.
- **UI:** the safety classification is visible at every interaction surface — operation card, applied-version header, export receipt. P4.
- **API:** `preview_ai_operation` always returns the safety classification alongside the preview; `apply_ai_operation` records it in provenance. P4.

A regression test in P7 walks a Perceptual operation end-to-end and asserts the provenance row matches the operation card the user saw.

### Resource intelligence (CR-06 §19, §20)

P3 wires `resource_estimate.rs` to the model registry's memory floors. P4 ensures the engine refuses to start an operation that exceeds the configured budget (or downgrades to a smaller tile / INT8 model with a clear notice). P6 quality gates flag operations that ran close to the memory budget (so users see when a model was scaled down).

### Non-destructive operation (CR-06 §4, §22, §24, §25)

Every AI operation creates a new `ImageVersion`. The previous version is never modified. Branching creates a new active version while preserving the parent. The `project-lifecycle.ts` reset path (added in P1) covers all the new project-scoped stores.

### Memory budget (CR-06 §20)

P4's operations estimate memory before starting and tile by default. P6's quality gates flag operations that ran close to the memory budget. P7 records any "exceeds budget" events as a DoD evidence item.

## Scope boundaries

- **Models:** the 7-model `astroforge-ai` catalog is the registry of record for P1–P6. New models land via the existing `crates/astroforge-ai/src/registry.rs` path; CR-16 (Model Hub UX) covers the surface.
- **Compare:** CR-06 creates the alternatives; CR-07 will provide the dedicated comparison experience. P4 wires `compare_ai_result` to the existing `CompareWorkspace.svelte`; the dedicated Compare UX lands in CR-07.
- **Plugin architecture:** CR-06 operations are first-class Rust traits under `crates/astroforge-ai/src/operations/`. Plugin-style extensibility is CR-14 / Phase 4 territory and is out of scope.
- **Planetary / lunar:** CR-06's analysis + recommendations are image-content-driven, so they should work for planetary / lunar content too — but per-dataset validation is deferred to whichever CR owns those pipelines (CR-04 lunar, M3 planetary).

## Acceptance criteria mapping

CR-06 §35's checklist maps onto phases as follows:

| Section | Criterion | Phase |
|---|---|---|
| AI analysis | can analyze an Image Version | P2 |
| AI analysis | image characteristics identified | P2 |
| AI analysis | regions identified | P2 + P5 |
| AI analysis | confidence exposed | P2 |
| AI analysis | evidence exposed | P2 |
| Recommendations | generated from analysis | P3 |
| Recommendations | explain rationale | P3 |
| Recommendations | include confidence | P3 |
| Recommendations | resource estimates | P3 |
| Recommendations | operation ordering | P3 |
| Enhancement | AI denoise through framework | P4 |
| Enhancement | AI detail enhancement | P4 |
| Enhancement | star enhancement / reduction | P4 |
| Enhancement | tiled SR | P4 |
| Enhancement | controlled inpainting | P4 |
| Enhancement | operations use masks | P5 |
| UX | image-first Studio | P4 |
| UX | preview before apply | P4 |
| UX | before/after compare | P4 |
| UX | split compare | P4 |
| UX | understandable without tech knowledge | P4 |
| UX | expert controls via progressive disclosure | P4 |
| Trust | classifications visible | P1 + P4 |
| Trust | perceptual labeled | P4 |
| Trust | generative opt-in | P4 |
| Trust | model identity recorded | P1 + P4 |
| Trust | model hash recorded | P1 + P4 |
| Trust | parameters recorded | P1 + P4 |
| Trust | seed recorded | P1 + P4 |
| Trust | input/output versions recorded | P1 + P4 |
| Trust | provenance survives restart | P1 |
| Resource | memory estimated | P3 + P4 |
| Resource | tile processing | P4 |
| Resource | resource limits respected | P4 |
| Resource | unsafe configs rejected | P4 |
| Non-destructive | source never modified | P1 + P4 |
| Non-destructive | new Image Versions | P1 + P4 |
| Non-destructive | branches | P4 |
| Non-destructive | previous versions recoverable | P1 + P4 |
| Standalone | no external apps | P4 |
| Standalone | missing models produce guidance | P3 + P4 |
| Standalone | automatic backend | P4 |
| Standalone | no Python/CLI required | P4 |

The audit doc at P7 walks the checklist and marks every box (with PR link or recorded gap).

## Test strategy

- Unit tests: image-analysis metrics, recommendation logic, confidence calculation, mask composition, operation ordering, provenance serialization. Each in its own `tests/<name>.rs` file under `crates/astroforge-core/tests/`.
- Integration tests: `crates/astroforge-core/tests/dod_enhancement.rs` walks the full CR-06 §40 DoD end-to-end. Mirrors the R6 `dod_workflow.rs`.
- Model tests: each bundled model has a known-input / expected-output, determinism test where applicable, memory test, tile / non-tile equivalence tolerance. Deferred until a bundled model is wired in.
- Visual regression: P2 records any reference-dataset gaps; P7 fills them or records them as deferred.

## Risk register

- **GPU availability for AI inference.** Local dev typically lacks a GPU. Mitigation: every operation falls back to CPU + tile-based inference; integration tests run on CPU only; visual regression deferred to CI-with-GPU if/when available.
- **Model license verification (CR-06 §34).** StableSR and DIP carry restrictive licenses. Mitigation: P3 surfaces license at recommendation time; P4 surfaces it at apply time; P6 quality gates flag missing-license models as "experimental, opt-in only".
- **Hallucination / fabrication safeguards.** CR-06 §17 demands provenance for every reconstructive operation. Mitigation: P1 enforces model_hash / seed / deterministic_class fields; P4 UI surfaces the classification; P7 audit walks the provenance trail.
- **Spec version bump.** CR-06 will warrant a 1.2.0 minor or 2.0.0 major. The bump lands in P7 (or a sibling PR), not in P0, so the spec version doesn't track a "proposed" CR.
- **Spec coverage gaps.** CR-06 references §11 (Recipe Format) and §17 (Test Strategy) of the v1.1.0 spec; P1–P6 may surface gaps. Mitigation: each phase documents any uncovered spec references in its PR body.

## Out of scope for CR-06

- Compare workspace rewrite (CR-07 territory).
- Model Hub UX (CR-16 territory; CR-06 consumes models but does not manage them).
- Recipe system (Phase 3 / CR-M3 territory; CR-06's outputs become recipe inputs downstream).
- Plugin architecture (Phase 4 / CR-14 territory).
- Export pipeline changes beyond provenance (CR-21 territory; CR-06's outputs feed export).

## Status

P0 (this PR) — proposed 2026-09-10. Awaiting your call on which phase to start.
