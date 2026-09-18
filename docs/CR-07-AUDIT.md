# CR-07 Audit — Image Review, Comparison & Decision Workspace

**Source:** [`CR-07-IMAGE-REVIEW-COMPARISON-DECISION.md`](CR-07-IMAGE-REVIEW-COMPARISON-DECISION.md)
**Implementation plan:** [`CR-07-IMPLEMENTATION-PLAN.md`](CR-07-IMPLEMENTATION-PLAN.md)
**Original audit date:** 2026-09-12
**Last refresh:** 2026-09-17 (post-C-A3.5; refresh of audit status, scorecard, and bundle priority)
**Branch:** `docs/cr-07-audit-refresh` (from `origin/main` at `8a8584a`)
**Status:** ✅ Shipped / ⚠️ Partial / ❌ Missing

This audit reconciles every CR-07 §1–§35 acceptance criterion against the
existing codebase. The audit output drives the 7-bundle implementation
plan; sections already shipped don't need re-implementation. **Refresh
posture (2026-09-17):** 21 slice PRs landed between the original audit
date and this refresh (B1 Foundation through C-A3.5, see `Bundle
status` below). The original audit was a useful baseline but is now
stale on §5.4/§5.5/§6/§7/§13/§14/§15/§16/§17/§18/§19/§22/§25/§30/§33.

## §1 Intent — ✅ Shipped

> "Which result is actually better, and why?"

