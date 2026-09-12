# CR-07 Audit — Image Review, Comparison & Decision Workspace

**Source:** [`CR-07-IMAGE-REVIEW-COMPARISON-DECISION.md`](CR-07-IMAGE-REVIEW-COMPARISON-DECISION.md)
**Implementation plan:** [`CR-07-IMPLEMENTATION-PLAN.md`](CR-07-IMPLEMENTATION-PLAN.md)
**Audit date:** 2026-09-12
**Branch:** `docs/cr-07-audit` (from `origin/main` at `890d249`)
**Status:** ✅ Shipped / ⚠️ Partial / ❌ Missing

This audit reconciles every CR-07 §1–§35 acceptance criterion against the
existing codebase. The audit output drives the 7-bundle implementation
plan; sections already shipped don't need re-implementation.

## §1 Intent — ✅ Shipped

> "Which result is actually better, and why?"

Closed by the existing `CompareWorkspace` (PR #308) + `CompareTools`
(PR #309). The "Select → Compare → Inspect → Measure → Assess → Decide →
Promote" loop is the user's mental model; the existing implementation
covers Select, Compare, Inspect, and partial Measure.

## §2 Product Decision — ✅ Shipped

> "The Image Version established in CR-02 becomes the central
> comparison object."

`ImageVersion` exists in `crates/astroforge-core/src/domain.rs:391`.
`CompareWorkspace.svelte` operates on Image Versions, not files.

## §3 Core Principle — ✅ Shipped (philosophically)

> "AstroForge measures image characteristics; the user decides what
> constitutes the better image."

The existing metrics + quality-gate modules produce numbers; the UI does
not auto-declare a winner. The existing recommendation engine (CR-06) is
advisory, not authoritative.

## §4 Review Experience — ✅ Shipped (partial)

The 4-zone layout (Version tree | Image canvas | Assessment |
Histogram/Zoom/Blink/Split/Difference) is in `CompareWorkspace.svelte`.
The "image remains the dominant element" principle is preserved (canvas
takes the bulk of the layout).

⚠️ The "version tree" left rail is a **flat list**, not a transformation
graph (§15). Improvement pending.

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

### §5.4 Difference View — ⚠️ Partial

Absolute difference mode is shipped. **Missing:**
- Signed difference
- Amplified difference
- Structural difference

### §5.5 Overlay — ❌ Missing

The current implementation has side-by-side, split, blink, and absolute
difference. Overlay (one version rendered above another with adjustable
opacity) is not implemented.

## §6 Synchronized Navigation — ❌ Missing

`CompareWorkspace.svelte` does not synchronize zoom, pan, cursor, region,
histogram selection, pixel inspection, or mask visualization across A
and B. Each canvas operates independently.

## §7 Comparison Scope — ❌ Missing

The whole-image comparison is shipped implicitly. The "selected region"
and "specific feature" scope axes (§7.2, §7.3) are not exposed in the UI.

## §8 Image Metrics — ✅ Shipped (substantial)

**Existing modules:**
- `crates/astroforge-core/src/quality.rs` (450 LOC) — `FrameQuality`,
  `QualityMetricSnapshot`, `compute_metrics`.
- `crates/astroforge-core/src/image_analysis/metrics.rs` (472 LOC) —
  `luminance_noise`, `chromatic_noise`, `background_gradient`,
  `highlight_clipping`, `local_contrast`.
- `crates/astroforge-core/src/processing_metrics.rs` (384 LOC) —
  `ProcessingMetrics`, `aggregate_processing_metrics`.
- `crates/astroforge-core/src/quality_gates/` (1,319 LOC across mod +
  gates + report) — `GateFinding`, `QualityVerdict`, `QualityGateReport`.

| CR-07 §8 metric | Existing coverage |
|---|---|
| Luminance noise | ✅ `luminance_noise` |
| Chrominance noise | ✅ `chromatic_noise` |
| Regional noise | ⚠️ Partial (no per-region variant) |
| Sharpness (FWHM) | ❌ Missing |
| Local sharpness | ⚠️ `local_contrast` covers related ground |
| Edge response | ❌ Missing |
| Star count / size / eccentricity | ❌ Missing (no star metrics module) |
| FWHM distribution / saturation | ❌ Missing |
| Star-to-background contrast | ❌ Missing |
| Mean background / variance / gradient | ✅ `background_gradient` |
| Color gradient | ⚠️ Partial |
| Black clipping / highlight clipping | ✅ `highlight_clipping` |
| Saturation percentage | ❌ Missing |
| Estimated SNR / local SNR | ❌ Missing |
| Structural contrast | ⚠️ `local_contrast` is related but distinct |
| AI segmentation confidence / artifact indicators / model confidence | ❌ Missing |

## §9 Metrics Must Be Contextual — ⚠️ Partial

Each metric function has a docstring, but the UI presentation does not
include the contextual explanation CR-07 §9 calls for (e.g. "FWHM:
3.1 px — Lower generally indicates tighter stars, but values depend
on acquisition and processing conditions"). Improvement pending in
`RecommendationCard.svelte` (which is the natural home for contextual
metric display).

## §10 Delta Analysis — ❌ Missing

No module computes A-vs-B delta tables. `quality_gates/gates.rs` has
`star_artifacts`, `color_shifts`, etc. that take `(source, result, t)`
pairs and produce `GateFinding`s, but these are pipeline-stage gates,
not A-vs-B comparison deltas.

## §11 Quality Assessment — ⚠️ Partial

`QualityGateReport` produces a verdict (`Pass`/`Warn`/`Fail`/`Inconclusive`)
with findings count by severity — this is the "Assessment Summary"
shape. **Missing:** the natural-language summary CR-07 §11 calls for
("✓ Noise substantially reduced", "⚠ Highlight clipping increased",
"Overall: Strong improvement with minor trade-offs.").

## §12 Astronomical Integrity Checks — ✅ Shipped (substantial)

`crates/astroforge-core/src/quality_gates/gates.rs` (784 LOC) ships:
- `clipping`
- `noise_amplification`
- `excessive_smoothing`
- `color_shifts`
- `star_artifacts`
- `halos`
- `false_structures`
- `edge_artifacts`
- `segmentation_leakage`
- `ringing`

This is exactly CR-07 §12's integrity check list. The **gates fire as
pipeline-stage validators**; they need to be wired into comparison
context (A-vs-B) rather than only stage-result validation.

## §13 AI-Aware Comparison — ⚠️ Partial

`Recipe.integrity` (`IntegrityBadge` in `recipe.rs:77`) carries
`perceptual_models_used` + `seed_recorded` + per-model usage. The
**data exists**; the **comparison UI does not surface it.**

## §14 Provenance Panel — ❌ Missing

The data is in `Recipe` + `RecipeStage` + `IntegrityBadge` + `ModelUsage`,
but no `ProvenancePanel` component exists. `Recipe.integrity_label()`
formats a string summary; no UI consumes it.

## §15 Version Tree — ❌ Missing

The left rail in `CompareWorkspace.svelte` is a flat list of
ImageVersions. There is no transformation graph showing parent → child
relationships, branches, or parallel derivations.

## §16 Comparison Sets — ❌ Missing

No `comparison_set` data type, no persistence, no UI. The user cannot
save "M42 Final Candidates" as a reusable set.

## §17 Decision State — ❌ Missing

The `WORKING / CANDIDATE / PREFERRED / FINAL / REJECTED / REFERENCE`
state machine does not exist. `ImageVersion` (in `domain.rs:391`) has no
`decision_state` field.

## §18 Promotion Model — ❌ Missing

No `promote_image_version` operation. Promotion flow (Candidate →
Preferred → Final) is not implemented. The existing branch flow
(`recipe.rs`'s `version` + `parent_version` + `branch` fields) is
recipe-level, not image-version-level.

## §19 Compare → Continue Workflow — ⚠️ Partial

`CompareWorkspace.svelte` has a "Compare needs at least two image
versions" empty state, but no "Continue Enhancing" / "Create Branch"
/ "Mark Preferred" / "Export" buttons from the comparison view.

## §20 Intelligent Recommendation After Comparison — ⚠️ Partial

`RecommendationEngine` (`crates/astroforge-core/src/recommendation.rs`,
557 LOC) + `astroforge-ai/src/recommendations/` (1,156 LOC) produce
recommendations. `RecommendationList.svelte` (377 LOC) +
`RecommendationCard.svelte` (211 LOC) render them. **Missing:** the
post-comparison feedback loop where the comparison itself (not just the
pipeline stage) generates a recommendation.

## §21 No "AI Winner" by Default — ✅ Shipped

No "AI Winner" auto-label exists. The recommendation engine is advisory.
The user owns the decision.

## §22 Optional Quality Profiles — ❌ Missing

No Natural / Detail / Clean / Publication profile selection exists.

## §23 Expert Comparison — ⚠️ Partial

The Quality Gate Panel (`QualityGatePanel.svelte`, 270 LOC) shows
expert-level metric detail. **Missing:** FWHM distributions, noise maps,
clipping masks, channel statistics — none of these are first-class
visualizations yet.

## §24 Beginner Comparison — ⚠️ Partial

Side-by-side A vs B is simple by default. **Missing:** the
"Which do you prefer? [Natural] [AI Enhanced]" with one-paragraph
explanation beneath each (CR-07 §24 beginner mode).

## §25 Comparison Data Model — ❌ Missing

None of the §25 types exist:
- `comparison_session`
- `comparison_item`
- `comparison_region`
- `comparison_metric`
- `comparison_delta`
- `quality_assessment`
- `image_decision`
- `comparison_set`

## §26 Semantic API — ❌ Missing

No `create_comparison`, `add_comparison_version`, `set_comparison_mode`,
`get_comparison_metrics`, `get_metric_delta`, `get_quality_assessment`,
`get_version_provenance`, `set_image_decision`, `promote_image_version`,
`create_comparison_set`, `save_comparison_set`,
`create_branch_from_version` commands. The corresponding
`ComparisonCreated`, `ComparisonVersionAdded`, etc. events also do not
exist.

## §27 Architecture — ✅ Shipped (structure)

The architecture diagram matches reality: processing → image versions →
compare → metrics/visual/provenance → decision → refine/branch/export.
This is already the AstroForge shape.

## §28 Implementation Map — ⚠️ Partial

The plan called for `crates/astroforge-persistence/` but **no such
crate exists.** Persistence is done via `db.rs` + `domain_store.rs` in
`astroforge-core`. Recommendation: comparison tables should live in
`crates/astroforge-core/src/comparison/` (matching §25's plan),
piggybacking on the existing `db.rs` sqlite connection rather than
introducing a parallel persistence crate.

## §29 Performance Requirements — ⚠️ Partial

- ✅ Existing artifacts reused (`ImageVersion.primary_artifact_id`).
- ✅ Lower-resolution previews: WebGL back-end (PR #310) renders at the
  canvas display size.
- ❌ Streaming high-resolution regions: not implemented.
- ❌ Cached difference images: not implemented (each compare-mode toggle
  recomputes).
- ❌ GPU/WebGPU acceleration for comparison: WebGL is used for the
  canvas, but difference / blink / split overlays run on Canvas2D.
- ✅ Canvas2D/CPU fallback: in place.

## §30 Export From Comparison — ⚠️ Partial

`export_multi_format` IPC handles general export. **Missing:** the
"Export comparison" side-by-side JPEG/PNG export and the "Export
analytical report" (CR-07 §30).

## §31 Acceptance Criteria — see sections above

| Criterion | Status |
|---|---|
| Any compatible Image Versions can be compared | ✅ |
| Side-by-side works | ✅ |
| Split view works | ✅ |
| Blink works | ✅ |
| Overlay works | ❌ |
| Difference view works | ⚠️ Absolute only |
| Synchronized zoom/pan works | ❌ |
| Regional comparison works | ❌ |
| Relevant image metrics are available | ⚠️ Partial |
| Metrics can be compared between versions | ❌ |
| Deltas are calculated | ❌ |
| Quality warnings can be surfaced | ✅ (gate findings) |
| Astronomical integrity checks can be surfaced | ✅ (gate findings) |
| AI processing is identified | ⚠️ Data exists, UI missing |
| Processing history is accessible | ⚠️ Data exists, panel missing |
| AI model information is accessible | ⚠️ Data exists, panel missing |
| Recipe information is accessible | ⚠️ Data exists, panel missing |
| Image Version ancestry is visible | ❌ |
| Provenance remains intact after comparison | ✅ (non-destructive) |
| Version can be marked Candidate / Preferred / Final / Rejected | ❌ |
| Preferred/Final state survives restart | ❌ |
| User can continue editing from a selected version | ⚠️ Partial |
| Branching remains non-destructive | ✅ |
| Image remains dominant | ✅ |
| Beginner mode is simple | ⚠️ Partial |
| Advanced metrics use progressive disclosure | ⚠️ Partial |
| Comparison does not expose internal DAG complexity | ❌ (flat list, no DAG) |
| Comparison remains usable offline | ✅ (no remote calls) |

## §32 Test Strategy — ⚠️ Partial

`cargo test --workspace` is exhaustive (739 tests). **Missing:**
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

## §33 ADRs — ❌ Missing

None of ADR-07.1 through ADR-07.8 exist as `docs/adr/0004-…md` files.

## §34 Definition of Done — see §31 + §15 + §17 + §18 + §19

Items 1-7 (process, generate, branch, compare modes except overlay) are
shipped. Items 8-10 (zoom sync, histograms, metrics) are partial. Items
11-13 (quality issues, preferred candidate, continue enhancing) are
partial. Items 14-15 (preserve alternatives, reopen intact) are partial
(alternatives are preserved by CR-02; decision history is not).

## §35 Strategic Outcome — ✅ Shipped (philosophically)

The "intelligent, iterative image-revision system" is the AstroForge
intent. CR-07 closes the comparison-decision loop; the philosophy is
already in product positioning.

## Summary scorecard

| Section | ✅ | ⚠️ | ❌ | Total |
|---|---|---|---|---|
| §1–§4 (intent, decision, principle, layout) | 3 | 1 | 0 | 4 |
| §5 (modes) | 3 | 1 | 1 | 5 |
| §6 (sync nav) | 0 | 0 | 1 | 1 |
| §7 (scope) | 0 | 0 | 1 | 1 |
| §8 (metrics) | 3 | 3 | 10 | 16 |
| §9 (contextual) | 0 | 1 | 0 | 1 |
| §10 (delta) | 0 | 0 | 1 | 1 |
| §11 (assessment) | 0 | 1 | 0 | 1 |
| §12 (integrity) | 1 | 0 | 0 | 1 |
| §13 (AI-aware) | 0 | 1 | 0 | 1 |
| §14 (provenance) | 0 | 1 | 0 | 1 |
| §15 (version tree) | 0 | 0 | 1 | 1 |
| §16 (sets) | 0 | 0 | 1 | 1 |
| §17 (decision state) | 0 | 0 | 1 | 1 |
| §18 (promotion) | 0 | 0 | 1 | 1 |
| §19 (continue workflow) | 0 | 1 | 0 | 1 |
| §20 (recommendation) | 0 | 1 | 0 | 1 |
| §21 (no AI winner) | 1 | 0 | 0 | 1 |
| §22 (quality profiles) | 0 | 0 | 1 | 1 |
| §23 (expert) | 0 | 1 | 0 | 1 |
| §24 (beginner) | 0 | 1 | 0 | 1 |
| §25 (data model) | 0 | 0 | 1 | 1 |
| §26 (semantic API) | 0 | 0 | 1 | 1 |
| §27 (architecture) | 1 | 0 | 0 | 1 |
| §28 (impl map) | 0 | 1 | 0 | 1 |
| §29 (performance) | 2 | 3 | 2 | 7 |
| §30 (export) | 0 | 1 | 0 | 1 |
| §31 (acceptance) | 5 | 7 | 14 | 26 |
| §32 (test strategy) | 0 | 1 | 0 | 1 |
| §33 (ADRs) | 0 | 0 | 1 | 1 |
| §34 (DoD) | 0 | 1 | 0 | 1 |
| §35 (strategic) | 1 | 0 | 0 | 1 |
| **Total** | **20** | **27** | **40** | **87** |

**Coverage:** 23% shipped, 31% partial, 46% missing.

## Bundle priority

Given the audit, the implementation plan's 7 bundles can be re-prioritized:

| Rank | Bundle | Reason |
|---|---|---|
| **1** | **B1 Foundation** | §25 data model + §33 ADRs unlock every other bundle |
| **2** | **B2 Metrics + Delta** | §8 + §10 + §11 + §12 — closes the largest gap (10 missing metrics) |
| **3** | **B4 UX** | §5 overlay + §6 sync nav + §7 region + §15 version tree — UI surface |
| **4** | **B3 Decisions** | §17 + §18 + §16 — workflow |
| **5** | **B5 Provenance + AI** | §13 + §14 + §20 — surfaces existing data |
| **6** | **B6 Polish** | §22 + §23 + §24 + §30 — progressive disclosure |
| **7** | **B7 Perf + tests** | §29 + §32 — underpinning |

Each bundle ships as one PR over the existing slice cadence.

## First concrete slice

**B1 Foundation** is paperwork-only:
- `crates/astroforge-core/src/comparison.rs` (NEW) — types from §25:
  `ComparisonSession`, `ComparisonItem`, `ComparisonRegion`,
  `ComparisonMetric`, `ComparisonDelta`, `QualityAssessment`,
  `ImageDecision`, `ComparisonSet`.
- `docs/adr/0004-cr07-comparison-primitive.md` through
  `0011-cr07-decision-user-owned.md` — 8 ADRs from §33.

No behaviour change; just data types + decisions.

## Items the audit surfaced that the original implementation plan missed

1. **No `astroforge-persistence` crate exists.** Recommendation: comparison
   tables live in `astroforge-core/src/comparison/` alongside the data model.
2. **Decision state (§17) is genuinely missing**, not partial. The
   implementation plan's B3 bundle correctly sized this.
3. **Synchronized navigation (§6) is genuinely missing.** Existing compare
   UIs do not synchronize zoom/pan/region across A and B.
4. **Overlay (§5.5) is genuinely missing** — not partial. The plan's B4
   bundle is correct.
5. **Astronomical integrity checks (§12) are well-shipped but only as
   pipeline-stage gates**, not as A-vs-B comparison checks. Reuse the
   existing `quality_gates/gates.rs` module in the comparison context.
6. **Version tree (§15) requires a DAG visualization**, not a flat list.
   The implementation plan correctly captured this in B4.
7. **Recommendation feedback loop (§20) is the second-largest opportunity**
   — the data + UI exist (`RecommendationEngine` + `RecommendationList`),
   but the post-comparison hookup is missing.

## Files referenced

### Rust modules (~6,000 LOC of related existing code)

| File | LOC | Relevance |
|---|---|---|
| `crates/astroforge-core/src/quality.rs` | 450 | §8 frame/metric snapshot |
| `crates/astroforge-core/src/image_analysis/metrics.rs` | 472 | §8 luminance/chrominance/contrast |
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

### Svelte components (~1,800 LOC of related existing UI)

| File | LOC | Relevance |
|---|---|---|
| `src/components/CompareWorkspace.svelte` | 577 | §4 review experience layout |
| `src/components/CompareTools.svelte` | 427 | §5.2 split + §5.3 blink + §5.4 abs difference |
| `src/components/RecommendationCard.svelte` | 211 | §9 contextual display |
| `src/components/RecommendationList.svelte` | 377 | §20 recommendations |
| `src/components/QualityGatePanel.svelte` | 270 | §11 quality assessment |
| `src/lib/image-canvas-webgl.ts` | ~400 | §29 WebGL back-end |