# AstroForge — Housekeeping

A running log of clean-up work that should land as standalone PRs. Each item below is small enough to ship without its own CR but big enough that dropping it into a feature PR muddies the diff.

## Active tickets

### HOUSE-1 — Wizard sidebars + dead code

**Source:** CR-03 P4b (wizard deprecation) intentionally left wizard sidebars + their dependencies in place to keep P4b's blast radius small.

**Files in scope:**

- `src/components/NodeSidebar.svelte`
- `src/components/ParameterSidebar.svelte`
- `src/components/ScreenCard.svelte`
- `src/components/InitialDialog.svelte`
- `src/components/ProfileManager.svelte`
- `src/components/ReceiptsPanel.svelte`
- `src/components/DestructiveConfirmDialog.svelte` (only delete if no other consumer — currently false positive: `DeleteProjectDialog` doc comment mentions it but does not import it)
- `src/lib/gl-renderer.ts` (and primitive shaders under `src/lib/gl/primitives/`)
- `src/state/pipeline-store.ts` (wizard-only pipeline node state)
- `src/lib/pipeline.ts`
- `src/lib/session.ts`

**Disposal rule:** walk each consumer graph with `grep -rln <symbol> src/`. Delete the file only if the only remaining reference is its own definition. If `gl-renderer.ts` has a future consumer (P5a's Compare workspace dual-canvas), keep it and document the plan. Otherwise delete.

**Outcome target:** P4b's deletion achieves its full intent. No dead code remains that references deleted store dependencies.

**Estimated blast radius:** low — every file is currently dead-code. ~2,000 LOC removal. Single PR.

### HOUSE-2 — ingest.rs vs import_scan consolidation

**Source:** CR-04 P0 Decision D-CR04-2 (user chose Option C: keep both `ingest.rs` and `import_scan.rs`). Documented drift risk: two code paths for the same problem invite divergence.

**Files in scope:**

- `crates/astroforge-core/src/ingest.rs` — legacy classification + manifest builder (`FrameType`, `FrameInfo`, `SessionManifest`, `LightGroup`, `scan_directory`, `classify_frame`, `group_lights`, `detect_anomalies`, 10+ tests).
- `crates/astroforge-core/src/import_scan.rs` (new in CR-04 P1) — canonical intelligent scan.

**Disposal rule:** delete `ingest.rs` only after CR-04's Import Review UI is the sole entry point and any existing call sites that used `ingest.rs::scan_directory()` have been routed through `import_scan.rs::start_import()`. Migration test: import the same folder through both paths and verify identical SourceAsset records emerge.

**Outcome target:** one canonical scan API. The "ingest.rs" naming is preserved only if it remains the module's name post-CR-04; otherwise rename to remove the legacy connotation.

**Estimated blast radius:** medium — every consumer of `ingest.rs` from `astroforge-app` needs to be migrated. Touches ~5 files.

## Closed tickets

_(empty; tickets move here when the housekeeping PR lands.)_

## Backlog tickets

### HOUSE-3 — AI target hook real model swap-in

**Source:** CR-04 P10 D-CR04-3 shipped a stub `astroforge_ai::target_classify::classify_target` that records the AI boundary but does not invoke a real model. The seam is in place; the real ONNX target-recognition model lands in a follow-on tranche (likely co-tranche with the first narrowband image-processing slice in CR-14).

**Files in scope:**

- `crates/astroforge-ai/src/target_classify.rs` — replace the `AiStubRequested` branch with a real `astroforge_ai::service::classify_target` call once the model lands.
- `crates/astroforge-ai/src/models.rs` — register the new target-recognition model + its catalog entry.
- `crates/astroforge-ai/src/registry.rs` — confirm the model resolves for the `classify_target` stage.

**Disposal rule:** swap in only when the real model has been benchmarked on a representative dataset AND the deterministic fallback (P4) is preserved for the asset-count > 1024 case. Migration test: a synthetic dataset where the deterministic P4 result is below the AI floor must round-trip through the new model and produce a confidence score.

**Outcome target:** the `ClassifyTargetProvenance::AiStubRequested` variant becomes `AiModel` (new variant). The UI surfaces "AI confirmed" instead of "AI confirming…".

**Estimated blast radius:** low — the seam is in place. Touches ~3 files. Single PR once the model ships.

### HOUSE-4 — narrowband image-processing re-home

**Source:** CR-04 P7 preserved the pre-existing `narrowband.rs` image-processing helpers (`extract_channel`, `extract_oiii`, `compose_palette`, `scnr_green`, `scnr_magenta`, `normalize_channel_ratio`, `is_narrowband_session`) verbatim in `narrowband_image.rs` so the new §11 detection engine could land cleanly. These helpers belong with the image-processing pipeline (CR-14 Narrowband Studio), not the import classification layer.

**Files in scope:**

- `crates/astroforge-core/src/narrowband_image.rs` — relocate to `astroforge-app` or a new `astroforge-image` crate.

**Disposal rule:** move only when CR-14 begins work on the narrowband palette compositor. Keep `narrowband_image.rs` in place until then.

**Outcome target:** `narrowband.rs` owns only the §11 detection engine; `narrowband_image.rs` is gone.

**Estimated blast radius:** low — the helpers are unreferenced today. Single PR.