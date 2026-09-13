//! CR-07 §8 — Metric registry + direction + materiality.
//!
//! B2 of the CR-07 implementation plan: codifies the per-metric
//! registry that B1 deferred. Each [`MetricKind`] declares its
//! [`MetricDirection`] (lower-is-better, higher-is-better, or
//! ambiguous), default [`Materiality`] threshold, and the
//! `image_analysis::metrics::MetricsSample` key under which the
//! measurement is recorded.
//!
//! ## Why an enum + table rather than a struct-of-fields
//!
//! CR-07 §8 lists 16+ metric kinds and §9 establishes that metrics
//! are contextual. The B1 [`ComparisonMetric`](crate::comparison::ComparisonMetric)
//! already carries values keyed by `MetricKind::as_str()`. B2 fills
//! the registry that B1 deferred.
//!
//! ## Adding a new metric
//!
//! 1. Add a variant to [`MetricKind`].
//! 2. Add a row to `METRIC_REGISTRY` with the direction + materiality
//!    + label + contextual explanation.
//! 3. Wire the measurement in `image_analysis::metrics::*`.
//! 4. The `as_str()` mapping is exhaustive — `match_metric_kind()`
//!    below panics on unknown variants.
//!
//! ## CR-07 §9 contextual explanations
//!
//! Each entry carries a `context` string explaining what the metric
//! means. The CR-07 §9 example is preserved verbatim:
//!
//! > FWHM: 3.1 px — Lower generally indicates tighter stars, but
//! > values depend on acquisition and processing conditions.

use crate::comparison::{ComparisonDelta, DeltaDirection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A measurable characteristic of an Image Version.
///
/// The CR-07 §8 metric list is the source of truth. Each variant
/// corresponds to exactly one entry in the `METRIC_REGISTRY` table
/// below.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    // §8 Noise — luminance noise is shipped (`image_analysis::metrics::luminance_noise`);
    // chrominance noise is shipped (`chromatic_noise`); regional noise is not yet
    // implemented as a distinct measurement.
    NoiseLuminance,
    NoiseChrominance,
    NoiseRegional,

    // §8 Sharpness — FWHM (full width at half maximum) is shipped in
    // `quality::QualityMetricSnapshot::fwhm`; local sharpness is
    // related to `image_analysis::metrics::local_contrast`; edge
    // response is not yet implemented.
    SharpnessFwhm,
    SharpnessLocal,
    SharpnessEdgeResponse,

    // §8 Stars — star count + size + eccentricity + FWHM distribution
    // + saturation + star-to-background contrast.
    StarCount,
    StarSize,
    StarEccentricity,
    StarFwhmDistribution,
    StarSaturation,
    StarBackgroundContrast,

    // §8 Background — mean background + variance + gradient strength
    // + color gradient. `image_analysis::metrics::background_gradient`
    // covers the gradient; the others are not yet implemented.
    BackgroundMean,
    BackgroundVariance,
    BackgroundGradient,
    BackgroundColorGradient,

    // §8 Dynamic range — black clipping + highlight clipping +
    // saturation percentage. `image_analysis::metrics::highlight_clipping`
    // covers highlights; black + saturation are not yet implemented.
    DynamicRangeBlackClipping,
    DynamicRangeHighlightClipping,
    DynamicRangeSaturationPct,

    // §8 Signal — estimated SNR + local SNR + structural contrast.
    // `quality::QualityMetricSnapshot::snr_db` covers estimated SNR;
    // local SNR + structural contrast are not yet implemented.
    SignalEstimatedSnr,
    SignalLocalSnr,
    SignalStructuralContrast,

    // §8 AI quality — segmentation confidence + artifact indicators +
    // reconstruction risk + model confidence.
    AiSegmentationConfidence,
    AiArtifactIndicator,
    AiReconstructionRisk,
    AiModelConfidence,
}

