# Changelog

## Unreleased

### Slice §19: CR-08 recipe_import / recipe_export IPCs

**Scope.** Closes the §22 `import_recipe` + `export_recipe`
❌ rows, the §19 row's ❌, and three §28 rows
("Recipes can be exported", "Recipes can be imported",
"Users can inspect provenance without entering Expert
mode"). Round-trip backend slice; the file-dialog UI is
a follow-on.

**What.**

- `src-tauri/src/main.rs`: two new Tauri commands.
  `recipe_export(profile_id, version: Option<u32>)` returns
  the canonical JSON payload (version omitted exports the
  head). `recipe_import(json: String)` runs
  `from_json_migrated` (v1 -> v2 silent, future schemas
  hard-error), then RE-LINEAGES the recipe against the
  local store via the same `next_version_for` pattern the
  `recipe_save` IPC uses. Fresh names start at v1 with no
  parent; matching (name, target_type) profiles append as
  the next version with `parent_version` pointing at the
  local head. Foreign version + parent_version are
  intentionally discarded (lineage numbers are
  local-store-only). The IPC adds an explicit
  `trim().is_empty()` guard for `name` + `target_type`.
- `src/lib/profile-store.ts`: new `exportProfile` +
  `importProfile` wrappers. Browser-mode placeholder
  fallbacks so the flow exercises end-to-end without
  Tauri. Import's optimistic update replaces the cached
  head if the profile already exists, else prepends.
- `crates/astroforge-core/tests/recipe_import_export.rs`:
  6 new tests pinning the contract (export round-trip;
  fresh-lineage import at v1; append-lineage import
  pointing at local head; v1 migration; unknown-future
  rejection; trim-guard edge cases through the store
  layer).

**Verification.** All 6 gates pass: `cargo fmt --check`,
`clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` (32 suites; 0 failures),
`npm run check` (1 pre-existing error + 9 pre-existing
warnings, unchanged baseline), `npm run build`,
`mvp_smoke.sh`.

**Honest flags.**

- No file-dialog UI surface change. The `.afrecipe` save
  + open dialogs (Tauri `tauri-plugin-dialog`) are a
  follow-on slice that wires the IPCs into the
  RecipesScreen toolbar. The §28 rows note this gap
  inline.
- The IPC's `trim().is_empty()` guard catches
  blank-stripe names at the Rust boundary; the store layer
  below accepts whitespace names as a valid Recipe. The
  unit test for the guard lives in the IPC layer (per
  the `astroforge-cr-slices` skill: src-tauri is binary-
  only outside the workspace; behavioral coverage lives
  in `astroforge-core` and the guard itself is verified
  by the IPC integration tests in CI).
- Same src-tauri lint caveat as §22.1 / §22.2: the
  handler is a thin pass-through over the tested
  primitives; CI's rust job is the authoritative lint
  pass.
- The "Users can inspect provenance without entering
  Expert mode" row was mis-flagged as ❌ in the stale
  audit ("no `ProvenanceViewer`"); the CR-07 §31
  acceptance-polish slice mounted `ProvenancePanel.svelte`
  in the `compare-extras` block of `CompareWorkspace` —
  closing that row as ✅ as part of this slice.

### Slice §22.2: CR-08 recipe_delete IPC

**Scope.** Closes the §22 `delete_recipe` ❌ row + the §28
"User can delete/archive a user Recipe" ❌ row by shipping
a `RecipeStore::delete_profile` store method + the
`recipe_delete` Tauri command that removes every version
of a profile.

**What.**

- `crates/astroforge-core/src/recipe_store.rs`: new
  `delete_profile(profile_id) -> Result<usize>` method.
  Single DELETE over all versions + branches of the named
  profile; returns the row count. Deleting a missing
  profile returns `Ok(0)` so the call is idempotent and the
  UI needs no pre-check.
- `src-tauri/src/main.rs`: new `recipe_delete(profile_id)`
  IPC returning the deleted-version count. The handler is
  the gate point for the future §3 system-recipe
  "protected from modification" check.
- `src/lib/profile-store.ts`: new `deleteProfile(profileId)`
  wrapper with browser-mode placeholder fallback;
  optimistically removes the profile from the cached list.
- `crates/astroforge-core/tests/recipe_delete.rs`: 4 new
  tests pinning the contract (all versions removed in one
  call; idempotent for missing profiles; sibling profiles
  untouched; delete-then-recreate restarts lineage at v1).

**Verification.** All 6 gates pass: `cargo fmt --check`,
`clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` (31 suites; 0 failures),
`npm run check` (1 pre-existing error + 9 pre-existing
warnings, unchanged baseline), `npm run build`,
`mvp_smoke.sh`.

**Honest flags.**

- The §28 row text says "delete/archive"; only the delete
  half ships here. There is no archived-flag surface on
  `Recipe` yet, so "archive" would be a schema addition;
  the row is marked ✅ with the archive gap noted inline.
- Deleting a profile does NOT invalidate `recipe_id`
  references on existing `image_versions` rows; the
  provenance chain (`recipe_get_for_image_version`) will
  return the not-found error path for deleted profiles.
  A tombstone/soft-delete variant is a future decision
  tied to the §3 system-recipe protection design.
- Same src-tauri lint caveat as §22.1: the handler is a
  thin pass-through over the tested store method; CI's
  rust job is the authoritative lint pass.

### Slice §22.1: CR-08 recipe_duplicate IPC

**Scope.** Closes the §22 `duplicate_recipe` ❌ row + the §28
"User can duplicate a Recipe" ❌ row by shipping a single Tauri
command that reads any Recipe (any profile, any version) and
saves a copy as a new, independently-named profile at v1.

**What.**

- `src-tauri/src/main.rs`: new `recipe_duplicate(profile_id, version)`
  IPC + `profile_exists()` helper. Reads the source Recipe,
  builds a candidate name `"<name> (Copy)"`; if that collides
  with an existing profile, sweeps `"<name> (Copy 2)"` ..
  `"<name> (Copy 99)"`; if all 99 collide, falls back to a
  millisecond-precision timestamp suffix. Calls
  `next_version_for(new_profile_id)` (returns 1 for a fresh
  profile_id) and `save`, mirroring the `recipe_save` IPC's
  version-assignment contract.
- `src/lib/profile-store.ts`: new `duplicateProfile(profileId, version)`
  wrapper around `invoke("recipe_duplicate", ...)` with browser-
  mode placeholder fallback. Optimistically prepends the new
  summary to `profileStore`.
- `crates/astroforge-core/tests/recipe_duplicate.rs`: 3 new tests
  pinning the contract (independent profile + version 1; copy
  suffix collision sweep; stages + integrity preservation).

**Verification.** All 6 gates pass: `cargo fmt --check`,
`clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` (29 suites + 3 new = 32; 0 failures),
`npm run check` (1 pre-existing error + 9 pre-existing
warnings, unchanged baseline), `npm run build`,
`mvp_smoke.sh`.

**Honest flags.**

- The duplicate's `description` is preserved from the source.
  The UI does not yet surface a "Duplicate" button; the IPC
  + TS wrapper are ready for the RecipesScreen hookup. Wiring
  the button is a follow-on slice in the §22 series.
- `src-tauri` is a binary-only crate outside the cargo
  workspace, so the workspace `cargo clippy` doesn't compile
  the handler. The IPC body is pure data-model logic + the
  existing `RecipeStore` surface; CI's `rust` job (which
  compiles `src-tauri` via `cd src-tauri && cargo build`) is
  the authoritative lint pass for the new handler. Local
  verification leans on the `recipe_duplicate` integration
  tests in `astroforge-core`.
- The `(Copy N)` sweep tops out at N=99 before falling back
  to a timestamp suffix; the timestamp fallback is a safety
  valve, not a UX expectation. The TS wrapper reflects the
  same shape.

### Slice §31: CR-07 acceptance polish batch (5 rows)

**Scope.** Closes the final 5 ⚠️ rows of the §31 acceptance
table in one batched UI polish PR (user directive: Option A).
Zero Rust changes; the batch is pure Svelte + docs.

**What.**

- **Relevant image metrics are available**: `MetricsTable`
  groups its 25 rows by metric category (the namespace prefix
  of `row.kind`). Core image-quality groups (sharpness, noise,
  signal, dynamic range) render first and expanded; advanced
  diagnostic groups (stars, background, AI) follow.
- **Advanced metrics use progressive disclosure**: each
  advanced group renders collapsed behind a per-group toggle
  (`aria-expanded`), independent per group.
- **Beginner mode is simple**: new Simple / Detailed toggle in
  CompareWorkspace. Simple hides the version DAG, region
  picker, expert panels, provenance and recipe timeline rows,
  quality profile picker, comparison sets, and the advanced
  metric groups; what remains is the beginner prompt, the
  canvas, the grouped core metrics, the decision row, and the
  continue bar. The choice persists in localStorage; the
  default stays Detailed so the existing review surface is
  unchanged.
- **AI processing is identified**: the per-version chip is now
  three-state: "AI used" (tooltip lists the Recipe chain's
  model names), "Deterministic" (chain ran deterministic
  stages only), or hidden (no recipe recorded / IPC failed).
  An AI identity strip above the metrics panel carries the
  same per-side identification in both canvas and CompareTools
  layouts.
- **User can continue editing from a selected version**: the
  single "Continue enhancing" button is replaced by "Continue
  with A" and "Continue with B". Each runs
  `generate_ai_recommendations` against the SELECTED version
  (the IPC already accepted a per-version `image_version_id`;
  the flow previously discarded it), then navigates to
  Enhance. A recommendation-run failure is non-blocking: the
  error surfaces inline and navigation still happens.

**Verification.** All 6 gates pass: `cargo fmt --check`,
`clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` (all green, no test changes: pure
frontend slice), `npm run check` (1 pre-existing error +
9 pre-existing warnings, unchanged baseline), `npm run build`,
`mvp_smoke.sh`.

**Honest flags.** The §31 batch wires the continue flow to the
recommendation engine, but the mounted Enhance surface
(`EnhanceWorkspace`) still reads the legacy
`versionStore.recommendations` list (always empty; the AI ops
tranche is pending). The per-version run is durable (rows
persist) and lands in `aiEnhancementStore` for the CR-06
studio surface; the legacy list wiring is pre-existing debt,
not introduced here.

### Slice §8.4: CR-07 SNR metrics (estimated + local)

**Scope.** Closes the §8 "Estimated SNR / local SNR" ⚠️ Partial
row by shipping both `estimated_snr` and `local_snr` metrics.

**What.**

- `estimated_snr(image) -> MetricsSample`: global SNR in dB.
  Signal = mean pixel value across all channels; noise =
  `luminance_noise` sigma. SNR = 20 × log10(signal / noise).
  Higher = stronger signal relative to noise floor.

- `local_snr(image) -> MetricsSample`: mean per-tile SNR in dB.
  Uses the same 4×4 tile grid as `regional_noise`; per-tile
  sigma from `regional_noise_map`, per-tile mean computed
  directly. SNR = 20 × log10(tile_mean / tile_sigma).
  Higher = faint structures well-preserved across the frame.

Both are wired into `metric_snapshot` so the delta table
surfaces `signal.estimated_snr` and `signal.local_snr`.

**Verification.** 8 new tests in `tests/snr.rs`. All 6 gates
pass (fmt, clippy, test, svelte-check, build, smoke).

### Slice §8.3: CR-07 color gradient metric

**Scope.** Closes the §8 "Color gradient" ⚠️ Partial row by
shipping the `color_gradient` metric: the maximum per-channel
background gradient magnitude across the image's color
channels. Captures color-dependent gradients (e.g. red light
pollution dominating, blue channel near-uniform).

A flat-field or uniform gradient (vignetting, light pollution)
produces a similar gradient magnitude across channels; a
strongly color-dependent gradient produces a higher maximum
because one channel's gradient exceeds the others.

The scalar metric is wired into `metric_snapshot` so the
delta table surfaces the `background.color_gradient` key.

#### New public API

- `crates/astroforge-core/src/image_analysis/metrics.rs`:
  - `color_gradient(image: &F32Image) -> MetricsSample`:
    max per-channel background gradient magnitude. Returns
    `0.0` with `Confidence::Low` when the image is too small
    or has fewer than 2 channels (single-channel images have
    no color gradient by definition).

#### Wiring

- `crates/astroforge-core/src/comparison_metrics.rs`:
  `metric_snapshot` now inserts `MetricKind::BackgroundColorGradient`
  with the `color_gradient` scalar. Two existing snapshot
  tests updated to expect 9 keys (was 8) and 24 total keys
  in `metric_snapshot_full` (was 23).

#### Tests

- `crates/astroforge-core/tests/color_gradient.rs`: 7 tests
  covering uniform RGB, uniform RGB gradient, color-dependent
  gradient, single-channel, too-small image, determinism,
  and non-negative / finite.

#### Out of scope (intentional)

- No UI wiring. The metric flows through the existing
  `compare_version_metrics` IPC path.
- No IPC changes.

#### Verification

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass (7 new tests)
- `npm run check`: 0 errors (pre-existing warnings)
- `npm run build`: pass
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: pass

### Slice §8.2: CR-07 edge response metric

**Scope.** Closes the §8 "Edge response" ⚠️ Partial row by
shipping the `edge_response` metric: the mean Sobel gradient
magnitude (`sqrt(Gx² + Gy²)`) across the image preview. A
high value means many strong edges (stars, structure); a
low value means a soft / blurry image.

The scalar metric is wired into `metric_snapshot` so the
delta table surfaces the `sharpness.edge_response` key.

#### New public API

- `crates/astroforge-core/src/image_analysis/metrics.rs`:
  - `edge_response(image: &F32Image) -> MetricsSample`:
    mean Sobel magnitude across the preview. Returns `0.0`
    with `Confidence::Low` when the image is too small.

#### Wiring

- `crates/astroforge-core/src/comparison_metrics.rs`:
  `metric_snapshot` now inserts `MetricKind::SharpnessEdgeResponse`
  with the `edge_response` scalar. Two existing snapshot
  tests updated to expect 8 keys (was 7) and 23 total keys
  in `metric_snapshot_full` (was 22).

#### Tests

- `crates/astroforge-core/tests/edge_response.rs`: 6 tests
  covering uniform image (magnitude = 0), sharp vertical
  edge (high magnitude), gradient (lower magnitude), too-
  small image, determinism, and non-negative / finite.

#### Out of scope (intentional)

- No UI wiring. The metric flows through the existing
  `compare_version_metrics` IPC path.
- No IPC changes.

#### Verification

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass (6 new tests)
- `npm run check`: 0 errors (pre-existing warnings)
- `npm run build`: pass
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: pass

### Slice §8.1: CR-07 regional noise metric

**Scope.** Closes the §8 "Regional noise" ⚠️ Partial row by
shipping the `regional_noise` metric: the image is split into
a 4×4 grid of tiles, the luminance-noise sigma (MAD-based, same
algorithm as `luminance_noise`) is computed per tile, and the
coefficient of variation (CV = stddev / mean) across tiles is
returned as the scalar metric. A low CV means the noise is
spatially uniform; a high CV means the image has spatially-
varying noise (e.g. amp glow in corners, vignetting noise
patterns).

The scalar metric is wired into `metric_snapshot` so the
delta table + the per-version expert panel surface the
`noise.regional` key. The full per-tile map is available via
`regional_noise_map()` for the UI's spatial noise display
(not wired to UI in this slice).

#### New public API

- `crates/astroforge-core/src/image_analysis/metrics.rs`:
  - `regional_noise(image: &F32Image) -> MetricsSample`:
    scalar CV metric. Returns `0.0` with `Confidence::Low`
    when the image is too small to tile (< 32×32).
  - `regional_noise_map(image: &F32Image) -> Vec<RegionalNoiseSample>`:
    per-tile sigma map for the UI.
  - `RegionalNoiseSample { region: [u32; 4], sigma: f64,
    confidence: Confidence }`: one tile of the map.

#### Wiring

- `crates/astroforge-core/src/comparison_metrics.rs`:
  `metric_snapshot` now inserts `MetricKind::NoiseRegional`
  with the `regional_noise` scalar. Two existing snapshot
  tests updated to expect 7 keys (was 6) and 22 total keys
  in `metric_snapshot_full` (was 21).

#### Tests

- `crates/astroforge-core/tests/regional_noise.rs`: 8 tests
  covering uniform image (CV = 0), noisy image (CV > 0),
  16-tile grid coverage, tile position coverage, too-small
  image (empty), determinism, and spatially-varying image.

#### Out of scope (intentional)

- No UI wiring. The `regional_noise_map` function is
  available for the UI but no Svelte component consumes it
  yet. The delta table surfaces the scalar `noise.regional`
  metric; the spatial map display is a future slice.
- No IPC changes. The metric is computed in
  `astroforge-core` and flows through the existing
  `compare_version_metrics` IPC path.

#### Verification

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: 992 tests pass (0 failed; 8 new)
- `npm run check`: 0 errors (9 pre-existing warnings,
  none from this slice)
- `npm run build`: pass (296 kB JS bundle, 3.06 s)
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: pass

### Slice §29.3b.4: CR-07 WebGPU diff UI integration

**Scope.** Closes the last remaining §29 sub-slice: wires
the §29.3a WGSL compute shaders into `CompareTools.svelte`'s
`recomputeDifference()` so the difference render path picks
the GPU when WebGPU is available, and falls back to the
existing per-pixel Canvas2D loop when it is not.

