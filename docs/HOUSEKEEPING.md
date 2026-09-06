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