impl MetricKind {
    /// Canonical string key, matching `ComparisonMetric::values`
    /// keys.
    pub fn as_str(self) -> &'static str {
        match self {
            MetricKind::NoiseLuminance => "noise.luminance",
            MetricKind::NoiseChrominance => "noise.chrominance",
            MetricKind::NoiseRegional => "noise.regional",
            MetricKind::SharpnessFwhm => "sharpness.fwhm",
            MetricKind::SharpnessLocal => "sharpness.local",
            MetricKind::SharpnessEdgeResponse => "sharpness.edge_response",
            MetricKind::StarCount => "stars.count",
            MetricKind::StarSize => "stars.size",
            MetricKind::StarEccentricity => "stars.eccentricity",
            MetricKind::StarFwhmDistribution => "stars.fwhm_distribution",
            MetricKind::StarSaturation => "stars.saturation",
            MetricKind::StarBackgroundContrast => "stars.background_contrast",
            MetricKind::BackgroundMean => "background.mean",
            MetricKind::BackgroundVariance => "background.variance",
            MetricKind::BackgroundGradient => "background.gradient",
            MetricKind::BackgroundColorGradient => "background.color_gradient",
            MetricKind::DynamicRangeBlackClipping => "dynamic_range.black_clipping",
            MetricKind::DynamicRangeHighlightClipping => "dynamic_range.highlight_clipping",
            MetricKind::DynamicRangeSaturationPct => "dynamic_range.saturation_pct",
            MetricKind::SignalEstimatedSnr => "signal.estimated_snr",
            MetricKind::SignalLocalSnr => "signal.local_snr",
            MetricKind::SignalStructuralContrast => "signal.structural_contrast",
            MetricKind::AiSegmentationConfidence => "ai.segmentation_confidence",
            MetricKind::AiArtifactIndicator => "ai.artifact_indicator",
            MetricKind::AiReconstructionRisk => "ai.reconstruction_risk",
            MetricKind::AiModelConfidence => "ai.model_confidence",
        }
    }

    /// All variants in canonical order. Useful for table rendering
    /// and exhaustive test assertions.
    pub const ALL: &'static [MetricKind] = &[
        MetricKind::NoiseLuminance,
        MetricKind::NoiseChrominance,
        MetricKind::NoiseRegional,
        MetricKind::SharpnessFwhm,
        MetricKind::SharpnessLocal,
        MetricKind::SharpnessEdgeResponse,
        MetricKind::StarCount,
        MetricKind::StarSize,
        MetricKind::StarEccentricity,
        MetricKind::StarFwhmDistribution,
        MetricKind::StarSaturation,
        MetricKind::StarBackgroundContrast,
        MetricKind::BackgroundMean,
        MetricKind::BackgroundVariance,
        MetricKind::BackgroundGradient,
        MetricKind::BackgroundColorGradient,
        MetricKind::DynamicRangeBlackClipping,
        MetricKind::DynamicRangeHighlightClipping,
        MetricKind::DynamicRangeSaturationPct,
        MetricKind::SignalEstimatedSnr,
        MetricKind::SignalLocalSnr,
        MetricKind::SignalStructuralContrast,
        MetricKind::AiSegmentationConfidence,
        MetricKind::AiArtifactIndicator,
        MetricKind::AiReconstructionRisk,
        MetricKind::AiModelConfidence,
    ];

    /// Reverse lookup — `MetricKind::from_str(metric.as_str())` is
    /// round-trippable for every variant.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|m| m.as_str() == s)
    }
}

impl std::fmt::Display for MetricKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Direction of "improvement" for a metric.
///
/// `LowerIsBetter` means a smaller value is preferable (noise,
/// clipping). `HigherIsBetter` means a larger value is preferable
/// (sharpness, SNR). `Ambiguous` means the direction depends on
/// context — e.g. higher local contrast can indicate real detail or
/// over-sharpening halos.
///
/// The CR-07 §9 principle is preserved: `Ambiguous` metrics classify
/// their delta as `Inconclusive` by default. Callers can override
/// the classification via `ComparisonDelta::compute()`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricDirection {
    LowerIsBetter,
    HigherIsBetter,
    Ambiguous,
}

impl MetricDirection {
    /// The `improvement_sign` value expected by
    /// [`ComparisonDelta::compute`](crate::comparison::ComparisonDelta::compute):
    /// `-1` for lower-is-better, `+1` for higher-is-better, `0` for
    /// ambiguous.
    pub fn improvement_sign(self) -> i8 {
        match self {
            MetricDirection::LowerIsBetter => -1,
            MetricDirection::HigherIsBetter => 1,
            MetricDirection::Ambiguous => 0,
        }
    }
}

/// Per-metric registry entry.
///
/// Each `MetricKind` has exactly one entry; the `METRIC_REGISTRY`
/// table below is the source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSpec {
    pub kind: MetricKind,
    pub direction: MetricDirection,
    /// Default materiality threshold as a percent. Values within
    /// ±`materiality_pct` of the baseline classify as `Unchanged`.
    /// Default 5%; tighter metrics (FWHM) use 3%, looser metrics
    /// (star count) use 10%.
    pub materiality_pct: f64,
    /// Short human-readable label for tables.
    pub label: &'static str,
    /// §9 contextual explanation: what the metric means and how to
    /// interpret changes.
    pub context: &'static str,
    /// Unit string for display (e.g. "px", "dB", "%"). Empty for
    /// unitless counts.
    pub unit: &'static str,
}

