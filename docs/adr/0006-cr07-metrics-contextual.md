# ADR-07.3 — Metrics Are Contextual

**Status:** Accepted (2026-09-12, CR-07 audit + B1 slice)
**Source:** CR-07 §9 + §33, ADR-07.3

## Context

CR-07 §9 establishes that no metric should be interpreted as an
absolute measure of astrophotographic quality. The §33 ADR text is
explicit:

> "Metrics Are Contextual"

For example, FWHM = 3.1 px indicates tighter stars than FWHM = 4.0 px,
but the absolute value depends on seeing, focal length, and pixel
scale. Likewise, higher local contrast does not necessarily mean more
real astronomical detail.

## Decision

`ComparisonMetric::values` is `BTreeMap<String, f64>` keyed by
`MetricKind::as_str()` (e.g. `"noise.luminance"`, `"stars.count"`,
`"background.gradient"`). The values are deliberately **not
strongly typed** in `comparison.rs` — each metric carries its
measurement but not its interpretation.

Direction ("is higher better?") is encoded in
`ComparisonDelta::compute()` via the `improvement_sign` parameter:

- `-1` for lower-is-better (noise, clipping, eccentricity)
- `+1` for higher-is-better (sharpness, SNR, star count up to a point)
- `0` for direction-ambiguous (local contrast), which forces the
  delta into `Inconclusive`

A `materiality_threshold` parameter classifies sub-material changes
as `Unchanged`. This is the same threshold used by the existing
`quality_gates::gate_*` functions, which are pipeline-stage
validators; the comparison data model reuses the same convention.

## Consequences

- **Positive:** The data model does not bake in a particular metric
  taxonomy; new metrics can be added without changing
  `comparison.rs`.
- **Positive:** Direction-aware deltas prevent the comparison UI from
  presenting a misleading "noise went up by 30%" as an improvement.
- **Negative:** Consumers must look up the `improvement_sign` for each
  metric kind. This is acceptable because the metric registry is
  centralized in `quality_gates/` + `image_analysis/metrics.rs`.
- **Negative:** Materiality thresholds are not encoded in the data
  model. Consumers must agree on the threshold at the call site.
  B2 (Metrics + Delta) will encode thresholds per metric kind.

## Alternatives considered

- **Strongly-typed `ComparisonMetric` with one variant per metric kind.**
  Rejected: would couple `comparison.rs` to the metric registry and
  require code changes every time a new metric is added.
- **Stored `improvement_sign` field per metric row.** Considered but
  deferred: storing the sign on the metric makes the data model
  opinionated about what "improvement" means. The CR-07 §9 principle
  treats metrics as contextual, so the sign belongs at the comparison
  call site, not on the row.
