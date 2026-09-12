# CR-07 Expanded Implementation Plan

**Source:** [`CR-07-IMAGE-REVIEW-COMPARISON-DECISION.md`](CR-07-IMAGE-REVIEW-COMPARISON-DECISION.md)
**Original scope (shipped):** [`CR-07-ZONE-B-CANVAS.md`](CR-07-ZONE-B-CANVAS.md)
**Status:** Proposed → 7 bundles staged
**Target:** AstroForge v1.0

The original CR-07 (`docs/CR-07-ZONE-B-CANVAS.md`) shipped in PR #308 + #309
and covered Zone B ImageCanvas + Compare surfaces (side-by-side, split,
blink, difference, overlay). The expanded CR-07 (sections 1–35) adds
review, comparison, decision, provenance, version tree, comparison sets,
recommendation, and beginner/expert profiles.

## Bundle order

Each bundle ships as **one PR** over the slice cadence. Bundled-batch
tightly-coupled sub-bullets ship together; larger bundles get broken into
multiple slices if a single PR exceeds the trusted reviewable size
(~1000 LOC).

The priority order below is informed by [`CR-07-AUDIT.md`](CR-07-AUDIT.md)
which reconciles every §1–§35 acceptance criterion against the existing
codebase (2026-09-12).

| # | Bundle | Slices | PR scope | LOC est. |
|---|---|---|---|---|
| **B1** | **Foundation** | Audit + §25 data model + §33 ADRs | `comparison.rs` types (session, item, region, metric, delta, assessment, decision, set) + ADR-07.1..07.8 | 1500 |
| **B2** | **Metrics + delta** | §8 + §10 + §11 + §12 | `metrics.rs` (noise, sharpness, stars, background, dynamic-range, signal, AI-quality) + `difference.rs` (absolute, signed, amplified, structural) + `assessment.rs` | 2500 |
| **B3** | **Decisions** | §17 + §18 + §16 | `image_decision` state machine + promotion flow + comparison sets (CRUD + persistence) | 1500 |
| **B4** | **UX** | §5 + §6 + §7 + §15 | modes UI (overlay + amplified/structural difference) + synchronized nav + comparison region + version tree (DAG visualization) | 2500 |
| **B5** | **Provenance + AI** | §13 + §14 + §20 | provenance panel + AI-aware comparison + recommendation feedback loop | 1500 |
| **B6** | **Polish** | §22 + §23 + §24 + §30 | optional quality profiles + expert comparison + beginner comparison + export from comparison | 1000 |
| **B7** | **Perf + tests** | §29 + §32 | streaming, GPU/WebGPU acceleration, visual regression, metric validation, version integrity, AI comparison tests, performance tests | 1500 |

**Total: ~12,000 LOC over 7 PRs.**

## Foundation audit

Before any code lands, B1's audit reconciles each CR-07 section against
the existing codebase. The audit's deliverable is the precise list of:

- ✅ Sections already shipped (with PR refs).
- ⚠️ Sections partially shipped.
- ❌ Sections genuinely missing.

The audit output lives in `docs/CR-07-AUDIT.md` and is itself a
paperwork-walk-down per the existing slice cadence.

## Data model (§25)

`crates/astroforge-core/src/comparison/` (NEW) — Rust types:

```rust
struct ComparisonSession { id, project_id, created_at, regions, items, mode }
struct ComparisonItem { session_id, version_a, version_b, mode }
struct ComparisonRegion { id, session_id, shape, scope (WholeImage|SelectedRegion|SpecificFeature) }
struct ComparisonMetric { id, session_id, region_id, kind (Noise|Sharpness|Stars|...), values }
struct ComparisonDelta { id, session_id, metric_id, direction (Improved|Degraded|Unchanged|Inconclusive), percent }
struct QualityAssessment { id, session_id, warnings, summary, integrity_checks }
struct ImageDecision { version_id, state (Working|Candidate|Preferred|Final|Rejected|Reference), history }
struct ComparisonSet { id, name, items, created_at }
```

## Comparison modes (§5)

Reuse the existing `CompareWorkspace` shipped in PR #308 + #309
(CR-07 Zone B) for side-by-side, split, blink, difference, overlay.
This bundle extends with synchronized nav, region selection, and
version tree visualization.

## Comparison sets (§16) + decisions (§17, §18)

`crates/astroforge-core/src/comparison/` (NEW) — SQLite tables
for comparison sets + decision history. ImageDecision state machine:
`Working → Candidate → Preferred → Final` (with branches to `Rejected`
and `Reference`). Promotion preserves complete history.

> **Note:** the implementation map in CR-07 §28 references a
> `crates/astroforge-persistence/` crate which does not exist. The
> recommendation from [`CR-07-AUDIT.md`](CR-07-AUDIT.md) is to put
> comparison tables in `crates/astroforge-core/src/comparison/`,
> reusing the existing `db.rs` sqlite connection. No new persistence
> crate is needed.

## ADRs (§33)

8 ADRs in `docs/adr/`:

- ADR-07.1 — Image Version Is the Comparison Primitive
- ADR-07.2 — Visual Comparison Is Primary
- ADR-07.3 — Metrics Are Contextual
- ADR-07.4 — Comparison Is Non-Destructive
- ADR-07.5 — Provenance Is Always Available
- ADR-07.6 — AI Processing Is Explicit in Comparison
- ADR-07.7 — Decision Is User-Owned
- ADR-07.8 — Comparison Is a Feedback Loop

## Performance (§29) — non-negotiable

Comparison must avoid duplicating large images in memory:

- Reuse existing artifacts (ImageVersion).
- Use lower-resolution previews for overview comparison.
- Stream high-resolution regions.
- Cache generated difference images.
- Release inactive comparison buffers.
- Use GPU/WebGPU acceleration where available.
- Fall back to Canvas2D/CPU.

This matters under the CR-09 4–8 GB resource constraint.

## Testing (§32)

- **Visual regression** — known reference datasets for split alignment,
  blink consistency, difference rendering, overlay accuracy, histogram
  consistency, zoom synchronization.
- **Metric validation** — controlled datasets with known noise, blur,
  clipping, star eccentricity, background gradients.
- **Version integrity** — `Version A + Version B + Comparison` must not
  modify either artifact.
- **AI comparison** — verify model/version/hash/classification/parameters/provenance.
- **Performance** — 4K / 8K / 16-bit / 32-bit / multiple simultaneous
  versions / large projects / limited RAM.

## Definition of done (§34)

A user can:

1. Process a Session.
2. Generate multiple Image Versions.
3. Apply different AI enhancements.
4. Create branches.
5. Open Compare.
6. Select any candidate versions.
7. Compare them side-by-side, split, blink, overlay or difference.
8. Zoom into identical regions.
9. Inspect histograms and relevant metrics.
10. Review AI/provenance information.
11. See potential quality or astronomical-integrity issues.
12. Select a preferred candidate.
13. Continue enhancing that candidate or export it.
14. Preserve every alternative version.
15. Reopen the project later with all comparison and decision history
    intact.

## Slice cadence

Bundles ship one per cadence. The user picks the next tranche from
"what is next" recommendations. Each PR follows the existing pattern:
robustness analysis → branch → code → tests → paperwork → PR → CI →
merge on CI green.

## First slice

**B1 — Foundation audit** (recommended next after the current LAN
spin-up lands). Reconciles every §1–§35 acceptance criterion against
existing code; produces `docs/CR-07-AUDIT.md` with the section-shipped
status. No new code; paperwork only. Sets up §25 data model + ADRs
for the second slice.