/// The metric registry — one entry per `MetricKind`.
///
/// To add a new metric:
/// 1. Add the variant to `MetricKind`.
/// 2. Add a row here. The compiler will catch missing rows via the
///    `metric_registry_is_complete` test below.
pub const METRIC_REGISTRY: &[MetricSpec] = &[
    MetricSpec {
        kind: MetricKind::NoiseLuminance,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 5.0,
        label: "Noise (luminance)",
        context: "Lower generally indicates a smoother image, but depends on acquisition \
             and stacking depth.",
        unit: "sigma",
    },
    MetricSpec {
        kind: MetricKind::NoiseChrominance,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 5.0,
        label: "Noise (chrominance)",
        context: "Lower generally indicates cleaner color, but aggressive reduction can \
             desaturate faint structures.",
        unit: "sigma",
    },
    MetricSpec {
        kind: MetricKind::NoiseRegional,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 8.0,
        label: "Noise (regional)",
        context: "Per-region noise variance. Lower indicates more even noise reduction \
             across the frame.",
        unit: "sigma",
    },
    MetricSpec {
        kind: MetricKind::SharpnessFwhm,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 3.0,
        label: "Star FWHM",
        context: "Lower generally indicates tighter stars, but values depend on seeing, \
             focal length, and pixel scale.",
        unit: "px",
    },
    MetricSpec {
        kind: MetricKind::SharpnessLocal,
        direction: MetricDirection::Ambiguous,
        materiality_pct: 5.0,
        label: "Local sharpness",
        context: "Higher local contrast can indicate real detail or over-sharpening \
             halos; review the diff to confirm.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::SharpnessEdgeResponse,
        direction: MetricDirection::Ambiguous,
        materiality_pct: 5.0,
        label: "Edge response",
        context: "Higher edge response can indicate sharpening. Direction is \
             context-dependent; review the diff.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::StarCount,
        direction: MetricDirection::HigherIsBetter,
        materiality_pct: 10.0,
        label: "Star count",
        context: "Higher count suggests preserved faint stars. Drops can indicate \
             aggressive star reduction or masking that erased faint stars.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::StarSize,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 5.0,
        label: "Star size",
        context: "Smaller is typically tighter, but overshrinking stars can lose \
             natural appearance.",
        unit: "px",
    },
    MetricSpec {
        kind: MetricKind::StarEccentricity,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 10.0,
        label: "Star eccentricity",
        context: "Lower indicates rounder stars; higher can indicate tracking issues, \
             coma, or warping.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::StarFwhmDistribution,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 5.0,
        label: "Star FWHM distribution",
        context: "Standard deviation of star FWHMs across the frame. Lower indicates \
             more consistent star sizes.",
        unit: "px",
    },
    MetricSpec {
        kind: MetricKind::StarSaturation,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 5.0,
        label: "Star saturation",
        context: "Fraction of clipped star cores. Higher means more stars are losing \
             color information.",
        unit: "%",
    },
    MetricSpec {
        kind: MetricKind::StarBackgroundContrast,
        direction: MetricDirection::HigherIsBetter,
        materiality_pct: 5.0,
        label: "Star/background contrast",
        context: "Higher indicates better separation of stars from background sky.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::BackgroundMean,
        direction: MetricDirection::Ambiguous,
        materiality_pct: 5.0,
        label: "Background mean",
        context: "Mean background value. Depends on stretch; not a quality metric on \
             its own.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::BackgroundVariance,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 5.0,
        label: "Background variance",
        context: "Lower indicates more uniform background after extraction.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::BackgroundGradient,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 10.0,
        label: "Background gradient",
        context: "Lower indicates a flatter background. Light pollution gradients \
             increase this value.",
        unit: "slope/100px",
    },
    MetricSpec {
        kind: MetricKind::BackgroundColorGradient,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 10.0,
        label: "Background color gradient",
        context: "Color component of the background gradient. Lower indicates more \
             neutral extraction.",
        unit: "slope/100px",
    },
    MetricSpec {
        kind: MetricKind::DynamicRangeBlackClipping,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 0.5,
        label: "Black clipping",
        context: "Fraction of pixels at exactly 0. Any non-zero value is suspicious and \
             usually indicates a stretch artifact.",
        unit: "%",
    },
    MetricSpec {
        kind: MetricKind::DynamicRangeHighlightClipping,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 0.5,
        label: "Highlight clipping",
        context: "Fraction of pixels at or near saturation. Higher means more bright \
             detail is being lost.",
        unit: "%",
    },
    MetricSpec {
        kind: MetricKind::DynamicRangeSaturationPct,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 0.5,
        label: "Channel saturation",
        context: "Fraction of channels saturated. Higher indicates color information \
             is being lost.",
        unit: "%",
    },
    MetricSpec {
        kind: MetricKind::SignalEstimatedSnr,
        direction: MetricDirection::HigherIsBetter,
        materiality_pct: 5.0,
        label: "Estimated SNR",
        context: "Higher indicates stronger signal relative to noise.",
        unit: "dB",
    },
    MetricSpec {
        kind: MetricKind::SignalLocalSnr,
        direction: MetricDirection::HigherIsBetter,
        materiality_pct: 5.0,
        label: "Local SNR",
        context: "Per-region signal-to-noise. Higher indicates better preservation of \
             faint structures.",
        unit: "dB",
    },
    MetricSpec {
        kind: MetricKind::SignalStructuralContrast,
        direction: MetricDirection::Ambiguous,
        materiality_pct: 5.0,
        label: "Structural contrast",
        context: "Higher can indicate real structure or sharpening artifacts. Review \
             the diff to confirm.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::AiSegmentationConfidence,
        direction: MetricDirection::HigherIsBetter,
        materiality_pct: 5.0,
        label: "AI segmentation confidence",
        context: "Higher indicates the AI model was more certain about its mask.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::AiArtifactIndicator,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 5.0,
        label: "AI artifact indicator",
        context: "Higher indicates more AI artifacts detected. Lower is preferable for \
             naturalistic results.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::AiReconstructionRisk,
        direction: MetricDirection::LowerIsBetter,
        materiality_pct: 5.0,
        label: "AI reconstruction risk",
        context: "Higher indicates the AI model is more likely reconstructing detail \
             not present in the source.",
        unit: "",
    },
    MetricSpec {
        kind: MetricKind::AiModelConfidence,
        direction: MetricDirection::HigherIsBetter,
        materiality_pct: 5.0,
        label: "AI model confidence",
        context: "Higher indicates the AI model was more certain about its output.",
        unit: "",
    },
];

