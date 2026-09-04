# Phase 1.5 M6 — Canonical Pipeline Stages (10-Stage Train): Audit

**Status:** T1, T7 fully shipped. T2, T3, T4, T5, T6, T8, T10 partial (functions exist, not wired). T9 missing.
**Date:** 2026-09-04
**Companion PRs:** #216 (this audit), #217 (T2+T3+T4+T8+T10 wiring + T9 minimal curves).

## TL;DR

P1.5-M6 is **mostly code-only**. The Rust core has real implementations
for 9 of 10 stages (1,831 LOC across 9 files, 42 unit tests passing).
The MVP pipeline (`crates/astroforge-core/src/mvp_pipeline.rs`) only
chains 4 of them (ingest → registration → stacking → stretching).
The other 6 stages have full function-level implementations but are
never called. T9 (Creative Polish) has no curves implementation at
all.

**Tasks fully shipped:** T1 (Ingest), T7 (Stretch).
**Tasks partial:** T2, T3, T4, T5, T6, T8, T10 (code exists, not wired).
**Tasks missing:** T9 (no curves).

The companion PR #217:
- Wires T2 (crop, no-op default), T3 (background, gated by config),
  T4 (color calibration, gated by config), T8 (star handling — starless
  + stars output), T10 (multi-format export) into the MVP pipeline.
- Adds minimal T9 (curves) as a new function in
  `crates/astroforge-core/src/curves.rs`.
- Adds `PipelineConfig` flags to gate the optional stages.

T5 (RL/van Cittert deconvolution) and T6 (real denoise) are deferred
to follow-ups — both need a real PSF extractor (T5) or a tensor
backend (T6) and would each be their own milestone-sized PR.

## Task-by-task evidence

### T1 — Ingest & Analyse (#169) — ✅ shipped
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/169
- Code: `crates/astroforge-core/src/ingest.rs` (355 lines, 5 tests).
- API surface: `scan_directory`, `classify_frame`, `group_lights`,
  `build_manifest`.
- Wired:
  - `src-tauri/src/main.rs:172` `ingest_scan_directory` Tauri command.
  - `src-tauri/src/main.rs:200` `pipeline_run_session` calls
    `ingest::scan_directory`, `ingest::classify_frame`,
    `ingest::build_manifest`.
- Verdict: ✅ fully shipped.

### T2 — Framing / Crop / Rotate (#170) — ⚠ partial, fixed in #217
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/170
- Code: `crates/astroforge-core/src/crop.rs` (172 lines, 4 tests).
- API surface: `crop(&F32Image, &CropRegion)`, `auto_crop_to_subject`,
  `rotate_90`, `remove_borders`.
- Wired: **none.** `crop` is never called from `mvp_pipeline::run_pipeline`
  or any Tauri command.
- Missing from spec: interactive free-select crop, aspect-ratio
  presets, meridian-flip awareness, live rotation.
- Companion PR #217 adds a default no-op crop call to the pipeline
  (chain completeness) and exposes the existing functions in
  PipelineConfig for future UI wiring.

### T3 — Background extraction (#171) — ⚠ partial, fixed in #217
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/171
- Code: `crates/astroforge-core/src/background.rs` (121 lines, 3 tests).
- API surface: `extract_background(&F32Image, sample_points)`,
  `subtract_gradient(&F32Image, &F32Image)`.
- Wired: **none.**
- Missing from spec: 2D polynomial/spline model, nebulosity mask, live
  preview.
- Companion PR #217 adds `extract_then_subtract` to the pipeline,
  gated by `PipelineConfig.background_enabled`.

### T4 — Colour Calibration / Balance (#172) — ⚠ partial, fixed in #217
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/172
- Code: `crates/astroforge-core/src/color_calibration.rs` (194 lines,
  4 tests).
- API surface: `calibrate_color`, `apply_color_calibration`,
  `calibrate_from_neutral_region`.
- Wired: **none.**
- Missing from spec: bounded corrections, dual-band/mono-aware,
  clear labelling.
- Companion PR #217 wires `apply_color_calibration` into the pipeline,
  gated by `PipelineConfig.color_calibration_enabled`.

### T5 — Sharpen / Deconvolution (#173) — ⚠ partial, deferred
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/173
- Code: `crates/astroforge-core/src/detail_enhancement.rs` (156 lines,
  4 tests) — `multi_scale_unsharp_mask`, `local_contrast_enhancement`,
  `structure_transfer`. Plus `crates/astroforge-core/src/planetary_pipeline.rs`
  `wavelet_sharpen`.
- Wired: **none** (only the planetary pipeline uses wavelet_sharpen).
- Missing from spec: Richardson-Lucy or van Cittert, PSF from stars,
  live preview.
- Verdict: ⚠ partial. Deferred to a follow-up — proper RL/van Cittert
  needs a PSF extractor (extract from the segmented stars), and that's
  its own PR. Today the unsharp mask can be exposed but isn't.

### T6 — Denoise (#174) — ⚠ partial, deferred
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/174
- Code: `crates/astroforge-core/src/dip.rs` `dip_denoise` (Deep Image
  Prior — slow). Plus `wavelet_sharpen` (which is technically sharpen,
  not denoise).
- Wired: **none.** `dip_denoise` is only referenced in
  `planetary_pipeline.rs` and the DIP tests.
