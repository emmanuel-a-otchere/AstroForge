# Changelog

## Unreleased

### Slice M7 — P1.5-M7-T1..T5 walk-down + T5 reversibility primitives

**Scope:** Bundled-batch-tight per the slice order. T1..T4
were shipped in earlier PRs (#225 checkpoint IPC, #226
reapplyStage, #227 multi-format dispatch, #228 warning gate).
T5 — exact reversibility for crop, stretch, and star-replace —
was genuinely missing in code (only forward operations existed);
this slice ships the inverses plus paperwork reconciliation for
the other four tasks.

#### T1..T4 paperwork reconciliation

- `docs/PROJECT_PLAN.md` — flipped P1.5-M7-T1..T5 status from
  `in_progress` / `pending` to `done (PR #...)` with canonical
  PR references (#225, #226, #227, #228) for T1..T4 and the
  in-flight PR for T5.

#### T5 reversibility primitives

- `crates/astroforge-core/src/crop.rs`:
  - `uncrop(canvas_size, cropped, region, fill) -> F32Image` —
    exact inverse of `crop(region)`. Pastes the cropped image
    back into a fresh canvas at the original `CropRegion`;
    outside-region pixels are initialised to `fill`. Out-of-bounds
    regions are silently clamped (matches `crop`'s silent
    out-of-bounds read behaviour).
  - 3 unit tests: full-region round-trip, fill-value isolation,
    out-of-bounds clamping.

- `crates/astroforge-core/src/stretching.rs`:
  - `AutoStretchParams { min, max, midtones }` — captures the
    parameters the forward `auto_stretch` discards. Without
    these captures, `auto_stretch` is non-reversible.
  - `auto_stretch_with_params(image) -> (F32Image, AutoStretchParams)`
    — forward entry point that returns the captured params.
  - `auto_stretch_inverse(image, params) -> F32Image` — exact
    round-trip when fed the forward's params.
  - `arcsinh_stretch_inverse(value, midtones) -> f64` — exact
    inverse of the Lupton 1999 arcsinh stretch.
  - `histogram_stretch_inverse(image, shadows, highlights, midtones) -> F32Image`
    — inverse that handles the saturated cases explicitly: pixels
    that the forward path clamped to `shadows` or `highlights`
    recover the boundary (the original was already discarded).
  - `midtone_transfer_inverse(value, midtones) -> f64` — solves
    `y = ((m - 1) * x) / ((2m - 1) * x - m)` for `x`.
  - 5 unit tests: endpoint round-trips, MTF round-trip across a
    range of midtones, auto_stretch image round-trip, histogram
    round-trip inside the unclamped window, and the documented
    lossy behaviour outside the window.

- `crates/astroforge-core/src/star_segmentation.rs`:
  - `replace_stars(star_layer, background_layer) -> F32Image` —
    exact inverse of `segment_stars`. The forward path partitions
    each pixel into `star_layer` + `background_layer` (one of
    them is 0); summing the layers recovers the original. Panics
    on geometry mismatch (caller bug, not recoverable).
  - `inverse_star_enhancement(enhanced, color_boost) -> F32Image`
    — exact inverse of `enhance_star_layer` (multiplies by
    `color_boost`); inverse divides. Asserts on zero `color_boost`.
  - `inverse_background_enhancement(enhanced, contrast) -> F32Image`
    — inverse of `enhance_background_layer` (multiplies
    difference from mean by `contrast`); inverse divides. Asserts
    on zero contrast.
  - 4 unit tests: `replace_stars` image round-trip,
    `inverse_star_enhancement` round-trip, panic-on-zero-boost,
    `inverse_background_enhancement` round-trip.

#### Robustness

| Risk | Mitigation |
|---|---|
| `arcsinh_stretch` midtones=0 division-by-zero | Forward `beta = midtones.max(1e-10)`; inverse mirrors |
| `histogram_stretch` saturation (input outside [shadows, highlights]) | Inverse explicitly recovers the envelope value for saturated pixels; documented lossy behaviour pinned by test |
| Float32 representation of envelope values | Inverse compares `post_mtf` against `envelope_f32 as f64` to match the forward path's f32-clamped output |
| `replace_stars` geometry mismatch | Panics with a precise message — caller bug, not recoverable runtime error |
| `inverse_star_enhancement(0.0)` | Asserts on zero `color_boost` to surface the contract violation |
| Float-precision drift in round-trip tests | Tolerance is `1e-4` for stretch and crop, matching the MTF formula's intrinsic error budget |

#### Tests / verification

- `cargo test --workspace` — 719 passed (656 + 30 + 3 + 2 + 8 + 3 + 5 + 2 + 5 + 5); 0 failed
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session` — green
- 12 new unit tests across `crop.rs`, `stretching.rs`, `star_segmentation.rs`

### Slice CD — GPU execution providers (P5.2) + DP#4 catalog license audit

**Scope:** Two tightly-coupled forward-look items bundled into one
slice per user-pick `B, E, A, CD` and the bundled-batch-tight
rationale (both items live in `crates/astroforge-ai/src/` and gate
each other — the GPU path is the runtime that the catalog audit
hardens the input to).

#### GPU execution providers (P5.2)

- **`crates/astroforge-ai/Cargo.toml`:** new `[features]` block
  with five opt-in features (`gpu-cuda`, `gpu-directml`, `gpu-coreml`,
  `gpu-openvino`, `gpu-tensorrt`), each enabling the matching `ort`
  feature flag. All five are **OFF by default**; default `cargo build`
  remains CPU-only. The CI matrix is unchanged in this slice (still
  CPU-only); per-platform GPU runners are a follow-up tracked in
  the Open issue list.
- **`crates/astroforge-ai/src/gpu_providers.rs` (NEW, ~270 LOC):**
  - `ExecutionProvider` enum (`Cuda | TensorRt | DirectMl | CoreMl | OpenVino | Cpu`).
  - `build_selection(probe)` — maps a `HardwareProbe` onto an
    ordered preference list (CUDA leads; CPU always trails).
  - `compiled_selection(probe)` — drops providers whose Cargo
    feature isn't enabled in this build.
  - `has_compiled_gpu(probe)` — convenience boolean.
  - Each provider's `is_compiled()` is `#[cfg(feature = "...")]`-gated
    so the unit tests prove the CPU-only default build has no
    compiled GPU providers.
  - 8 unit tests cover probe→selection mapping for every backend +
    CPU fallback + label stability.
- **Verified `cargo check -p astroforge-ai --features gpu-cuda` and
  `--features gpu-cuda,gpu-directml,gpu-coreml` both compile cleanly**
  (sandbox-validated). Runtime CUDA/DirectML/CoreML behavior is
  per-platform and not exercised in this sandbox.

#### DP#4 catalog license audit

- **`crates/astroforge-ai/src/inference.rs`:**
  - New `LicenseSpdx` enum (`Apache20 | Mit | Bsd3Clause | CcBySa40 |
    CcByNc40 | Unknown`) with `as_spdx()` canonical-string serialization
    and `is_commercial_ok()` for the integrity-badge signal.
  - `CatalogModel` gains a `license: Option<LicenseSpdx>` field.
    All 5 pinned builtins carry `Some(LicenseSpdx::Mit)` (matching
    workspace root license). All 7 unpinned real-catalog entries
    carry `license: None` per the audit contract (license is
    meaningless until the hash lands).
  - `CatalogAuditGap` struct + `verify_catalog_audit() -> Result<(), Vec<CatalogAuditGap>>`.
    Audit contract: every entry must be either fully pinned (with
    license) or explicitly unpinned (no license). The function
    flags three failure modes: pinned entry missing license,
    pinned entry declaring `Unknown`, unpinned entry claiming a
    license.
  - 6 new unit tests cover SPDX strings, commercial classification,
    audit pass on current registry, and the three failure modes.
- **`docs/adr/0003-dp4-catalog-audit-checklist.md` (NEW):**
  per-entry contract + close-out procedure for each of the 7
  real-catalog entries. Each entry becomes audit-clean via a
  one-line table update when upstream publishes hash + license.
- **Robustness:** the audit fails closed on every contract
  violation; unpinned entries with a license are flagged because
  the license is meaningless without a hash to pin it to.
- **Open follow-ups:** per-platform CI runners (one job per GPU
  feature × OS); CC-BY-NC-4.0 integrity-badge wiring (consumer-facing
  signal that the output was produced by a non-commercial model);
  MODEL_PROVENANCE.md file with source URLs (one row per model).

#### Tests / verification

- `cargo test --workspace` — green (full test count +14 over slice #311).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo check -p astroforge-ai --features gpu-cuda` — clean (CPU + CUDA build).
- `cargo check -p astroforge-ai --features gpu-cuda,gpu-directml,gpu-coreml` — clean (CPU + 3 GPUs).
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session` — green.

Slice **CD** (combined) per user-pick `B, E, A, CD`.

### P3-M2-T1..T5 walk-down — Recipe system paperwork reconciliation

**Status:** No-op audit. P3-M2-T1 (`#100`), T2 (`#101`), T3 (`#102`),
T4 (`#103`), T5 (`#104`) all shipped across three commits
(`4405218` + `2a9c566` + `ad9b052`). The PROJECT_PLAN status column
was not updated when the milestone landed; this slice corrects the
drift.

- `crates/astroforge-core/src/recipe.rs` (566 LOC) — Recipe,
  RecipeStage, IntegrityBadge, ModelUsage, ModelType, migrate_recipe,
  migrate_v1_to_v2, sanitize_recipe, validate_compatibility,
  apply_recipe, to_json, from_json, from_json_migrated,
  SCHEMA_VERSION_V1 + SCHEMA_VERSION_CURRENT.
- `crates/astroforge-core/src/recipe_store.rs` (452 LOC) —
  persistence + versioning (parent_version, branch, linear version
  counter).
- `src/components/RecipesScreen.svelte` — Recipe gallery UI.
- `docs/PROJECT_PLAN.md` § Milestone 3.2 — P3-M2-T1..T5 status
  flipped from `pending` to **done** (with the canonical commit SHA
  for each row) plus a walk-down note capturing the reconciliation
  rationale.
- **No Rust changes**; no CI gates needed beyond `mvp_smoke` for
  correctness confirmation.

### P3-M4-T1..T4 walk-down — Bayer detection paperwork reconciliation

**Status:** No-op audit. P3-M4-T1 (`#110`), T2 (`#111`), T3 (`#112`), T4
(`#113`) all shipped in commit `56d2184` ("Phase 3 M3.4: PNG/JPG/DNG
Bayer detection — statistical Bayer detection (autocorrelation, green
variance, camera signature DB), DNG parser (CFA tags, BlackLevel,
WhiteLevel), Bayer uncertainty prompt (telescope selection / pattern
selection), confidence scoring (>0.85 auto, 0.5–0.85 prompt, <0.5 assume
RGB)"). The PROJECT_PLAN status column was not updated when the
milestone landed; this slice corrects the drift.

- `crates/astroforge-core/src/bayer_detection.rs` (364 LOC) —
  autocorrelation + green-variance + pattern detection + camera
  signature DB.
- `crates/astroforge-core/src/bayer_intelligence.rs` (436 LOC) —
  camera signature DB wiring + four-state taxonomy
  (`BayerInferenceKind` / `BayerRoute`).
- `crates/astroforge-core/src/dng_parser.rs` (229 LOC) — CFA tags +
  BlackLevel + WhiteLevel.
- `src/components/BayerPromptDialog.svelte` — uncertainty prompt UI.
- `docs/PROJECT_PLAN.md` § Milestone 3.4 — P3-M4-T1..T4 status
  flipped from `pending` to **done** (`56d2184`) with a walk-down
  note capturing the reconciliation rationale.
- **No Rust changes**; no CI gates needed beyond `mvp_smoke` for
  correctness confirmation.

### Spec carrier authoring — v1.3.0 + v1.4.0 + delta summary

- **`docs/specs/AstroForge_Spec_v1.3.0.md` (NEW, carrier, 660 lines):**
  preserves the v1.1.0 base content unchanged and appends a **Delta
  from 1.2.0** section that captures the substantive changes that
  landed between 1.1.0 and 1.3.0 (CR-06 P5.1, CR-07 + follow-ons).
  The delta section links to the canonical CR documents rather than
  re-authoring prose.
- **`docs/specs/AstroForge_Spec_v1.4.0.md` (NEW, carrier, 661 lines):**
  preserves the 1.3.0 carrier content unchanged and appends a **Delta
  from 1.3.0** section that captures the substantive changes that
  landed in the forward-look bundle (DP#4, SessionCache, ADRs).
- **`docs/specs/SPEC_INDEX.md`:** 1.4.0 flipped to ✅ Active; 1.3.0
  moved to historical (📦 Superseded); 1.2.0 documented as a delta
  commit (not on disk). Open issue list items 1+2 (carrier
  authoring) closed; remaining items reshuffled. CR-06 P5.1, CR-07 +
  follow-ons, forward-look bundle, and carrier authoring are all
  now noted as Resolved.
- **Robustness:** each carrier preserves the previous carrier's full
  content unchanged; the delta section links to canonical CR docs
  rather than re-authoring prose. The 1.3.0 + 1.4.0 carriers are
  ~660 lines each (vs ~555 lines for the 1.1.0 base) because the
  delta sections are short and the base content is preserved verbatim.

### Forward-look slice — DP#4, SessionCache, ADRs, spec index reconciliation

- **DP#4 catalog license verification (`crates/astroforge-ai/src/inference.rs`):**
  - New `CATALOG_MODELS` registry shadows the 5 builtin entries (with
    real SHA-256 digests) and lists the 7 real-catalog entries from
    PROJECT_PLAN P2-M1-T5..T11 with the `UNVERIFIED_SHA256` sentinel.
  - `OnnxEngine::open_catalog(id, bytes)` is now digest-pinned: looks
    up the registry, verifies the runtime SHA-256 against the pinned
    value, fails closed on unknown id, fails closed on unpinned entry.
  - New `InferenceError` variants: `UnknownCatalogModel`, `CatalogUnpinned`.
  - Legacy `open_catalog(bytes, kind)` renamed to `open_catalog_unpinned`
    so test paths + pre-DP#4 callers keep working.
- **`SessionCache` (same file):** process-wide `Arc<Mutex<HashMap>>`
  cache keyed by `(kind, sha256)`; `get_or_build(key, || …)` runs the
  build closure outside the cache lock so a slow build doesn't block
  reads; `clear()` exposed for model-registry changes. Cuts session-
  build cost from N (per-apply) to 1 (per-model lifetime) for batches.
- **`docs/adr/` (new):** Architecture Decision Records folder.
  - `0001-plate-solve-dependency.md` — adopt ASTAP, bundled, offline;
    close issue #73.
  - `0002-smart-telescope-sdk.md` — file-only ingest is the v1.x
    contract; SDK integration deferred to Phase 4 plugin; close #133.
  - `README.md` — index + workflow.
- **`docs/specs/SPEC_INDEX.md`:** 1.3.0 + 1.4.0 carrier authoring
  deferred (Open issue list items 1 + 2); CR-06 / P5.1 / CR-07 status
  blocks now reflect the resolved state; forward-look slice noted.
- **Tests:** 11 new unit tests on `inference.rs` (catalog registry,
  fail-closed catalog paths, SessionCache behaviour, cache-key
  distinctness). All workspace + clippy + mvp_smoke green.
- **Forward-look items closed:** DP#4 fail-closed machinery; SessionCache
  for apply-round session reuse; ADRs #73 and #133.

### CR-07 follow-on 2 — ImageCanvas WebGL back-end

- Adds a GPU shader path to `src/components/ImageCanvas.svelte`:
  16-bit TIFF pixels are decoded once, packed into a Float32
  RGBA texture, and rendered through a `zoom / pan / clip`
  fragment shader. The existing Canvas 2D path is preserved
  verbatim and serves as the auto-fallback when WebGL context
  creation fails (sandbox, headless, very old webview).
- New `src/lib/image-canvas-webgl.ts` — self-contained WebGL
  adapter purpose-built for the CR-07 surface (avoids coupling
  the ImageCanvas path to the P1.5 wizard's `gl-renderer.ts`
  which is wired for MTF / SCNR / star-compositing rather than
  raw 16-bit TIFF). Uses Float32 RGBA texture upload when
  `OES_texture_float` is available; falls back to 8-bit RGBA
  upload (same display precision as the Canvas 2D path) on
  contexts without the extension.
- Toolbar gains a Back-end selector (`auto / webgl / canvas2d`)
  with FPS readout (rolling 30-frame average) and per-frame
  upload-cost tooltip for the scorecard.
- The existing Canvas 2D path is unchanged (no behaviour
  regression for the default `auto` profile on a system where
  WebGL init fails on first probe).
- Spec bump target: AstroForge v1.4.0 (unchanged; follow-on
  paperwork carrier).

### CR-07 follow-on — Compare tools (split, blink, difference, region)

- New `src/components/CompareTools.svelte` — split-slider (vertical
  `clip-path` overlay with keyboard-accessible `<input
  type="range">`), blink comparator (1Hz toggle, pauses via
  `Page Visibility API` so backgrounded tabs don't burn cycles,
  speed slider 200ms–3s, pause/resume), and difference-map
  overlay (per-pixel absolute delta with `1×–16×` gain slider).
- `src/components/CompareWorkspace.svelte` — adds a "Compare
  tools" toggle (shown only when both picked versions have a
  `primary_artifact_id`); the toggle swaps the side-by-side
  canvas layout for `<CompareTools />`.
- `src/components/ImageCanvas.svelte` — region-inspection
  readout in the toolbar; the live rect from the most recent
  shift-drag is rendered as `x0,y0 → x1,y1` with a "Clear"
  button. (The `onRegion` callback was wired in slice 1; the
  follow-on adds the local readout.)
- Spec bump target: AstroForge v1.4.0 (unchanged).

### CR-07 — Zone B Canvas, Image Rendering & Compare Surfaces

- Closes M9 §35 criteria U2 (preview rendering), U3 (before/after
  surface), U4 (split comparison) on top of the real pixels P5.1's
  apply round now produces.
- New `src-tauri::commands_ai_enhancement::read_image_artifact`
  Tauri command (path-confined to
  `<root>/.astroforge/applied/<project_id>/`) returns base64-encoded
  16-bit TIFF bytes plus width / height / channels. Defense-in-
  depth: artifact path canonicalization + `starts_with(applied_root)`
  check refuses absolute paths, `../` traversal, and symlinks
  pointing outside the project dir.
- Minimal TIFF dimension reader handles II / MM byte orders, single
  IFD entries, only accepts 16-bit grayscale or RGB samples.
  Companion `read_tiff_dimensions` unit tests cover grayscale,
  RGB, big-endian, non-TIFF, and 8-bit rejection.
- New `src/components/ImageCanvas.svelte` — pure rendering
  component. Fetches artifact bytes via `readImageArtifact`,
  decodes the 16-bit TIFF inline (single-strip uncompressed),
  scales to viewport, draws to `<canvas>`. Zoom (fit / 1:1 /
  0.25×–4× slider), pan (mouse-drag), mask overlay (translucent
  red wash where the Float32Array mask is set), histogram
  (256 bins per channel, RGB or grayscale), clipping overlay.
- `src/components/EnhancementStudio.svelte` — Zone B metadata
  placeholder replaced with `<ImageCanvas />` against the active
  Image Version.
- `src/components/CompareWorkspace.svelte` — side-by-side compare
  panes render two `<ImageCanvas />` instances when the picked
  versions have a `primary_artifact_id`; pre-P5.1 versions still
  show honest metadata cards (no fake images).
- `src/lib/astroforge-api.ts` — `readImageArtifact` IPC client +
  `ImageArtifactResponse` type.
- Spec bump target: AstroForge v1.4.0 (after P5.1's 1.3.0 lands).
- Forward-look: split comparison slider, blink comparator,
  difference map, region inspection ship in a CR-07 follow-on PR.

### CR-06 P5.1 — Real ONNX Inference, Tile Execution, Mask-Aware Apply

- New `crates/astroforge-ai/src/inference.rs` (~700 lines) wires
  real ONNX Runtime 1.28 inference via the `ort` crate (rustls TLS,
  CPU execution provider; GPU providers land in P5.2). Bundles five
  self-authored classical-kernel ONNX graphs under
  `crates/astroforge-ai/models/builtin/` (blur-blend, sharpen-blend,
  upscale-2x, hotpixel, masked-fill) generated by
  `scripts/generate_builtin_models.py`, pinned by real SHA-256
  digests and verified at session-build time.
- `dispatch_operation` now consumes the source `F32Image`, runs
  tiled inference via the existing `tiling` crate (probe-clamped
  tile size, cosine-blended overlap), composites the result with
  the attached mask, and persists the produced pixels as a
  16-bit TIFF artifact under `~/.astroforge/applied/<project>/`.
  Returns the full `DispatchResult` (outcome + pixels +
  model_id / version / hash / backend / runtime / tile config /
  duration_ms) so the `AiOperation` row carries a real provenance
  chain.
- `SegmentedLeakage` quality gate (CR-06 §37) was a no-op in P6;
  P5.1 threads the operation mask into the gate and compares
  inside / outside mean deltas with a configurable ratio + absolute
  floor. New tests cover no-mask, bleed-past-mask, full-coverage,
  and quiet paths.
- New `ai_quality_reports` table (migration v9) persists every
  §37 verdict + findings JSON keyed to the result Image Version.
  `enhancement_apply_operation` runs the gate on the real
  (source, result) pixel pair for the first time; the verdict is
  surfaced in the apply response and in `latest_ai_quality_report_for_version`.
- New `crates/astroforge-ai/src/operations/model_binding` table
  wires every registered operation onto a builtin graph (and an
  optional catalog model that supersedes it when its artifact is
  present in `~/.astroforge/models/`). The dispatcher's contract
  is stable; existing `OperationOutcome` consumers see no break.
- Tiler closure is `Fn(&F32Image, &Tile) -> F32Image` (was
  `Fn(&F32Image) -> F32Image`); a single-line update for any
  existing tile caller.
- Forward-look: DP#4 license verification gates catalog-model
  digest pinning (builtin digests are pinned today; catalog
  digests skip the check until real hashes land). GPU execution
  providers (CUDA / DirectML / Metal) are P5.2 work gated on
  per-platform CI runners.

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
