# Changelog

## Unreleased

### CR-04 P4 — Target Detection

- Added `crates/astroforge-core/src/target_detection.rs` (~480 lines,
  13 unit tests) — `TargetKind` enum, `TargetCandidate`,
  `TargetObservation`, `TargetIntelligence`, `normalize`, ~50-target
  embedded catalog, and the `detect` entry point. Walks the §9
  evidence hierarchy (FITS OBJECT → filename → directory → no
  signal). PR #253.

### CR-05 P7 audit

- Added the CR-05 P7 implementation audit and Definition-of-Done evidence
  boundary in `docs/M8_AUDIT.md`.
- Recorded CR-05 as implemented through P6 in `docs/specs/SPEC_INDEX.md`.
- Preserved the remaining visual-corpus and full DoD-harness gaps as explicit
  follow-up work rather than claiming unverified completion.

### CR-06 AI Enhancement Studio — P1 through P7

- **P1** — Data model + provenance + safety classification. PR #293.
  `ai_recommendations`, `image_analyses`, `image_regions`, `ai_masks`,
  `ai_operations`, `enhancement_stacks`, `enhancement_previews`,
  `image_versions` (v6 schema migration) all land behind the durable
  `DomainStore`. `AiSafetyClassification` (Deterministic / Perceptual /
  Generative) on every persisted operation.
- **P2** — Image analysis engine + `analyze_image` Tauri command +
  `ImageAnalysisPanel.svelte`. PR #294.
- **P3** — Recommendation engine + intelligent ordering (denoise →
  star_refine → deconv → detail_enhance per CR-06 §23). PR #295.
- **P4** — Enhancement operations + stack engine + apply round +
  Enhancement Studio shell + operations registry (11 ops across 8
  categories; passthrough dispatcher that is the swap-in point for
  real ONNX). PR #296.
- **P5** — Region-aware masks (auto / parametric / user / composite)
  + `MaskEditor.svelte` + 4 mask Tauri commands. PR #297.
- **P6** — Quality gate engine (10 §37 checks) +
  `QualityGatePanel.svelte` + branching UX polish. PR #298.
- **P7** — Audit (`docs/M9_AUDIT.md`: 34/39 §35 shipped, 5 partial,
  0 missing) + §40 DoD integration test (`crates/astroforge-core/tests/dod_enhancement.rs`)
  + spec bump 1.1.0 → 1.2.0. PR #299.

### CR closure reconciliation (this PR)

- Bumped CR-02 / CR-03 / CR-05 / CR-06 status headers from
  `Status: Proposed` to their actual landed state
  (Implemented / Partial — Target Detection / Implemented through P6
  / Shipped P1–P7).
- CR-04 is recorded as `Partial` because P5/P6/P7
  (session_grouping / capture_analysis / narrowband) are still open.
- Updated this `CHANGELOG.md` with retroactive entries for the
  CR-04 P4 (PR #253) and CR-06 P1–P7 (PRs #292–#299) tranches that
  pre-date the CHANGELOG's existence.
- **Refreshed** `docs/PROJECT_PLAN.md` (was 8 days stale: said
  "Spec version: 1.1.0", "50 of 50 processing-pipeline issues
  OPEN", and pointed at the M2 tranche plan as if it were the
  active slice plan; reality is 1.2.0 spec, CR-02..06 shipped, and
  the active plans are CR-03 / CR-04 / CR-05 / CR-06 tranches).
  The plan now points at the M9 audit for current programme state.