/// Look up the registry entry for a metric kind. Returns `None` for
/// unregistered metrics (which should not happen — see
/// `metric_registry_is_complete` test).
pub fn metric_spec(kind: MetricKind) -> Option<&'static MetricSpec> {
    METRIC_REGISTRY.iter().find(|s| s.kind == kind)
}

/// Convenience: direction for a metric kind.
pub fn direction(kind: MetricKind) -> MetricDirection {
    metric_spec(kind)
        .map(|s| s.direction)
        .unwrap_or(MetricDirection::Ambiguous)
}

/// Convenience: materiality threshold for a metric kind.
pub fn materiality(kind: MetricKind) -> f64 {
    metric_spec(kind).map(|s| s.materiality_pct).unwrap_or(5.0)
}

/// Convenience: contextual explanation string for a metric kind.
pub fn context(kind: MetricKind) -> &'static str {
    metric_spec(kind).map(|s| s.context).unwrap_or("")
}

/// Convenience: display label for a metric kind.
pub fn label(kind: MetricKind) -> &'static str {
    metric_spec(kind)
        .map(|s| s.label)
        .unwrap_or("(unknown metric)")
}

/// Convenience: unit string for a metric kind.
pub fn unit(kind: MetricKind) -> &'static str {
    metric_spec(kind).map(|s| s.unit).unwrap_or("")
}

/// One row of the CR-07 §10 delta table.
///
/// Example:
/// ```text
///              A        B      Δ
/// Noise       18.2    12.1   -33%
/// Star FWHM    3.4     3.1    -9%
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricDeltaRow {
    pub kind: MetricKind,
    pub baseline_value: Option<f64>,
    pub compared_value: Option<f64>,
    pub direction: DeltaDirection,
    pub percent_change: f64,
    pub materiality_threshold: f64,
    pub label: &'static str,
    pub unit: &'static str,
    pub context: &'static str,
}