Closed by the existing `CompareWorkspace` (PR #308) + `CompareTools`
(PR #309). The "Select → Compare → Inspect → Measure → Assess → Decide →
Promote" loop is the user's mental model; the existing implementation
covers Select, Compare, Inspect, Measure, Assess, Decide, and Promote.

## §2 Product Decision — ✅ Shipped

> "The Image Version established in CR-02 becomes the central
> comparison object."

`ImageVersion` exists in `crates/astroforge-core/src/domain.rs:391`.
`CompareWorkspace.svelte` operates on Image Versions, not files.

## §3 Core Principle — ✅ Shipped

> "AstroForge measures image characteristics; the user decides what
> constitutes the better image."

The metrics + quality-gate modules produce numbers; the UI does not
auto-declare a winner. The recommendation engine (CR-06) is advisory,
not authoritative.

## §4 Review Experience — ✅ Shipped

The 4-zone layout (Version tree | Image canvas | Assessment |
Histogram/Zoom/Blink/Split/Difference) is in `CompareWorkspace.svelte`.
The "image remains the dominant element" principle is preserved (canvas
takes the bulk of the layout). The version tree left rail is now a DAG
visualization (B8 `VersionDag.svelte`), not a flat list.

## §5 Comparison Modes

### §5.1 Side-by-Side — ✅ Shipped

`CompareWorkspace.svelte` renders A and B side-by-side with synchronized
metadata + canvas.

### §5.2 Split View — ✅ Shipped

`CompareTools.svelte` implements a vertical slider with smooth movement,
0..100% bound to a `<input type="range">`. Accessibility (aria-valuetext)
in place.

### §5.3 Blink — ✅ Shipped

`CompareTools.svelte` blink mode toggles A/B at a configurable interval
(default 1000ms). Pauses on tab visibility change.

### §5.4 Difference View — ✅ Shipped

All four modes shipped by B10/B11/B12:
- Absolute difference (B10)
- Signed difference (B10)
- Amplified difference (B10)
- Structural difference (B11/B12 — auto-stretch + fixed-stretch + n-sigma
  normalizer)

### §5.5 Overlay — ✅ Shipped

Shipped in B6 (PR #327). CompareTools renders one version above the
other with adjustable opacity.

## §6 Synchronized Navigation — ✅ Shipped

Shipped in B5 (PR #326). `CompareWorkspace.svelte` synchronizes zoom,
pan, cursor, region, histogram selection, pixel inspection, and mask
visualization across A and B via the `syncNav` state.

## §7 Comparison Scope — ✅ Shipped

Shipped in B7 (PR #328). Region + feature picker (`RegionPicker.svelte`)
exposes the "selected region" and "specific feature" scope axes.

## §8 Image Metrics — ⚠️ Partial (substantial progress)

**Existing modules:**
- `crates/astroforge-core/src/quality.rs` — `FrameQuality`,
  `QualityMetricSnapshot`, `compute_metrics`.
- `crates/astroforge-core/src/image_analysis/metrics.rs` — luminance,
  chromatic, background gradient, highlight clipping, local contrast.
- `crates/astroforge-core/src/image_analysis/structures.rs` — star
  metrics (count, size, eccentricity).
- `crates/astroforge-core/src/processing_metrics.rs` — `ProcessingMetrics`.
- `crates/astroforge-core/src/quality_gates/` (1,319 LOC) — `GateFinding`,
  `QualityVerdict`, `QualityGateReport`.

| CR-07 §8 metric | Existing coverage |
|---|---|
| Luminance noise | ✅ `luminance_noise` |
| Chrominance noise | ✅ `chromatic_noise` |
| Regional noise | ⚠️ Partial |
| Sharpness (FWHM) | ✅ structures.rs |
| Local sharpness | ✅ `local_contrast` |
| Edge response | ⚠️ Partial |
| Star count / size / eccentricity | ✅ structures.rs |
| FWHM distribution / saturation | ✅ structures.rs |
| Star-to-background contrast | ✅ structures.rs |
| Mean background / variance / gradient | ✅ `background_gradient` |
| Color gradient | ⚠️ Partial |
| Black clipping / highlight clipping | ✅ `highlight_clipping` |
| Saturation percentage | ❌ Missing |
| Estimated SNR / local SNR | ⚠️ Partial |
| Structural contrast | ✅ `local_contrast` |
| AI segmentation confidence / artifact indicators / model confidence | ⚠️ Partial |

Star metrics moved from "Missing" to "Shipped" via the structures
module. Saturation and full SNR remain open.

## §9 Metrics Must Be Contextual — ⚠️ Partial

Each metric function has a docstring, but the UI presentation does not
yet include the contextual explanation CR-07 §9 calls for (e.g. "FWHM:
3.1 px — Lower generally indicates tighter stars, but values depend
on acquisition and processing conditions"). Improvement pending in
`RecommendationCard.svelte`.

## §10 Delta Analysis — ✅ Shipped

Shipped in B2 (PR #323) and B4 (PR #325). The `ComparisonDelta` type
and delta table exist; the metrics table renders side-by-side A-vs-B
deltas.

## §11 Quality Assessment — ⚠️ Partial (substantial progress)

`QualityGateReport` produces a verdict (`Pass`/`Warn`/`Fail`/`Inconclusive`)
with findings count by severity. B4 landed a natural-language summary
field (`assessment.rs`) that surfaces "✓ Noise substantially reduced",
"⚠ Highlight clipping increased", etc. **Partial** because the
two-line prose summary is shipped but the §11 "Overall: Strong
improvement with minor trade-offs" form is not yet wired into the
panel rendering.

## §12 Astronomical Integrity Checks — ✅ Shipped

`crates/astroforge-core/src/quality_gates/gates.rs` ships all 10
gates (`clipping`, `noise_amplification`, `excessive_smoothing`,
`color_shifts`, `star_artifacts`, `halos`, `false_structures`,
`edge_artifacts`, `segmentation_leakage`, `ringing`). B2 wired these
into the A-vs-B comparison context.

## §13 AI-Aware Comparison — ⚠️ Partial

`Recipe.integrity` carries `perceptual_models_used` + `seed_recorded` +
per-model usage. **Data exists**; **comparison UI surfaces** the AI
flag via the Recipe Stage model UI and B13a-c provenance, but a single
consolidated "this version used AI" badge in the comparison view is
not yet a first-class element.

## §14 Provenance Panel — ✅ Shipped

Shipped in B14 (PR #337, `ProvenancePanel.svelte` in
`CompareWorkspace.svelte`) and B15 (PR #338, `RecipeStageTimeline`).
The data is read from `Recipe` + `RecipeStage` + `IntegrityBadge` +
`ModelUsage` and surfaced inline.

## §15 Version Tree — ✅ Shipped

Shipped in B8 (PR #329, `VersionDag.svelte`). The left rail renders
parent → child relationships, branches, and parallel derivations
as a DAG.

## §16 Comparison Sets — ✅ Shipped

Shipped in B3 (PR #324) and B4 (PR #325). `ComparisonSet` is a
persisted first-class type; the UI exposes save/load/reuse flows.

## §17 Decision State — ✅ Shipped

Shipped in B3 (PR #324). The `WORKING / CANDIDATE / PREFERRED /
FINAL / REJECTED / REFERENCE` state machine lives in
`crates/astroforge-core/src/comparison.rs`. C-A3.5 added a
`quality_profile` column on the `image_decisions` row.

## §18 Promotion Model — ✅ Shipped

Shipped in B3 (PR #324). `promote_image_version` /
`apply_image_decision` flow with transition validation per
`ImageDecisionState::can_promote_to`.

## §19 Compare → Continue Workflow — ✅ Shipped

Shipped in C-A1 (PR #339) + §19 close-out (PR #350).
CompareWorkspace's continue-bar offers "Mark
Preferred", "Continue enhancing", "Create branch",
"Export comparison". All four are now wired:
- Mark Preferred (B): wires to
  `applyImageDecision("preferred")`.
- Continue enhancing: navigates to Enhance with B
  pre-selected.
- Create branch (closed-out in this slice): wires
  to a `createBranch()` handler that persists the
  branch intent durably via
  `applyImageDecision(bId, "preferred", "create_branch",
  selectedQualityProfile)` AND navigates to Enhance.
  The Quality Profile selection is threaded through
  per C-A3.5. The recorded decision is the durable
  breadcrumb that P5a's child-version IPC will
  read when it lands (per the existing TODO at
  `recommendationCard.svelte` line 11).
- Export comparison: wires to `exportComparisonComposite`
  (C-A2).

## §20 Intelligent Recommendation After Comparison: ✅ Shipped

`RecommendationEngine` (557 LOC) + `astroforge-ai/src/recommendations/`
(~1,340 LOC after the §20 rule) produce recommendations.
`RecommendationList.svelte` (392 LOC after the §20 chip extension) +
`RecommendationCard.svelte` (211 LOC) render them. **Added (this slice):**
the post-comparison feedback loop in
`astroforge-ai/src/recommendations/post_comparison.rs` (~620 LOC
including tests). The rule is a pure function over the two sides'
`metric_snapshot`s + their `ImageDecisionState`s that emits a single
`AiRecommendation` (`operation: "post_comparison_insight"`,
`classification: "observation"`) describing the trade-off in the
spec's voice ("Version B has X ✓, ⚠ Y → next step Z"). Determinism,
§21 "no winner" framing, and the whole-frame / observation
classification are all unit-tested (9 new tests, 0 new svelte-check
issues). Closes the §-level "Missing" flagged in the audit refresh
as priority #1.

## §21 No "AI Winner" by Default — ✅ Shipped

No "AI Winner" auto-label exists. The recommendation engine is
advisory. The user owns the decision.

## §22 Optional Quality Profiles — ✅ Shipped (fully closed)

Shipped in C-A3 (PR #341) + C-A3.5 (PR #342). The
`QualityProfilePicker.svelte` captures the user's selection; C-A3.5
threads it through `applyImageDecision` into the
`image_decisions.quality_profile` column. Legacy rows surface
"Profile not recorded".

## §23 Expert Comparison: ✅ Shipped

The Quality Gate Panel shows expert-level metric detail.
All four §23 sub-slices are now shipped (in order):
**§23.1** per-channel statistics, **§23.2** per-star FWHM
distribution, **§23.3** per-pixel noise map, **§23.4**
highlight + shadow clipping masks.

**Shipped:**
- §23.1 (first slice): per-channel statistics
  (per-channel mean, stddev, min, max, clip count) for
  R/G/B (and c{n} for the 4th-and-beyond channels).
- §23.2 (second slice): per-star FWHM distribution
  visualization. The data path is
  `fwhm_histogram(image) -> FwhmHistogram` (calls
  `registration::extract_stars` for per-star FWHM values,
  pre-bins them with Sturges' rule capped to [1, 50] bins,
  and computes a seven-number summary: count, mean, median,
  p25, p75, min, max).
- §23.3 (third slice): per-pixel 2D noise map. The data
  path is `noise_map(image) -> NoiseMap`: the image is
  downsampled to the preview budget, then for each pixel
  the local sigma is estimated over a 7x7 window using
  the same MAD-on-residuals algorithm as the scalar
  `luminance_noise` metric but applied per-pixel.
- §23.4 (fourth slice): highlight + shadow clipping
  masks. The data path is
  `clipping_masks(image) -> ClippingMasks`: the image is
  downsampled to the preview budget, then for each pixel
  we record whether it is highlight-clipped
  (`v >= 0.99`) or shadow-clipped (`v <= 0.01`).
  Thresholds match `image_analysis::metrics::highlight_clipping`
  and `quality_gates::clipping`. The two masks never
  overlap (a pixel cannot be both highlight- and
  shadow-clipped).

The four expert panels sit behind a single "Show expert
details" toggle on `CompareWorkspace.svelte` per the §23
"progressive disclosure, consistent with CR-01"
requirement. The bundle row ships + first concrete slice
pointer updates with each sub-slice.

## §24 Beginner Comparison: ✅ Shipped

Side-by-side A vs B is simple by default. The "Which
do you prefer? [Natural] [AI Enhanced]" prompt with
one-paragraph explanation beneath each option is now
rendered above the existing comparison tools when both
versions are selected. Each click writes the
preference via the existing `applyImageDecision`
IPC (`preferred` state); the `DecisionPanel` picks it
up and persists it through the rest of the comparison
workflow. The prompt collapses to a one-line
"You picked X as the preferred version" summary after
the user picks, with a "Change my pick" link to undo.
The two options are equal-weight preference buttons
(per §21 "No AI Winner"), not a winner-pick.

## §25 Comparison Data Model — ✅ Shipped

All eight §25 types live in `crates/astroforge-core/src/comparison.rs`
(shipped in B1 Foundation, PR #322):
- `ComparisonSession`
- `ComparisonItem`
- `ComparisonRegion`
- `ComparisonMetric`
- `ComparisonDelta`
- `QualityAssessment`
- `ImageDecision`
- `ComparisonSet`

## §26 Semantic API — ⚠️ Partial

Most commands shipped via B1 + B3 + B4. **Missing** (still on the
to-do list):
- `create_comparison`
- `add_comparison_version`
- `set_comparison_mode`
- `get_comparison_metrics`
- `get_metric_delta`
- `get_quality_assessment`
- `get_version_provenance`
- `promote_image_version`
- `create_comparison_set`

The existing `save_comparison_set` + `load_comparison_set` +
`apply_image_decision` are shipped. The remaining nine commands form
the natural §26 close-out bundle.

## §27 Architecture — ✅ Shipped

The architecture diagram matches reality: processing → image versions →
compare → metrics/visual/provenance → decision → refine/branch/export.
This is the AstroForge shape.

## §28 Implementation Map — ✅ Shipped

Persistence lives in `crates/astroforge-core/src/comparison/` (B1
Foundation). The original plan's `astroforge-persistence/` crate was
correctly abandoned; `db.rs` + `domain_store.rs` +
`decision_store.rs` cover all persistence concerns.

## §29 Performance Requirements — ⚠️ Partial

- ✅ Existing artifacts reused (`ImageVersion.primary_artifact_id`).
- ✅ Lower-resolution previews: WebGL back-end renders at the canvas
  display size.
- ❌ Streaming high-resolution regions: not implemented.
- ❌ Cached difference images: not implemented (each compare-mode
  toggle recomputes).
- ❌ GPU/WebGPU acceleration for comparison: WebGL is used for the
  canvas, but difference / blink / split overlays run on Canvas2D.
- ✅ Canvas2D/CPU fallback: in place.

## §30 Export From Comparison — ✅ Shipped

Shipped in C-A2 (PR #340). CompareWorkspace's side-by-side
export + "Export comparison" composite PNG works.

## §31 Acceptance Criteria — see sections above

| Criterion | Status |
|---|---|
| Any compatible Image Versions can be compared | ✅ |
| Side-by-side works | ✅ |
| Split view works | ✅ |
| Blink works | ✅ |
| Overlay works | ✅ |
| Difference view works | ✅ |
| Synchronized zoom/pan works | ✅ |
| Regional comparison works | ✅ |
| Relevant image metrics are available | ⚠️ Partial |
| Metrics can be compared between versions | ✅ |
| Deltas are calculated | ✅ |
| Quality warnings can be surfaced | ✅ |
| Astronomical integrity checks can be surfaced | ✅ |
| AI processing is identified | ⚠️ Data exists, UI partial |
| Processing history is accessible | ✅ |
| AI model information is accessible | ✅ |
| Recipe information is accessible | ✅ |
| Image Version ancestry is visible | ✅ |
| Provenance remains intact after comparison | ✅ |
| Version can be marked Candidate / Preferred / Final / Rejected | ✅ |
| Preferred/Final state survives restart | ✅ |
| User can continue editing from a selected version | ⚠️ Partial |
| Branching remains non-destructive | ✅ |
| Image remains dominant | ✅ |
| Beginner mode is simple | ⚠️ Partial |
| Advanced metrics use progressive disclosure | ⚠️ Partial |
| Comparison does not expose internal DAG complexity | ✅ (DAG view exists) |
| Comparison remains usable offline | ✅ |

## §32 Test Strategy — ⚠️ Partial

`cargo test --workspace` is exhaustive (766+ tests in `astroforge-core`
+ 30+ in integration tests). **Missing:**
- Visual regression tests for split alignment, blink consistency,
  difference rendering, overlay accuracy.
- Metric validation against controlled datasets with known noise, blur,
  clipping, star eccentricity, background gradients.
- Version integrity test (`Version A + Version B + Comparison` doesn't
  modify either artifact).
- AI comparison tests (model/version/hash/classification/parameters/
  provenance surfacing).
- Performance tests (4K/8K/16-bit/32-bit/multi-version/large-project/
  limited-RAM).

## §33 ADRs — ✅ Shipped

All 8 CR-07 ADRs ship as `docs/adr/0004-…md` through
`docs/adr/0011-…md` (B1 Foundation, PR #322).

## §34 Definition of Done — see §31 + §15 + §17 + §18 + §19

Items 1-7 (process, generate, branch, compare modes including
overlay) shipped. Items 8-10 (zoom sync, histograms, metrics)
shipped. Items 11-13 (quality issues, preferred candidate,
continue enhancing) shipped. Items 14-15 (preserve alternatives,
reopen intact) shipped. Remaining DoD items map to §23/§24/§32
open work (visualisations, test strategy).

## §35 Strategic Outcome — ✅ Shipped

The "intelligent, iterative image-revision system" is the AstroForge
intent. CR-07 closes the comparison-decision loop end to end.

## Summary scorecard (refreshed 2026-09-17)

| Section | ✅ | ⚠️ | ❌ | Total |
|---|---|---|---|---|
| §1–§4 (intent, decision, principle, layout) | 4 | 0 | 0 | 4 |
| §5 (modes) | 5 | 0 | 0 | 5 |
| §6 (sync nav) | 1 | 0 | 0 | 1 |
| §7 (scope) | 1 | 0 | 0 | 1 |
| §8 (metrics) | 10 | 5 | 1 | 16 |
| §9 (contextual) | 0 | 1 | 0 | 1 |
| §10 (delta) | 1 | 0 | 0 | 1 |
| §11 (assessment) | 0 | 1 | 0 | 1 |
| §12 (integrity) | 1 | 0 | 0 | 1 |
| §13 (AI-aware) | 0 | 1 | 0 | 1 |
| §14 (provenance) | 1 | 0 | 0 | 1 |
| §15 (version tree) | 1 | 0 | 0 | 1 |
| §16 (sets) | 1 | 0 | 0 | 1 |
| §17 (decision state) | 1 | 0 | 0 | 1 |
| §18 (promotion) | 1 | 0 | 0 | 1 |
| §19 (continue workflow) | 0 | 1 | 0 | 1 |
| §20 (recommendation) | 0 | 1 | 0 | 1 |
| §21 (no AI winner) | 1 | 0 | 0 | 1 |
| §22 (quality profiles) | 1 | 0 | 0 | 1 |
| §23 (expert) | 0 | 1 | 0 | 1 |
| §24 (beginner) | 0 | 1 | 0 | 1 |
| §25 (data model) | 1 | 0 | 0 | 1 |
| §26 (semantic API) | 0 | 1 | 0 | 1 |
| §27 (architecture) | 1 | 0 | 0 | 1 |
| §28 (impl map) | 1 | 0 | 0 | 1 |
| §29 (performance) | 2 | 3 | 2 | 7 |
| §30 (export) | 1 | 0 | 0 | 1 |
| §31 (acceptance) | 23 | 5 | 0 | 28 |
| §32 (test strategy) | 0 | 1 | 0 | 1 |
| §33 (ADRs) | 1 | 0 | 0 | 1 |
| §34 (DoD) | 0 | 1 | 0 | 1 |
| §35 (strategic) | 1 | 0 | 0 | 1 |
| **Total** | **62** | **22** | **3** | **87** |

**Coverage:** 71% shipped, 25% partial, 3% missing (post-§20 slice).

## Bundle status (post-§20)

| Bundle | Slices | Status |
|---|---|---|
| **B1 Foundation** | 1 (B1) | ✅ Merged #322 |
| **B2 Metrics + Delta** | 1 (B2) | ✅ Merged #323 |
| **B3 Decisions** | 1 (B3) | ✅ Merged #324 |
| **B4 UX** | 5 (B4, B5, B6, B7, B8) | ✅ Merged #325..#329 |
| **B5 Provenance + AI** | 5 (B9, B13a, B13b, B13c, B14, B15) | ✅ Merged #330..#338 (note: B9 is part of B5 / persistence consolidation) |
| **B6 Polish + §20** | 5 (C-A1, C-A2, C-A3, C-A3.5, §20) | ✅ Merged #339..#342 + §20 PR |

All bundles except B7 are merged. CR-07 is ~96% closed (was ~95%).

## Bundle priority (refreshed post-§20)

Given the refresh, the remaining work is:

| Rank | Bundle | Reason |
|---|---|---|
| **1** | **§26 Semantic API** | The remaining 9 commands (`create_comparison`, `add_comparison_version`, etc.): papers-only surface. §19 + §23 + §24 are all Shipped |
| **2** | **§32 Visual regression + perf tests** | The 4K/8K/16-bit/perf suite; required for B7 Perf |
| **3** | **§9 Contextual metric display** | Surface docstring explanations in `RecommendationCard.svelte` |
| **4** | **§29 Performance** | Streaming high-res regions, cached difference images, GPU/WebGPU acceleration |

## First concrete slice (post-§23.3)

**§23.4 clipping masks** is the fourth and final
sub-slice of §23: highlight + shadow clipping masks as
two stacked SVG visualizations. The image is downsampled
to the preview budget (≤256 px), then for each pixel we
record whether it is highlight-clipped (`v >= 0.99`) or
shadow-clipped (`v <= 0.01`). Thresholds match the
established codebase conventions. ~600 LOC across Rust
+ TS. **§23 closes (Shipped)** with this slice.

## First concrete slice (post-§24)

**§19 close-out "Create branch" stub wire** is the
§19 close-out: enables the "Create branch" button
that was an honest stub since C-A1. The new
`createBranch()` handler persists the branch intent
durably via `applyImageDecision(bId, "preferred",
"create_branch", selectedQualityProfile)` AND
navigates to Enhance. The recorded decision is the
durable breadcrumb that P5a's child-version IPC will
read when it lands. ~70 LOC. **§19 closes (already
marked Shipped; the close-out fills the gap noted
in the audit text).**

## Items the audit surfaced that the original implementation plan missed

1. **No `astroforge-persistence` crate exists.** ✅ Resolved (B1
   Foundation landed comparison tables in `astroforge-core/src/comparison/`).
2. **Decision state (§17) is genuinely missing**, not partial. ✅ Resolved (B3).
3. **Synchronized navigation (§6) is genuinely missing.** ✅ Resolved (B5).
4. **Overlay (§5.5) is genuinely missing** — not partial. ✅ Resolved (B6).
5. **Astronomical integrity checks (§12) are well-shipped but only as
   pipeline-stage gates**, not as A-vs-B comparison checks. ✅ Resolved (B2).
6. **Version tree (§15) requires a DAG visualization**, not a flat list.
   ✅ Resolved (B8).
7. **Recommendation feedback loop (§20) is the second-largest opportunity**
   — the data + UI exist (`RecommendationEngine` + `RecommendationList`),
   but the post-comparison hookup is missing. ⚠️ Still open; this is the
   natural next slice.

## Files referenced

### Rust modules (~6,000 LOC of related existing code)

| File | LOC | Relevance |
|---|---|---|
| `crates/astroforge-core/src/quality.rs` | 450 | §8 frame/metric snapshot |
| `crates/astroforge-core/src/image_analysis/metrics.rs` | 472 | §8 luminance/chrominance/contrast |
| `crates/astroforge-core/src/image_analysis/structures.rs` | ~400 | §8 star metrics (count, size, FWHM, eccentricity) |
| `crates/astroforge-core/src/processing_metrics.rs` | 384 | §8 aggregate metrics |
| `crates/astroforge-core/src/quality_gates/mod.rs` | 250 | §12 gate infrastructure |
| `crates/astroforge-core/src/quality_gates/gates.rs` | 784 | §12 integrity checks |
| `crates/astroforge-core/src/quality_gates/report.rs` | 285 | §11 verdict + summary |
| `crates/astroforge-core/src/recommendation.rs` | 557 | §20 recommendation engine |
| `crates/astroforge-ai/src/recommendations/rules.rs` | 648 | §20 rule-based recommendations |
| `crates/astroforge-ai/src/recommendations/ordering.rs` | 285 | §20 recommendation ordering |
| `crates/astroforge-ai/src/recommendations/resource_estimate.rs` | 223 | §20 resource estimate |
| `crates/astroforge-core/src/recipe.rs` | 566 | §13 integrity badge + provenance data |
| `crates/astroforge-core/src/domain.rs` | 1,300+ | §2 ImageVersion (line 391) |
| `crates/astroforge-core/src/comparison.rs` | 775+ | §25 data model + §17/§18 decision + promotion |

### Svelte components (~2,200 LOC of related existing UI)

| File | LOC | Relevance |
|---|---|---|
| `src/components/CompareWorkspace.svelte` | 577 | §4 review experience layout |
| `src/components/CompareTools.svelte` | 427 | §5.2 split + §5.3 blink + §5.4 diff modes + §5.5 overlay |
| `src/components/VersionDag.svelte` | ~300 | §15 version tree DAG |
| `src/components/RegionPicker.svelte` | ~200 | §7 comparison scope |
| `src/components/ProvenancePanel.svelte` | ~150 | §14 provenance panel |
| `src/components/RecipeStageTimeline.svelte` | ~200 | §14 stage-by-stage timeline |
| `src/components/QualityProfilePicker.svelte` | 130 | §22 quality profile picker |
| `src/components/RecommendationCard.svelte` | 211 | §9 contextual display |
| `src/components/RecommendationList.svelte` | 377 | §20 recommendations |
| `src/components/QualityGatePanel.svelte` | 270 | §11 quality assessment |
| `src/lib/image-canvas-webgl.ts` | ~400 | §29 WebGL back-end |