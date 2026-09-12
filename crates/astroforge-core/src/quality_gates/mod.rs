//! CR-06 P6 — AI Quality Gate engine.
//!
//! Per CR-06 §37, every applied AI operation is
//! validated against a battery of post-operation
//! checks before the user accepts the result. The
//! checks:
//!
//! - `Clipping` — sudden loss of highlight /
//!   shadow detail (channel saturation at 0 or 1).
//! - `NoiseAmplification` — variance increases by
//!   more than `max_variance_increase`.
//! - `StarArtifacts` — high-frequency energy
//!   concentrated around the brightest pixels
//!   (suggesting the operation sharpened stars
//!   unevenly).
//! - `Halos` — local contrast spike at object
//!   boundaries (the operation brightens / darkens
//!   edges more than the surrounding area).
//! - `Ringing` — periodic banding near high-
//!   contrast edges.
//! - `FalseStructures` — high-frequency energy
//!   appears in regions that were flat in the
//!   source (suggesting the operation hallucinated
//!   detail).
//! - `ColorShifts` — hue drift across the image
//!   (per-channel mean change beyond a threshold).
//! - `EdgeArtifacts` — high-frequency energy at
//!   the image border.
//! - `SegmentationLeakage` — brightness in mask-
//!   excluded regions changes more than the
//!   included regions (the operation bled past the
//!   mask).
//! - `ExcessiveSmoothing` — variance drops below a
//!   floor (the operation smoothed out signal).
//!
//! Every gate produces a `GateFinding` with a
//! severity (Ok / Info / Warning / Failure) and a
//! numeric score. The aggregate report carries the
//! findings + the per-finding score + an overall
//! verdict. P6 ships the engine; the apply round
//! (P4) and the recommendation engine (P3) use it
//! to gate accepted operations.

pub mod gates;
pub mod report;

use serde::{Deserialize, Serialize};

/// Per-finding severity. Maps to the §37 example
/// output ("✓ Noise reduced" / "⚠ Moderate star halo
/// increase").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// No issue detected.
    Ok,
    /// Cosmetic note (e.g. "variance moved 5%").
    Info,
    /// User should review before accepting.
    Warning,
    /// Operation should be rejected or re-run.
    Failure,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Ok => "ok",
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Failure => "failure",
        }
    }
}

/// Gate identifier. The names match the §37 checklist
/// so the report serialises into a shape the
/// frontend can render without translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateId {
    Clipping,
    NoiseAmplification,
    StarArtifacts,
    Halos,
    Ringing,
    FalseStructures,
    ColorShifts,
    EdgeArtifacts,
    SegmentationLeakage,
    ExcessiveSmoothing,
}

impl GateId {
    pub fn as_str(&self) -> &'static str {
        match self {
            GateId::Clipping => "clipping",
            GateId::NoiseAmplification => "noise_amplification",
            GateId::StarArtifacts => "star_artifacts",
            GateId::Halos => "halos",
            GateId::Ringing => "ringing",
            GateId::FalseStructures => "false_structures",
            GateId::ColorShifts => "color_shifts",
            GateId::EdgeArtifacts => "edge_artifacts",
            GateId::SegmentationLeakage => "segmentation_leakage",
            GateId::ExcessiveSmoothing => "excessive_smoothing",
        }
    }
}

/// A single gate finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateFinding {
    pub gate: GateId,
    pub severity: Severity,
    /// Human-readable summary (e.g. "Moderate star
    /// halo increase detected").
    pub message: String,
    /// Numeric score (interpretation is per-gate;
    /// higher usually means worse, but the gates
    /// define their own convention).
    pub score: f32,
    /// Optional recommended action (e.g. "Reduce
    /// enhancement strength from 0.7 → 0.5").
    #[serde(default)]
    pub recommendation: Option<String>,
}