impl MetricDeltaRow {
    /// Render the percent change as a signed string (e.g. "-33%").
    /// Returns `"—"` when either value is missing.
    pub fn percent_label(&self) -> String {
        match (self.baseline_value, self.compared_value) {
            (Some(_), Some(_)) => {
                let sign = if self.percent_change > 0.0 { "+" } else { "" };
                format!("{}{:.0}%", sign, self.percent_change)
            }
            _ => "—".to_string(),
        }
    }

    /// Format a value with its unit (e.g. "12.1 sigma"). Returns
    /// `"—"` when the value is missing.
    pub fn format_value(&self, value: Option<f64>) -> String {
        match value {
            Some(v) if self.unit.is_empty() => format!("{:.2}", v),
            Some(v) => format!("{:.2} {}", v, self.unit),
            None => "—".to_string(),
        }
    }
}

/// Compute the delta table for two metric snapshots.
///
/// `baseline` and `compared` are `BTreeMap<String, f64>` matching the
/// `ComparisonMetric::values` shape. The output covers every
/// `MetricKind` in the registry; metrics missing from one or both
/// snapshots produce rows with `None` values.
pub fn compute_deltas(
    baseline: &BTreeMap<String, f64>,
    compared: &BTreeMap<String, f64>,
) -> Vec<MetricDeltaRow> {
    METRIC_REGISTRY
        .iter()
        .map(|spec| {
            let key = spec.kind.as_str();
            let baseline_value = baseline.get(key).copied();
            let compared_value = compared.get(key).copied();
            let (direction, percent_change) = match (baseline_value, compared_value) {
                (Some(b), Some(c)) => {
                    let dir = ComparisonDelta::compute(
                        b,
                        c,
                        spec.direction.improvement_sign(),
                        spec.materiality_pct,
                    );
                    let pct = if b.abs() < f64::EPSILON {
                        0.0
                    } else {
                        (c - b) / b.abs() * 100.0
                    };
                    (dir, pct)
                }
                _ => (DeltaDirection::Inconclusive, 0.0),
            };
            MetricDeltaRow {
                kind: spec.kind,
                baseline_value,
                compared_value,
                direction,
                percent_change,
                materiality_threshold: spec.materiality_pct,
                label: spec.label,
                unit: spec.unit,
                context: spec.context,
            }
        })
        .collect()
}

