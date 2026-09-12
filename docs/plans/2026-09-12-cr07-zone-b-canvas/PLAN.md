# CR-07 Implementation Plan — Zone B Canvas, Image Rendering & Compare Surfaces

**Date:** 2026-09-12
**Source CR:** [CR-07 — Zone B Canvas, Image Rendering & Compare Surfaces](../../CR-07-ZONE-B-CANVAS.md)
**Branch:** `feat/cr-07-zone-b-canvas`
**Target spec:** AstroForge v1.4.0 (after this slice; P5.1's 1.3.0 bump
lands first).
**Status:** Active.

## Why this slice now

M9 audit closes with three §35 gaps in Zone B (U2 preview rendering, U3
before/after surface, U4 split comparison) and one Zone B entry from the
CR-06 spec that requires real pixels to render. P5.1 (merged PR #307)
produces the first 16-bit TIFF artifacts through `enhancement_apply_operation`,
so the canvas finally has data to draw. This slice is the surface that
consumes that data.

## Slice shape

Bundled-batch per user preference (tightly coupled sub-bullets ship as one
PR). Three sub-bullets in this slice:

1. **`read_image_artifact` Tauri command + IPC client.**
   Path-confined to `<projects_root>/applied/`. Returns base64-encoded
   bytes. Defense-in-depth via the same pattern as
   `read_preview_artifact` (absolute-path + `../` traversal refused).
2. **`ImageCanvas.svelte` core component.**
   Pure rendering component, takes `versionId` prop, decodes the
   16-bit TIFF, scales to viewport. Zoom (fit / 1:1 / 0.25×–4× slider),
   pan (mouse-drag, momentum), mask overlay (translucent red wash),
   histogram (256 bins per channel), clipping overlay.
3. **`CompareWorkspace` pixel render.**
   Side-by-side picker driven by `ImageCanvas`, synchronized zoom + pan.
   The R3 honest-placeholder metadata card stays for versions without an
   artifact; the canvas renders for versions that have one.

Defer to CR-07 follow-up PRs: split comparison slider, blink comparator,
difference map, blink / region inspection.

## Robustness analysis

| Risk | Mitigation |
|---|---|
| 16-bit TIFF round-trip: 4 MP = 16 MiB unencoded base64 | Cap at 8 MP (32 MiB); larger versions fall back to 8-bit downsampled PNG plus the full 16-bit artifact kept on disk for export. |
| Path confinement (zone-b reads arbitrary file paths from the IPC) | Reuse `commands_preview::read_preview_artifact` pattern; absolute-path + `../` refused; symlink-outside-dir refused (test). |
| Canvas tearing when swapping `versionId` mid-mount | `$state` async pipeline + key prop on the canvas; old ImageData is GC'd on swap. |
| Histogram cost on 8-MP 16-bit | Compute in Rust (the existing `astroforge-core::stats` histogram helper) and ship the 768 bins per channel as a JSON payload — render is just SVG bars. |
| Zoom / pan state synchronization between two side-by-side canvases | Shared `viewport` store in Svelte; both canvases derive their projection from it. |
| Mask overlay coordinate alignment | Mask coordinates are version-relative (already enforced by the AI mask encoder). The canvas renders the mask at the same projection as the image; mask canvas shares the projection. |
| TIFF decoder coverage | Existing `astroforge-core::decoders::tiff` covers the 16-bit grayscale + RGB cases P5.1 produces; the canvas-side decoder only needs to handle those two formats. |

## Acceptance criteria

- AC-1: `read_image_artifact` returns base64 bytes for a valid 16-bit
  TIFF, refuses files outside the applied dir, refuses symlinks pointing
  outside.
- AC-2: `ImageCanvas.svelte` renders the bytes, supports zoom and pan,
  histogram bins equal the raw count for a known pattern.
- AC-3: `CompareWorkspace` renders two `ImageCanvas` instances for two
  picked versions; synchronized zoom / pan.
- AC-4: Mask overlay renders as translucent red wash where the mask
  is set.
- AC-5: All existing tests still pass.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --no-fail-fast`
- `npm run check` (vitest + svelte-check)
- `npm run e2e` (Playwright smoke)
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`

## Plan

| Step | Verify |
|---|---|
| 1. `read_image_artifact` Tauri command + path confinement tests | `cargo test -p astroforge-core` |
| 2. IPC client `readImageArtifact` + `imageVersionStore` artifact fetch | `npm run check` |
| 3. `ImageCanvas.svelte` (zoom / pan / mask overlay / histogram / clipping) | component tests pass; visual smoke via Playwright |
| 4. `CompareWorkspace` pixel render path with synchronized viewport | visual smoke |
| 5. Paperwork: CR-07 spec, M9 audit flip, PROJECT_PLAN refresh | grep for stale entries |

## Forward-look (deferred to a CR-07 follow-on PR)

- Split comparison slider (`clip-path` based)
- Blink comparator (interval-driven)
- Difference map (4× gain on absolute delta)
- Region inspection (rectangle drag → readout)

## Out of scope (later)

- WebGL / WebGPU canvas acceleration. CPU + canvas 2D is sufficient at ≤ 4 MP.
- Annotations / regions of interest. Distinct CR.
- Per-version diff metrics (PSNR / SSIM).