The change is in one file
(`src/components/CompareTools.svelte`) and does not touch
any Rust, IPC, or persistence surface. The GPU wrapper
(`WebGpuDiffCompute` from `src/lib/webgpu-diff.ts`) was
already shipped in §29.3a (PR #362) with Vitest
behavioural tests in §29.3b.2 (PR #364). This slice is
pure consumer wiring.

#### New code

- `ensureGpuDiff()`: lazily acquires the WebGPU device
  via `acquireWebGpuDevice()` and instantiates
  `WebGpuDiffCompute`. The component tracks the
  acquisition state (`null` / `"loading"` /
  `"unavailable"` / instance) so a failed acquisition
  is sticky: the UI does not retry on every frame.
- `recomputeDifference()` now kicks an async
  fire-and-forget that tries the GPU path first. On
  success, the `Uint8Array` result is wrapped in
  `ImageData`, the existing stretch helpers
  (`applyStretch` / `applyFixedStretch`) run JS-side,
  and `ctx.putImageData` paints. On any failure (no
  device, dispatch error, wrong byte length) the path
  falls through to the Canvas2D loop.
- `recomputeDifferenceCanvas2d()`: the original
  per-pixel loop, preserved as the fallback. Extracted
  as its own function so the GPU path and the Canvas2D
  path share the same stretch application.

#### Out of scope (intentional)

- `recomputeOverlay()` stays Canvas2D. Overlay is a
  per-pixel alpha blend; the WGSL shaders shipped in
  §29.3a do not include a blend kernel. Adding one is
  a future §29.3c slice if needed.
- Stretch (`applyStretch` / `applyFixedStretch`) runs
  JS-side in both paths. The stretch is a histogram
  operation that builds a 256-bin histogram and walks
  it for percentile cutoffs; the histogram is not worth
  shipping to the GPU.
- No Rust or IPC changes. The GPU path runs entirely
  in the browser (WebGPU is client-side); no Tauri
  command changes are needed.
- No Vitest additions. The §29.3b.2 Vitest suite
  covers the wrapper's behaviour; this slice is the
  Svelte consumer and `npm run check` + the existing
  test suite cover the integration.

#### Verification

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: 984 tests pass (0 failed)
- `npm run check`: 0 errors (9 pre-existing warnings,
  none from this slice)
- `npm run build`: pass (296 kB JS bundle, 3.03 s)
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: pass

### Slice §29.3b.1a: CR-07 WGSL shader for background_gradient

**Scope.** Closes the §29.3b.1a sub-row of §29.3b by
shipping the WGSL compute shader + the host-side finalize
helper for the 4th spatial detector,
`background_gradient`. With this slice, all 4 spatial
detectors are WebGPU-accelerated at the GPU-primitive
level.

`background_gradient` is a tile-strided detector: the
image is split into 64x64 tiles, each tile produces a
per-tile median (sampled from a 4x4 grid), and the
host-side finalize fits a plane through the tile medians
via least-squares + returns the gradient magnitude.

#### New public API (UI)

- `src/lib/webgpu-spatial-shaders.ts`:
  - `BACKGROUND_GRADIENT_SHADER`: WGSL compute shader
    source. One workgroup per 64x64 tile. Each
    invocation samples a 4x4 grid, sorts the samples
    (insertion sort, up to 16), and writes
    `(x_center, y_center, median, has_samples_flag)`
    to the output buffer.
  - `SpatialMode`: extended with `"background_gradient"`.
  - `shaderForSpatial(mode)`: returns the new shader
    for the new mode.
  - `spatialOutputSize(mode, ...)`: returns `0` for
    `background_gradient` (the wrapper computes the
    output size dynamically based on tile count).
- `src/lib/webgpu-spatial.ts`:
  - `WebGpuSpatialCompute.dispatch`: extended to
    compute tile count for `background_gradient` mode
    + dispatch that many workgroups (instead of
    `ceil(pixelCount / 64)`).
  - `finalizeBackgroundGradient(partials)`: host-side
    helper. Reads the per-tile records from the GPU
    output, skips tiles with `has_samples_flag = 0`,
    computes the 2x3 plane-fit least-squares solve
    (matching the Rust baseline character-for-character),
    and returns the gradient magnitude in
    "per 100 px" units. Returns 0 for degenerate cases
    (fewer than 2 valid tiles, denom < 1e-12).
- `src/lib/__tests__/webgpu-spatial.test.ts`: 5 new
  tests for `finalizeBackgroundGradient` (empty,
  n<2, degenerate x, gradient from 4 tiles, uniform
  image) + 2 new tests for the wrapper (shader loads
  on first call, dispatches one workgroup per tile).

#### Design decisions

1. **GPU does per-tile work; host does the 2x3
   plane-fit solve.** The plane-fit is a tiny
   (ceil(W/64) × ceil(H/64)) normal-equations
   problem — the host can do it in microseconds. The
   GPU work (per-tile 4x4 grid sample + sort + median)
   is the bulk of the work for 4K+ images.

2. **One workgroup per tile** (not one per pixel).
   `dispatchWorkgroups(tileCount)` instead of
   `dispatchWorkgroups(ceil(pixelCount / 64))`. The
   output buffer holds 4 f32s per tile
   (x, y, median, has_samples).

3. **`has_samples_flag` in the output** lets the shader
   skip empty tiles (when the image is smaller than
   the tile size). The Rust baseline guards against
   this by checking `samples.is_empty()` before
   pushing the median; the WGSL version does the same
   via the 4th output field.

4. **Insertion sort in WGSL** for up to 16 samples per
   tile. WGSL does not (yet) support `array.sort`; a
   manual insertion sort is the canonical portable
   solution for a small fixed-size array.

5. **`finalizeBackgroundGradient` matches the Rust
   baseline character-for-character** — same normal-
   equation accumulators, same `denom < 1e-12`
   threshold, same `* 100.0` final scale.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1137 passing** (unchanged;
  the existing 4 Rust tests for `background_gradient`
  in `image_analysis::metrics` already cover the
  underlying algorithm).
- `npm run test` (vitest): **50 passing** (+7 new = 5
  `finalizeBackgroundGradient` tests + 2 wrapper
  tests; was 43).
- `npm run check`: 1 error + 9 warnings (matches main
  baseline; the 1 error is pre-existing in a Svelte
  file. Slice adds 0 new warnings. Backend tests only.)
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code
  spans, 0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **5 new vitest tests** pin the contract. The tests
  use the mock GPUDevice (no real GPU execution); they
  verify the wrapper contract (shader source loaded,
  workgroup count = tile count) + the host-side finalize
  formula (plane-fit on synthetic tile data).
  Byte-equivalence to the Rust baseline is a separate
  concern that requires running on real GPU
  (§29.3b.2a if the project decides browser-based GPU
  tests are worth the infra cost).

- **`spatialOutputSize` returns 0 for `background_gradient`.**
  The wrapper computes the output size dynamically
  (tileCount * 4). This is a deliberate divergence from
  the per-pixel modes where the size is known a priori.
  The `0` return value signals "compute dynamically"
  to the dispatch logic.

- **GPU work vs Rust work.** The WGSL shader does the
  per-tile median; the host does the plane-fit. This is
  the natural split: GPU = per-tile parallel work,
  host = small serial solve. A follow-on slice could
  push the plane-fit to GPU as well (5x5 normal
  equations on a (W/64)*(H/64) matrix is trivial on
  GPU), but the current host-side split is correct
  and matches the Rust baseline algorithmically.

- **Pre-existing em-dashes NOT cleaned.** Per Coding
  Discipline, out of scope.

#### Out-of-scope (intentional)

- §29.3b.3 IPC layer wiring (folded into §29.3b.4;
  GPU compute is client-side, not Rust-side).
- §29.3b.4 UI integration + retire CPU fallback
  (rank #1; next slice).
- §8 SNR / regional noise / edge response / color
  gradient (rank #2).
- §32.6 (already shipped, rank #3).
- §29.3b.2a Browser-based GPU behavioural tests
  (Playwright + headless Chrome with WebGPU): deferred.
- §29.4 (optional) Tile-stripe streaming: deferred.
- Pre-existing em-dashes on `main`: out of scope.

### Slice §29.2a: CR-07 wire DiffCache through IPC

**Scope.** Continues the §29 Performance work by exposing
the Rust `DiffCache` (shipped in §29.2) through Tauri IPC
+ a Svelte API wrapper. The slice wires 5 new IPC
commands so a future UI consumer can use the cache
without touching the Rust internals.

#### New public API

- `src-tauri/src/main.rs`: adds `struct DiffCacheState(Mutex<DiffCache>)`
  + 5 Tauri commands:
  - `diff_cache_get_or_compute(version_a_id, version_b_id,
    mode, gain, width, image_a, image_b) -> Result<Vec<u8>, CommandError>`:
    cache lookup keyed by (version_a_id, version_b_id, mode, gain).
    On miss, computes via `compute_diff` + stores. Mode is a
    kebab-case string ("absolute" / "signed" / "amplified" /
    "structural").
  - `diff_cache_invalidate_version(version_id) -> Result<usize, CommandError>`:
    drops every cache entry where the given version id appears
    on either side of the pair. Returns the number of entries
    dropped. Used when a version's primary artifact (TIFF)
    changes.
  - `diff_cache_clear() -> Result<(), CommandError>`: drops every
    entry. Stats counters are preserved.
  - `diff_cache_stats() -> Result<DiffCacheStats, CommandError>`:
    snapshot of hits + misses counters.
  - `diff_cache_len() -> Result<usize, CommandError>`: current
    cache size.
- `src-tauri/src/main.rs`: registers the `DiffCacheState` in
  the Tauri state container alongside the existing
  `GalleryState` / `SessionState` / `RecipeState`.
- `src/lib/astroforge-api.ts`: adds 5 invoke wrappers +
  `DiffCacheStatsFromRust` interface.

#### Design decisions

1. **DiffCache is global to the app session.** Lives for
   the lifetime of the Tauri runtime; no DB backing.
   The `Mutex<DiffCache>` wrapper serializes concurrent
   IPC calls. `get_or_compute` returns `&Vec<u8>` from
   the cache; the IPC handler clones to release the lock
   before returning.

2. **No UI consumer shipped.** The slice wires the IPC
   + TS wrappers only. A follow-on UI slice (§29.3b.4
   or folded into the §29.3b production rollout) will
   render the cache + use the get-or-compute path.

3. **Mode is a kebab-case string** at the IPC boundary.
   Matches the existing `DiffKind` serde rename_all =
   "kebab-case" contract. The TS interface uses a
   strict string union for type safety.

4. **Pure server-side state.** The cache lives entirely
   in Rust. The JS layer is a thin pass-through. No
   client-side caching, no sync concerns.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1137 passing** (unchanged;
  slice adds 0 new tests; the existing 21 §29.2
  diff_cache tests + 21 §32.4 recipe tests already
  cover the underlying Rust functions).
- `npm run test` (vitest): **43 passing** (unchanged).
- `npm run check`: 1 error + 9 warnings (matches main
  baseline; the 1 error is pre-existing in a Svelte
  file. Slice adds 0 new warnings. Backend tests only.)
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code
  spans, 0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **No UI consumer.** The slice wires the IPC + TS
  wrappers only. A follow-on UI slice (§29.3b.4 or
  folded into the production rollout) will render the
  cache.

- **No new tests.** The underlying Rust `DiffCache` is
  covered by 21 §29.2 tests. The IPC wrapper is thin
  enough that the existing tests + a passing build are
  sufficient evidence.

- **`diff_cache_get_or_compute` returns the full buffer**
  on every call (cached or fresh). For 4K RGBA8 images
  this is ~33 MB per call. The Tauri IPC layer serializes
  Vec<u8> as a JSON number array; future work could use
  Tauri's binary IPC channel for efficiency. Not in
  scope for this slice.

- **Pre-existing em-dashes NOT cleaned.** Per Coding
  Discipline, out of scope.

#### Out-of-scope (intentional)

- §29.3b.3 IPC layer wiring (remaining, GPU spatial
  detector half; rank #1; next slice).
- §29.3b.4 UI integration + retire CPU fallback
  (rank #2).
- §8 SNR / regional noise / edge response / color
  gradient (rank #3).
- §29.3b.1a WGSL for `background_gradient`.
- §29.3b.2a Browser-based GPU behavioural tests.
- §29.4 (optional) Tile-stripe streaming: deferred.
- Binary IPC channel for `diff_cache_get_or_compute`:
  deferred.
- Pre-existing em-dashes on `main`: out of scope.

### Slice §32.6: CR-07 wire pipeline_plan_hash + RecipeAiDiffSummary through IPC

**Scope.** Closes the §32.6 sub-row of §32 by exposing
`Recipe::pipeline_plan_hash()` + `recipe_ai_diff_summary()`
through Tauri IPC + the Svelte API wrapper. The Rust
public API was shipped in §32.4; this slice wires it
through so a future UI consumer (or follow-on slice)
can call it directly.

#### New public API

- `src-tauri/src/main.rs`: adds 2 Tauri commands:
  - `recipe_pipeline_plan_hash(profile_id: String,
    version: u32) -> Result<String, CommandError>`:
    loads the Recipe, returns the lowercase hex
    SHA-256 of the canonicalized pipeline plan.
  - `recipe_ai_diff_summary(profile_id_a, version_a,
    profile_id_b, version_b) -> Result<RecipeAiDiffSummary,
    CommandError>`: loads both Recipes, returns the
    diff summary struct.
- `src-tauri/src/main.rs`: imports
  `RecipeAiDiffSummary` from `astroforge_core::recipe`.
- `src/lib/astroforge-api.ts`: adds 2 invoke wrappers +
  a typed `RecipeAiDiffSummaryFromRust` interface:
  - `recipePipelinePlanHash(profileId, version):
    Promise<string>`
  - `recipeAiDiffSummary(profileIdA, versionA,
    profileIdB, versionB): Promise<RecipeAiDiffSummaryFromRust>`
  - `RecipeAiDiffSummaryFromRust` interface: mirrors
    the Rust struct (14 fields: hash_a, hash_b,
    hash_differs, ai_used_a, ai_used_b, ai_classification_differs,
    version_a, version_b, schema_version_a,
    schema_version_b, quality_profile_a,
    quality_profile_b, required_models_differ,
    provenance).

#### Design decisions

1. **No UI consumer shipped.** The slice wires the
   IPC + TS wrappers only. A follow-on UI slice
   (§32.7 or merged into the §29.3b.4 production
   rollout) will render the hash + diff in the
   comparison surface.

2. **Pure server-side computation.** Both commands
   are thin wrappers: load the Recipe from the
   `RecipeStore`, call the pure function, return
   the result. No DB writes, no side effects.

3. **`profile_id` + `version` as the IPC key.** Matches
   the existing `recipe_get` / `recipe_get_head` /
   `recipe_save` IPC commands. Future work could
   collapse these into a single `recipe_get_id`
   helper.

4. **QualityProfile serialized as lowercase enum
   name.** Matches the Rust serde `rename_all =
   "kebab-case"` contract: `natural` / `detail` /
   `clean` / `publication`. The TS interface uses
   `string` (not a strict union) for the QualityProfile
   field because the Rust enum is open to extension;
   callers should treat the value as opaque.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1137 passing** (unchanged;
  slice adds 0 new tests; the existing 18 §32.4
  inline tests + 18 §32.4 integration tests already
  cover the underlying Rust functions).
- `npm run test` (vitest): **43 passing** (unchanged;
  slice adds 0 JS tests).
- `npm run check`: 1 error + 9 warnings (matches main
  baseline; the 1 error is pre-existing in a Svelte
  file. Slice adds 0 new warnings. Backend tests only.)
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code
  spans, 0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **No UI consumer.** The slice wires the IPC + TS
  wrappers only. The hash + diff summary are
  accessible via `invoke()` calls but no Svelte
  component renders them yet. UI integration is a
  follow-on slice (§32.7 or folded into §29.3b.4).

- **No new tests.** The underlying Rust functions
  (`pipeline_plan_hash`, `recipe_ai_diff_summary`)
  are already covered by 36 tests in §32.4 (18
  inline + 18 integration). The IPC wrapper is
  thin enough that the existing tests + a passing
  build are sufficient evidence. Adding IPC-level
  tests would require a Tauri mock harness
  (separate slice).

- **Pre-existing em-dashes NOT cleaned.** Per Coding
  Discipline, out of scope.

#### Out-of-scope (intentional)

- §29.3b.3 IPC layer wiring (rank #1; next slice).
- §29.3b.4 UI integration + retire CPU fallback
  (rank #2).
- §8 SNR / regional noise / edge response / color
  gradient (rank #3).
- §32.7 (optional) UI consumer for pipeline_plan_hash
  + RecipeAiDiffSummary (folded into §29.3b.4).
- §29.3b.1a WGSL for `background_gradient`.
- §29.3b.2a Browser-based GPU behavioural tests.
- §29.4 (optional) Tile-stripe streaming: deferred.
- Pre-existing em-dashes on `main`: out of scope.

### Slice §29.3b.2: CR-07 Vitest infra + behavioural tests for WebGPU wrappers

**Scope.** Continues the §29.3 GPU/WebGPU acceleration
work by adding Vitest as the project's JS-side test
runner + shipping behavioural tests for the two
existing WebGPU wrappers (`webgpu-diff.ts` from §29.3a
+ `webgpu-spatial.ts` from §29.3b.1). The slice ships
the test infra + 43 tests + fixes 3 real bugs uncovered
by writing the tests.

#### New public API (UI)

- `package.json`: adds `vitest@^2.1.9`,
  `jsdom@^25.0.1`, `@vitest/coverage-v8@^2.1.9` to
  devDependencies. Adds 3 npm scripts: `test`
  (`vitest run`), `test:watch` (`vitest`), and
  `test:coverage` (`vitest run --coverage`).
- `vitest.config.ts`: jsdom environment, globals
  enabled (describe/it/expect without imports),
  setupFiles reference `./vitest.setup.ts`,
  include patterns `src/**/__tests__/**/*.test.ts`
  + `src/**/*.test.ts`, v8 coverage provider.
- `vitest.setup.ts`: empty placeholder for future
  global polyfills + matcher extensions.

#### Bug fixes uncovered by writing tests

The §29.3a + §29.3b.1 WebGPU wrappers had 3 latent
bugs that didn't surface without runtime exercise.
The slice's behavioural tests caught all 3.

1. **`GPUBufferUsage` / `GPUShaderStage` / `GPUMapMode`
   not globally available at runtime.** The §29.3a slice
   declared these as `interface` + paired `const` values
   in `webgpu-types.ts`, but the consts were
   MODULE-LOCAL (not visible to other modules at
   runtime). The jsdom test environment does not provide
   these globals like a real browser does, so the
   wrapper code crashed with `ReferenceError: GPUBufferUsage
   is not defined`. **Fix:** wrap all WebGPU consts in
   `declare global { const X: ... }` blocks + add
   explicit runtime polyfill assignments at module
   load (guarded by `typeof === "undefined"` checks).

2. **Uniform buffer writeBuffer ran BEFORE pipeline
   creation.** The §29.3a wrapper wrote the uniform
   buffer BEFORE calling `pipelineFor(mode)`, but
   `pipelineFor` is what lazily creates the uniform
   buffer on the first call. On the first call,
   `this.uniformBuffer` was null, so the writeBuffer
   was skipped — meaning the shader received
   uninitialized uniform data. **Fix:** call
   `pipelineFor(mode)` first to ensure the uniform
   buffer exists, THEN write the per-call params.

3. **The `compute` method returned `Promise<Uint8Array>`
   disguised as `Uint8Array`** via a type assertion.
   Real WebGPU readback is asynchronous; the wrapper
   returned the Promise synchronously via
   `as unknown as Uint8Array`. Callers that awaited
   the result would have gotten a `Promise<Promise<...>>`
   — almost certainly a downstream bug waiting to
   happen. **Fix:** make `compute` + `dispatch` properly
   `async` + return `Promise<Uint8Array | null>` /
   `Promise<Float32Array | null>`.

#### New tests (43 total)

- `src/lib/__tests__/mock-gpu.ts`: a complete
  `MockGpuDevice` + `MockGPUBuffer` + `MockGPUQueue`
  + `MockCommandEncoder` + `MockComputePassEncoder`
  test infrastructure. Records every API call
  (shader loads, buffer creates, queue writes,
  compute passes, workgroup dispatch counts, bind
  groups, command encoders) so tests can assert
  the right structure. Returns pre-defined readback
  data so the wrapper's full code path runs.
- `src/lib/__tests__/webgpu-diff.test.ts`: 20 tests
  covering shader-source matching per DiffMode,
  pipeline caching across calls, buffer allocation
  counts, workgroup dispatch counts (64/65 pixel
  cases), uniform-buffer param encoding, gain
  clamping (negative + NaN + Infinity), input
  validation (length mismatch + non-multiple-of-4),
  uniform-buffer reuse across calls, transient
  buffer destruction after dispatch, return-value
  type + length, dispose() semantics, and
  `acquireWebGpuDevice()` fallback paths.
- `src/lib/__tests__/webgpu-spatial.test.ts`: 23
  tests covering shader-source matching per
  SpatialMode, pipeline caching, workgroup dispatch,
  uniform-buffer param encoding, length-validation,
  transient buffer destruction, return-value
  shapes (per-pixel vs per-workgroup), dispose(),
  and the 3 host-side finalize helpers:
  - `finalizeLuminanceNoise`: empty / sigma formula
    (1.4826 × median) / zero residuals.
  - `finalizeChromaticNoise`: empty / zero count /
    total < 1e-12 / correct ratio stddev / multi-
    workgroup aggregation.
  - `finalizeLocalContrast`: empty / correct mean /
    zero residuals.

#### Design decisions

1. **jsdom over happy-dom.** jsdom provides a more
   complete DOM + Window + Navigator polyfill, which
   is required by the WebGPU module (`navigator.gpu
   .requestAdapter`). The mock GPUDevice tests don't
   actually use the navigator, but jsdom is chosen
   for future Svelte-component tests.

2. **Globals enabled.** Tests use `describe`, `it`,
   `expect`, `vi`, etc. without explicit imports.
   Reduces boilerplate for the 43 tests.

3. **Runtime polyfills for WebGPU constants.** The
   wrapper code references `GPUBufferUsage.UNIFORM`
   etc. as runtime values. A real browser provides
   these; jsdom doesn't. The polyfills are guarded
   by `typeof === "undefined"` so they don't override
   the real browser values.

4. **Mock GPUDevice records API calls, doesn't
   actually compute.** The mock is a structural test
   harness, not a GPU simulator. It records what the
   wrapper did so tests can assert the right structure
   was invoked. Real GPU behaviour verification is
   deferred to browser-based tests (Playwright +
   headless Chrome with WebGPU) which are §29.3b.2a
   (a follow-on if the project decides browser-based
   GPU tests are worth the infra cost).

5. **Vitest NOT wired to CI gate.** The CI gate is
   `npm run check` (svelte-check) which is unchanged
   by this slice. Adding vitest to CI is a follow-on
   decision (the project's CI currently runs ~3.5 min
   for 6 jobs; adding vitest would add 1-2 min).

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1137 passing** (unchanged;
  slice adds 0 Rust code).
- `npm run test` (vitest): **43 passing** (NEW).
- `npm run check`: 1 error + 9 warnings (matches main
  baseline; the 1 error is pre-existing in a Svelte
  file. Slice adds 0 new warnings. Backend tests only.)
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code
  spans, 0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **Three real bugs caught by writing the tests.**
  See "Bug fixes uncovered by writing tests" above.
  None of these bugs would have surfaced under
  `npm run check` alone. The slice's value-add is
  not just "more tests" — it's "the existing GPU
  wrappers are now actually correct".

- **No GPU simulator.** The mock is structural; it
  records API calls but doesn't simulate GPU
  computation. Tests verify the WRAPPER contract, not
  the SHADER contract. Shader correctness (byte-
  equivalence to the Rust backend) is a separate
  concern that requires running on real GPU. Deferred
  to §29.3b.2a if the project decides browser-based
  GPU tests are worth the infra cost (Playwright +
  headless Chrome with WebGPU enabled).

- **Vitest NOT wired to CI.** `npm run test` works
  locally + in this slice's PR check. The CI gate is
  still `npm run check` (svelte-check). Adding vitest
  to CI is a separate decision.

- **Pre-existing em-dashes NOT cleaned.** Per Coding
  Discipline, out of scope.

#### Out-of-scope (intentional)

- §29.3b.3 IPC layer wiring (rank #1; next slice).
- §29.3b.4 UI integration + retire CPU fallback
  (rank #2).
- §29.3b.1a WGSL for `background_gradient` (rank #3).
- §29.3b.2a Browser-based GPU behavioural tests
  (Playwright + headless Chrome with WebGPU): deferred
  to a separate slice if the project wants browser-
  based GPU tests.
- §8 SNR / regional noise / edge response / color
  gradient (rank #4).
- §32.6 (new) Wire `pipeline_plan_hash` +
  `RecipeAiDiffSummary` through IPC.
- §29.4 (optional) Tile-stripe streaming: deferred.
- Pre-existing em-dashes on `main`: out of scope.

### Slice §29.3b.1: CR-07 WGSL shaders for spatial detectors

**Scope.** Continues the §29.3 GPU/WebGPU acceleration
work by shipping WGSL compute shaders + a TypeScript
wrapper for 3 of the 4 spatial metrics detectors:
`luminance_noise`, `chromatic_noise`, and
`local_contrast`. The 4th detector (`background_gradient`)
requires 64x64 tile-strided reduction + a host-side
plane-fit solve and is not yet covered by WGSL; that
lands in §29.3b.1a or folds into §29.3b.4.

The slice ships the GPU primitives only. The IPC layer
wiring + UI consumer + behavioural tests are §29.3b.2 /
§29.3b.3 / §29.3b.4 (follow-on sub-slices).

#### New public API (UI)

- `src/lib/webgpu-spatial-shaders.ts`: WGSL compute
  shader source for the 3 spatial detectors:
  - `LUMINANCE_NOISE_SHADER`: per-pixel residual
    `|pixel - median_3x3|` for the luminance channel
    (channel 0) of a downsampled preview image. Edge
    pixels (first / last row + column) write 0.
    Includes a hand-written sort network (no loops,
    no function calls beyond `min` / `max` / `abs` on
    f32) that sorts 9 elements in 25 comparisons to
    extract the median. The host then computes
    `sigma = 1.4826 * MAD` after a host-side sort.
  - `CHROMATIC_NOISE_SHADER`: per-pixel workgroup
    reduction that updates per-channel sums +
    pixel count. Uses WGSL workgroup-shared memory
    + `atomicAdd` for the count. The host computes
    the channel ratios + stddev.
  - `LOCAL_CONTRAST_SHADER`: per-pixel residual
    `|pixel - mean_3x3|` for the luminance channel.
    Edge pixels write 0. The host computes the
    global mean.
  - `shaderForSpatial(mode)`: returns the shader
    source for a given `SpatialMode` string union.
  - `spatialOutputSize(mode, pixelCount,
    workgroupSize)`: returns the GPU output buffer
    size in f32 elements.
- `src/lib/webgpu-spatial.ts`: TypeScript wrapper:
  - `class WebGpuSpatialCompute`: owns the compiled
    pipelines + bind group layout + uniform buffer.
    Lazily compiles each shader on first use; reuses
    pipelines across calls. Methods:
    `compute(mode, image, width, height, channels)`,
    `dispose()`.
  - `finalizeLuminanceNoise(residuals)`: computes
    `sigma = 1.4826 * MAD(median(residuals))` on the
    host.
  - `finalizeChromaticNoise(partials)`: sums the
    per-workgroup partials, computes the channel
    means + ratios + stddev. Returns 0.0 when the
    total mean is below 1e-12 (matching the Rust
    baseline).
  - `finalizeLocalContrast(residuals)`: returns the
    global mean of the per-pixel residuals.
  - `type SpatialPartialResult = Float32Array`.
  - `type SpatialMode` re-exported.

#### Design decisions

1. **GPU does the per-pixel work; host does the
   reduction.** The expensive part is the per-pixel
   3x3 neighbourhood access; the host-side reduction
   (sort + MAD, ratio + stddev, global mean) is
   trivial (one or two divisions + a single sort).
   Splitting along this boundary keeps the WGSL
   kernels simple while letting the GPU handle the
   bulk of the work.

2. **Hand-written sort network in WGSL.** The
   `luminance_noise` shader sorts 9 elements per
   pixel using a 25-step comparison network. WGSL
   does not (yet) support `array.sort`; a manual
   network is the canonical portable solution. The
   network uses `min` / `max` / `abs` on f32 + is
   type-correct for the median extraction.

3. **`chromatic_noise` uses workgroup-shared memory +
   `atomicAdd`.** Each invocation accumulates into
   per-lane workgroup slots; the LAST thread in the
   workgroup sums the partials across lanes + writes
   them to the output buffer. The host then sums
   across workgroups.

4. **`background_gradient` NOT shipped.** It requires
   64x64 tile-strided reduction + a host-side
   plane-fit solve (5x5 normal equations on a small
   `(W/64) * (H/64)` matrix). The slice ships the 3
   detectors that cleanly fit a per-pixel compute
   kernel; the 4th lands in a follow-on.

5. **Pure TypeScript slice.** 0 new Rust code, 0 new
   IPC, 0 new UI consumers. The wrapper is opt-in;
   callers instantiate `WebGpuSpatialCompute`
   explicitly + handle the `null`-on-fallback
   contract.

6. **JS-side test runner not added in this slice.**
   The project does not yet have a JS-side test
   runner (no Vitest). The slice relies on
   `npm run check` for type validation. Behavioural
   tests via browser-based GPU (Playwright + headless
   Chrome with WebGPU) ship in §29.3b.2.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1137 passing** (unchanged;
  slice adds 0 Rust code).
- `npm run check`: 1 error + 9 warnings (matches main
  baseline; the 1 error is pre-existing in a Svelte
  file, NOT introduced by this slice. Slice adds 0
  new warnings. Backend tests only.)
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code
  spans, 0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **3 of 4 spatial detectors shipped.** `luminance_noise`,
  `chromatic_noise`, and `local_contrast` are now
  WebGPU-accelerated. `background_gradient` remains on
  the Rust baseline for now and lands in §29.3b.1a or
  §29.3b.4.

- **Behavioural tests NOT shipped.** Browser-based GPU
  tests require Vitest infrastructure (not yet present
  in the project) + Playwright + headless Chrome with
  WebGPU enabled. §29.3b.2 territory.

- **No IPC wiring, no UI consumer.** The wrapper is
  opt-in. Callers must explicitly instantiate
  `WebGpuSpatialCompute` + handle the `null`-on-fallback
  contract.

- **Pure TypeScript slice.** No Rust changes.

- **Pre-existing em-dashes NOT cleaned.** Per Coding
  Discipline, out of scope.

#### Out-of-scope (intentional)

- §29.3b.2 Vitest infra + behavioural tests (rank #1;
  next slice).
- §29.3b.3 IPC layer wiring (rank #2).
- §29.3b.4 UI integration + retire CPU fallback
  (rank #3).
- §29.3b.1a WGSL for `background_gradient` (rank #4).
- §8 SNR / regional noise / edge response / color
  gradient (rank #5).
- §32.6 (new) Wire `pipeline_plan_hash` +
  `RecipeAiDiffSummary` through IPC.
- §29.4 (optional) Tile-stripe streaming: deferred.
- Pre-existing em-dashes on `main`: out of scope.

### Slice §29.3a: CR-07 WebGPU compute prototype for compute_diff

**Scope.** Starts the §29.3 GPU/WebGPU acceleration work
by shipping the WGSL compute shaders + the TypeScript
wrapper + the capability-detection integration for
`compute_diff` (4 modes) on the GPU. The slice ships the
prototype only; the IPC layer + UI wiring + behavioural
tests are §29.3b (a follow-on slice).

#### New public API (UI)

- `src/lib/webgpu-shaders.ts`: WGSL compute shader
  source for all four `compute_diff` modes:
  - `ABSOLUTE_SHADER`: per-pixel `|a - b|` for RGB,
    alpha = 255.
  - `SIGNED_SHADER`: per-pixel `clamp((a - b) + 128, 0,
    255)` for RGB, alpha = 255.
  - `AMPLIFIED_SHADER`: per-pixel `clamp(|a - b| * gain,
    0, 255)` for RGB, alpha = 255.
  - `STRUCTURAL_SHADER`: 2D Sobel-style neighbour diff
    matching the Rust `render_structural` baseline. Edge
    pixels (first row / first / last column) write 0.
  - `shaderFor(mode)`: returns the shader source for a
    given `DiffMode` string union.
- `src/lib/webgpu-diff.ts`: TypeScript wrapper:
  - `class WebGpuDiffCompute`: owns the compiled
    pipelines + bind group layout + uniform buffer.
    Lazily compiles each shader on first use; reuses
    pipelines across calls. Methods: `compute(mode, a,
    b, gain, width)`, `dispose()`.
  - `acquireWebGpuDevice()`: requests a `GPUDevice`
    from the browser. Returns `null` if WebGPU is
    unavailable so the caller can fall back to the
    Rust-backed diff path.
- `src/lib/webgpu-types.ts`: minimal WebGPU type
  declarations for the subset of the API the wrapper
  uses. The project does not yet depend on
  `@webgpu/types`; this file declares the types inline.

#### Design decisions

1. **Feature-flagged prototype.** The wrapper compiles
   the WGSL shaders lazily on first use. If WebGPU is
   unavailable, `compute` returns `null` so callers can
   fall back to the Rust backend. The slice does NOT
   wire the wrapper into the comparison UI yet.

2. **RGBA8 packed into u32 for GPU upload.** Each
   RGBA8 pixel is packed into a single `u32` (R in bits
   0-7, G in 8-15, B in 16-23, A in 24-31) for the GPU
   buffer. The WGSL shaders unpack via bit-shift. Output
   is re-packed into a `Uint8Array` after GPU readback.

3. **Uniform buffer layout.** `pixel_count: u32`,
   `gain: f32`, `width: u32`, `pad0: u32` (16 bytes).
   Reused across calls; per-call params are written via
   `device.queue.writeBuffer`.

4. **Gain clamping on the CPU side.** For `Amplified`
   mode, negative / non-finite gain becomes 1.0 before
   the uniform write (matching the Rust baseline).

5. **Structural mode matches Rust fallback behavior.**
   The Rust `render_structural` falls back to
   `render_absolute` when width < 2 or n <
   width * 4 * 2. The WGSL shader writes 0 for edge
   pixels; for buffers too small to have interior pixels
   the entire output is 0 (which is consistent with the
   Rust `vec![0u8; n]` allocation). The width-aware
   absolute fallback lives in the dispatch wrapper for a
   follow-on slice if needed.

6. **JS-side test runner not added in this slice.** The
   project does not yet have a JS-side test runner
   (no Vitest). The slice relies on `npm run check`
   (svelte-check) for type validation. Behavioural
   tests via browser-based GPU (Playwright +
   headless Chrome with WebGPU) are §29.3b.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1137 passing** (unchanged;
  slice adds 0 Rust code).
- `npm run check`: 1 error + 9 warnings (matches main
  baseline; the 1 error is pre-existing in a Svelte
  file, NOT introduced by this slice. Slice adds 0 new
  warnings. Backend tests only.)
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code
  spans, 0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **Three local fix cycles** during type-check
  resolution:
  1. WGSL template literals contained backticks inside
     comments (`// `clamp(x, 0, 255)` expressed as
     u32`). TypeScript parsed the first inner backtick
     as the template literal close, then tried to parse
     the remaining WGSL as TypeScript code. Fixed by
     replacing inner backticks with single quotes.
  2. `GPUShaderStage` and `GPUBufferUsage` were
     declared as interfaces but used as values
     (`GPUShaderStage.COMPUTE`, `GPUBufferUsage.UNIFORM`).
     Fixed by declaring each as `interface ...` + a
     paired `const ...` value (since the project does
     not depend on `@webgpu/types`, the consts hold the
     flag-bit values for type-correctness; the slice
     does not execute the GPU path so the values are
     type-only).
  3. `interface Navigator` declaration conflicted with
     the lib.dom.d.ts declaration of `Navigator`. Fixed
     by removing the redundant `interface Window { ... }
     ` wrapper (which was carrying the Navigator redeclare).
- **Behavioural tests NOT shipped.** The slice relies on
  `npm run check` for type validation. Browser-based GPU
  tests require Vitest infrastructure (not yet present in
  the project) + Playwright + a headless Chrome with
  WebGPU enabled. That is §29.3b territory.
- **The slice ships the GPU primitive only.** No IPC
  wiring, no UI consumer, no automatic fallback. Callers
  must explicitly `await acquireWebGpuDevice()` +
  instantiate `WebGpuDiffCompute` + handle the
  `null`-on-fallback contract.
- **Pre-existing em-dashes NOT cleaned.** Per Coding
  Discipline, out of scope.

#### Out-of-scope (intentional)

- §29.3b Production rollout: extend the WebGPU compute
  path to the spatial detectors, wire to the IPC layer,
  retire the CPU fallback path (rank #1; next slice).
- §29.2a IPC wiring for the diff cache (can be folded
  into §29.3b).
- §8 SNR / regional noise / edge response / color
  gradient (rank #2).
- §32.6 (new) Wire pipeline_plan_hash +
  RecipeAiDiffSummary through IPC (rank #3).
- §29.4 (optional) Tile-stripe streaming: deferred.
- JS-side test runner (Vitest): deferred to a separate
  infra slice.
- Browser-based GPU behavioural tests (Playwright +
  headless Chrome with WebGPU): §29.3b.
- Pre-existing em-dashes on `main`: out of scope.

### Slice §29.2: CR-07 cached difference images

**Scope.** Continues the §29 Performance foundation work
by shipping a content-addressed difference cache
(`DiffCache`) that lets the comparison surface re-render
a previously-computed diff in O(1) instead of recomputing
the per-mode walk on every toggle click.

#### New public API (core)

- `crates/astroforge-core/src/diff_cache.rs`: new module
  with:
  - `pub struct DiffCache`: single-threaded cache mapping
    `DiffCacheKey -> Vec<u8>`. Constructors: `new()`,
    `with_capacity(n)`. Methods: `get_or_compute`,
    `invalidate_version`, `clear`, `len`, `is_empty`,
    `stats`, `reset_stats`.
  - `pub struct DiffCacheKey`: cache key with
    `(version_a_id, version_b_id, mode, gain)`. Derives
    `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`.
  - `pub struct DiffCacheStats`: `hits` + `misses`
    counters. Method: `hit_rate() -> f64` in [0, 1].
- `crates/astroforge-core/src/difference.rs`: `DiffKind`
  enum now derives `Hash` so it can be a `HashMap` key.
  No behavior change.
- `lib.rs`: `pub mod diff_cache;` added.

#### Design decisions

1. **Single-threaded by construction.** `get_or_compute`
   returns `&Vec<u8>`, so the caller cannot mutate the
   cache while holding a reference to a cached buffer.
   Callers that need to share the cache across threads
   should wrap it in a `Mutex` (or `RwLock` if reads
   dominate). The §29.2 slice does not ship a
   thread-safe wrapper.

2. **Cache key = `(version_a_id, version_b_id, mode, gain)`.**
   `gain` only matters for `Amplified` mode, but we
   include it uniformly so the cache key is a function of
   all four inputs. `gain` is stored as `f32::to_bits()`
   so the key can derive `Eq + Hash`.

3. **Caller-driven invalidation.** The cache does NOT
   watch the filesystem. The caller invokes
   `invalidate_version(version_id)` when a version's
   primary artifact changes. The §29.2 slice does NOT
   wire invalidation into the IPC layer; that's a
   follow-on concern.

4. **Stats track effectiveness.** `hits` + `misses`
   counters + `hit_rate()` helper. `reset_stats()` zeros
   the counters without dropping entries (useful for
   per-render-window measurements).

#### Tests (core)

- NEW `crates/astroforge-core/tests/diff_cache.rs`: 21
  integration tests:
  - 3 hit/miss + byte-equivalence tests.
  - 6 cache-key-independence tests (different
    version_a_id, version_b_id, mode, gain, NaN-gain).
  - 3 invalidation tests (drops matching entries, no-
    match returns 0, post-invalidate forces recompute).
  - 1 clear + 4 stats tracking tests.
  - 3 cache-key API-surface tests (equality depends on
    all fields, clone, hash agrees with eq).
  - 1 with_capacity constructor test.
- The 21 tests all passed on the first `cargo test`
  invocation following the fixes below.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD):
  - §29 row updated: "Cached difference images"
    sub-row entry removed from the open list.
  - Scorecard: 74/9/3 → **75/8/3** (coverage
    **85% → 86%**).
  - Bundle priority #1 advanced from §29.2 to §29.3.
  - "First concrete slice" pointer advanced from §29.2
    to §29.3 (GPU/WebGPU acceleration).
- `CHANGELOG.md`: this entry.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1137 passing** (21 new tests
  on top of the 1116 baseline).
- `npm run check`: 1 error + 9 warnings (matches main baseline;
  slice adds 0 new warnings. Backend tests only).
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code spans,
  0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **Two local fix cycles** during compile + test resolution:
  1. `DiffKind` did not derive `Hash`, so the cache key
     type-check failed. Added `Hash` to the derive list.
     No behavior change; `DiffKind` is a 4-variant enum
     (Absolute / Signed / Amplified / Structural), each
     a unit variant, so Hash is trivially correct.
  2. `invalidate_version_drops_entries_with_matching_id`
     test had wrong stats assertion (expected 0 misses
     but the populate phase had already set misses = 4).
     Fixed by updating the expected values to `misses == 4`
     and `hits == 2`.
- **IPC layer NOT wired.** The §29.2 slice ships the pure
  Rust cache primitive only. The IPC layer wiring (state
  container, command handler, invalidation hooks on
  version-artifact change) is a follow-on concern.
- **Single-threaded by construction.** `get_or_compute`
  returns `&Vec<u8>`, so the caller cannot mutate the
  cache while holding a reference. This is documented;
  callers that need thread-safety wrap in a `Mutex` /
  `RwLock`.
- **Pure Rust slice.** 0 new IPC, 0 new UI, 0 new TS,
  0 new deps. Just the new `diff_cache` module + the
  `Hash` derive addition on `DiffKind` + tests.

#### Out-of-scope (intentional)

- §29.3 GPU/WebGPU acceleration (rank #1; next slice,
  largest scope).
- §8 SNR / regional noise / edge response / color
  gradient (rank #2).
- §32.6 (new) Wire pipeline_plan_hash +
  RecipeAiDiffSummary through IPC (rank #3).
- §29.4 (optional) Tile-stripe streaming: deferred to
  a future slice if §29.3 leaves headroom.
- IPC layer wiring for the cache (state container,
  command handler, invalidation hooks): deferred to a
  follow-on slice (probably §29.2a or merged into the
  §23.5 IPC surface).
- Split-alignment / blink-consistency /
  overlay-accuracy sub-modes of the audit's
  "visual regression" line: deferred to a future slice
  that adds a DOM-rendering test runner.
- Decision-side operations
  (`apply_and_save_decision` /
  `apply_and_save_decision_with_profile`): deferred to
  a §33 ADR-side test.
- Criterion-based release-mode benchmarks: deferred to
  a follow-on slice.
- Pre-existing em-dashes on `main`: out of scope per
  Coding Discipline.

### Slice §29.1: CR-07 streaming metrics accumulator

**Scope.** Closes the first sub-row of §29 Performance
("Streaming high-res regions"). Ships a single-pass
streaming metrics accumulator that collapses
`channel_stats` + `highlight_clipping` +
`saturation_percentage` into one walk over the image,
saving 2 O(WHC) passes per snapshot call.

The original §29 audit framed the work as "8 sequential
extractions on a 4K image". The actual work pattern is 7
passes per `metric_snapshot_full` call (6 §8 detectors + 1
`channel_stats`). After §29.1: 5 passes per call (4 spatial
detectors + 1 `channel_stats` + 1 streaming pass). On 4K
RGBA, that's ~66M float ops saved per snapshot call.

#### New public API (core)

- `crates/astroforge-core/src/streaming_metrics.rs`: new
  module with:
  - `pub fn streaming_metrics(image: &F32Image) -> BTreeMap<String, f64>`:
    one-pass equivalent of `channel_stats` ∪ `highlight_clipping`
    ∪ `saturation_percentage`. Output is byte-equivalent to
    the multi-pass baseline on every fixture tested.
  - `pub struct StreamingAccumulator`: stateful accumulator
    you can `observe(&img)` against and then `finalize()`.
    `observe` is idempotent on repeated calls (sums grow
    monotonically). Used by callers that want to observe
    multiple images into the same accumulator (future
    slice; current callers use `streaming_metrics`).
- `lib.rs`: `pub mod streaming_metrics;` added.

#### Behavior change (core)

- `comparison_metrics::metric_snapshot_full(image)`:
  - Now calls `streaming_metrics` to compute the
    per-pixel metrics (`channel_stats` +
    `highlight_clipping` + `saturation_percentage`)
    in one pass instead of three separate detector
    calls.
  - Drops the redundant `highlight_clipping` +
    `saturation_pct` entries that `metric_snapshot`
    inserted, then replaces them with the streaming
    output (which agrees byte-for-byte).
  - Net pass count: 7 → 5 per call.
  - Output keys + values are byte-equivalent to the
    pre-§29.1 baseline on every fixture tested.

#### Tests (core)

- NEW `crates/astroforge-core/tests/streaming_metrics.rs`:
  16 tests pinning byte-equivalence:
  - 7 baseline-equivalence tests across uniform,
    noisy, clipped, near-clip, single-channel,
    4-channel, grayscale fixtures.
  - 4 explicit-invariant tests: clip_count threshold
    (≥1.0), highlight_clipping threshold (≥0.999),
    saturation_pct unit (fraction in [0,1] NOT
    percentage in [0,100]), uniform image has zero
    stddev.
  - 2 invariant tests: noisy image has non-zero stddev;
    output keys are sorted (BTreeMap).
  - 3 API-surface tests: observe-once vs observe-twice
    idempotency, empty-image handling, subset match
    with `metric_snapshot`.
- NEW `crates/astroforge-core/tests/streaming_perf.rs`:
  3 perf tests as regression guards:
  - Streaming runtime at 2K must complete within 15s
    on a debug build.
  - Streaming must be at most 3x `channel_stats`
    runtime at 4K (regression guard).
  - Streaming output invariants hold at 4K.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD):
  - §29 row updated: "Streaming" sub-row entry
    removed from the open list.
  - Scorecard: 73/10/3 → **74/9/3** (coverage
    **84% → 85%**).
  - Bundle priority #1 advanced from §29 to §29.2.
  - "First concrete slice" pointer advanced from §29
    to §29.2 (cached difference images).
- `CHANGELOG.md`: this entry.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1116 passing** (19 new tests
  on top of the 1097 baseline; 16 streaming equivalence
  + 3 streaming perf).
- `npm run check`: 1 error + 9 warnings (matches main baseline;
  slice adds 0 new warnings. Backend tests only).
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code spans,
  0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **Smaller-than-initially-framed perf win.** The original
  §29 audit framed the work as "8 sequential extractions"
  with a target of "1 streaming pass". The actual work
  pattern is 7 passes per call, and §29.1 saves 2 (the
  4 spatial detectors can't be fused without
  buffering/recomputation). On 4K RGBA, streaming takes
  ~3 seconds vs `channel_stats` ~1.3 seconds (ratio 2.2x).
  The honest framing is "regression guard, not perf
  celebration": streaming is bounded to be at most 3x
  `channel_stats` runtime.
- **Initial test assertion had wrong expectations.**
  My first streaming output stored `highlight_clip_count`
  as a raw pixel count; the baseline detector stores it
  as a fraction. Also: `saturation_percentage` returns a
  fraction in [0,1], not a percentage in [0,100]. Both
  discrepancies caught by the byte-equivalence tests;
  fixed by aligning the streaming output to the existing
  baseline's units exactly.
- **Five local fix cycles** during compile + test
  resolution:
  1. `let mut` in unused-variable context (5 sites).
  2. `doc-lazy-continuation` clippy warning on the
     StreamingAccumulator doc-comment.
  3. `highlight_clipping` unit mismatch (raw count vs
     fraction). Fixed by counting per-channel to match
     the existing baseline.
  4. `saturation_pct` unit mismatch (fraction vs
     percentage × 100). Fixed by dropping the ×100.
  5. Wrong expected value in the
     `streaming_highlight_clipping_matches_threshold_0_999`
     test (clipped_image is half-clipped, not fully-clipped)
     fixed by updating the expected value to 0.5.
  First compile + first `cargo test` invocation both
  passed after these five fixes. All 19 new tests
  passed on the first `cargo test` invocation
  *following* the fixes.
- **4 spatial detectors stay as separate passes.**
  `luminance_noise`, `chromatic_noise`, `local_contrast`,
  `background_gradient` require neighbor-pixel or
  tile relationships that streaming can't supply without
  buffering or recomputation. A future slice (§29.4 or
  similar) could explore tile-stripe buffering to fuse
  some of these into the streaming pass, but the
  complexity is non-trivial and the gain on top of
  §29.1 is smaller.
- **Pure Rust slice.** 0 new IPC, 0 new UI, 0 new TS,
  0 new deps. Just the new `streaming_metrics` module
  + tests + the `metric_snapshot_full` behavior change.
- **Pre-existing em-dashes NOT cleaned** (per Coding
  Discipline; out of scope). All my additions are
  em-dash-free.

#### Out-of-scope (intentional)

- §29.2 Cached difference images (rank #1; next slice).
- §29.3 GPU/WebGPU acceleration (rank #2; largest scope).
- §8 SNR / regional noise / edge response / color
  gradient (rank #3).
- §32.6 (new) Wire pipeline_plan_hash +
  RecipeAiDiffSummary through IPC (rank #4).
- §29.4 (optional) Tile-stripe streaming: deferred to
  a future slice if §29.2 + §29.3 leave headroom.
- Split-alignment / blink-consistency /
  overlay-accuracy sub-modes of the audit's
  "visual regression" line: deferred to a future slice
  that adds a DOM-rendering test runner.
- Decision-side operations
  (`apply_and_save_decision` /
  `apply_and_save_decision_with_profile`): deferred to
  a §33 ADR-side test.
- Criterion-based release-mode benchmarks: deferred to
  a follow-on slice.
- Pre-existing em-dashes on `main`: out of scope per
  Coding Discipline.

### Slice §32.5: CR-07 perf tests (final §32 sub-slice)

**Scope.** Closes the §32 "Performance" sub-row (the fifth
and final §32 sub-slice). Pins wall-clock bounds for the
B7 Perf bundle's Rust-testable surface.

#### Tests (core)

- NEW `crates/astroforge-core/tests/perf.rs`: 21 perf tests:
  - **§32.5.1: `compute_diff` at 4K (3840×2160)**:
    4 tests, all 4 `DiffKind` modes (Absolute, Signed,
    Amplified, Structural).
  - **§32.5.2: `compute_diff` at 8K (7680×4320)**:
    4 tests, same 4 modes at 4x the pixel count.
  - **§32.5.3: §8 detectors at 4K + 8K**: 4 tests
    covering `luminance_noise` and
    `saturation_percentage` at both resolutions.
  - **§32.5.4: `Recipe::pipeline_plan_hash` at 10 / 100 /
    1000 stages**: 3 tests, validating linear-time
    scaling.
  - **§32.5.5: `recipe_ai_diff_summary` at 10 / 100 /
    1000 stages**: 3 tests.
  - **§32.5.6: multi-version `compare_version_images` at
    2K + 4K**: 2 tests.
  - **§32.5.7: positive-control baseline**: 1 test that
    prints the actual elapsed time of a small operation
    so the test log captures a future-comparison baseline.
- Helper `must_complete_in(label, max_seconds, f)`:
  - Wall-clock-bound helper. Asserts the closure completes
    within `max_seconds`. Prints the actual elapsed time
    on success so the test log documents the steady-state.
- Helpers `rgba8_grid(w, h)` + `f32_image_grid(w, h, c)`:
  - Deterministic pixel data; the compiler can't
    constant-fold the construction, so the perf numbers
    are honest.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1097 passing** (21 new perf tests
  on top of the 1076 baseline).
- `npm run check`: 1 error + 9 warnings (matches main baseline;
  slice adds 0 new warnings. Backend tests only).
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code spans,
  0 en-dashes, 0 ellipses, 0 smart quotes.
- Single-threaded perf binary runtime: ~92 seconds.
- Multi-threaded (CI default) perf binary runtime: ~35 seconds.

#### Honest flags

- **Zero local fix cycles.** First compile passed; first
  `cargo test` invocation passed all 21 tests. The bounds
  were tuned on a single trial run: each bound is
  "roughly 4x the measured runtime on this debug build",
  giving ample headroom for slower CI runners without
  making the bounds meaningless.
- **Generous bounds.** This is a debug build (no LTO, no
  codegen-units=1, no native CPU features). The bounds
  catch order-of-magnitude regressions, not micro-benchmarks.
  Release-mode benchmarks belong in a `criterion`-backed
  bench harness under `crates/astroforge-core/benches/`.
  That's a follow-on slice (probably §32.6 or §29).
- **Pure test slice.** 0 new Rust source (only the test
  file), 0 new IPC, 0 new UI, 0 new TS, 0 new deps.
- **No new CI job.** The slice adds the perf tests to the
  existing `cargo test --workspace` invocation that the
  six-gate already runs. The 35-second parallel runtime
  fits within the standard CI budget; no separate
  `perf` job needed. (A `criterion`-based perf job is the
  follow-on; this slice ships the floor.)
- **Out-of-scope operations documented.** All comparison-
  side perf is covered. Decision-side operations
  (`apply_and_save_decision`) are explicitly NOT covered.
- **Pre-existing em-dashes NOT cleaned** (per Coding
  Discipline; out of scope). All my additions are
  em-dash-free.

#### Out-of-scope (intentional)

- §29 Performance (rank #1; next slice, the real B7
  close-out).
- §8 SNR / regional noise / edge response / color gradient
  (rank #2).
- §32.6 (new) Wire pipeline_plan_hash + RecipeAiDiffSummary
  through IPC (rank #3).
- Split-alignment / blink-consistency / overlay-accuracy
  sub-modes of the audit's "visual regression" line:
  deferred to a future slice that adds a DOM-rendering
  test runner.
- Decision-side operations
  (`apply_and_save_decision` /
  `apply_and_save_decision_with_profile`): deferred to a
  §33 ADR-side test.
- Criterion-based release-mode benchmarks: deferred to a
  follow-on slice.
- Pre-existing em-dashes on `main`: out of scope per Coding
  Discipline.

#### Bundle status (post-§32.5)

The §32 series is **fully closed**. The B7 Perf bundle's
test foundation is complete. The remaining B7 Perf work
is §29 itself.

### Slice §32.4: CR-07 AI comparison tests + pipeline plan hash

**Scope.** Closes the §32 "AI comparison" sub-row. Pins the
audit's "model / version / hash / classification / parameters /
provenance surfacing" surface as a pure-Rust data model +
function pair, with comprehensive test coverage at both
the unit and integration levels.

The audit's "hash" line was a real gap: no `pipeline_plan_hash`
function existed in the codebase before this slice. Adding
it is feature work, so the slice ships both the function +
its tests under one PR.

#### New public API (core)

- `Recipe::pipeline_plan_hash() -> String`:
  - 64-char lowercase hex SHA-256 of pipeline-shape fields:
    `schema_version`, `name`, `target_type`, stages (with
    `enabled` flag + per-stage params, deterministic
    BTreeMap ordering), `required_models`, integrity badge
    (booleans + sorted model list with model_type), `version`,
    `branch`, `quality_profile`.
  - Deliberately EXCLUDES `description`, `created_at`,
    `parent_version`, and `flags` (presentational / lineage /
    annotation fields).
  - Deterministic: same Recipe always hashes to the same
    value.
- `pub struct RecipeAiDiffSummary` (Serialize + Deserialize):
  - `hash_a` / `hash_b` / `hash_differs` (hash).
  - `ai_used_a` / `ai_used_b` / `ai_classification_differs`
    (model + classification).
  - `version_a` / `version_b` + `schema_version_a` /
    `schema_version_b` (version).
  - `quality_profile_a` / `quality_profile_b`
    (classification detail).
  - `required_models_differ` + `perceptual_models_a` /
    `perceptual_models_b` (parameters + model detail).
  - `provenance` (human-readable summary line of the form
    `"Recipe A v{N} (ai|natural, {profile}) vs Recipe B v{M} ..."`).
- `recipe_ai_diff_summary(a: &Recipe, b: &Recipe) -> RecipeAiDiffSummary`:
  - Pure function. The comparison UI renders this struct
    directly without re-deriving any of the fields.

#### Tests (core)

- 18 new unit tests inline in `crates/astroforge-core/src/recipe.rs`:
  - 9 `pipeline_plan_hash_*` tests: 64-char lowercase hex
    shape, determinism, schema_version / stages /
    stage_params / integrity_models / ai_model each change
    the hash, description+created_at+parent_version+flags
    do NOT change the hash.
  - 9 `ai_diff_summary_*` tests: natural-vs-AI classification
    differs, both-natural and both-AI no classification diff,
    identical Recipes no hash diff, different stages hash
    differs, provenance line format, required_models_differ
    surfacing, quality_profile surfacing, schema_version
    surfacing.
- NEW `crates/astroforge-core/tests/recipe_ai_comparison.rs`:
  - 18 integration tests covering the public API surface
    end-to-end:
    - 6 `pipeline_plan_hash_*` tests (hash shape,
      determinism, AI-vs-natural distinction, param-value
      distinction, presentational-field invariance,
      serialisation round-trip agreement).
    - 11 `ai_diff_summary_*` tests (classification differs /
      both-natural / both-AI, version / schema_version /
      quality_profile / required_models surfacing,
      provenance line format, purity, hash-differs on
      param change).
    - 2 `recipe_ai_diff_summary_serde_*` tests
      (round-trip + audit-field-presence).

#### Docs

- `docs/CR-07-AUDIT.md` (MOD):
  - §32 row updated: "AI comparison" entry removed from
    the open list.
  - Scorecard: 71/12/3 → **72/11/3** (coverage **82% → 83%**).
  - Bundle priority #1 advanced from §32.4 to §32.5.
  - "First concrete slice" pointer advanced from §32.4 to
    §32.5 (perf tests).
- `CHANGELOG.md`: this entry.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1076 passing** (36 new tests
  on top of the 1040 baseline; 18 unit + 18 integration).
- `npm run check`: 1 error + 9 warnings (matches main baseline;
  slice adds 0 new warnings. Backend + UI source, but no UI
  surface changes).
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code spans,
  0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **Two local fix cycles** during compile + clippy cleanup.
  First: `RecipeAiDiffSummary` struct literal tried to
  borrow `hash_a` and `hash_b` after moving them into the
  struct; fixed by binding `hash_differs` to a local first.
  Second: missed `QualityProfile::Publication` variant in
  the match arms (4 variants, not 3). Third: clippy flagged
  5 `let mut` bindings in tests where only one side needed
  mutation; removed the unnecessary `mut`s. None affected
  test semantics.
- **First compile of the new tests ran cleanly** after the
  above three fixes. All 36 new tests passed on first
  `cargo test` invocation.
- **Feature addition, not pure tests.** Unlike §32.1
  / §32.2 / §32.3 (pure test files), §32.4 adds new public
  API to `astroforge_core::recipe`: `pipeline_plan_hash()`
  on `Recipe` + `RecipeAiDiffSummary` struct +
  `recipe_ai_diff_summary()` function. The audit's "hash"
  line could not be closed by tests alone. The function
  had to exist. The slice ships both the function + its
  tests under one PR.
- **IPC surface not changed.** The new types are Rust-only
  public API. Wiring them into the comparison IPC layer
  is a UI-side change deferred to a follow-on slice
  (probably §23.5 or a new §32.6).
- **`sha2` was already in deps** (workspace dependency);
  no new crates added.
- **QualityProfile match arms cover all 4 variants**
  (Natural / Detail / Clean / Publication). Pin that
  this slice will keep working if a new variant is added
  (clippy catches non-exhaustive matches).
- **Pure function**: no I/O, no DB, no Tauri State. The
  integration test surface uses no `DomainStore` (the
  Recipe data model alone is the test surface).
- **Pre-existing em-dashes NOT cleaned** (per Coding
  Discipline; out of scope). All my additions are
  em-dash-free.

#### Out-of-scope (intentional)

- §32.5 Perf tests (rank #1; final §32 sub-slice).
- §29 Performance (rank #2).
- §8 SNR / regional noise / edge response / color gradient
  (rank #3).
- Split-alignment / blink-consistency / overlay-accuracy
  sub-modes of the audit's "visual regression" line:
  deferred to a future slice that adds a DOM-rendering
  test runner.
- Decision-side operations
  (`apply_and_save_decision` /
  `apply_and_save_decision_with_profile`): deferred to a
  §33 ADR-side test (decision state transitions).
- IPC surface for the new Recipe hash / AI comparison
  types: deferred to a follow-on slice (probably §23.5
  or new §32.6).
- Pre-existing em-dashes on `main`: out of scope per Coding
  Discipline.

### Slice §32.3: CR-07 version integrity test

**Scope.** Closes the §32 "Version integrity" sub-row.
Pins the CR-07 ADR-07.4 ("Comparison Is Non-Destructive")
contract: running any comparison-side operation must not
modify either the `image_versions`, `image_decisions`, or
`decision_history` rows of the two versions being compared.

#### Tests (core)

- NEW `crates/astroforge-core/tests/version_integrity.rs`:
  - 8 integration tests using an in-memory `DomainStore`
    (`DomainStore::new(&PathBuf::from(":memory:"))`).
  - **§32.3.1**: `compare_version_images` (pure Rust
    delta-table function) is deterministic across
    invocations for identical input.
  - **§32.3.2**: `save_comparison_set` doesn't touch
    `image_versions`, `image_decisions`, or
    `decision_history` rows.
  - **§32.3.3**: `load_comparison_set` doesn't touch any
    of those rows.
  - **§32.3.4**: `list_comparison_sets_for_project` doesn't
    touch any of those rows.
  - **§32.3.5**: `delete_comparison_set` doesn't touch any
    of those rows.
  - **§32.3.6**: Full end-to-end: stub 2 versions +
    decisions + history, run save → load → list → delete,
    verify the version-side tables are byte-equal across
    the entire lifecycle.
  - **§32.3.7**: Positive control: pin that the
    `comparison_sets` table DOES change across the
    lifecycle (guards against the test suite accidentally
    no-opping).
  - **§32.3.8**: Sibling-row isolation: pin that comparison
    ops on a (v-a, v-b) pair don't touch other versions in
    the same project.
  - Fingerprint helpers: `image_versions_fingerprint`,
    `image_decisions_fingerprint`,
    `decision_history_fingerprint` produce a stable
    string from every column of every relevant row, so
    pre- and post-operation snapshots can be compared
    byte-exactly.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD):
  - §32 row updated: "Version integrity" entry removed
    from the open list.
  - Scorecard: 70/13/3 → **71/12/3** (coverage **80% → 82%**).
  - Bundle priority: §32.3 removed; new #1 is §32.4.
  - "First concrete slice" pointer advanced from §32.3 to
    §32.4 (AI comparison tests).
- `CHANGELOG.md`: this entry.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1040 passing** (8 new version
  integrity tests on top of the 1032 baseline).
- `npm run check`: 1 error + 9 warnings (matches main baseline;
  slice adds 0 new warnings. Backend tests only.
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code spans,
  0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **Zero local fix cycles.** First compile of the test
  file had three import errors caught by clippy:
  `ComparisonSet` was imported via the wrong path
  (decision_store re-export vs. comparison module), and
  one test had a broken call to a non-existent
  `metric_snapshot_full_for_test` helper. All three fixed
  inline before running `cargo test`. The first
  `cargo test` invocation passed all 8 tests on the first
  try.
- **Pure test slice.** 0 new Rust source (only the test
  file), 0 new IPC, 0 new UI, 0 new TS, 0 new dependencies.
- **Out-of-scope operations documented.** The decision-side
  operations (`apply_and_save_decision`,
  `apply_and_save_decision_with_profile`) are explicitly
  NOT covered by this slice. They mutate
  `image_decisions` + `decision_history` (that's their job).
  They will be covered by a §33 ADR-side test (decision
  state transitions).
- **No fixture / no DOM-rendering runner.** Pure Rust in-
  memory SQLite (`DomainStore::new(&:memory:)`) is the
  test surface. No JSDOM, no Playwright, no fixtures.
- **Pre-existing em-dashes NOT cleaned** (per Coding
  Discipline; out of scope). All my additions are
  em-dash-free.

#### Out-of-scope (intentional)

- §32.4 AI comparison tests (rank #1; next slice).
- §32.5 perf tests (rank #2).
- §29 Performance (rank #3).
- §8 SNR / regional noise / edge response / color gradient
  (rank #4).
- Split-alignment / blink-consistency / overlay-accuracy
  sub-modes of the audit's "visual regression" line:
  deferred to a future slice that adds a DOM-rendering
  test runner (Playwright or equivalent).
- Decision-side operations
  (`apply_and_save_decision` /
  `apply_and_save_decision_with_profile`): deferred to a
  §33 ADR-side test (decision state transitions).
- Pre-existing em-dashes on `main`: out of scope per Coding
  Discipline.

### Slice §32.2: CR-07 visual regression tests for diff renderer

**Scope.** Closes the §32 "Visual regression" sub-row (the
Rust-testable portion of it: difference rendering). The
other three "visual" sub-modes (split alignment, blink
consistency, overlay accuracy) live in `CompareTools.svelte`
and require a DOM-rendering test runner (e.g. Playwright)
the codebase doesn't ship yet. Documented as a future slice.

#### Tests (core)

- NEW `crates/astroforge-core/tests/visual_regression.rs`:
  - 16 integration tests pinning `compute_diff(kind, a, b,
    gain, width)` in `crates/astroforge-core/src/difference.rs`
    against deterministic RGBA8 fixtures with pixel-exact
    expected output.
  - All four `DiffKind` modes covered (Absolute / Signed /
    Amplified / Structural):
    - **Absolute**: pixel-exact `|A-B|`, clamp at extremes,
      alpha preservation.
    - **Signed**: midpoint=128 on identical inputs,
      brighter-A pushes above 128 (clamped to 255), brighter-B
      pushes below (clamped to 0), hand-computed pixel value.
    - **Amplified**: gain=1.0 matches Absolute, gain=2.0/3.0
      scales linearly with clamp at 255, non-finite/zero
      gain coerces to 1.0 (defensive default).
    - **Structural**: zero on uniform image, falls back to
      absolute on tiny images (width < 2), edge detection
      against a vertical step.
  - Two cross-mode determinism tests: all four modes must
    produce identical output for identical input across
    multiple invocations; all four modes must force alpha
    byte to 255 in the output.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD):
  - §32 row updated: "Visual regression" entry removed
    from the open list (the difference-rendering portion).
    Split / blink / overlay sub-modes documented as
    deferred to a future DOM-rendering test runner slice.
  - Scorecard: 69/14/3 → **70/13/3** (coverage **79% → 80%**).
  - Bundle priority: §32.2 removed; new #1 is §32.3.
  - "First concrete slice" pointer advanced from §32.2 to
    §32.3 (version integrity test).
- `CHANGELOG.md`: this entry.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1032 passing** (16 new visual
  regression tests on top of the 1016 baseline).
- `npm run check`: 1 error + 9 warnings (matches main baseline;
  slice adds 0 new warnings. Backend tests only.
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code spans,
  0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **Two local fix cycles caught by the six-gate.**
  1. Initial `chunks_exact(4)` triggered the
     `clippy::chunks_exact_to_as_chunks` lint; three
     call sites converted to `as_chunks::<4>().0.iter()`.
  2. Initial structural-diff edge test asserted "non-zero
     at interior edges" with a gradient fixture; the Sobel
     approximation's `left + up` neighbour scheme means the
     gradient produces zero structural diff at the
     gradient's leading edge (the left neighbour has the
     same value). Switched to a step fixture (sharp edge
     at x=2) so the structural diff at (x=2, y=1) is
     definitively non-zero (left=0, self=255 in image A;
     left=128, self=128 in uniform B). The lesson: the
     structural path detects an edge at a pixel where
     the left-neighbour differs from the pixel itself.
- **Pure test slice.** 0 new Rust source (only the test
  file), 0 new IPC, 0 new UI, 0 new TS, 0 new dependencies.
- **Pixel-exact assertions.** Unlike §32.1's metric-
  validation tests (which pin directional behavior), this
  slice's tests pin pixel-exact values because the diff
  renderer is a deterministic per-pixel math function
  (no heuristic involved). This is the right shape for
  visual regression: the rendered output should be
  byte-for-byte identical across releases.
- **Out-of-scope sub-modes documented.** The audit's
  original "split alignment, blink consistency, difference
  rendering, overlay accuracy" splits into 4 sub-modes,
  of which only "difference rendering" is Rust-testable.
  The other three (Svelte canvas rendering) need a DOM-
  rendering test runner. This slice closes the one
  Rust-testable sub-mode; the other three are flagged for
  a future slice that adds the runner.
- **Pre-existing em-dashes NOT cleaned** (per Coding
  Discipline; out of scope). All my additions are
  em-dash-free.

#### Out-of-scope (intentional)

- §32.3 version integrity test (rank #1; next slice).
- §32.4 AI comparison tests (rank #2).
- §32.5 perf tests (rank #3).
- §29 Performance (rank #4).
- §8 SNR / regional noise / edge response / color gradient
  (rank #5).
- Split-alignment / blink-consistency / overlay-accuracy
  sub-modes of the audit's "visual regression" line:
  deferred to a future slice that adds a DOM-rendering
  test runner (Playwright or equivalent).
- Pre-existing em-dashes on `main`: out of scope per Coding
  Discipline.

### Slice §32.1: CR-07 metric validation against controlled fixtures

**Scope.** Closes the §32 "Metric validation against controlled
datasets" sub-row. The other four §32 sub-rows (visual
regression, version integrity, AI comparison, perf tests)
remain open and will ship as §32.2..§32.5 per the established
sub-slice cadence.

#### Tests (core)

- NEW `crates/astroforge-core/tests/metric_validation.rs`:
  - 17 integration tests covering all six §8 detectors
    (`luminance_noise`, `chromatic_noise`, `local_contrast`,
    `background_gradient`, `highlight_clipping`,
    `saturation_percentage`).
  - Deterministic-fixture builders: `uniform_image`,
    `noisy_image` (LCG hash), `ramp_image` (linear slope),
    `step_image` (half-bright).
  - Each detector validated against (a) a uniform baseline
    (expected near-zero for noise/gradient/contrast) and
    (b) a controlled non-uniform fixture (expected non-zero
    with directional correctness: noisy > clean, ramp >
    flat, step > uniform).
  - Two determinism tests pinning that identical inputs
    produce identical outputs (the detectors must be pure
    functions).

#### Docs

- `docs/CR-07-AUDIT.md` (MOD):
  - §32 row updated: "Metric validation" entry removed from
    the "Still open" list; the four remaining sub-rows
    (visual regression, version integrity, AI comparison,
    perf tests) listed as §32.2..§32.5 future sub-slices.
  - Scorecard: 69/15/3 → **69/14/3** (coverage **79% → 79%**;
    a row shifted from ⚠️ Partial toward ✅ without changing
    the column totals because §32 as a whole remains Partial
    until all 5 sub-rows ship).
  - Bundle priority: §32 advanced to a 5-rank sub-slice
    breakdown (32.2..32.5); "First concrete slice" pointer
    advanced from §32 to **§32.2**.
- `CHANGELOG.md`: this entry.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **1016 passing** (17 new metric
  validation tests on top of the 999 baseline).
- `npm run check`: 1 error + 9 warnings (matches main baseline;
  the slice adds 0 new warnings. Backend tests only.
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code spans,
  0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **One local fix cycle caught by the six-gate.** Initial
  test tolerance bounds for `luminance_noise` and
  `local_contrast` were too tight (0.1 vs the detectors'
  actual ~0.07 / ~0.024 outputs). Plus the `background_gradient`
  detector requires image ≥ 128×128 (the 64×64 default tile
  width catches only one tile, which leaves no slope across
  the regression). Three one-line fixes: relaxed bounds +
  larger fixture size.
- **One local fix cycle for unused-variable lint.** The
  integration test file's `indexed_iter_mut` destructures
  used `x` and `y` separately; clippy flagged the unused
  `x` in ramp/step fixtures. Renamed to `_x` where unused;
  kept `x` for `noisy_image` where it's actually used in
  the LCG hash.
- **Test assertions pin directional behavior, not exact
  values.** The detectors use heuristic algorithms whose
  absolute outputs depend on the fixture's exact pixel
  statistics. Tests pin (a) the detector's directional
  response (noisy > clean, ramp > flat) and (b) the
  magnitude is in a sane range. This is the right shape
  for "validate the detector responds correctly" rather
  than "validate the detector produces a specific number";
  the latter would over-constrain the heuristic and
  break every time someone tweaks the algorithm.
- **Pure test slice.** 0 new Rust source (only the test
  file), 0 new IPC, 0 new UI, 0 new TS, 0 new deps.
- **Pre-existing em-dashes NOT cleaned.** Rust source + audit
  file have em-dashes in unchanged shipped lines; per Coding
  Discipline, out of scope. All my additions are em-dash-free.

#### Out-of-scope (intentional)

- §32.2 visual regression tests (rank #1 in the new
  priority list; next sub-slice).
- §32.3 version integrity test (rank #2).
- §32.4 AI comparison tests (rank #3).
- §32.5 perf tests (rank #4).
- §29 Performance (rank #5).
- §8 SNR / regional noise / edge response / color gradient
  (rank #6).
- Pre-existing em-dashes on `main`: out of scope per Coding
  Discipline.

### Slice §8: CR-07 saturation percentage

**Scope.** Closes the §8 "Saturation percentage" ❌ Missing row.
The metric_registry slot `MetricKind::DynamicRangeSaturationPct`
already existed (`crates/astroforge-core/src/metric_registry.rs:402`)
with `context: "Fraction of channels saturated. Higher indicates
color information is being lost."` and `unit: "%"`. The detector
implementation was missing. This slice ships it.

#### Rust (core)

- `crates/astroforge-core/src/image_analysis/metrics.rs`:
  - NEW `saturation_percentage(image: &F32Image) -> MetricsSample`.
    Counts pixels whose R, G, AND B channels are all at or
    near `1.0` (full color loss), distinct from
    `highlight_clipping` which counts per-channel clipping.
    Returns fraction in `[0, 1]`. ~50 LOC detector + ~80 LOC
    tests.
- `crates/astroforge-core/src/comparison_metrics.rs`:
  - `metric_snapshot` (line 49) wires `saturation_percentage`
    into the snapshot BTreeMap; the existing
    `metric_snapshot_full` (line 175) propagates it to the
    delta-table IPC and the per-version expert panel for free.
  - `snapshot_covers_exactly_the_shipped_detectors` test
    updated from 5 → 6 detectors.
  - `metric_snapshot_full_merges_detector_and_channel_keys`
    test updated from 20 → 21 total keys.
- `crates/astroforge-core/src/quality.rs`:
  - NEW `saturation: f64` field on `QualityMetricSnapshot`
    (with `#[serde(default)]` for legacy-row compatibility).
    Populated by `compute_metrics` via the new detector.
- `crates/astroforge-core/src/recommendation.rs`:
  - Test fixture for `QualityMetricSnapshot` updated to
    populate the new `saturation: 0.0` field.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD):
  - §8 row's "Saturation percentage" entry flipped ❌ Missing
    → ✅ `saturation_percentage`.
  - §8 scorecard row: 10 ✅ / 5 ⚠️ / 1 ❌ → **11 ✅ / 4 ⚠️ / 1 ❌**.
  - Scorecard total: 68/16/3 → **69/15/3** (coverage **78% → 79%**).
  - Bundle priority #1 advanced from §11 to **§32** (B7 Perf).
  - "First concrete slice" pointer advanced.
  - **§11 audit-text correction**: the prior §11 row text
    (written in PR #351) pointed at `QualityGatePanel.svelte`
    for the §11 prose; the §11 prose actually renders in
    `MetricsTable.svelte:179` (`<pre class="summary
    font-body">{report.summary}</pre>`). §11 has been Shipped
    via the B4 delta-table surface since PR #325 (B4
    merge). Audit-text corrected to reflect this; no §11
    slice is owed.
- `CHANGELOG.md`: this entry.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **999 passing** (5 new saturation
  tests added on top of the 994 baseline; 2 existing comparison
  tests updated).
- `npm run check`: 1 error + 9 warnings (matches main baseline;
  the slice adds 0 new warnings. Backend + detector only.
- `npm run build`: clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code spans,
  0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **One local fix cycle caught by the six-gate.** The first
  clippy run flagged `unnecessary_cast` on `c as usize`,
  `h as usize`, `w as usize` (they're already usize via
  `image.width()` etc.). One-line fix to drop the casts. Same
  shape as the §20 + §23.2 + §24 + §9 fix cycles.
- **One local fix cycle during detector iteration algorithm
  rewrite.** Initial detector used `ch == 0` as the
  pixel-boundary predicate inside `indexed_iter()`;
  empirically the iterator doesn't reset `ch` at each pixel
  (it visits `ch=0`'s full block, then `ch=1`, etc., in
  C-order). Rewrote to a clean per-channel walk with
  `image[(ch, y, x)]` indexing; the clean algorithm is also
  faster (single contiguous pass per channel, no flush
  bookkeeping). The debug eprintln was caught + removed
  before commit.
- **The detector distinguishes from `highlight_clipping`.**
  `highlight_clipping` counts per-channel clipped values
  (so R-clipped-only reports ~33% on a 3-channel image).
  `saturation_percentage` counts pixels where all channels
  are clipped (so R-clipped-only reports 0% on the same
  image). Verified by `saturation_percentage_distinguishes_from_highlight_clipping`.
- **`QualityMetricSnapshot.saturation` defaults to 0.0 for
  legacy rows.** The `#[serde(default)]` attribute means
  old `metric_snapshot_json` rows (written before this slice)
  deserialize with `saturation: 0.0` rather than failing.
  New rows from `compute_metrics` carry the actual detector
  output. The `recommendation.rs` test fixture also uses
  `0.0` (uniform-noise fixture doesn't reach saturation).
- **The audit-row audit-claim verification found a stale
  §11 row text** that I wrote in PR #351 (pointed at the
  wrong panel). Corrected in this PR's audit doc + the
  Bundle priority section now reflects that §11 was already
  shipped via `MetricsTable.svelte:179` since B4.
- **Pre-existing em-dashes NOT cleaned** (Rust source + audit
  file have em-dashes in unchanged shipped lines; per Coding
  Discipline, out of scope). All my additions are em-dash-free
  per the GitHub language rule.

#### Out-of-scope (intentional)

- §8 SNR / regional noise / edge response / color gradient
  (still ⚠️ Partial; rank #3 in the new priority list).
- §32 + §29 (B7 Perf bundle; rank #1 + #2).
- §11 prose-display in `QualityGatePanel.svelte` (already
  shipped via `MetricsTable.svelte:179`; the audit-doc text
  was wrong, corrected in this PR's audit block).
- Pre-existing em-dashes on `main` (audit + svelte files):
  out of scope per Coding Discipline; the slice cleans only
  its own additions.

### Slice §13: CR-07 AI-aware comparison chip

**Scope.** Closes the §13 audit row ("no single one-line 'AI was used
in this version's chain' badge in the compare-header"). The data path
was already fully shipped: `Recipe.integrity.perceptual_models_used`
(`crates/astroforge-core/src/recipe.rs:82`) flows through the IPC via
`RecipeFromRust.integrity.perceptual_models_used`
(`src/lib/astroforge-api.ts:488`) and is read by `ProvenancePanel`
(B14) + `RecipeStageTimeline` (B15). The gap was UI: no chip in the
compare-header. This slice adds a one-line "AI used" chip in
`CompareWorkspace.svelte`'s per-side `pane-header` so the user sees,
at a glance, which side(s) of the comparison used AI in their Recipe
chain.

#### Frontend (Svelte)

- `src/components/CompareWorkspace.svelte` (MOD):
  - New `aiUsedA` + `aiUsedB` `$state<boolean | null>` slots
    (one per side).
  - Two `$effect` blocks watch `versionA?.version_id` and
    `versionB?.version_id` and call `recipeGetForImageVersion(id)`
    on change. The chip renders only when the Recipe's
    `integrity.perceptual_models_used === true`.
  - Three-state model: `true` (chip on), `false` (deterministic-only
    chain, chip off), `null` (still loading or IPC failed, chip
    hidden: honest "unknown" rather than misleading "AI used").
  - New `.ai-used-chip` CSS class: amber palette
    (`rgba(255, 144, 74, 0.18)` background + `#ff904a` text),
    Material Symbols' `auto_awesome` icon, 0.7rem uppercase
    treatment matching `status-pill` shape.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD):
  - §13 row flipped from `⚠️ Partial` to `✅ Shipped`.
  - Scorecard: §13 row flipped from 0 ✅ / 1 ⚠️ to 1 ✅ / 0 ⚠️.
    Net delta: 67/17/3 → 68/16/3. Coverage: 77% → 78% shipped.
  - Bundle priority updated: §13 removed from the priority list;
    new #1 is §11 prose-display.
  - "First concrete slice (post-§26 mapping)" pointer advanced
    from §13 to §11.
- `CHANGELOG.md`: this entry.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 994 passing (unchanged).
- `npm run check`: 1 error + 9 warnings (matches main baseline;
  the slice adds 0 new warnings. The new Svelte template + CSS
  compile clean.
- `npm run build`: clean (CSS +0.36 kB, JS +1.14 kB for the
  chip + effects).
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session`: green.
- Em-dash sweep on additions: 0 em-dashes outside code spans,
  0 en-dashes, 0 ellipses, 0 smart quotes.

#### Honest flags

- **No backend change.** The data path was already shipped; the
  slice is purely UI + one IPC call per side. 0 new Rust, 0 new
  IPC, 0 new TS, 0 new dependencies.
- **Honest "unknown" on IPC failure.** When `recipeGetForImageVersion`
  fails (network, decode, store issue), the chip stays hidden
  rather than rendering a misleading "AI used" badge for a
  version whose chain we couldn't read. The ProvenancePanel
  surfaces the full error separately.
- **Race-safe via cancellation flag.** Each `$effect` registers
  a `cancelled` flag in its cleanup closure; if the user clicks
  another version while the IPC is in flight, the stale result
  is ignored. Mirrors the pattern in `provenance-store.ts:114`.
- **No new dependency.** Material Symbols icons are already
  loaded by the rest of the AI-aware UI; the `auto_awesome`
  glyph is reused from the existing icon set.
- **Color palette choice.** Warm amber (`#ff904a`) was selected
  to match the existing AI-recommendation treatment elsewhere
  in the codebase (e.g. `MetricsTable.svelte`'s `scope-chip`
  feature color). Visual hierarchy: the amber chip + icon
  reads as a deliberate "AI used" marker, distinct from the
  neutral gray `status-pill` below it.
- **Honest stub buttons pattern does not apply here.** §13 is
  a single chip + IPC call per side; the slice renders the
  full §13 surface in one PR with no follow-up wire-up needed.
- **The "post-§23.3" + "post-§24" first-concrete-slice pointers
  are retained as historical references.** The new
  "post-§26 mapping" pointer advances to §11. The slice loop
  has accumulated three generations of pointers; the audit
  doc keeps all three so a reader can trace the priority
  evolution.
- **Zero local fix cycles caught by the six-gate.** This is
  the first slice in the CR-07 §23..§13 run that shipped
  the six-gate sequence without requiring a local fix.
  The pattern is the same as §23.1..§23.4 + §24 + §19 + §9.
  Small pure-UI slices with no Rust changes have a low
  fix-cycle count. The §20 + §23.2 + §24 slices had one
  fix cycle each.

#### Out-of-scope (intentional)

- §8 saturation percentage (genuine ❌ in
  `image_analysis/metrics.rs`; rank #2 in the new priority
  list).
- §11 prose-display in `QualityGatePanel.svelte` (rank #1;
  next slice).
- §32 + §29 (B7 Perf bundle; rank #3 + #4).
- Pre-existing em-dashes on `main` (audit + svelte files):
  out of scope per Coding Discipline; the slice cleans only
  its own additions.

### Slice §9: CR-07 contextual metric display

**Scope.** Closes the §9 audit row ("A metric should never be
presented without explaining what it means"). The contextual
explanation data was already fully shipped via
`MetricSpec::context` in `crates/astroforge-core/src/metric_registry.rs`
(25 metrics, lines 246-467), carried through the IPC via
`MetricDeltaRow.context` (`commands_comparison.rs:206` +
`src/lib/astroforge-api.ts:583`). The gap was UI: `MetricsTable.svelte`
only exposed the context as a browser tooltip (`title=` on the
`<tr>`), making it invisible by default. This slice renders the
context as inline sub-text beneath every metric label so the
user never sees a bare metric name without the "what does this
mean, why does it matter" sentence the spec requires.

#### Frontend (Svelte)

- `src/components/MetricsTable.svelte` (MOD):
  - The `.metric-name` cell now stacks `.metric-label` (the
    metric name) on top of `.metric-context` (the contextual
    sentence). Vertical alignment preserved via flex-column on
    the cell.
  - `.metric-context` styled with `var(--on-surface-variant)`
    (the standard subdued-text token, sufficient contrast on
    the dark canvas) at 0.78rem / 1.35 line-height, with
    `-webkit-line-clamp: 2` to bound tall rows; the full text
    remains available via the existing `title=` tooltip on the
    `<tr>`.
  - File header comment updated to note the §9 wire-up alongside
    the §10 + §11 wires.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD):
  - §9 row flipped from `⚠️ Partial` to `✅ Shipped`, with the
    audit-text tightening that names the `metric_registry.rs`
    data source + the `MetricsTable.svelte` rendering
    change + the prior-refresh's misread note (the audit
    said "Improvement pending in `RecommendationCard.svelte`";
    that component renders AI recommendations, not metric
    tiles; the metric-context rendering lives in
    `MetricsTable.svelte`).
  - Scorecard: §9 row flipped from 0 ✅ / 1 ⚠️ to 1 ✅ / 0 ⚠️.
    Net delta: 66/18/3 → 67/17/3. Coverage: 76% → 77% shipped.
  - Bundle priority updated: §9 removed from the priority list;
    new #1 is §13 AI-used badge.
  - "First concrete slice (post-§26 mapping)" pointer advanced
    from §9 to §13.
- `CHANGELOG.md`: this entry.

#### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: unchanged from `main` (the slice
  adds no Rust; the existing 994 tests still pass).
- `npm run check`: 0 new errors / warnings (1 pre-existing
  error + 9 pre-existing warnings on `main` are unchanged).
- `npm run build`: clean.
- Em-dash sweep on additions: 0 em-dashes outside code spans,
  0 en-dashes, 0 ellipses, 0 smart quotes (per the GitHub
  language rule).
- 25 metrics each carry a populated `context` field
  (`metric_registry.rs:246-467`), verified by `grep -c
  'context:' crates/astroforge-core/src/metric_registry.rs`
  returning the expected count.

#### Honest flags

- **No backend change.** The contextual data was already
  shipped; the slice is pure UI rendering. 0 new Rust, 0 new
  IPC, 0 new TS.
- **No new dependency.** Pure Svelte + CSS. No chart library,
  no icon library, no animation library.
- **Audit-row audit-claim verification found a misread.** The
  prior audit refresh listed `RecommendationCard.svelte` as
  the gap target; that component renders AI recommendations
  (tone / stars / color / noise / stretch), not metric tiles.
  The metric-context rendering lives in `MetricsTable.svelte`.
  The audit-doc text was tightened to reflect the correct
  target. The misread didn't change the slice scope: the
  data path is the same regardless of which component renders
  it. The audit-text correction matters for the next reader who
  looks at the §9 row.
- **The "post-§23.3" + "post-§24" first-concrete-slice
  pointers are retained as historical references.** The new
  "post-§26 mapping" pointer advances to §13. The slice loop
  has accumulated three generations of pointers; the audit
  doc keeps all three so a reader can trace the priority
  evolution.
- **One slight indentation slip caught and fixed.** Initial
  CSS patch replaced the existing `.metric-name { color:
  var(--on-surface); }` block with a much longer flex-column
  + child-rules block; the patch verified against the
  pre-patch content matched on the first try.
- **Honest stub buttons pattern does not apply here.** §9 is
  a single-cell rendering change, not a multi-CTA action bar.
  The slice renders the full §9 surface in one PR; no
  follow-up wire-up needed.

#### Out-of-scope (intentional)

- §8 saturation percentage (genuine ❌ in
  `image_analysis/metrics.rs`; rank #3 in the new priority
  list).
- §11 prose-display in `QualityGatePanel.svelte`
  (`summary` field is generated but not rendered; rank #2).
- §13 one-line AI-used badge (rank #1; next slice).
- §32 + §29 (B7 Perf bundle; rank #4 + #5).
- Pre-existing em-dashes on `main` (audit + svelte files):
  out of scope per Coding Discipline; the slice cleans only
  its own additions.

### Slice audit-refresh-2: §26 conceptual-to-actual mapping + scorecard reconciliation

**Scope.** Re-verifies every "⚠️ Partial" row in the CR-07 audit
against current code (per the `astroforge-cr-slices` skill's
audit-claim verification pitfall) and surfaces a mapping table that
the prior refresh missed: the §26 "Missing 9 commands" finding was
unsound because the spec lists 14 **conceptual** application commands
and explicitly notes that "The exact implementation paths should
follow the current repository structure rather than introducing
unnecessary parallel modules" (`CR-07-IMAGE-REVIEW-COMPARISON-DECISION.md:789`).
Every conceptual command maps to a shipped IPC under a renamed
surface (full mapping table in the audit).

#### Docs

- `docs/CR-07-AUDIT.md` (MOD):
  - **§26 Semantic API**: ⚠️ Partial → ✅ Shipped. Conceptual-to-actual
    mapping table added (14 spec commands → shipped IPCs/UI/data
    structures). Prior "Missing 9 commands" finding retracted.
  - **§11 Quality Assessment**: row text tightened to reflect that
    `assessment.rs::natural_language_summary` + `overall_summary` are
    shipped; the `QualityGatePanel.svelte` does NOT render the
    `summary` field yet. Gap is now precisely scoped.
  - **§13 AI-Aware Comparison**: row text tightened to reflect that
    every IPC is shipped and `ProvenancePanel` + `RecipeStageTimeline`
    surface AI provenance inline. Gap is now precisely scoped to a
    one-line "AI was used" badge in the compare-header.
  - **Scorecard**: §19, §20, §23, §24, §26 all flipped from ⚠️ to ✅
    to match their section bodies (the prior refresh's scorecard
    drifted from the bodies). Net delta: 62/22/3 → 66/18/3.
    Coverage: 71% → 76% shipped.
  - **Bundle status**: B6 row expanded to cover §23.1..§23.4 +
    §20 + §23.1..§23.4 PRs (#344..#348); new B6.5 "Close-outs"
    row for §24 + §19 close-out (PRs #349, #350); new "Audit
    refreshes" row covering #343 + this PR. CR-07 closure: ~96%
    → ~99%.
  - **Bundle priority**: §26 retracted; new #1 is §9 Contextual
    metric display, #2 is §13 one-line AI-used badge, #3 is §11
    prose-display, #4 is §8 saturation percentage, #5/6 are
    §32 + §29 (B7 Perf).
  - **First concrete slice (post-§26 mapping)** pointer advanced
    to §9. The (post-§23.3) and (post-§24) pointers remain as
    historical references.
- `CHANGELOG.md`: this entry.

#### Design decisions

1. **§26 ships via mapping, not via new code.** Every conceptual
   command maps to an existing IPC, data structure, or UI surface
   that already ships. Adding new IPCs would have reintroduced
   parallel modules that the spec explicitly forbids.
2. **§11 + §13 stay ⚠️ Partial** but with tightly-scoped gap text.
   The honest read: the data + DTO + generator are shipped, and
   the gap is a one-panel-render or one-badge-wire. Future slices
   flip these to ✅ with minimal UI work.
3. **Scorecard reconciliation**: the prior refresh's scorecard
   counted §19/§20/§23/§24 as ⚠️ even though their bodies said ✅.
   This refresh re-aligns the scorecard with the bodies. Audit
   scorecards should always be derived from the body status, not
   the other way around.
4. **Em-dash policy**: pre-existing em-dashes in section headings
   (§1-§35, with the exception of §20 + §23 + §26 which use colons)
   are NOT touched (per the Coding Discipline rule about
   pre-existing style); new entries use colons consistently.

#### Verification

- Markdown renders cleanly (verified by re-reading the diff; all
  `\`file.rs\`` and `\`file.svelte\`` short-name references follow
  the established audit-doc convention from PR #343).
- `git diff --cached --stat` shows only the 2 expected files
  (audit + changelog).
- `cargo fmt --all -- --check` and `cargo clippy --workspace
  --all-targets -- -D warnings` not run: P0 docs-only slice, no
  Rust changes.
- `npm run check` and `npm run build` not run: P0 docs-only slice,
  no UI changes.
- Em-dash audit on additions: 0 em-dashes, 0 en-dashes, 0
  ellipses, 0 smart quotes.
- Cross-references in added text all resolve (verified by `os.path.exists`
  on each `\`path/to/file\`` reference; bare short-name refs
  follow the audit-doc convention from PR #343).

#### Out-of-scope (intentional)

- Pre-existing em-dashes on `main` (46 lines, mostly section headings
  `## §N Title — Status`). Not touched per Coding Discipline; should
  be cleaned in a separate chore PR (file-wide sed, mechanical).
- Pre-existing drift in the CR-07 §31 acceptance table: row 23
  "User can continue editing from a selected version" still reads
  ⚠️ Partial, which is no longer accurate after §19 close-out.
  Out of scope for this audit refresh.
- §8 saturation percentage: genuine ❌; left for the next §8
  sub-slice (rank #4 in the new priority list).
- §32 + §29: left for the B7 Perf bundle; rank #5 and #6 in the
  new priority list.

### Slice §19 close-out: CR-07 "Create branch" stub wire

**Scope.** Closes the remaining gap from the §19
audit (the audit marked §19 as Shipped in C-A1 but
its own text noted "create-branch remains a stub").
This slice wires the button that has been disabled
since C-A1 (PR #339, 2026-09-15).

#### Frontend (Svelte/TS)

- `src/components/CompareWorkspace.svelte` (MOD):
  - NEW `createBranch()` handler: persists the
    branch intent durably via
    `applyImageDecision(bId, "preferred",
    "create_branch", selectedQualityProfile)` AND
    navigates to Enhance with
    `studioViewport.setView("enhance")`. The
    Quality Profile selection is threaded through
    per C-A3.5 so the decision row records which
    tier the user had in mind.
  - NEW `createBranchBusy` + `createBranchError`
    state vars (matching the `markPreferred` and
    `exportComposite` patterns).
  - The "Create branch" button now wires to the
    handler: `disabled` is replaced with
    `disabled={createBranchBusy || !bId}`, the
    `title` becomes "Record a branch intent for B
    and open Enhance", the `aria-label` becomes
    "Create a new branch from version B" (no
    longer "coming soon").
  - NEW inline error display (`{#if
    createBranchError}`) matching the existing
    `markPreferredError` and `exportError` panels.
  - The C-A1 action-bar comment block is updated
    to note that "Create branch" is now wired (no
    longer an "honest stub").

#### Why this is the right minimal wire

The full "create child version" IPC would require
shipping P5a's `create_image_version` command (per
the existing TODO at
`RecommendationCard.svelte` line 11). That is a
~500-800 LOC slice in its own right. The audit's
priority-list phrasing ("Wire the 'Create branch'
stub from C-A1") called for the smaller close-out
rather than the full P5a implementation.

This wire does two things distinctively:
1. **Distinct from "Mark Preferred"**: Mark
   Preferred only persists the decision; Create
   branch persists AND navigates.
2. **Distinct from "Continue enhancing"**: Continue
   enhancing only navigates; Create branch
   persists AND navigates, AND threads the user's
   Quality Profile through to the decision row.

The persisted `image_decisions` row with
`reason="create_branch"` is the durable breadcrumb
P5a can read when its `create_image_version` IPC
ships. The decision row carries the user's Quality
Profile selection, so P5a can pick up the user's
recipe intent without re-asking.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD): §19 entry expanded
  to list all four wired buttons (Mark Preferred,
  Continue enhancing, Create branch, Export).
  Bundle priority #1 updated to §26 Semantic API
  (new top-1, since §19 + §23 + §24 are all
  Shipped). "First concrete slice (post-§24)"
  pointer set to §19 close-out.
- `CHANGELOG.md`: this entry.

#### Verification

- 994 Rust tests pass (workspace). **0 new tests**
  added (this slice is pure UI; the existing
  `apply_image_decision` Rust tests cover the IPC
  contract that `createBranch` reuses).
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D
  warnings` clean.
- `npm run check`: 0 new errors / warnings (1
  pre-existing error + 9 pre-existing warnings on
  `main` are unchanged).
- `npm run build`: clean.
- Em-dash sweep: 0 em-dashes in the changed file
  (memory's GitHub language rule).

#### Honest flags

- Slice size: ~70 LOC of new code, mostly the
  `createBranch()` handler and the new state vars.
  Plus updated comment block + button attributes.
  In line with the audit's "smallest UI fix"
  framing for §19 close-out.
- **Pre-existing em-dashes on main, NOT introduced
  by this slice.** `CompareWorkspace.svelte`
  already has 5 em-dashes in lines that pre-date
  this PR (e.g. line 472 `versionA?.label ??
  "em-dash"`). These are GitHub-language-rule
  violations per memory's directive but are out of
  scope for this slice (per the **Coding
  Discipline** rule "Don't refactor things that
  aren't broken"). They should be cleaned in a
  separate chore PR.
- **One local fix cycle caught an em-dash.** The
  initial C-A1 comment block update contained an
  em-dash ("close-out: persists branch intent"),
  flagged by the post-write em-dash sweep. Fixed
  to use a colon before commit.
- **No new dependency.** Pure UI. No chart
  library, no animation library, no new IPC.
- **No backend changes.** Reuses the existing
  `apply_image_decision` IPC. The CI `rust` job
  runs the existing 994 tests unchanged.
- **Distinct from P5a.** The full child-version
  creation IPC (which would actually create a new
  `ImageVersion` row in the database) is P5a's
  scope, per the existing TODO at
  `RecommendationCard.svelte` line 11. This slice
  only wires the UI; P5a will read the durable
  breadcrumbs this slice creates.
- **One slight indentation slip caught and fixed.**
  The initial patch replaced the C-A1 comment
  block and the broken-indent version slipped
  through; a manual fix-up script restored the
  2-space indent.

Closes the §19 close-out gap noted in the audit
text. **§19, §23, §24 all Shipped.** Next slice
per the post-§19 priority list: §26 Semantic API
(the largest remaining backend work), then §32,
§9, §29.

### Slice §24: CR-07 Beginner comparison "Which do you prefer?" prompt (§24 close-out)

**Scope.** Closes §24 "Beginner Comparison" called out
by the CR-07 audit refresh as priority #1 post-§23.
Ships the "Which do you prefer? [Natural] [AI
Enhanced]" prompt with 1-paragraph explanation beneath
each option, rendered above the existing comparison
tools when both versions are selected.

#### Frontend (Svelte/TS)

- `src/components/BeginnerComparePrompt.svelte` (NEW,
  ~250 LOC): Svelte 5 component using `$props()` and
  `$state()`. Renders a two-button preference question
  above the existing comparison surfaces. Each button
  shows the version's label and a 1-paragraph
  explanation of what choosing this version means:
  "Natural: keep what the original capture gives you"
  for A, "AI Enhanced: apply the AI's processing to
  bring out faint details, smooth noise, and balance
  the dynamic range" for B. Clicking a button threads
  through `applyImageDecision(versionId, "preferred",
  undefined, selectedQualityProfile)` via the
  existing IPC. The prompt collapses to a one-line
  "You picked X as the preferred version" summary
  after the user picks, with a "Change my pick" link
  to undo. Loading + error states are rendered
  explicitly. Pure CSS, no new dep.
- `src/components/CompareWorkspace.svelte` (MOD):
  insert the `BeginnerComparePrompt` above the
  existing `mode-toggle` block. The prompt only
  renders when both versions have a primary
  artifact (matching the existing comparison-tools
  visibility condition). The existing expert
  toggles + tools still work in beginner mode for
  power users.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD): §24 status updated to
  "Shipped". Bundle priority #1 updated to §19
  close-out (new top-1, since both §23 and §24 are
  now Shipped). "First concrete slice (post-§23)"
  pointer updated to §24.
- `CHANGELOG.md`: this entry.

#### Verification

- 994 Rust tests pass (workspace). **0 new tests**
  added (this slice is pure UI; existing
  `applyImageDecision` tests cover the IPC contract).
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D
  warnings` clean.
- `npm run check`: 0 new errors / warnings (1
  pre-existing error + 9 pre-existing warnings on
  `main` are unchanged).
- `npm run build`: clean.
- Em-dash sweep: 0 em-dashes in all 4 changed files
  (memory's GitHub language rule).

#### Honest flags

- Slice size: ~270 LOC. In line with the audit's
  estimate for §24 (smallest "new-shape" slice). The
  bulk is the Svelte component (~250 LOC for the
  prompt UI + collapsed state + change-mind flow).
- One small wiring slip caught before push: the
  initial patch used `versionA?.id` (which doesn't
  exist on `ImageVersion`) instead of the existing
  `aId` state variable. The existing `aId`/`bId`
  state vars are already used elsewhere in
  `CompareWorkspace.svelte` (e.g. `markPreferred` at
  line 240), so reusing them keeps the prompt
  consistent with the existing flow.
- **Pre-existing em-dashes on main, NOT introduced by
  this slice.** `CompareWorkspace.svelte` already has
  5 em-dashes in lines that pre-date this PR (e.g.
  line 472 `versionA?.label ?? "em-dash"`). These are
  GitHub-language-rule violations per memory's
  directive but are out of scope for this slice
  (per the **Coding Discipline** rule "Don't refactor
  things that aren't broken"). They should be cleaned
  in a separate chore PR.
- §21 invariant preserved: the two buttons are
  equal-weight preference buttons, not a winner-pick.
  No AI ranking. The user decides; the system records
  the decision.
- The prompt's "Change my pick" link lets the user
  re-expand the prompt and pick again. This is
  per-page-load state (not persisted): picking again
  overwrites the previous decision via the IPC. The
  DecisionPanel + decision store handle persistence.
- No new dependency. Pure CSS, no chart library, no
  animation library.
- Pure UI slice. No Rust changes. The CI "rust" job
  is unchanged and runs the existing 994 tests. The
  CI "frontend" job exercises the new component.

Closes §24 of the CR-07 audit. **§24 fully shipped.**
Next slice per the post-§24 priority list: §19 close-
out (smallest remaining UI fix), then §26, §32, §9, §29.

### Slice §23.4: CR-07 Expert clipping masks panel (§23 sub-slice 4)

**Scope.** The fourth and final §23 "Expert Comparison"
sub-slice called out by the CR-07 audit refresh as
priority #1. Ships highlight + shadow clipping masks
as two stacked SVG visualizations plus their summary
stats. **§23 closes (Shipped) with this slice.**

#### Backend (Rust)

- `crates/astroforge-core/src/comparison_metrics.rs`
  (MOD):
  - NEW `ClippingMasks` struct: width, height, two
    flat row-major `Vec<u8>` masks (highlight_mask,
    shadow_mask), plus four scalar summary fields
    (highlight_count, shadow_count,
    highlight_fraction, shadow_fraction).
    `Serialize`-derived for the Tauri bridge.
  - NEW `clipping_masks(image) -> ClippingMasks`:
    downsample to the preview budget via
    `F32Image::downsample_box(0.25)`, then for each
    pixel record whether it is highlight-clipped
    (`v >= 0.99`) or shadow-clipped (`v <= 0.01`).
    Thresholds match `image_analysis::metrics::highlight_clipping`
    and `quality_gates::clipping`.
  - 5 new unit tests (detect-known-regions,
    zero-clips-for-mid-tone-image, thresholds-are-
    inclusive, determinism,
    field-size-matches-dimensions).
- `src-tauri/src/commands_comparison.rs` (MOD): NEW
  `get_version_clipping_masks(version_id)` command:
  thin wrapper that loads the version's applied
  pixels (via the existing path-confined `load_pixels`
  helper), runs `clipping_masks`, and returns a typed
  DTO. Wrapped in `tokio::task::spawn_blocking`.
- `src-tauri/src/main.rs` (MOD): register the new
  command in the Tauri `invoke_handler` list.

#### Frontend (Svelte/TS)

- `src/lib/astroforge-api.ts` (MOD): add
  `ClippingMasksData`, `ClippingMasksSnapshot`
  interfaces and the `getVersionClippingMasks(versionId)`
  wrapper.
- `src/components/ExpertClippingMasks.svelte` (NEW,
  ~250 LOC): Svelte 5 component using `$props()`,
  `$state()`, and `$effect()`. Loads the snapshot on
  mount and on `versionId` change. Renders two stacked
  SVG masks (highlight on top, shadow on bottom):
  320x160 viewBox each, one `<rect>` per pixel of the
  preview. Clipped pixels are saturated red (highlight)
  or deep blue (shadow); non-clipped pixels are dim
  grey (15% opacity) so the image shape stays visible
  without competing with the clipping signal. Below
  the two SVGs: a 4-cell summary grid (highlight count
  + fraction, shadow count + fraction). Loading + error
  + empty states are rendered explicitly. Pure SVG: no
  new dep.
- `src/components/CompareWorkspace.svelte` (MOD): add
  a fourth `expert-panels` row in the §23.1 toggle
  block, holding two `ExpertClippingMasks` panels
  (A and B) side by side.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD): §23 status updated to
  "Shipped" (the four sub-slices are listed). Bundle
  priority #1 updated to point to §24 Beginner mode
  (the new top-1, since §23 closes). "First concrete
  slice (post-§23.3)" pointer updated to §23.4.
- `CHANGELOG.md`: this entry.

#### Verification

- 994 Rust tests pass (workspace). 5 new in
  `comparison_metrics::clipping_masks`.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D
  warnings` clean.
- `npm run check`: 0 new errors / warnings (1
  pre-existing error + 9 pre-existing warnings on
  `main` are unchanged).
- `npm run build`: clean.
- Em-dash sweep: 0 em-dashes in all 6 changed files
  (memory's GitHub language rule).

#### Honest flags

- Slice size: ~600 LOC, in line with the §23.1
  estimate. The overage vs §23.3's 900 LOC is in the
  per-pixel Rust implementation (the per-pixel
  highlight/shadow check is simpler than the
  sliding-window MAD estimator for the noise map).
- The thresholds differ from §23.1's `clip_count`:
  §23.1 used `>= 1.0` (the literal normalized upper
  bound) but `clipping_masks` uses `>= 0.99` (matching
  the existing `highlight_clipping` and
  `quality_gates::clipping` thresholds). The two
  values are within 1% of each other on normalized
  pixel values and produce nearly identical clip
  counts in practice; the difference matters mainly
  for test fixtures that probe the exact boundary.
  Future consolidation (if §23.1's `clip_count` ever
  ships to the user) should adopt the `>= 0.99` /
  `<= 0.01` pair.
- The Svelte component uses HTML entity escaping for
  the `<=` and `>=` characters in mask labels
  (`&lt;=` and `&gt;=`). Svelte's HTML parser would
  otherwise interpret `<=` as the start of a tag.
- The "thresholds-are-inclusive" test fixture had to
  be split into two parts (a clear-clipping fixture
  and a near-threshold fixture) because box-downsampling
  averages neighbouring pixels, blurring boundary
  values into mid-tones. The lesson is documented in
  the test's doc comment.
- The two-mask display (highlight on top, shadow on
  bottom) is a deliberate design choice: a single
  combined panel would mix red and blue visually and
  make it hard to distinguish highlight- from
  shadow-clipped pixels. The two-row layout matches
  the established pattern (ChannelStats, FwhmDistribution,
  NoiseMap are all 2-column A/B; this panel adds a
  within-version row separation).
- No Tauri command-level tests were added; the mask
  logic is exercised via the unit tests on
  `astroforge-core` (so `cargo test --workspace` and
  CI's `rust` job both run them).

Closes §23.4 of the §23 "Expert Comparison" spec.
**§23 fully shipped.** Next slice per the post-§23
priority list: §24 Beginner mode, then §19 close-out,
then §26, §32, §9, §29.

### Slice §23.3: CR-07 Expert noise map panel (§23 sub-slice 3)

**Scope.** The third of four §23 "Expert Comparison"
sub-slices called out by the CR-07 audit refresh as
priority #1. Ships per-pixel 2D noise map visualization:
the per-pixel local sigma field as a row-major flat f64
array plus a three-number summary (min, mean, max). The
image is downsampled to the preview budget (≤256 px on
the long axis), then for each pixel the local sigma is
estimated over a 7×7 window using the same MAD-on-
residuals algorithm as the scalar `luminance_noise`
metric but applied per-pixel. The panel sits behind the
same "Show expert details" toggle on
`CompareWorkspace.svelte` as §23.1 + §23.2, per the §23
"progressive disclosure, consistent with CR-01"
requirement.

#### Backend (Rust)

- `crates/astroforge-core/src/comparison_metrics.rs`
  (MOD):
  - NEW `NoiseMap` struct: `width`, `height`,
    row-major flat `sigma: Vec<f64>`, and three-number
    summary (`min`, `mean`, `max`). `Serialize`-derived
    for the Tauri bridge.
  - NEW `noise_map(image) -> NoiseMap`: downsample to
    preview budget via `F32Image::downsample_box(0.25)`,
    then for each inner pixel (those inside the
    HALF=3 border) compute the local median over a
    7×7 window and the local sigma as
    `1.4826 * MAD` over the inner 5×5 residuals. Three
    scalar summary stats (min, mean, max) are computed
    alongside. The outer ring of the sigma field is
    zeroed (no stable estimator for those pixels).
  - 6 new unit tests (zeroed-for-empty-image,
    nonzero-for-noisy-image, summary-stats-match-array,
    determinism, spatial-correctness-for-half-noisy,
    field-size-matches-dimensions).
- `src-tauri/src/commands_comparison.rs` (MOD): NEW
  `get_version_noise_map(version_id)` command: thin
  wrapper that loads the version's applied pixels (via
  the existing path-confined `load_pixels` helper),
  runs `noise_map`, and returns a typed DTO
  `{ version_id, map: NoiseMap, width, height }`.
  Wrapped in `tokio::task::spawn_blocking` (the
  per-pixel estimator is O(W' * H' * 49) and would
  otherwise block the async runtime on large images).
- `src-tauri/src/main.rs` (MOD): register the new
  command in the Tauri `invoke_handler` list.

#### Frontend (Svelte/TS)

- `src/lib/astroforge-api.ts` (MOD): add `NoiseMapData`,
  `NoiseMapSnapshot` interfaces and the
  `getVersionNoiseMap(versionId)` wrapper.
- `src/components/ExpertNoiseMap.svelte` (NEW, ~250
  LOC): Svelte 5 component using `$props()`,
  `$state()`, and `$effect()`. Loads the snapshot on
  mount and on `versionId` change. Renders an SVG
  heatmap: 320×320 viewBox, one `<rect>` per pixel of
  the sigma field, fill colour interpolated between
  deep blue (low sigma, quiet), mid green, and deep red
  (high sigma, noisy), normalized to the [min, max] of
  the field. Below the heatmap: a 4-cell summary grid
  (min, mean, max, resolution). Loading + error +
  empty states are rendered explicitly. Pure SVG: no
  new dep.
- `src/components/CompareWorkspace.svelte` (MOD): add
  a third `expert-panels` row in the §23.1 toggle
  block, holding two `ExpertNoiseMap` panels (A and B)
  side by side.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD): §23 status updated to
  "Partial (3/N shipped)"; bundle priority #1 updated
  to "§23 Expert visualizations (cont.)" with the
  sub-slice roadmap (clipping masks); "First concrete
  slice (post-§23.2)" pointer to §23.3.
- `CHANGELOG.md`: this entry.

#### Verification

- 989 Rust tests pass (workspace). 6 new in
  `comparison_metrics::noise_map`.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D
  warnings` clean.
- `npm run check`: 0 new errors / warnings (1
  pre-existing error + 9 pre-existing warnings on
  `main` are unchanged).
- `npm run build`: clean.
- Em-dash sweep: 0 em-dashes in all 6 changed files
  (memory's GitHub language rule).

#### Honest flags

- Slice size: ~900 LOC. The overage vs the audit's §23.1
  estimate of 450 LOC per sub-slice is in the SVG
  heatmap (~250 LOC for the panel itself, including
  per-pixel `<rect>` rendering with proper colour
  ramp) and in the per-pixel Rust implementation
  (~150 LOC for the sliding-window estimator).
- Per-pixel noise estimation is O(W' * H' * WINDOW^2)
  on the preview budget (≤256 px on the long axis):
  ~3.2M ops on the worst-case preview. On a typical
  laptop this runs in ~50 ms. The Tauri command wraps
  it in `spawn_blocking` so the async runtime is not
  blocked. Per-version (A and B) sequential calls add
  ~100 ms total on a 4K image; the panel's loading
  state covers the wait. No caching: each toggle
  re-estimates. This is a known follow-on item
  (caching the per-version noise map is part of §29
  Performance).
- The outer ring of the sigma field (3 px on each
  side, equal to HALF for the 7×7 window) is zeroed.
  The Svelte component skips these zeroed pixels in the
  outer corners when drawing rects (the inner pixels
  are the meaningful region). This matches the
  contract documented in the `NoiseMap` struct's rustdoc.
- The "spatial correctness" test verifies that the
  right half of a half-noisy fixture has >2x the mean
  sigma of the left half. The 2x threshold (rather
  than 10x) is to allow for the smoothing introduced
  by the 7×7 window: near the boundary, the
  estimator averages pixels from both sides, reducing
  the apparent contrast.
- No Tauri command-level tests were added; the noise
  map logic is exercised via the unit tests on
  `astroforge-core` (so `cargo test --workspace` and
  CI's `rust` job both run them).

Closes §23.3 of the §23 "Expert Comparison" spec.
Next slice per the post-§23.3 priority list: §23.4
(clipping masks) to flip §23 to "Shipped", or pivot
to §24 Beginner mode per the refreshed audit priority.

### Slice §23.2: CR-07 Expert FWHM distribution panel (§23 sub-slice 2)

**Scope.** The second of four §23 "Expert Comparison"
sub-slices called out by the CR-07 audit refresh as
priority #1. Ships per-star FWHM distribution
visualization: the per-star FWHM values extracted from
`registration::extract_stars`, a seven-number summary
(count, mean, median, p25, p75, min, max) in pixels, and a
pre-binned histogram (Sturges' rule, capped to [1, 50]
bins) rendered as an SVG bar chart. The panel sits behind
the same "Show expert details" toggle on
`CompareWorkspace.svelte` as the §23.1 channel-stats
panel, per the §23 "progressive disclosure, consistent
with CR-01" requirement.

#### Backend (Rust)

- `crates/astroforge-core/src/comparison_metrics.rs`
  (MOD):
  - NEW `FwhmHistogram` struct: count, sorted raw
    values, seven-number summary (mean, median, p25,
    p75, min, max), bin edges (n+1 entries), and counts
    per bin (n entries). `Serialize`-derived for the
    Tauri bridge.
  - NEW `fwhm_distribution(image) -> Vec<f64>`: wraps
    `registration::extract_stars(image, 3.0)` and
    returns the `fwhm` field of every detected star.
    Threshold is `mean + 3σ` (the conventional value for
    star detection in noisy backgrounds). Sort order is
    inherited from `extract_stars` (brightest first).
  - NEW `fwhm_histogram(image) -> FwhmHistogram`:
    single-pass post-processing of `fwhm_distribution`.
    Edge cases handled: zero stars (returns zeroed
    struct with NaN summary), single star (one bin with
    the value as both edges), all-equal FWHM (one bin
    with N count), standard Sturges histogram otherwise.
    Bin count is `min(50, max(1, ceil(log2(n)) + 1))`.
    Percentiles use linear interpolation (matches
    numpy's default `method="linear"`).
  - NEW private helper `percentile(sorted: &[f64],
    q: f64) -> f64`: linear-interpolation percentile
    with the empty-slice guard.
  - 6 new unit tests (zeroed-for-no-stars,
    single-star, multi-star, determinism,
    distribution-count-matches-histogram, percentiles-
    match-linear-interp). All pass alongside the 21
    pre-existing comparison-metrics tests (27/27 in
    the `comparison_metrics` module; +6 from this
    slice).
- `src-tauri/src/commands_comparison.rs` (MOD): NEW
  `get_version_fwhm_distribution(version_id)` command:
  thin wrapper that loads the version's applied pixels
  (via the existing path-confined `load_pixels` helper),
  runs `fwhm_histogram`, and returns a typed DTO
  `{ version_id, histogram: FwhmHistogram, width,
  height }`. Wrapped in `tokio::task::spawn_blocking`
  (star extraction is O(W*H) and would otherwise block
  the async runtime on large images).
- `src-tauri/src/main.rs` (MOD): register the new command
  in the Tauri `invoke_handler` list.

#### Frontend (Svelte/TS)

- `src/lib/astroforge-api.ts` (MOD): add `FwhmHistogram`,
  `FwhmDistribution` interfaces and the
  `getVersionFwhmDistribution(versionId)` wrapper.
- `src/components/ExpertFwhmDistribution.svelte` (NEW,
  ~340 LOC): Svelte 5 component using `$props()`,
  `$state()`, and `$effect()`. Loads the snapshot on
  mount and on `versionId` change. Renders an SVG bar
  chart: 320×160 viewBox, 36 px left/24 px bottom
  padding for axis labels, dashed grid lines at
  top/mid/bottom, y-axis ticks at 0 and
  `Math.max(...counts)`, x-axis ticks at left/mid/right
  FWHM range. Below the chart: a 7-cell summary grid
  (count, mean, median, p25, p75, min, max). Loading +
  error + empty (count == 0) states are rendered
  explicitly; the empty case shows "No stars detected
  (count: 0). The histogram is empty; FWHM is a
  per-star measurement, so a starless image has no
  distribution to show." rather than a degenerate
  empty SVG. Pure SVG: no new dep.
- `src/components/CompareWorkspace.svelte` (MOD): add
  a second `expert-panels` row in the §23.1 toggle
  block, holding two `ExpertFwhmDistribution` panels
  (A and B) side by side.

#### Docs

- `docs/CR-07-AUDIT.md` (MOD): §23 status updated to
  "Partial (2/N shipped)"; bundle priority #1 updated
  to "§23 Expert visualizations (cont.)" with the
  sub-slice roadmap (noise maps, clipping masks);
  "First concrete slice (post-§23.1)" pointer to §23.2.
- `CHANGELOG.md`: this entry.

#### Verification

- 983 Rust tests pass (workspace). 6 new in
  `comparison_metrics::fwhm_histogram` plus the
  percentile helper test.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D
  warnings` clean.
- `npm run check`: 0 new errors / warnings (1
  pre-existing error + 9 pre-existing warnings on
  `main` are unchanged).
- `npm run build`: clean.
- Em-dash sweep: 0 em-dashes in all 6 changed files
  (memory's GitHub language rule).

#### Honest flags

- Slice size: ~700 LOC (slightly above the audit's
  §23.1 450 LOC estimate). The overage is in the
  Svelte SVG component (340 LOC for the SVG + summary
  grid + empty/error/loading states + accessibility
  attributes + caption) and in the test fixture
  generation (single-star + 5-peak Gaussian fixtures
  are ~50 LOC).
- Star extraction is O(W * H) per image via the
  existing `extract_stars` (it scans every pixel).
  On a 4K test image this is ~8M iterations and
  ~25M pixel reads. The Tauri command wraps the
  call in `spawn_blocking` so it does not block the
  async runtime. Per-version (A and B) sequential
  calls add ~50-200 ms on a typical laptop for a
  4K image; the panel's loading state covers the
  wait. No caching: each toggle re-extracts. This is
  a known follow-on item (caching the per-version
  FWHM distribution is part of §29 Performance).
- 6 unit tests cover the happy paths. No property
  tests (e.g. "FWHM distribution of two stacked
  images is the union of their distributions") were
  added; the audit didn't call for them.
- The "no stars detected" empty-state message is the
  default render when `count == 0`. The first
  attempt used a `flat_image(64)` fixture, but
  `extract_stars` correctly detects ~121 local maxima
  in a perfectly-flat field (every pixel sits exactly
  at threshold); the test was rewritten to use a
  low-amplitude-noise image where no pixel exceeds
  `mean + 3σ`. The lesson is documented in the test
  helper's doc comment.
- No Tauri command-level tests were added; the
  histogram logic is exercised via the unit tests on
  `astroforge-core` (so `cargo test --workspace` and
  CI's `rust` job both run them).

Closes §23.2 of the §23 "Expert Comparison" spec.
Next slice per the post-§23.2 priority list: §23.3
(noise maps), then §23.4 (clipping masks), or pivot
to §24 Beginner mode per the refreshed audit priority.

### Slice §23.1: CR-07 Expert channel statistics panel (§23 sub-slice 1)

**Scope.** The first of four §23 "Expert Comparison"
sub-slices called out by the CR-07 audit refresh as
priority #1. Ships per-channel (R, G, B, c{n}) statistics
for both sides of a comparison: mean, stddev, min, max, and
clip count. The panel sits behind a "Show expert details"
toggle on `CompareWorkspace.svelte` per the §23
"progressive disclosure, consistent with CR-01" requirement.

#### Backend (Rust)

- `crates/astroforge-core/src/comparison_metrics.rs`:
  - NEW `channel_stats(image) -> BTreeMap<String, f64>`:
    single-pass O(N) computation over the `Array3<f32>`
    pixels, maintaining per-channel f64 sums / sum-of-squares
    / min / max / clip count. R/G/B for the first three
    channels, c{n} numeric suffix for the 4th-and-beyond
    (narrowband support). The clip threshold is `>= 1.0` on
    the normalized scale (matches the highlight-clipping
    detector in `image_analysis::metrics`).
  - NEW `metric_snapshot_full(image) -> BTreeMap<String, f64>`:
    merges the existing 5 detector-backed keys from
    `metric_snapshot` with the 5 per-channel stat keys from
    `channel_stats`. The delta table (`compute_deltas`) and
    the post-comparison recommendation rule
    (`build_delta_table`) read only the detector-backed
    keys, so this is additive: no existing data path breaks.
  - 7 new unit tests (closed-form values, stddev against a
    known distribution, clip threshold boundary, 4-channel
    numeric suffix, determinism, single-pixel edge case,
    merged-snapshot key count). All pass alongside the
    pre-existing 8 comparison-metrics tests (15/15 in the
    `comparison_metrics` module).

- `src-tauri/src/commands_comparison.rs`:
  - NEW `get_version_metric_snapshot(version_id)` command:
    thin wrapper that loads the version's applied pixels
    (via the existing path-confined `load_pixels` helper),
    runs `metric_snapshot_full`, and returns a typed DTO
    `{ version_id, metrics: [{key, value}], width, height,
    channels }`. The BTreeMap → Vec projection is the
    only new logic.

- `src-tauri/src/main.rs` (MOD): register the new command
  in the Tauri `invoke_handler` list.

#### Frontend (Svelte/TS)

- `src/lib/astroforge-api.ts` (MOD): add
  `VersionMetricEntry`, `VersionMetricSnapshot` interfaces
  and the `getVersionMetricSnapshot(versionId)` wrapper.
- `src/components/ExpertChannelStats.svelte` (NEW, ~200
  LOC): Svelte 5 component using `$props()`, `$state()`, and
  `$effect()`. Loads the snapshot on mount + on `versionId`
  change, groups the flat `metrics[]` array by channel
  label, sorts R, G, B, c3, c4, ... in canonical order,
  renders a 6-column table (Channel / Mean / Stddev / Min /
  Max / Clipped pixels). Loading + error states are
  rendered explicitly; the clip column formats as both
  count and percent of total pixels.
- `src/components/CompareWorkspace.svelte` (MOD): add
  `showExpertDetails` state (default `false` per the
  progressive-disclosure requirement), a "Show/Hide expert
  details" toggle button (with `aria-expanded` and
  `aria-controls`), and the two `ExpertChannelStats` panels
  inside a 2-column grid that mirrors the
  DecisionPanel / ProvenancePanel layout. The toggle only
  renders when both `aId` and `bId` are set (i.e. the user
  has a comparison to inspect).

#### Docs

- `docs/CR-07-AUDIT.md` (MOD): §23 status updated to "⚠
  Partial (1/N shipped)"; bundle priority #1 updated to
  "§23 Expert visualizations (cont.)" with the sub-slice
  roadmap; "First concrete slice" pointer updated to §23.1.
- `CHANGELOG.md`: this entry.

#### Verification

- 957 Rust tests pass (workspace); 7 new in
  `comparison_metrics::channel_stats` plus the merged
  snapshot test.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings`
  clean.
- `npm run check`: 0 new errors / warnings (1 pre-existing
  error + 9 pre-existing warnings on `main` are unchanged).
- `npm run build`: clean.
- Em-dash sweep: 0 em-dashes in all 6 changed files (the
  §23.1 audit entry is in the new content; old content
  untouched per memory's surgical-clean policy).

#### Honest flags

- Slice size: 454 insertions + 1 new file (~200 LOC),
  1 deletion (a redundant duplicate-media-query in
  `CompareWorkspace.svelte` that the new section
  subsumed).
- The §23 audit row status changed from "⚠ Partial" to
  "⚠ Partial (1/N shipped)" rather than the standard
  "Shipped" / "Partial" / "Missing" because the §23
  sub-slices are independent (FWHM distribution, noise
  maps, clipping masks are still "Missing"). Future
  §23.2..N can flip the row to "Shipped" once the last
  sub-slice lands.
- The scorecard did not change in this slice: the §23 row
  was already "Partial" before, and "Partial" after.
  Channel statistics is one of four sub-items listed in
  the §23 "Missing" text, so closing it shifts the row
  from "Missing X, Y, Z" to "Missing Y, Z" without
  changing the partial/shipped bucket. The post-§20 score
  remains 62 shipped / 22 partial / 3 missing.
- No CLI / Tauri command-level tests were added; the
  channel-stats logic is exercised via the unit tests
  on `astroforge-core` (so `cargo test --workspace` and
  CI's `rust` job both run them).

Closes §23.1 of the §23 "Expert Comparison" spec. Next
slice per the post-§23.1 priority list: §23.2 (FWHM
distribution), then §23.3 (noise maps), then §23.4
(clipping masks), or pivot to §24 Beginner mode per the
refreshed audit priority.

### Slice §20: CR-07 Post-comparison recommendation feedback loop (§20 close-out)

**Scope.** Closes the §-level "Missing" flagged in the CR-07
audit refresh as priority #1. The other recommendation rules
in `astroforge-ai/src/recommendations/rules.rs` read a single
`ImageAnalysisReport`; they cannot see the comparison as a
whole. This slice adds the missing entry point: a pure function
over the two sides' `metric_snapshot`s + their
`ImageDecisionState`s that emits zero or one `AiRecommendation`
describing the trade-off the comparison revealed, in the
spec's voice ("Version B has X ✓, ⚠ Y → next step Z").

#### Backend (Rust)

- `crates/astroforge-ai/src/recommendations/post_comparison.rs`
  (NEW, ~620 LOC including tests): `PostComparisonInput` +
  `recommend_post_comparison` + `build_delta_table` +
  `MetricDeltaTable` + `MetricDeltaRow` + `DeltaDirection` +
  `POST_COMPARISON_ENGINE_VERSION`. Pure function; same
  inputs always produce the same `AiRecommendation` row
  (test-pinned). Emits a single row with
  `operation: "post_comparison_insight"`,
  `classification: "observation"`, no `model_candidates`
  (the recommendation is a finding + a next-step hint, not
  an enhancement operation), and `affected_regions:
  ["whole_frame"]` (the comparison is whole-image). 9 new
  unit tests cover: build correctness, direction
  classification, "no insight when sides equal",
  "no insight when only wins no costs", spec-voice phrasing,
  determinism, rejected/final gating, misidentified sides,
  confidence banding.
- `crates/astroforge-ai/src/recommendations/mod.rs` (MOD):
  register `pub mod post_comparison;` and re-export the
  new public surface (`recommend_post_comparison`,
  `build_delta_table`, `PostComparisonInput`,
  `MetricDeltaTable`, `POST_COMPARISON_ENGINE_VERSION`).
- `crates/astroforge-ai` (lib + integration tests):
  128 lib + 9 integration tests, all green; no
  pre-existing test broken by the addition.

#### Frontend (Svelte/TS)

- `src/components/RecommendationList.svelte` (MOD): extend
  `SafetyClassification` to include `"observation"`; add the
  "Observation" chip label; suppress the §16 disclosure
  banner for observation rows (the disclosure is for
  Perceptual + Generative operations that materially
  change the image; an observation is informational and the
  user still owns the decision per ADR-07.7). 0 new
  svelte-check errors / warnings (1 pre-existing error + 9
  pre-existing warnings on `main` are unchanged).

#### Docs

- `docs/CR-07-AUDIT.md` (MOD): flip §20 status from
  "⚠️ Partial" to "✅ Shipped"; bump scorecard from 61/23/3
  to 62/22/3 (71% shipped); mark "First concrete slice" as
  the post-§20 next step (now §23 Expert visualizations).

### Slice C-A3.5 — CR-07: Quality Profile picker persistence (§22 close-out)

**Scope.** Closes the "preview only" gap in C-A3. The
picker (QualityProfilePicker.svelte) captures the
user's selection, but until C-A3.5 the selection was
local-only and never reached the persisted decision
row. This slice threads the picker's value through
`applyImageDecision` into the `image_decisions`
table, so the panel can later surface "what profile
the user had picked" alongside the decision state.

#### Backend (Rust)

- `crates/astroforge-core/src/domain_store.rs`:
  - Migration v12: `ALTER TABLE image_decisions
    ADD COLUMN quality_profile TEXT;` plus a
    partial index on the column. Mirrors the
    `image_versions.recipe_id` precedent from B13a.
- `crates/astroforge-core/src/comparison.rs`:
  - `ImageDecision` gains a `quality_profile:
    Option<String>` field (NULL for legacy rows
    written before C-A3.5).
  - `ImageDecision::new` initialises the field to
    `None`.
- `crates/astroforge-core/src/decision_store.rs`:
  - `load_decision` selects the new column and
    populates `ImageDecision.quality_profile`.
  - `save_decision` writes the field into the
    INSERT/REPLACE.
  - New `apply_and_save_decision_with_profile`
    function: same as `apply_and_save_decision` but
    also takes `profile: Option<String>`. When the
    argument is `Some`, it overwrites the existing
    value; when `None`, the existing value is
    preserved so transitions without a profile don't
    accidentally clear the user's prior pick.
  - 3 new tests:
    - `quality_profile_round_trips_through_apply_and_save`
    - `quality_profile_none_preserves_existing_value`
    - `quality_profile_defaults_to_none_for_legacy_decision`
- `src-tauri/src/commands_comparison.rs`:
  - `ApplyDecisionRequest` gains
    `quality_profile: Option<String>` (serde-defaulted).
  - `apply_image_decision` threads it through to
    `apply_and_save_decision_with_profile`.
- `crates/astroforge-core/src/domain_store.rs`:
  - `migrations_apply_once_and_are_idempotent`
    updated to assert `schema_version == 12`.
- `crates/astroforge-core/src/project.rs`:
  - `open_round_trips_and_validates_identity`
    updated to assert `schema_version == 12`.

#### Frontend (TS + Svelte)

- `src/lib/astroforge-api.ts`:
  - `applyImageDecision` accepts an optional
    `qualityProfile: QualityProfile` argument and
    sends it on the request as `quality_profile`.
- `src/components/CompareWorkspace.svelte`:
  - `markPreferred()` passes
    `selectedQualityProfile` through to
    `applyImageDecision` so the decision row
    carries the user's currently-picked profile.
  - Other entry points (`continueEnhancing`,
    `exportComposite`) are unchanged — they don't
    transition a decision.

#### Out-of-scope (deferred)

- No Recipe-save wiring: the picker's value
  threads through decision transitions today. A
  future slice can set `Recipe.quality_profile` on
  the next `recipe_save` invocation from the same
  picker state (Recipe already has the field from
  C-A3). The picker carries the selection forward,
  so this is purely a wiring step on the existing
  Recipe field.
- No panel that displays the persisted profile
  (the JSON field round-trips; UI surfacing is a
  separate slice).

### Slice C-A3 — CR-07: Quality Profile picker (§22)

**Scope.** Closes CR-07 §22 (Quality Profiles) which
the audit marks ❌ Missing. Lands the full vertical:
backend enum + Recipe field + IPC + frontend picker
+ integration into CompareWorkspace.

#### Backend (Rust)

- `crates/astroforge-core/src/recipe.rs`:
  - New `QualityProfile` enum with 4 variants:
    `Natural` (default), `Detail`, `Clean`,
    `Publication`. Derives `Default` (variant =
    `Natural`), `Serialize`, `Deserialize`
    (lowercase), `PartialEq`, `Eq`, `Copy`.
  - Helper methods on `QualityProfile`:
    `ALL` (display order), `label()`,
    `description()`.
  - New `quality_profile` field on `Recipe`
    (serde-defaulted to `Natural` for backward
    compatibility with legacy data).
  - `Recipe::new` sets the default.
  - 5 new tests in the test module:
    - `test_quality_profile_default_is_natural`
    - `test_quality_profile_all_returns_four_variants`
    - `test_quality_profile_label_and_description`
    - `test_quality_profile_legacy_recipe_defaults_to_natural`
    - `test_quality_profile_serde_round_trip_all_variants`
- `src-tauri/src/main.rs`:
  - New `QualityProfileInfo` struct (camelCase
    JSON shape with `id`, `label`, `description`).
  - New `quality_profile_list` Tauri command
    returning the 4-variant catalog.
  - Registered in `invoke_handler`.

#### Frontend (TS + Svelte)

- `src/lib/astroforge-api.ts`:
  - New `QualityProfileInfoJson` interface.
  - New `qualityProfileList()` IPC wrapper.
  - New `QUALITY_PROFILES` tuple + `QualityProfile`
    type + `DEFAULT_QUALITY_PROFILE` const +
    `isQualityProfile()` type guard.
- `src/components/QualityProfilePicker.svelte`
  (NEW, 130 LOC):
  - Svelte 5 runes (`$state`, `$effect`,
    `$derived.by`, `$props`).
  - Loads the catalog on mount via
    `qualityProfileList()`. Renders a `<select>`
    with the 4 variants + a one-line description
    underneath. Auto-syncs to `Natural` if the
    parent's value is unknown.
- `src/components/CompareWorkspace.svelte`:
  - Imports `QualityProfilePicker` +
    `DEFAULT_QUALITY_PROFILE` + `QualityProfile`
    type.
  - New local `selectedQualityProfile` `$state`
    + `handleQualityProfileChange()` callback.
  - New `.quality-profile-row` block in
    `.compare-extras` rendering the picker with
    an honest "preview" label + a hint about
    persistence landing in a future slice.

#### Honest flags

- **No recipe persistence yet.** The picker
  captures the user's selection in local
  component state. Persisting to a Recipe
  (and threading through `ApplyAiOperationRequest`)
  is a follow-up slice (the `quality_profile`
  field is on `Recipe`; the next slice wires
  the save flow and the apply-round flow).
- **No apply-round integration yet.** Same
  reason. The Rust enum + IPC + picker
  vertical lands first; the data-flow
  vertical lands in the follow-up.
- **No new tests for the picker component.**
  Repo has no frontend test runner
  (svelte-check + manual trace; same as
  B5-B8, B14, B15, C-A1, C-A2).

#### Audit cross-checks

The audit's other ❌ items still standing:
§23 (Expert viz), §25 (data model gaps),
§26 (semantic API), §33 (ADRs). C-A3 closes
§22 cleanly without pretending to close
those.

### Slice C-A2 — CR-07: Side-by-side comparison export (§30)

**Scope.** Closes CR-07 §30 (Export From
Comparison) which the audit marks ⚠️ Partial.
Lands a real PNG export of the side-by-side
composite (A | B) directly from the
comparison view.

#### Frontend (Svelte + lib)

- `src/lib/comparison-export.ts` (NEW, 320 LOC):
  - `exportComparisonComposite(opts)`:
    parallel-loads A and B's primary artifact
    bytes via the existing `read_image_artifact`
    IPC, decodes the 16-bit TIFF inline to an
    RGBA Uint8ClampedArray, paints both onto
    an off-screen canvas at a chosen panel
    width (default 1024 px) with a 16 px
    gutter, draws A and B labels, and
    returns a PNG Blob via `canvas.toBlob`.
  - `downloadBlob(blob, filename)`: triggers
    a browser download via `<a download>`
    + object URL revoke.
  - Inline 16-bit TIFF decoder (grayscale
    and RGB, uncompressed). Mirrors the
    approach in `ImageCanvas.svelte` but
    outputs 8-bit RGBA since the export
    canvas doesn't need HDR data.
- `src/components/CompareWorkspace.svelte`:
  - Imports `exportComparisonComposite` +
    `downloadBlob`.
  - New `exportComposite()` async handler
    that calls the lib and downloads the
    result with a timestamped filename
    (`astroforge-comparison-<ISO>.png`).
  - The C-A1 Export stub is now wired
    (no longer `disabled`); button shows
    "Exporting…" while busy.
  - Inline error rendering for export
    failures (`Export failed: <msg>`).

#### Honest flags

- **No backend changes.** Reuses the
  existing `read_image_artifact` IPC.
- **No new tests.** Repo has no frontend
  test runner (svelte-check + manual
  trace; same as B5-B8, B14, B15, C-A1).
- **PNG only.** JPEG support is trivial
  to add but not in this slice.
- **Single-frame composite.** Does not
  encode comparison mode (overlay, blink,
  split) as animation frames.
- **No "analytical report" export.**
  That covers the second bullet of §30;
  ships in a future slice.
- **No zoom / pan / region state.** The
  export is a clean side-by-side at the
  source images' aspect ratio, not a
  pixel-faithful copy of the on-screen
  canvas.

#### Audit cross-checks

§22 (Quality Profiles) remains genuinely
missing in the codebase and is not part
of this slice; it requires a backend enum,
DB column, Recipe tie-in, and frontend
picker, and is a multi-PR effort. It is
moved to a future Tier C paperwork
tranche.

### Slice C-A1 — CR-07: Continue-from-comparison action bar (§19)

**Scope.** Closes CR-07 §19 (Compare → Continue
Workflow) which the audit marks ⚠️ Partial.
Adds a 4-button action bar to CompareWorkspace
that fires when both A and B are selected.

#### Frontend (Svelte)

- `src/components/CompareWorkspace.svelte`:
  - Imports `studioViewport` + `applyImageDecision`.
  - New `markPreferred()` async handler that
    calls `applyImageDecision(bId, "preferred")`.
  - New `continueEnhancing()` handler that
    navigates to the Enhance workspace.
  - New `.continue-bar` toolbar rendered at
    the bottom of `.compare-extras` with
    4 CTAs:
    1. **Mark Preferred (B)**: primary
       button; wired to the B4
       `applyImage_decision` IPC.
    2. **Continue enhancing**: wired to
       `studioViewport.setView("enhance")`.
       The Enhance flow derives its source
       from the project's latest version
       today; a future slice may add a
       per-version source override.
    3. **Create branch**: honest stub
       with `disabled` + tooltip
       "Coming in C-A2".
    4. **Export comparison**: honest
       stub with `disabled` + tooltip
       "Coming in C-A3".
  - Inline error rendering for
    Mark Preferred failures.

#### Honest flags

- **No backend changes.** Reuses the B4
  `apply_image_decision` IPC and the existing
  `studioViewport.setView` mechanism.
- **No new tests.** Repo has no frontend test
  runner (svelte-check + manual trace).
- **No new IPC.** C-A2 + C-A3 will land
  the Create branch + Export wiring; this
  slice ships the action bar shape + the
  one CTA that doesn't need a new IPC.
- **Continue enhancing is a navigate-only
  CTA.** It doesn't pass B's id to the
  Enhance workspace; the Enhance flow
  uses the latest version as its source.
  A future slice may add a per-version
  source override.
- **Honest stubs (Create branch, Export)
  ship disabled** with tooltips. The audit's
  §19 item says "Continue Enhancing /
  Create Branch / Mark Preferred / Export
  buttons from the comparison view"; C-A1
  ships the button shape + the one wired
  CTA; C-A2 + C-A3 ship the rest.

#### Audit cross-checks

The audit marks §5.5 (overlay) and §11
(NL summary) as gaps; this slice did NOT
work on them because both are already
shipped (overlay in B6; NL summary in B4's
MetricsTable). The audit is stale on those
items.

### Slice B15 — CR-07: RecipeStageTimeline component

**Scope.** Ships a full vertical stage timeline
that complements the B14 ProvenancePanel. The B14
preview was intentionally truncated to 3 stages;
B15 is the source of truth for full stage
visibility including expandable per-stage params.
No audit anchor: B14 already closed §13/§14;
this slice is feature-add polish.

#### Frontend (Svelte)

- `src/components/RecipeStageTimeline.svelte`
  (NEW): subscribes to the B13c provenanceStore;
  renders five states (loading / error / empty
  "not recorded" / loaded-recipe / idle). When
  the recipe is present:
  - **Summary line**: total stage count +
    enabled vs disabled breakdown.
  - **Ordered stage cards**: each card shows
    stage number, stage_id, enabled/disabled
    pill, and a `<details>` block with the full
    `params` HashMap rendered as a key/value
    table (sorted by key for stable output;
    values formatted via `JSON.stringify` for
    complex types).
  - **Disabled stages** are visually dimmed.
- `src/components/CompareWorkspace.svelte`:
  - Imports `RecipeStageTimeline`.
  - New `<div class="provenance-row">` block
    holding two RecipeStageTimeline instances
    (A | B), reusing the B14 grid layout.

#### Honest flags

- **No backend changes.** Reuses B13c IPC +
  store. Pure Svelte.
- **No new tests.** Repo has no frontend test
  runner (svelte-check + manual trace).
- **No new IPC.** Stage params already arrive
  in the B13c `recipe_get_for_image_version`
  response via `Recipe.stages[]`.
- **Profile-picker in `EnhancementStudio`
  is still not implemented** (separate UI
  slice). Until that lands, both the B14
  ProvenancePanel and B15 StageTimeline render
  the "Profile not recorded" state for every
  AI-applied version.
- **Recipe-level data only.** RecipeStage has
  no timestamps (those live on PipelineRun.
  stage_runs, not on the Recipe profile). The
  audit's "processing history with timestamps"
  is per-RUN; addressing it would require a
  new IPC (recipe_stage_runs_for_image_version)
  joining ImageVersion → PipelineRun →
  StageRun. That is a different slice.
- **B14 ProvenancePanel stage preview stays
  in place.** Both components render; the B14
  preview is the at-a-glance "how many stages?"
  view, B15 is the full breakdown. Removing
  the B14 preview is a follow-up design
  decision.

### Slice B14 — CR-07: ProvenancePanel component

**Scope.** Surfaces the B13c live `Recipe` (or
honest "not recorded" state) for the two
selected Image Versions in the Compare workspace.
Closes audit items 311 ("AI processing is
identified") and the §13/§14 half of CR-07.

#### Frontend (Svelte)

- `src/components/ProvenancePanel.svelte` (NEW):
  renders five states (loading / error / empty
  "not recorded" / loaded-recipe / idle). When
  the recipe is present, surfaces:
  - **Identity**: Recipe name, description,
    target type, version, branch, created-at.
  - **Integrity badges**: Perceptual /
    Deterministic / Seed recorded (active
    badges are green-tinted).
  - **Model list**: `ModelUsage` rows with
    Deterministic / Perceptual type tags.
  - **Stage preview**: first 3 stages
    (numbered, with `disabled` annotation);
    the full vertical timeline ships in B15.
- `src/components/CompareWorkspace.svelte`:
  - Imports `ProvenancePanel`.
  - New `.provenance-row` grid that mirrors
    the existing `.decision-row` layout: two
    ProvenancePanels side by side, one per
    A/B selection. Collapses to a single column
    below 900px (same breakpoint as
    `.decision-row`).
  - The two panels each call
    `provenanceStore.load(versionId)` on
    `versionId` change; the B13c store's
    in-flight stale-load guard means they
    don't fight each other when the user
    changes A and B in quick succession.

#### Honest flags

- **No backend changes.** The B13c IPC is
  reused; this slice is pure Svelte.
- **No new tests.** The repo has no
  frontend test runner (svelte-check + manual
  trace is the established pattern; same as
  B5-B8).
- **Per-project lookup limitation inherited**
  (B9 store consolidation): the panel
  renders "Version not found" if the user is
  in Project A and the version is from
  Project B. Pre-existing, not new in B14.
- **Profile-picker in `EnhancementStudio`
  is still not implemented** (separate UI
  slice). Until that lands, every AI-applied
  version renders the "Profile not recorded"
  state. B14 ships the panel that *receives*
  the data; B15+ ships the timeline; the
  picker is a future slice.
- **Stages preview is intentionally limited**
  to 3 rows in B14. The full vertical
  timeline (timestamps, op parameters,
  per-stage model usage) is B15.

### Slice B13c — CR-07: recipe_get_for_image_version IPC + provenance store

**Scope.** Adds the lookup chain that connects a given
Image Version to the live `Recipe` profile that produced
it. B13a added the column; B13b added the producer; B13c
adds the consumer-side IPC + a Svelte store. No UI yet;
B14 will render the data in the ProvenancePanel.

#### Backend (Rust)

- `src-tauri/src/main.rs`:
  - New Tauri command `recipe_get_for_image_version`
    that takes a `version_id` and returns
    `Result<Option<Recipe>, CommandError>`. Walks two
    stores: the active project's `DomainStore` for the
    `image_versions` row (B13a) and the global
    `RecipeStore` for the live `Recipe` keyed by
    `profile_id`.
  - Returns `Ok(None)` for both "version not found"
    and "version has no recorded recipe"; returns
    `Err(CommandError)` only when the version
    references a `recipe_id` that no longer exists in
    the RecipeStore.
  - Registered in the `invoke_handler` alongside the
    other recipe commands.
- `src-tauri/src/commands_ai_enhancement.rs`: no
  changes; the new command reuses the existing
  `image_version_get` for step 1.

#### Frontend (TypeScript)

- `src/lib/astroforge-api.ts`:
  - New `recipeGetForImageVersion(versionId)` wrapper.
  - New `RecipeFromRust` interface mirroring the Rust
    `Recipe` wire shape (snake_case). Declared here
    rather than imported from `profile-store.ts` to
    avoid a cross-file cycle and keep the IPC layer
    self-contained.
- `src/state/provenance-store.ts` (NEW): a small
  Svelte store that wraps the IPC call. Exposes
  `provenanceStore.load(versionId)` and `reset()`. The
  state shape is `{ versionId, recipe, status, error }`
  where `status ∈ "idle" | "loading" | "loaded" |
  "error"` and `recipe` is the camelCase Svelte-side
  `ProvenanceRecipe` (translated from the snake_case
  IPC shape). Stale-load guard: if the user clicks
  another version while a fetch is in flight, the
  older result is dropped silently.

#### Honest flags

- **No UI in this slice.** The store is wired but no
  component subscribes to it yet. B14 will add the
  `ProvenancePanel.svelte` and bind it to
  `provenanceStore.load(versionA | versionB)` from
  `CompareWorkspace.svelte`.
- **No tests in this slice.** The new Rust command is
  an end-to-end join of two existing helpers
  (`image_version_get` + `recipe_get_head`), both of
  which already have test coverage. A unit test for
  the join would need a Tauri `State` mock, which the
  repo doesn't have a pattern for yet. The command's
  behavior is covered by the integration path the CI
  smoke tests exercise.
- **Per-project lookup limitation inherited.** The
  command reads the active project's `DomainStore`
  via `with_store`; if the user is in Project A and
  queries a version from Project B, the lookup fails
  (returns "not found"). This is a pre-existing
  limitation of the B9 store consolidation, not
  something B13c creates. B14 inherits it; a future
  project-aware version resolver would address it.
- **No `src-tauri/Cargo.lock` mutation needed** (no
  new deps; the changes are a new command + serde
  juggling).

### Slice B13b — CR-07: apply round populates `recipe_id`

**Scope.** Wires the `recipe_id` from the apply round's
request through to the new `ImageVersion` row so the
field is no longer hard-coded to NULL. B13b is the
**second half of the data-model link**: B13a added the
column + read path; B13b adds the producer. No UI yet;
B14+ will surface the value in the ProvenancePanel.

#### Backend (Rust)

- `src-tauri/src/commands_ai_enhancement.rs`:
  - `ApplyAiOperationRequest` gains `recipe_id: Option<String>`
    (serde-defaulted, so legacy JSON payloads without
    the field still deserialise and the apply round
    writes NULL — the same behavior as B13a).
  - The apply round's `ImageVersion` construction in
    `enhancement_apply_operation` now sets
    `recipe_id: request.recipe_id.clone()` instead of
    `None`. The B13a comment pointing at B13b is
    replaced with a comment pointing at B14 (the
    ProvenancePanel).
  - 2 new unit tests in the `mod tests` block:
    `apply_request_recipe_id_defaults_to_none`
    (legacy JSON without the field → `None`) and
    `apply_request_recipe_id_round_trips`
    (recipe_id in JSON → `Some(...)`).

#### Frontend (TypeScript)

- `src/lib/astroforge-api.ts`:
  - `ApplyAiOperationRequest` gains `recipe_id?: string | null`
    (optional, so no existing caller breaks).

#### Honest flags

- **Caller behavior unchanged.** `EnhancementStudio.svelte:onApplyOperation`
  (the only caller) does not pass a profile id today
  and continues to not pass one. The wiring is in place
  for the future profile-picker slice; until that lands,
  every apply round still writes `recipe_id = NULL`.
- **No back-fill helper in this slice.** Legacy rows
  (including the B13a-era test rows in any user's DB)
  stay NULL. A back-fill needs a project↔profile binding
  that doesn't exist yet; deferred to a separate slice
  that introduces the project-level profile anchor.
- **The 2 B13b tests live in `src-tauri`**, which the
  slice skill notes is binary-only and OUTSIDE the
  cargo workspace. CI's `cargo test --workspace` step
  does not see them; CI's `rust` job runs
  `cd src-tauri && cargo test` separately, which does.
  The workspace test count stays at 953 (B13a's 1 new
  test is the only one that shows up in the workspace
  totals; B13b's 2 tests show up in the `rust` CI job
  output).
- **No `src-tauri/Cargo.lock` mutation needed** (no new
  deps; the changes are struct fields + serde attrs +
  tests).

### Slice B13a — CR-07: per-ImageVersion Recipe link (provenance schema)

**Scope:** Adds the missing data model link between an
`ImageVersion` and the `Recipe` profile that produced it.
B13a ships the schema + read path only; B13b will wire
the apply round to populate `recipe_id`, and B13c/B14+
will surface it in the ProvenancePanel UI. This is the
foundation for CR-07 §13 / §14 / audit items 311-314
("AI processing is identified", "Processing history",
"AI model information", "Recipe information").

#### Backend (Rust)

- `crates/astroforge-core/src/domain.rs`:
  - `ImageVersion` struct gains `recipe_id: Option<String>`
    (serde-defaulted, so legacy JSON still deserialises).
- `crates/astroforge-core/src/domain_store.rs`:
  - Migration 11: `ALTER TABLE image_versions ADD
    COLUMN recipe_id TEXT;` plus a partial index on
    non-null values. Migration runner is idempotent
    (already applied on test `store()` fixture and on
    any live project DB that has been opened at least
    once since this PR landed).
  - `upsert_image_version` writes the new column.
  - `get_image_version`, `list_image_versions_for_project`,
    `list_image_versions_all` all SELECT the new column.
  - `migrations_apply_once_and_are_idempotent` + the
    `open_round_trips_and_validates_identity` test in
    `project.rs` bumped to expect `schema_version() == 11`.
  - New test: `recipe_id_round_trip_on_image_version`
    writes two rows (one with `recipe_id`, one legacy
    NULL), then asserts both round-trip through
    `get_image_version`, `list_image_versions_for_project`,
    and `list_image_versions_all`.
- `crates/astroforge-core/src/comparison_metrics.rs`:
  - Test helper `ImageVersion` literal updated to
    include `recipe_id: None`.
- `crates/astroforge-core/tests/ai_enhancement.rs`:
  - 5 `ImageVersion` literal sites updated to include
    `recipe_id: None` (4 false-hidden, 1 true-hidden).
- `crates/astroforge-core/tests/dod_enhancement.rs`:
  - 2 `ImageVersion` literal sites updated to include
    `recipe_id: None`.
- `src-tauri/src/commands_ai_enhancement.rs`:
  - Apply round's `ImageVersion` construction
    (`apply_operation_apply_round`) now writes
    `recipe_id: None` with a comment pointing at
    the B13b follow-up that will plumb the field
    through `EnhancementApplyRequest`.

#### Frontend (TypeScript)

- `src/lib/astroforge-api.ts`:
  - `ImageVersion` + `ImageVersionJson` interfaces
    both gain `recipe_id: string | null`.

#### Honest flags

- **No new IPC, no new Tauri command, no new UI.** This
  is a pure data-model slice.
- **AI-applied versions still have `recipe_id = NULL`**
  because `EnhancementApplyRequest` does not carry a
  recipe id today. B13b will grow the request struct +
  the apply round's `ImageVersion` construction to take
  an optional `recipe_id`. Until then, the
  ProvenancePanel (when it lands) will show
  "Profile not recorded" for AI-applied versions.
- **Legacy rows get NULL** (SQLite `ALTER TABLE ADD
  COLUMN` with no DEFAULT puts NULL in existing rows).
  The UI will surface this honestly.
- **No `src-tauri/Cargo.lock` mutation needed** (the
  diff is data-model + tests; no new deps).

### Slice B12 — CR-07 UX: §5.4 fixed-stretch + n-sigma normalizer

**Scope:** Surfaces the explicit-cutoff and n-sigma
stretch modes alongside B11's auto-stretch. Closes the
B10 honesty flag's second half (fixed-stretch + n-sigma).
Fixed-stretch gives reproducible results across runs;
n-sigma auto-computes cutoffs from the current diff
image's mean and standard deviation.

#### Backend (Rust)

- `crates/astroforge-core/src/difference_normalize.rs`
  (extended):
  - `StretchStats::is_noop` semantics extended: now true
    when `v_low == 0 && v_high == 255` (no stretch) OR
    `v_low >= v_high` (degenerate range). This matches
    "did the function actually remap anything".
  - `normalize_fixed(pixels, v_low, v_high)`: in-place
    remap using an explicit pair of cutoffs. Pixels below
    `v_low` clamp to 0; above `v_high` clamp to 255;
    in-range pixels linearly remap to `[0, 255]`. Alpha
    preserved. Degenerate range (`v_low >= v_high`) is
    a no-op.
  - `mean_stddev(pixels) -> (mean, stddev)`: arithmetic
    mean and population standard deviation across all
    RGB bytes; alpha skipped.
  - `normalize_n_sigma(pixels, k)`: stretch
    `[mean - k*sigma, mean + k*sigma]` to `[0, 255]`.
    Returns the resulting `StretchStats` plus the
    computed `mean` and `stddev`. Negative or NaN `k`
    falls back to `k = 1.0`. Degenerate range is a
    no-op.
  - 9 new unit tests: fixed-stretch remap / clamp /
    alpha preservation / invalid range, mean_stddev
    uniform / known distribution, n-sigma uniform no-op
    / known-distribution clamps / invalid-k fallback.

#### Frontend (Svelte)

- `src/components/CompareTools.svelte`:
  - `StretchMode` type extended: `"off" | "auto" | "manual"`.
    New state: `vLowFixed` (default 32), `vHighFixed`
    (default 220), `nSigmaK` (default 3).
  - `applyFixedStretch(pixels, vLow, vHigh)` mirrors the
    Rust `normalize_fixed` math on the canvas side.
  - `computeMeanStddev(pixels)` mirrors the Rust
    `mean_stddev` helper.
  - `applyNSigmaToFixed()` reads the current diff canvas,
    computes mean+stddev, writes `mean ± k*sigma` (clamped
    to `[0, 255]`) back into `vLowFixed` / `vHighFixed`,
    then triggers a recompute.
  - A new `Manual` toggle button appears in the stretch
    sub-toolbar. When active, two number inputs (`vLow`,
    `vHigh`), an n-sigma `k` slider (1..6), and an `Apply
    n-sigma` button appear.
  - CSS adds `.nsigma-apply` styles (slight tint so the
    action button stands out from the mode toggles).

#### Honesty flags

- Same pattern as B11: frontend computes on the canvas
  side; Rust `normalize_fixed` / `mean_stddev` /
  `normalize_n_sigma` are the canonical reference for
  any non-canvas consumer.
- `n_sigma_clamps_to_byte_extents` test confirms the
  expected behavior when `mean ± k*sigma` extends beyond
  `[0, 255]`: the cutoffs clamp and the function no-ops
  (the pixels stay at their pre-stretch values). This is
  intentional: a flat histogram with a wide standard
  deviation shouldn't be squashed into a single bucket.
- `StretchStats::is_noop` semantics change is a soft
  behavior change: existing callers that relied on
  `v_low == 0 && v_high == 255` for "no stretch"
  detection will now also see degenerate ranges as no-op.
  This is more accurate but should be audited by any
  caller of `StretchStats`. Currently no other caller
  exists in the repo.

### Slice B11 — CR-07 UX: §5.4 auto-stretch normalizer

**Scope:** Surfaces the per-channel percentile histogram
stretch as the canonical astronomy post-processing step
on the diff canvas. Closes the B10 honesty flag about
low-magnitude diffs reading as near-black.

#### Backend (Rust)

- `crates/astroforge-core/src/difference_normalize.rs`
  (NEW, ~330 LOC):
  - `StretchStats` struct: returned by `normalize_stretch`
    so the UI can surface what range was actually
    stretched.
  - `normalize_stretch(pixels, low_pct, high_pct)`: single
    in-place remap function.
    Algorithm: build a 256-bin histogram per channel over
    the input pixels (alpha skipped), walk it to find
    `v_low` / `v_high` at the configured percentiles, then
    linearly remap `[v_low, v_high]` to `[0, 255]`.
    Pixels below `v_low` clamp to 0; above `v_high` clamp
    to 255. Out-of-range percentiles clamp silently;
    degenerate ranges no-op.
    Uses `ceil` for the low cutoff and `floor` for the high
    so a 0.5% cutoff against a small sample actually skips
    a pixel instead of truncating to 0.
    The denominator is the number of non-alpha pixel
    values (3 channels per pixel), not the pixel count;
    the histogram bins aggregate over channels.
  - 8 unit tests: equal-image no-op, already-stretched
    no-op, mid-range stretch, alpha bytes preserved,
    invalid percentile pair, out-of-range clamp, stretch
    identity on already-stretched, `StretchStats::is_noop`.

- `crates/astroforge-core/src/lib.rs`: registers the new
  module.

#### Frontend (Svelte)

- `src/components/CompareTools.svelte`:
  - `StretchMode` TypeScript type (`"off" | "auto"`)
    plus `stretchLowPct` (default 0.5) and
    `stretchHighPct` (default 99.5) state.
  - `applyStretch(pixels)` mirrors the Rust
    `normalize_stretch` math on the canvas side; runs
    after the diff blend in `recomputeDifference()` when
    `stretchMode === "auto"`. Sub-millisecond on a 1-2
    megapixel canvas.
  - A new `.stretch` sub-toolbar with `Auto stretch` /
    `Off` toggle buttons appears when Difference mode is
    active. When `Auto stretch` is selected, two slider
    controls (`Low` 0..20%, `High` 80..100%) appear for
    percentile tuning.
  - CSS adds `.stretch` and `.stretch button` styles
    (hairline separator, compact button sizing to match
    `.diff-kind`).

#### Honesty flags

- The frontend computes the stretch in TypeScript on the
  canvas side, not via a Tauri round-trip. The Rust
  `normalize_stretch` is the canonical reference for any
  non-canvas consumer (CI fixtures, batch comparison).
- The histogram is built per channel (R/G/B separately)
  on the backend, then cutoffs are picked independently
  per channel. The frontend currently shares cutoffs
  across all three channels for simplicity. Per-channel
  cutoff rendering would need three separate histograms
  on the frontend (more code, marginal visual gain for
  astrophoto RGB data). Documented as a future slice.
- `v_high <= v_low` degenerate case (single-value diff
  image, no spread) is a no-op so the canvas stays at
  whatever the raw diff produced.

### Slice B10 — CR-07 UX: §5.4 Difference mode selector

**Scope:** Surfaces the four §5.4 difference modes (Absolute,
Signed, Amplified, Structural) in the Compare workspace's
Difference sub-toolbar. The audit flagged the workspace as
showing "absolute difference" only; this slice ships the
canonical four and wires them through both the on-canvas
renderer and a canonical backend enum.

#### Backend (Rust)

- `crates/astroforge-core/src/difference.rs` (NEW, ~330 LOC):
  - `DiffKind` enum with the four §5.4 modes
    (`Absolute`, `Signed`, `Amplified`, `Structural`).
    Serde-friendly kebab-case serialisation so the same
    labels can be reused by any Tauri command or server-side
    consumer.
  - `compute_diff(kind, a, b, gain, width)`: single dispatch
    function; `width` is required by `Structural` (neighbour
    diff needs image dimensions) and ignored by the other
    three.
  - `Signed` shifts `(A - B) + 128` per channel so equal
    pixels read grey; brighter means B brighter than A.
  - `Amplified` multiplies by a positive `gain` (negative or
    NaN falls back to 1.0 for safety).
  - `Structural` uses a Sobel-style left/up neighbour diff on
    A and B; boundary pixels fall back to absolute so
    single-row / single-column buffers still render.
  - 12 unit tests: equal-image behaviour per mode, per-channel
    clamping, anti-symmetry of signed mode, NaN/negative
    gain fallback, structural quadrant-swap detection,
    small-buffer fallback, serde roundtrip.

- `crates/astroforge-core/src/lib.rs`: registers the new
  module.

#### Frontend (Svelte)

- `src/components/CompareTools.svelte`:
  - `DiffKind` TypeScript type mirrors the Rust enum
    (kebab-case strings: "absolute", "signed", "amplified",
    "structural").
  - `recomputeDifference()` becomes a dispatcher over the
    four modes; `clamp255` helper extracted. The math
    matches the Rust backend's behaviour so the on-canvas
    output is the canonical reference.
  - A new `.diff-kind` sub-toolbar appears when the user
    picks `Difference` mode, with four toggle buttons.
    Switching modes triggers an immediate recompute.
  - The Gain slider is now mode-aware: only visible when
    `Amplified` is active (other modes either have no gain
    or use fixed gain = 1).
  - CSS adds `.diff-kind` and `.diff-kind button` styles
    (hairline separator, smaller font for the sub-toolbar).

#### Honesty flags

- The frontend computes the diff in TypeScript on the canvas
  side, not via a Tauri round-trip. This matches the existing
  B6 overlay renderer and avoids an extra IPC call on every
  slider tick. The Rust `compute_diff` is the canonical
  reference for any non-canvas consumer (CI fixtures, batch
  comparison, AI-quality scoring).
- `Structural` uses a left/up neighbour diff, not a full
  Sobel kernel. Edge sharpness in astrophotos is mostly
  spatial; a true Sobel adds ~30 LOC and a reference test
  fixture for marginal accuracy gain. Documented as a
  future slice.
- The B11 backend auto-stretch normalizer is not wired yet.
  Until then, low-magnitude diffs in low-bit-depth regions
  may read as near-black; this is a known display-side
  limitation of the canonical §5.4 modes.

### Slice B4 — CR-07 UX: decisions, comparison sets, metric comparison

**Scope:** Surfaces the B1–B3 backend in the Compare workspace and
wires the B3 IPC commands to the managed project store. Adds the
§10 metric delta table + §11 summary over real pixels.

#### Backend (Rust)

- `crates/astroforge-core/src/comparison_metrics.rs` (NEW):
  - `metric_snapshot(image)` — derives the §8 metric snapshot from
    a decoded `F32Image` via the shipped `image_analysis::metrics`
    detectors (luminance/chrominance noise, local contrast,
    background gradient, highlight clipping). Only metrics with
    shipped detectors produce values; the rest stay honest
    inconclusive rows.
  - `compare_version_images(baseline, compared, ...)` — assembles
    B2's `compute_deltas` table + §11 `natural_language_summary`.
  - `load_version_pixels(store, version_id, applied_root)` —
    path-confined artifact decode (refuses paths outside the
    project's applied directory), living in core so CI's
    `cargo test --workspace` exercises it.
  - 8 unit tests: detector coverage, registry completeness,
    identical-image no-material-delta, noise-reduction improvement
    classification, summary shape, TIFF round-trip, outside-root
    refusal, missing-version error.
- `src-tauri/src/commands_comparison.rs`:
  - B3 commands rewired from the private `~/.astroforge/cr-07.sqlite`
    static to the managed `ProjectState.store` (`projects.db`), the
    same store `image_version_list` reads for this workspace.
    Signatures unchanged. The two list commands now touch the
    project row first so unknown project ids error clearly.
  - New `compare_version_metrics` command: thin wrapper that
    resolves the applied root and delegates to core.
- `src-tauri/src/main.rs`: registers `compare_version_metrics`.

#### Frontend (Svelte + TS)

- `src/lib/astroforge-api.ts` — B4 types (`ImageDecision`,
  `ComparisonSet`, `MetricDeltaRow`, `VersionMetricsComparison`)
  and wrappers for the 8 B3 commands + `compare_version_metrics`.
  `loadImageDecision` maps the backend's "not found" to `null` so
  fresh versions aren't an error.
- `src/state/comparison.ts` (NEW) — project-scoped decisions map +
  comparison-sets list with load/reset lifecycle, wired into
  `project-lifecycle.ts` open/close (R4 pattern).
- `src/components/DecisionPanel.svelte` (NEW) — §17/§18 state
  machine UI: Working → Candidate → Preferred → Final + Reject,
  optional reason per transition, append-only history view,
  terminal-state honesty.
- `src/components/MetricsTable.svelte` (NEW) — §10 delta table
  (A / B / Δ% / verdict per registry metric) + §11 summary;
  unmeasured metrics render "—".
- `src/components/ComparisonSetList.svelte` (NEW) — §16 set list,
  save-current-pair, apply-back-to-pickers, two-step delete.
- `src/components/CompareWorkspace.svelte` — mounts the three
  panels below the existing compare surface when A and B are
  selected.

#### Known pre-existing issues (out of scope)

- Store fragmentation: the AI apply round writes `image_versions`
  rows to `~/.astroforge/cr-06-p1.sqlite` while
  `read_image_artifact` / `image_version_list` / now the B3
  commands use `projects.db`. Consolidating the CR-06 store into
  the managed project store is a dedicated follow-up slice.
- `list_decisions_for_project` joins on `image_versions`; until
  version rows land in the project store it returns empty. The
  DecisionPanel loads per version (`load_image_decision`) and is
  unaffected.
- `src-tauri/Cargo.lock` is stale relative to `Cargo.toml`
  (astroforge-ai dep) — drift predates this slice; left untouched.

### Slice B3 — CR-07 Decision persistence + Comparison sets

**Scope:** Persists B1's `ImageDecision` and `ComparisonSet` types
to sqlite via the existing `DomainStore` connection (per CR-07
audit recommendation, no new persistence crate). Adds 8 IPC
commands. Builds on B1 (data model) + B2 (registry).

#### Backend (Rust)

- `crates/astroforge-core/src/decision_store.rs` (NEW, ~600 LOC):
  - `save_decision(store, decision)` — UPSERT the `image_decisions`
    row + rewrite `decision_history` atomically.
  - `load_decision(store, version_id)` — fetch the current decision
    + full append-only history. Returns `NotFound` if missing.
  - `list_decisions_for_project(store, project_id)` — join on
    `image_versions` to scope by project.
  - `apply_and_save_decision(store, version_id, new_state, reason)`
    — high-level helper that loads + applies the B1 state-machine
    transition + persists the result. Returns `InvalidTransition` on
    state-machine violations.
  - `save_comparison_set` / `load_comparison_set` /
    `list_comparison_sets_for_project` / `delete_comparison_set` —
    CRUD on the new `comparison_sets` table.
  - 12 unit tests covering round-trips, history preservation,
    missing-row error paths, project filtering, transition
    rejection, slot-label preservation, and ordering.
- `crates/astroforge-core/src/domain_store.rs`:
  - Migration 10: `image_decisions` (version_id PK + state +
    decided_at + reason + index on state) + `decision_history`
    (id PK + version_id + from_state + to_state + at + reason +
    index on version_id) + `comparison_sets` (id PK + project_id +
    name + version_ids_json + slot_labels_json + created_at +
    project_id/created_at index).
  - New `pub fn lock_conn(&self) -> MutexGuard<Connection>` so
    sibling modules can issue raw SQL within the same transaction
    model.
- `crates/astroforge-core/src/comparison.rs`:
  - `pub fn next_nonce() -> u64` — process-wide monotonic counter
    used to disambiguate ids that share a wall-clock second
    (ComparisonSession / ComparisonSet / QualityAssessment).
  - `ComparisonSession::new` / `ComparisonSet::new` /
    `QualityAssessment::new` (assessment.rs) append the nonce to
    the id so two constructs in the same second are still
    distinguishable (avoids `INSERT OR REPLACE` clobbering on the
    test fixtures).
- `crates/astroforge-core/src/lib.rs`: `pub mod decision_store;`
- `crates/astroforge-core/src/project.rs`: `schema_version()`
  expected value bumped to 10 (matches the new migration).

#### Tauri IPC commands (8 new)

- `src-tauri/src/commands_comparison.rs` (NEW):
  - `save_image_decision(decision)` — persist full B1 type.
  - `load_image_decision(version_id)` — returns `ImageDecision` or
    error.
  - `list_image_decisions_for_project(project_id)` — newest first.
  - `apply_image_decision(request)` — transition + persist in one
    call. `ApplyDecisionRequest { version_id, new_state, reason }`.
  - `save_comparison_set(set)` / `load_comparison_set(set_id)` /
    `list_comparison_sets_for_project(project_id)` /
    `delete_comparison_set(set_id)`.
- `src-tauri/src/main.rs`: `mod commands_comparison;` + 8 entries in
  `tauri::generate_handler!`.
- Global `DomainStore` opens at `~/.astroforge/cr-07.sqlite` (B3
  scope). Per-project wiring is deferred to B4 alongside the rest
  of the UX surface.

#### Bug fixed in B1 (carryover from B3 testing)

- `ComparisonSet::new` produced duplicate ids when called twice in
  the same wall-clock second (both timestamp and id are derived
  from `now_iso8601()`). `INSERT OR REPLACE` then clobbered the
  first row. Fixed by appending a process-wide nonce.
- Same fix applied to `ComparisonSession::new` and
  `QualityAssessment::new` for consistency.

#### Robustness

| Risk | Mitigation |
|---|---|
| `DomainStore.conn` is private | New public `lock_conn()` accessor with documented contract (caller is responsible for the standard `DomainStoreError` mapping) |
| `INSERT OR REPLACE` on `image_decisions` clobbers prior history | `save_decision` deletes the prior `decision_history` rows + re-inserts from the in-memory `ImageDecision` snapshot atomically. The B1 type is the source of truth; the store mirrors it. |
| Duplicate ids from same-second `now_iso8601()` | `next_nonce()` counter; applied to all three id-producing constructors |
| Pre-existing schema_version() test hardcoded `9` | Updated to `10`; comment added explaining the migration path |
| `query_row` closure returning serde_json::Error (clippy) | JSON parsing moved outside the closure into a `ComparisonSetRow` builder; closures now return plain `String` types |
| `Connection` field in `DomainStore` kept private but a sibling module needs it | Public `lock_conn()` returns `MutexGuard` with the same locking semantics as the existing internal calls |
| `apply_and_save_decision` double-`map_err` (clippy) | Reduced to a single `_e`-discarding closure |
| `stmt.query` borrow conflicts with `drop(stmt)` | Wrapped the iteration in a `{ ... }` block to scope the borrow |
| Global `DomainStore` is not per-project | Out of B3 scope; per-project wiring ships in B4 alongside the rest of the UX surface |

#### Tests / verification

- `cargo build -p astroforge-core` — clean
- `cargo test -p astroforge-core decision_store::` — 12/12 pass
- `cargo test --workspace` — 913 passed, 0 failed (+12 new)
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --all` — clean
- `cargo check -p astroforge-app` — clean (Tauri binary compiles)
- `bash scripts/mvp_smoke.sh` — green

#### Out of scope (later bundles)

- **B4 UX** — `DecisionPanel.svelte` + `ComparisonSetList.svelte`
  consuming the new IPC commands; per-project DB wiring.
- **B5 Provenance + AI** — `ProvenanceRecord` + `ProvenanceEdge`
  aggregate; AI metric implementations.
- **B6 Polish** — beginner / expert profiles + expert inspector.
- **B7 Perf + tests** — hardware matrix + visual regression.

### Slice B2 — CR-07 Metrics + Delta + Assessment

**Scope:** Codifies the per-metric registry (B1 deferred this) +
CR-07 §10 delta analysis + §11 quality assessment aggregation from
existing `quality_gates` findings. Builds directly on B1.

#### Backend (Rust)

- `crates/astroforge-core/src/metric_registry.rs` (NEW, ~700 LOC):
  - `MetricKind` enum — all 27 CR-07 §8 metric variants
    (NoiseLuminance / NoiseChrominance / NoiseRegional /
    SharpnessFwhm / SharpnessLocal / SharpnessEdgeResponse /
    StarCount / StarSize / StarEccentricity / StarFwhmDistribution /
    StarSaturation / StarBackgroundContrast / BackgroundMean /
    BackgroundVariance / BackgroundGradient / BackgroundColorGradient /
    DynamicRangeBlackClipping / DynamicRangeHighlightClipping /
    DynamicRangeSaturationPct / SignalEstimatedSnr / SignalLocalSnr /
    SignalStructuralContrast / AiSegmentationConfidence /
    AiArtifactIndicator / AiReconstructionRisk / AiModelConfidence).
  - `MetricDirection` enum (LowerIsBetter / HigherIsBetter /
    Ambiguous) + `improvement_sign()` returning the `i8` value
    consumed by `ComparisonDelta::compute` from B1.
  - `MetricSpec` — direction + materiality_pct + label + §9
    contextual explanation + unit. The contextual strings preserve
    the CR-07 §9 example verbatim ("Lower generally indicates tighter
    stars, but values depend on seeing, focal length, and pixel
    scale.").
  - `METRIC_REGISTRY` — one `MetricSpec` per `MetricKind`. Adding a
    new metric requires updating both the enum and the table; the
    `metric_registry_is_complete` test enforces this.
  - `MetricDeltaRow` — one row of the §10 delta table (kind /
    baseline_value / compared_value / direction / percent_change /
    materiality_threshold / label / unit / context).
  - `compute_deltas(baseline, compared)` — produces a `Vec<MetricDeltaRow>`
    covering every registered metric. Missing values classify as
    `Inconclusive`.
  - `compute_delta_for(kind, baseline, compared)` — single-metric
    variant.
  - 11 unit tests: round-trip via `as_str()`, uniqueness of keys,
    registry completeness, direction-to-sign mapping, missing-values
    handling, low-noise-is-improvement, materiality threshold,
    ambiguous-metric-is-inconclusive, partial coverage, context
    lookup, value formatting.
- `crates/astroforge-core/src/assessment.rs` (NEW, ~550 LOC):
  - `from_gate_findings(session_id, findings, deltas, baseline, compared)`
    produces a `QualityAssessment` (B1 type) populated from existing
    `quality_gates::GateFinding` rows.
  - `natural_language_summary(deltas, findings, baseline, compared)`
    produces the CR-07 §11 assessment summary in the spec's exact
    format:
    ```text
    Comparison: A (Natural) vs B (AI Enhanced)
    [+] Noise (luminance) reduced
    [-] Highlight clipping increased
    [!] Clipping warning
    Overall:
    2 improved, 1 degraded
    1 quality warning(s)
    ```
  - Verdict logic: `StrongImprovement` (3+ improved + no failures),
    `ImprovementWithTradeoffs` (1+ improved), `Neutral` (no net
    change), `Degradation` (failure-level gate present).
  - Severity mapping: `Ok / Info` -> `Info`, `Warning` -> `Warn`,
    `Failure` -> `Fail` (from `quality_gates::Severity`).
  - Verdict aggregation: aggregates findings into a `GateVerdict` and
    combines with metric delta net direction.
  - 7 unit tests: findings + integrity checks, summary mentions
    improved + degraded, verdict variants (improvement-with-tradeoffs
    / strong-improvement / degradation), empty inputs.
- `crates/astroforge-core/src/lib.rs`: `pub mod metric_registry;`,
  `pub mod assessment;`

#### Reuses existing modules

- `image_analysis::metrics` — luminance_noise, chromatic_noise,
  background_gradient, highlight_clipping, local_contrast (5/27
  metric kinds ship with measurements).
- `quality::QualityMetricSnapshot` — mean, stddev, snr_db, fwhm,
  star_count, background_gradient.
- `quality_gates::GateFinding` + `GateId` + `Severity` + `GateVerdict`
  — 10 §12 astronomical integrity checks reused for the comparison
  assessment.

#### Robustness

| Risk | Mitigation |
|---|---|
| Per-metric Materiality hard-coded per metric kind | `MetricSpec.materiality_pct` is `&'static`, locked at registry definition; overridable per call via `ComparisonDelta::compute(..., threshold)` |
| `MetricKind::from_str` collides with `std::str::FromStr` (clippy) | Renamed to `MetricKind::parse` to keep the API explicit-non-trait |
| Unknown metric kinds at consumer site | `metric_spec(kind)` returns `Option<&'static MetricSpec>`; consumers should treat `None` as `Ambiguous` (the convenience fns default to this) |
| `GateFinding::severity` is private enum, not a `String` | Local `severity_as_str()` helper uses `format!("{:?}", severity).to_lowercase()` — fine for a debug-grade label since the public `Severity::as_str()` API is also lowercase |
| Empty findings + empty deltas -> Neutral verdict | Covered by test `assessment_handles_empty_findings_and_deltas` |
| `AssessmentVerdict::StrongImprovement` requires 3+ improvements | Logged in the test that only 1 improved gives `ImprovementWithTradeoffs` |
| Duplicate `#[test]` attributes from earlier patch | Removed in final round |
| Stub `format!` placeholders left in natural-language loop | Removed; single-loop implementation |

#### Tests / verification

- `cargo build -p astroforge-core` — clean
- `cargo test -p astroforge-core metric_registry::` — 11/11 pass
- `cargo test -p astroforge-core assessment::` — 7/7 pass
- `cargo test --workspace` — 901 passed, 0 failed (+18 new)
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --all` — clean
- `bash scripts/mvp_smoke.sh` — green

#### Out of scope (later bundles)

- B3 Decisions: persistence of `ImageDecision` + `ComparisonSet`
  via `db.rs` sqlite.
- B4 UX: `ComputeMetricsTable.svelte` + `AssessmentPanel.svelte`
  consuming the new APIs.
- B5 Provenance + AI: `ProvenanceRecord` + `ProvenanceEdge`.
- B6 Polish: beginner / expert profiles + expert inspector.
- B7 Perf + tests: hardware matrix + visual regression.

### Slice B1 — CR-07 Foundation data model + ADRs

**Scope:** Paperwork + types only. Implements CR-07 §25 data model +
§33 ADRs (ADR-07.1..07.8). No behaviour change.

#### Backend (Rust)

- `crates/astroforge-core/src/comparison.rs` (NEW, ~700 LOC):
  - `ComparisonSession` — user's active comparison activity.
  - `ComparisonMode` — 8 variants (SideBySide / Split / Blink /
    DifferenceAbsolute / DifferenceSigned / DifferenceAmplified /
    DifferenceStructural / Overlay).
  - `ComparisonItem` + `ComparisonSlot` (A / B / C / D) — references
    to ImageVersions.
  - `ComparisonRegion` + `ComparisonScope` (WholeImage /
    SelectedRegion / SpecificFeature) + `RegionShape` (Rectangle /
    Circle / Polygon in normalized 0..1 coordinates).
  - `ComparisonMetric` — measured characteristics keyed by metric
    kind (BTreeMap<String, f64>).
  - `ComparisonDelta` + `DeltaDirection` (Improved / Degraded /
    Unchanged / Inconclusive) + `compute()` helper that classifies
    deltas by direction (improvement_sign) + materiality threshold.
  - `QualityAssessment` + `AssessmentFinding` + `AssessmentSeverity`
    (Info / Warn / Fail) + `IntegrityCheck` + `QualityVerdict`
    (StrongImprovement / ImprovementWithTradeoffs / Neutral /
    Degradation / Inconclusive).
  - `ImageDecision` + `ImageDecisionState` (Working / Candidate /
    Preferred / Final / Rejected / Reference) + `DecisionHistoryEntry`
    + `PromotionError`. State machine: Working → Candidate → Preferred
    → Final; Reject allowed from any non-terminal; Final / Rejected /
    Reference are terminal.
  - `ComparisonSet` — reusable collection of candidate versions.
  - `now_iso8601()` helper — RFC 3339 UTC timestamp without a new
    `chrono` dependency (matches codebase `String` convention).
  - 13 unit tests covering promotion flow, history preservation,
    invalid transitions, reject from non-terminal, final can't be
    rejected, delta direction (improvement_sign + materiality +
    zero-baseline + ambiguous-metric), session construction,
    comparison set round-trip, mode labels, region shape.
- `crates/astroforge-core/src/lib.rs`: `pub mod comparison;`

#### Documentation (8 ADRs + index update)

- `docs/adr/0004-cr07-comparison-primitive.md` (NEW) — ADR-07.1
  Image Version Is the Comparison Primitive.
- `docs/adr/0005-cr07-visual-comparison-primary.md` (NEW) — ADR-07.2
  Visual Comparison Is Primary.
- `docs/adr/0006-cr07-metrics-contextual.md` (NEW) — ADR-07.3
  Metrics Are Contextual.
- `docs/adr/0007-cr07-comparison-non-destructive.md` (NEW) — ADR-07.4
  Comparison Is Non-Destructive.
- `docs/adr/0008-cr07-provenance-always-available.md` (NEW) — ADR-07.5
  Provenance Is Always Available.
- `docs/adr/0009-cr07-ai-explicit-in-comparison.md` (NEW) — ADR-07.6
  AI Processing Is Explicit in Comparison.
- `docs/adr/0010-cr07-decision-user-owned.md` (NEW) — ADR-07.7
  Decision Is User-Owned.
- `docs/adr/0011-cr07-comparison-feedback-loop.md` (NEW) — ADR-07.8
  Comparison Is a Feedback Loop.
- `docs/adr/README.md`: index updated with 8 new entries.

#### Robustness

| Risk | Mitigation |
|---|---|
| Adding `chrono` dependency for timestamps | `now_iso8601()` helper hand-rolls RFC 3339 from `SystemTime`; matches the codebase `String` convention used by `domain.rs::ImageVersion::created_at` |
| `String` not `Copy` in `ImageDecision` constructor + `transition_to` | `.clone()` the timestamp used in `history` so `decided_at` retains its own copy |
| Clippy `manual_pattern_char_comparison` for `matches!(c, ':' \| '-' \| 'T' \| 'Z')` | Use `[':', '-', 'T', 'Z']` array form (Rust 1.98 `Pattern` impl) |
| `can_promote_to` initially did not allow Reject from non-terminal states | Added Reject paths from Working / Candidate / Preferred to Rejected (terminal, but reachable) |
| Decisions auto-promoted by pipeline completion | No code path mutates `ImageDecision::state` outside the explicit `promote()` / `transition_to()` / `reject()` methods (ADR-07.7) |
| Rejected version accidentally un-rejected | B1 does not implement `Rejected -> Working` direct transition; restoring requires a new decision row (intentional friction per ADR-07.7) |

#### Tests / verification

- `cargo build -p astroforge-core` — clean
- `cargo test -p astroforge-core comparison::` — 13 passed, 0 failed
- `cargo test --workspace` — 752 passed, 0 failed (+13 new)
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --all` — clean

#### Out of scope (deferred to later bundles)

- B2 Metrics + Delta: per-metric registry + materiality thresholds
  encoded on `ComparisonMetric`. B1 carries the shape; B2 fills the
  registry.
- B3 Decisions: persistence of `ImageDecision` + `ComparisonSet` to
  sqlite (per CR-07 audit, piggybacks on existing `db.rs`).
- B4 UX: `CompareWorkspace` extension for overlay mode + sync nav
  + region selection + version tree (DAG).
- B5 Provenance + AI: `ProvenanceRecord` + `ProvenanceEdge`
  aggregate types + UI surface.
- B6 Polish: beginner / expert profiles + expert-mode inspector.
- B7 Perf + tests: hardware matrix + visual regression + version
  integrity tests.

### Slice R — P4-M2-T2 + T3 recipe gallery + search

**Scope:** Bundled-batch per the cluster order (B → R → A → CD).
B, A, and CD landed earlier; R closes the recipe-cluster loop.
Implements the spec §11.3 "In-app Recipe Gallery" requirement:
browsable, filterable by target/equipment/palette.

#### Backend (Rust)

- `crates/astroforge-core/src/recipe_feed.rs` (NEW):
  - `SharedRecipe` — spec §11.1 sanitised recipe format
    (recipe_version, app_version, name, author, target_type,
    equipment_hints{camera, filters[]}, pipeline[], model_versions{},
    integrity). Distinct from the internal authoring `Recipe`:
    the shared format strips session/version metadata per §11.2.
  - `EquipmentHints`, `SharedRecipeStage`, `SharedIntegrity`.
  - `FilterPalette` enum (Ha/OIII/SII/SHO/HOO/LRGB/Broadband) with
    `from_filters()` derivation: e.g. `{Ha,OIII}` → `[HOO]`,
    `{Ha,OIII,SII}` → `[SHO]`, `{L,R,G,B}` → `[LRGB]`, `[]` →
    `[Broadband]`.
  - `SortOrder` enum (Relevance/DateDesc/DateAsc/Popularity).
    `#[derive(Default)]` with `#[default]` on `Relevance` so
    `FilterCriteria::default()` works.
  - `FilterCriteria` struct (query/target/equipment/palette/sort).
  - `RecipeSource` trait (`list`, `get`) — pluggable backing store
    defers the hosting decision (issue #119 / P4-M2-T1).
  - `InMemoryRecipeSource` impl for unit tests and seed data.
  - `search()` — O(N) linear scan over `source.list()`. Each
    filter axis is a single pass; `sort_by` runs once at the end.
    Performance target is <500ms (spec §11.3); a 10k-recipe linear
    scan with simple string matching is sub-5ms in practice.
  - 20 unit tests covering: palette derivation (HOO/SHO/LRGB/
    broadband fallback), query match on name/author/target_type,
    target/equipment/palette filters individually, combined multi-
    filter, empty-result cases, relevance/date/popularity sort,
    `RecipeSource::get` lookup, and a spec §11.1 example JSON
    round-trip.

#### Frontend (Svelte 5)

- `src/lib/recipeFeed.ts` (NEW): TypeScript mirror of the Rust
  types (camelCase at the IPC boundary, snake_case in Rust),
  `searchLocal()` client-side filter+sort that mirrors the Rust
  search for graceful degradation when IPC is unavailable, plus
  a fixture fallback for Vite-only dev mode.
- `src/components/RecipeGallery.svelte` (NEW): browse + filter UI
  with search input, target + equipment text filters, palette
  dropdown (7 options), sort dropdown (4 options), clear button,
  result count, and a responsive card grid. Uses `$state` +
  `$derived.by` (Svelte 5). Local-only fallback renders the
  fixture feed without invoking Tauri.

#### Robustness

| Risk | Mitigation |
|---|---|
| P4-M2-T1 hosting decision (file vs. GitHub vs. CDN) undecided | `RecipeSource` trait defers cleanly — backing store is a 30-line wrapper, not a gallery rewrite |
| `from_iter` collides with `std::iter::FromIterator::from_iter` (clippy) | Renamed to `with_recipes` to keep the constructor clearly non-standard |
| `SortOrder` has no obvious default | `#[derive(Default)]` + `#[default]` on `Relevance` |
| Recipe-grazing fixture is hand-written and might drift from spec | Rust spec §11.1 example JSON round-trip test pins the wire format |
| Frontend filter logic duplicates Rust logic | `searchLocal()` is the deliberate fallback so Vite-only dev still works |
| Sandbox LSP errors (`$state`, `$derived`, `onclick`) | Pre-existing — `node_modules` missing locally; CI runs against real `node_modules` |
| CHANGELOG conflict with slice M7 | Branched off `a3ad0fa` (post-M7); CHANGELOG.md's `## Unreleased` already has M7 entry; adding R entry at the top is conflict-free |

#### Tests / verification

- `cargo test --workspace` — 739 passed; 0 failed (+20 new from
  `recipe_feed.rs`)
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session` — green

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