/// Per-finding thresholds. The defaults are tuned
/// for astrophotography (the §37 example shows star
/// halo detection at strength 0.72 → 0.54).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateThresholds {
    /// Channel saturation increase that triggers a
    /// clipping warning. The score is the
    /// fractional increase in saturated pixels (at
    /// the bright + dark ends).
    pub clipping_warn: f32,
    pub clipping_fail: f32,
    /// Variance ratio (result / source) that
    /// triggers a noise amplification warning.
    pub noise_amplify_warn: f32,
    pub noise_amplify_fail: f32,
    /// Variance ratio (result / source) below which
    /// the gate fires for excessive smoothing.
    pub smoothing_warn: f32,
    pub smoothing_fail: f32,
    /// Per-channel mean delta that triggers a color
    /// shift warning.
    pub color_shift_warn: f32,
    pub color_shift_fail: f32,
    /// Bright-pixel 3×3 contrast increase that
    /// triggers a star artifact / halo warning.
    pub star_contrast_warn: f32,
    pub star_contrast_fail: f32,
    /// Variance in flat regions of the result that
    /// triggers a false structures warning.
    pub false_structure_warn: f32,
    pub false_structure_fail: f32,
    /// Border-energy ratio that triggers an edge
    /// artifact warning.
    pub edge_artifact_warn: f32,
    pub edge_artifact_fail: f32,
    /// CR-06 P5.1 — segmentation leakage tuning.
    /// Mean absolute change below this floor in the
    /// excluded region is ignored even when the
    /// inside/outside ratio is large.
    #[serde(default = "default_leak_abs_floor")]
    pub leak_abs_floor: f32,
    /// Outside/inside mean-delta ratio that triggers
    /// a leakage warning.
    #[serde(default = "default_leak_warn_ratio")]
    pub leak_warn_ratio: f32,
    /// Outside/inside mean-delta ratio that triggers
    /// a leakage failure.
    #[serde(default = "default_leak_fail_ratio")]
    pub leak_fail_ratio: f32,
}

fn default_leak_abs_floor() -> f32 {
    0.005
}
fn default_leak_warn_ratio() -> f32 {
    0.25
}
fn default_leak_fail_ratio() -> f32 {
    1.0
}

impl Default for GateThresholds {
    fn default() -> Self {
        Self {
            clipping_warn: 0.02,
            clipping_fail: 0.10,
            noise_amplify_warn: 1.40,
            noise_amplify_fail: 2.00,
            smoothing_warn: 0.70,
            smoothing_fail: 0.45,
            color_shift_warn: 0.05,
            color_shift_fail: 0.15,
            star_contrast_warn: 1.30,
            star_contrast_fail: 2.00,
            false_structure_warn: 0.01,
            false_structure_fail: 0.05,
            edge_artifact_warn: 1.50,
            edge_artifact_fail: 2.50,
            leak_abs_floor: default_leak_abs_floor(),
            leak_warn_ratio: default_leak_warn_ratio(),
            leak_fail_ratio: default_leak_fail_ratio(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_strings_match_documentation() {
        assert_eq!(Severity::Ok.as_str(), "ok");
        assert_eq!(Severity::Info.as_str(), "info");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Failure.as_str(), "failure");
    }

    #[test]
    fn gate_id_strings_match_documentation() {
        assert_eq!(GateId::Clipping.as_str(), "clipping");
        assert_eq!(GateId::NoiseAmplification.as_str(), "noise_amplification");
        assert_eq!(GateId::StarArtifacts.as_str(), "star_artifacts");
        assert_eq!(GateId::Halos.as_str(), "halos");
        assert_eq!(GateId::Ringing.as_str(), "ringing");
        assert_eq!(GateId::FalseStructures.as_str(), "false_structures");
        assert_eq!(GateId::ColorShifts.as_str(), "color_shifts");
        assert_eq!(GateId::EdgeArtifacts.as_str(), "edge_artifacts");
        assert_eq!(GateId::SegmentationLeakage.as_str(), "segmentation_leakage");
        assert_eq!(GateId::ExcessiveSmoothing.as_str(), "excessive_smoothing");
    }

    #[test]
    fn thresholds_default_to_reasonable_values() {
        let t = GateThresholds::default();
        // Default thresholds are tuned so the
        // typical passthrough (no change) produces
        // Ok findings.
        assert!(t.clipping_warn < t.clipping_fail);
        assert!(t.noise_amplify_warn < t.noise_amplify_fail);
        assert!(t.smoothing_warn > t.smoothing_fail);
    }
}