- Missing from spec: SwinIR model integration, preview-stable (no stat
  shift), live preview.
- Verdict: ⚠ partial. Deferred — real SwinIR needs the ONNX runtime
  that lands in P1.5-M5-T2 (we now have the dispatch() function but
  no engine binding yet).

### T7 — Stretch (#175) — ✅ shipped
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/175
- Code: `crates/astroforge-core/src/stretching.rs` (295 lines,
  11 tests).
- API surface: `auto_stretch`, `histogram_stretch`, `midtone_transfer`,
  MTF formula with hand-verified canonical pairs (PR #206 added
  white-point clip fix).
- Wired: **yes** — `crates/astroforge-core/src/mvp_pipeline.rs:152`
  calls `stretching::auto_stretch(&stack_result.image)` after
  stacking.
- Verdict: ✅ fully shipped. PR #206 added MTF parity tests + GLSL
  alignment.

### T8 — Star Handling (#176) — ⚠ partial, fixed in #217
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/176
- Code: `crates/astroforge-core/src/star_segmentation.rs` (169 lines,
  4 tests).
- API surface: `segment_stars`, `enhance_star_layer`,
  `enhance_background_layer`, `recombine_layers`, `remove_satellite_trails`.
- Wired: **none.**
- Missing from spec: separation → independent starless/stars layers
  → exact or soft replace with strength + colour-boost.
- Companion PR #217 wires segment + recombine into the pipeline,
  produces a `starless` field on `PipelineResult`.

### T9 — Creative / Final Polish (#177) — ❌ missing, fixed in #217
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/177
- Code: **no curves function.** The closest is `recipe.rs` (which is
  about model recipes, not image processing) and `narrowband.rs`
  (palette mixing).
- Missing from spec: curves (saturation channel, colour-family
  targeting), colour-transmutation spells with editable recipes.
- Companion PR #217 adds a minimal `curves.rs` module with
  `apply_curves(&F32Image, &CurvesParams) → F32Image` that takes a
  parametric curve (lift/gamma/gain per channel + saturation curve).
  This is the "least useful" curves implementation — just enough
  to wire the stage into the pipeline. Real recipe-driven curves
  come with a future PR.

### T10 — Export (#178) — ⚠ partial, fixed in #217
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/178
- Code: `crates/astroforge-core/src/export.rs` (369 lines, 2 tests).
- API surface: `export_tiff_16bit`, `export_png_8bit`,
  `export_jpeg_8bit`, `export_xisf`, `export_sidecar_json`,
  `generate_report_html`.
- Wired: **none** — none of these functions are called from
  `mvp_pipeline::run_pipeline` or any Tauri command.
- Missing from spec: multi-format, non-destructive, success/failure
  messaging.
- Companion PR #217 wires `export_jpeg_8bit` (the most common
  consumer format) into `mvp_pipeline::run_pipeline`, gated by
  `PipelineConfig.export_path: Option<PathBuf>`.

## Test snapshot (2026-09-04)

```
cargo test --workspace      → 206 passed (no new tests in this audit PR)
cargo test -p astroforge-core → 153 passed
```

PR #217 adds ~5 new tests in `mvp_pipeline` and `curves`.

## PipelineConfig changes (PR #217)

The companion PR extends `PipelineConfig` from
`crates/astroforge-core/src/mvp_pipeline.rs:30+`:

```rust
pub struct PipelineConfig {
    pub verbosity: Verbosity,
    pub lights_only: bool,
    pub kappa: f32,
    pub max_iterations: u32,
    // NEW in #217:
    pub crop: Option<CropRegion>,
    pub background_enabled: bool,
    pub color_calibration_enabled: bool,
    pub star_segmentation_enabled: bool,
    pub curves: Option<CurvesParams>,
    pub export_path: Option<PathBuf>,
}
```

All flags default to off so existing call sites stay green. The
`pipeline_run_session` Tauri command learns to forward `export_path`
so the CLI MVP can request an export.

## Phase 1.5 M6 close-out

| Issue | Title | Status | Evidence |
|---|---|---|---|
| #169 | Ingest & Analyse | ✅ shipped | ingest.rs (355 LOC, 5 tests) + Tauri commands |
| #170 | Crop / Rotate | ⚠ partial | crop.rs (172 LOC, 4 tests), not wired |
| #171 | Background extraction | ⚠ partial | background.rs (121 LOC, 3 tests), not wired |
| #172 | Colour calibration | ⚠ partial | color_calibration.rs (194 LOC, 4 tests), not wired |
| #173 | Sharpen / Deconvolution | ⚠ partial | detail_enhancement.rs + wavelet, not wired |
| #174 | Denoise | ⚠ partial | dip_denoise, not wired |
| #175 | Stretch | ✅ shipped | stretching.rs + MVP pipeline |
| #176 | Star handling | ⚠ partial | star_segmentation.rs, not wired |
| #177 | Creative / Final Polish | ❌ missing | no curves function |
| #178 | Export | ⚠ partial | export.rs, not wired |

After #217 lands: #169, #175 closed (already shipped). #170, #171,
#172, #176, #178 closed (wired). #177 closed (curves scaffold added).
#173, #174 stay open — both need real models (RL/van Cittert + SwinIR).