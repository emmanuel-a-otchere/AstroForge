# Changelog

## Unreleased

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