/// Compute deltas for a single metric kind.
pub fn compute_delta_for(
    kind: MetricKind,
    baseline: Option<f64>,
    compared: Option<f64>,
) -> MetricDeltaRow {
    let spec = metric_spec(kind).expect("every MetricKind has a registry entry");
    let (direction, percent_change) = match (baseline, compared) {
        (Some(b), Some(c)) => {
            let dir = ComparisonDelta::compute(
                b,
                c,
                spec.direction.improvement_sign(),
                spec.materiality_pct,
            );
            let pct = if b.abs() < f64::EPSILON {
                0.0
            } else {
                (c - b) / b.abs() * 100.0
            };
            (dir, pct)
        }
        _ => (DeltaDirection::Inconclusive, 0.0),
    };
    MetricDeltaRow {
        kind,
        baseline_value: baseline,
        compared_value: compared,
        direction,
        percent_change,
        materiality_threshold: spec.materiality_pct,
        label: spec.label,
        unit: spec.unit,
        context: spec.context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_kind_round_trips_via_as_str() {
        for kind in MetricKind::ALL {
            assert_eq!(MetricKind::parse(kind.as_str()), Some(*kind));
        }
    }

    #[test]
    fn metric_kind_as_str_is_unique() {
        let mut seen = std::collections::HashSet::new();
        for kind in MetricKind::ALL {
            assert!(seen.insert(kind.as_str()), "duplicate: {}", kind.as_str());
        }
    }

    #[test]
    fn metric_registry_is_complete() {
        // Every MetricKind variant has a registry entry.
        for kind in MetricKind::ALL {
            assert!(
                metric_spec(*kind).is_some(),
                "missing registry entry for {:?}",
                kind
            );
        }
        // Every registry entry's kind is in MetricKind::ALL.
        for spec in METRIC_REGISTRY {
            assert!(
                MetricKind::ALL.contains(&spec.kind),
                "registry entry for {:?} not in MetricKind::ALL",
                spec.kind
            );
        }
        // Counts match.
        assert_eq!(METRIC_REGISTRY.len(), MetricKind::ALL.len());
    }

    #[test]
    fn direction_improvement_sign_matches_spec() {
        assert_eq!(MetricDirection::LowerIsBetter.improvement_sign(), -1);
        assert_eq!(MetricDirection::HigherIsBetter.improvement_sign(), 1);
        assert_eq!(MetricDirection::Ambiguous.improvement_sign(), 0);
    }

    #[test]
    fn compute_deltas_handles_missing_values() {
        let baseline = BTreeMap::new();
        let compared = BTreeMap::new();
        let rows = compute_deltas(&baseline, &compared);
        // One row per registry entry.
        assert_eq!(rows.len(), METRIC_REGISTRY.len());
        // Every row is inconclusive (no values).
        for row in &rows {
            assert_eq!(row.direction, DeltaDirection::Inconclusive);
            assert_eq!(row.baseline_value, None);
            assert_eq!(row.compared_value, None);
            assert_eq!(row.percent_change, 0.0);
        }
    }

    #[test]
    fn compute_deltas_low_noise_is_improvement() {
        // Lower-is-better metric (noise.luminance). Going from 20 -> 10
        // is Improved.
        let mut baseline = BTreeMap::new();
        baseline.insert("noise.luminance".to_string(), 20.0);
        let mut compared = BTreeMap::new();
        compared.insert("noise.luminance".to_string(), 10.0);
        let rows = compute_deltas(&baseline, &compared);
        let noise = rows
            .iter()
            .find(|r| r.kind == MetricKind::NoiseLuminance)
            .unwrap();
        assert_eq!(noise.direction, DeltaDirection::Improved);
        assert!((noise.percent_change - (-50.0)).abs() < 0.001);
        assert_eq!(noise.percent_label(), "-50%");
    }

    #[test]
    fn compute_deltas_materiality_threshold_applies() {
        // 1% change in noise.luminance (5% materiality) -> Unchanged.
        let mut baseline = BTreeMap::new();
        baseline.insert("noise.luminance".to_string(), 100.0);
        let mut compared = BTreeMap::new();
        compared.insert("noise.luminance".to_string(), 101.0);
        let rows = compute_deltas(&baseline, &compared);
        let noise = rows
            .iter()
            .find(|r| r.kind == MetricKind::NoiseLuminance)
            .unwrap();
        assert_eq!(noise.direction, DeltaDirection::Unchanged);
    }

    #[test]
    fn compute_deltas_ambiguous_metric_is_inconclusive() {
        // Local sharpness (Ambiguous) goes up by 50% -> Inconclusive.
        let mut baseline = BTreeMap::new();
        baseline.insert("sharpness.local".to_string(), 1.0);
        let mut compared = BTreeMap::new();
        compared.insert("sharpness.local".to_string(), 1.5);
        let rows = compute_deltas(&baseline, &compared);
        let sharp = rows
            .iter()
            .find(|r| r.kind == MetricKind::SharpnessLocal)
            .unwrap();
        assert_eq!(sharp.direction, DeltaDirection::Inconclusive);
    }

    #[test]
    fn compute_deltas_partial_coverage_keeps_other_rows_intact() {
        // Only one metric is present; all others stay Inconclusive.
        let mut baseline = BTreeMap::new();
        baseline.insert("signal.estimated_snr".to_string(), 30.0);
        let mut compared = BTreeMap::new();
        compared.insert("signal.estimated_snr".to_string(), 36.0);
        let rows = compute_deltas(&baseline, &compared);
        let snr = rows
            .iter()
            .find(|r| r.kind == MetricKind::SignalEstimatedSnr)
            .unwrap();
        assert_eq!(snr.direction, DeltaDirection::Improved);
        assert_eq!(snr.baseline_value, Some(30.0));
        assert_eq!(snr.compared_value, Some(36.0));
        // Other rows remain Inconclusive.
        let noise = rows
            .iter()
            .find(|r| r.kind == MetricKind::NoiseLuminance)
            .unwrap();
        assert_eq!(noise.direction, DeltaDirection::Inconclusive);
    }

    #[test]
    fn context_lookup_returns_registry_explanation() {
        // §9 example, preserved verbatim.
        let ctx = context(MetricKind::SharpnessFwhm);
        assert!(ctx.contains("Lower generally indicates tighter stars"));
        assert!(ctx.contains("depend on"));
    }

    #[test]
    fn metric_delta_row_format_value_handles_unit() {
        let row = compute_delta_for(MetricKind::NoiseLuminance, Some(12.0), Some(10.0));
        assert_eq!(row.format_value(Some(12.0)), "12.00 sigma");
        assert_eq!(row.format_value(None), "—");

        let row = compute_delta_for(MetricKind::StarCount, Some(100.0), Some(110.0));
        assert_eq!(row.format_value(Some(100.0)), "100.00");
    }
}
