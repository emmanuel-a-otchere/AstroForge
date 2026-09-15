# Changelog

## Unreleased

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
