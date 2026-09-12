CR-07 — Zone B Canvas, Image Rendering & Compare Surfaces

Status: Drafting (target post-P5.1)
Target: AstroForge v1.4.0
Priority: High
Depends on: CR-01, CR-02, CR-03, CR-06 (incl. P5.1 real ONNX), CR-04
Enables: CR-08, CR-13, CR-16, CR-19

## 1. Intent

Close the **last three M9 §35 gaps** that survived P5.1: U2 (preview
rendering), U3 (before/after surface), U4 (split comparison). Today
the Studio's data model is complete and the apply round produces a
real TIFF artifact, but the canvas that would render those pixels is
still the R3 honest-placeholder metadata card from CR-03 P5. CR-07
is the surface that consumes P5.1's output.

The objective is not simply "add an `<img>` tag". The CR-06 spec
defines Zone B capabilities: zoom, pan, fit, 1:1, before/after,
split comparison, blink, difference, clipping, histogram, mask
overlay, region inspection. The image remains the dominant UI
element. The canvas must be responsive to the active image version
selection without re-mounting the surrounding application shell.

## 2. Background

- `CompareWorkspace.svelte` was deliberately pixel-free in R3
  (CR-03 P5) because the `image_versions` table did not yet have a
  primary artifact column. P4 added the table; P5.1 wires the
  artifact bytes through the apply round as a 16-bit TIFF.
- `commands_preview::read_preview_artifact` already serves PNG
  bytes from `<projects_root>/previews/` as base64 with a
  path-confinement guard. The new `read_image_artifact` mirrors
  that for `~/.astroforge/applied/<project>/`.
- `astroforge-api.ts` exposes `readPreviewArtifact` and the
  `PreviewRunDto` IPC surface. CR-07 adds `readImageArtifact` and
  the per-version artifact lookup.
- The Studio's three-zone shell (Zone A context + Zone B canvas +
  Zone C intelligence) already exists in `EnhancementStudio.svelte`;
  the canvas pane is the swap target.

## 3. Sub-bullets

### 3.1 `read_image_artifact` Tauri command + IPC client

- Path-confined to `<projects_root>/applied/`. Reuses the existing
  `ContentStore::sha256_hex` and `artifact::Category::AiOutput`
  conventions. Returns base64-encoded bytes.
- `ImageVersion::artifact_id` becomes the lookup key (analogous to
  `PreviewRun::preview_artifact_id`). Schema bump to v10 if absent;
  CR-06 P4 already shipped the column, so the migration is a no-op.
- Adds `src/lib/astroforge-api.ts::readImageArtifact(versionId)` and
  the matching `invoke("read_image_artifact", ...)` handler.
- Tests: 1 integration (path-confinement refuses files outside
  `applied/`); 1 integration (returns bytes for a known artifact).

### 3.2 `ImageCanvas.svelte` core component

- Pure rendering component: takes a `versionId` prop, fetches the
  artifact bytes, decodes the 16-bit TIFF (via
  `astroforge-core::decoders::tiff`), scales to viewport, and
  renders to `<canvas>`.
- Zoom levels: fit (default), 1:1, and a slider range 0.25×–4×.
- Pan via mouse-drag with momentum; pinch / wheel-zoom on touch.
- Loads through a `$state` async pipeline so swapping
  `versionId` mid-mount doesn't tear the canvas.
- Mask overlay: optional `FloatMask` prop renders as a translucent
  red wash where the mask is set.
- Clipping: toggleable per-channel low / high clip overlay.
- Histogram: 256-bin per-channel histogram computed on the
  raw 16-bit data (so the user can see before / after saturation
  honestly).
- Tests: vitest snapshot of a 64×48 stub through the renderer;
  integration that histogram equals the raw bin count when fed a
  known pattern.

### 3.3 Compare tools

- **Before / after**: side-by-side version picker (existing
  CompareWorkspace infrastructure), but `ImageCanvas` renders
  the picked versions with synchronized zoom / pan.
- **Split comparison**: a draggable vertical slider overlays the
  before image under the after image; the left half of the
  after image is hidden by `clip-path`. Keyboard-accessible
  (arrow keys move the slider).
- **Blink**: an `interval`-based toggle (default 1Hz) that swaps
  the displayed version. Toggle button + speed control.
- **Difference map**: a third-renderer that takes the absolute
  per-pixel delta of the two canvases and applies a 4× gain so
  changes are visible.
- Tests: each tool gets a smoke component test (DOM presence,
  click handler dispatch).

### 3.4 Paperwork

- Bump AstroForge_Spec from 1.3.0 to 1.4.0 (image-canvas surface).
- `SPEC_INDEX.md` gains a 1.4.0 row.
- `M9_AUDIT.md` flips U2 / U3 / U4 from partial to shipped.
- `PROJECT_PLAN.md` notes CR-07 as the active slice.

## 4. Out of scope (CR-07)

- GPU acceleration of the canvas (WebGL / WebGPU). The CR-06 path
  is CPU + WebRender native; canvas 2D is sufficient at ≤ 4 MP.
  WebGL is a §35-follow-up if users hit perf walls.
- Annotations / regions of interest. Distinct CR.
- Export pipelines (PNG / JPEG / FITS round-trip). Covered by
  CR-08 / CR-13.
- Per-version diff metrics (PSNR / SSIM). The difference map is a
  visualization, not a metric.

## 5. Robustness notes

- The `ImageCanvas` decodes 16-bit TIFF in Rust and ships the
  pixels to the frontend as `Uint16Array`. A 4-MP image is 16 MiB
  unencoded; we cap at 8 MP (32 MiB) per version. Larger versions
  fall back to a downsampled 8-bit PNG alongside the full 16-bit
  artifact, with the full artifact accessible through
  `read_image_artifact` for export.
- The path-confinement guard for `read_image_artifact` follows
  the same pattern as `read_preview_artifact`. Both guard against
  absolute-path and `../` traversal; the test asserts a symlink
  pointing outside the dir is refused.
- The split-comparison slider is keyboard-accessible by default
  (the `ImageCanvas` toolbar renders an aria-label "Comparison
  position" and listens for arrow keys).
- The blink comparator is paused when the tab is hidden, so a
  backgrounded tab doesn't burn cycles.

## 6. Verification

- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo test --workspace --no-fail-fast` green (existing
  92 + 642 + 30 + ... tests plus new ImageCanvas tests).
- `npm run check` (vitest + svelte-check) green.
- `npm run e2e` (Playwright smoke: launch Studio, pick two
  versions, confirm both canvases render bytes).
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session` still
  passes.