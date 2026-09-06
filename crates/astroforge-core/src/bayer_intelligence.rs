//! CR-04 §7 — Bayer Pattern Intelligence.
//!
//! Wraps the existing `bayer_detection::detect_bayer` and
//! `bayer_detection::match_camera_signature` with the
//! four-state taxonomy from CR-04 §7:
//!
//! 1. Explicit metadata (FITS BAYERPAT keyword) — confidence 1.0
//! 2. Strong inferred detection (statistical autocorrelation +
//!    camera signature) — confidence ~0.7–0.9
//! 3. Weak inference (statistical only, low confidence) —
//!    requires confirmation, confidence ~0.4–0.7
//! 4. No Bayer pattern (RGB already, or monochrome already
//!    debayered) — confidence 1.0
//!
//! Each decision carries:
//! - the `BayerPattern` if applicable
//! - a confidence score 0.0..=1.0
//! - a list of `BayerObservation`s so the UI can show
//!   "Observation → Evidence → Confidence → Decision" (per
//!   CR-04 §2.3 ADR-04.2)
//! - a `BayerRoute` action so the UI knows whether to
//!   auto-proceed, prompt the user, or fall back to AssumeRGB

use crate::bayer_detection::{self, DetectionMethod};
use crate::debayer::BayerPattern;
use crate::import_scan::ExtractedMetadata;
use serde::{Deserialize, Serialize};

/// CR-04 §7 — the four-state taxonomy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BayerInferenceKind {
    /// FITS BAYERPAT keyword explicitly set.
    ExplicitMetadata,
    /// Strong autocorrelation + camera signature match.
    StrongInferred,
    /// Weak statistical evidence; user confirmation needed.
    WeakInference,
    /// No Bayer pattern (RGB, monochrome, or already debayered).
    NoBayerPattern,
}

impl BayerInferenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BayerInferenceKind::ExplicitMetadata => "explicit_metadata",
            BayerInferenceKind::StrongInferred => "strong_inferred",
            BayerInferenceKind::WeakInference => "weak_inference",
            BayerInferenceKind::NoBayerPattern => "no_bayer_pattern",
        }
    }
}

/// One piece of evidence that contributed to a Bayer decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayerObservation {
    pub signal: String,
    pub value: String,
    pub weight: f64,
    pub confidence: f64,
}

impl BayerObservation {
    pub fn new(
        signal: impl Into<String>,
        value: impl Into<String>,
        weight: f64,
        confidence: f64,
    ) -> Self {
        Self {
            signal: signal.into(),
            value: value.into(),
            weight,
            confidence,
        }
    }
}

/// CR-04 §7 — the result of Bayer pattern intelligence for one file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayerIntelligence {
    pub kind: BayerInferenceKind,
    pub pattern: Option<BayerPattern>,
    pub confidence: f64,
    pub observations: Vec<BayerObservation>,
    pub route: BayerRoute,
}

/// CR-04 §7 + §13 — the action the UI should take.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BayerRoute {
    /// Confidence is high enough to auto-proceed.
    AutoProceed,
    /// Confidence is in the medium range; prompt the user.
    PromptUser,
    /// No Bayer pattern; skip debayer.
    NoDebayerNeeded,
}

impl BayerRoute {
    pub fn as_str(self) -> &'static str {
        match self {
            BayerRoute::AutoProceed => "auto_proceed",
            BayerRoute::PromptUser => "prompt_user",
            BayerRoute::NoDebayerNeeded => "no_debayer_needed",
        }
    }
}

/// CR-04 §7 — run Bayer intelligence on a single file's metadata.
///
/// Inputs:
/// - `metadata`: the `ExtractedMetadata` from `import_scan`
///   (P1). Carries the FITS BAYERPAT keyword if present, the
///   image dimensions (for camera signature matching), and
///   channel count (for the NoBayerPattern fallback).
///
/// Output:
/// - A typed `BayerIntelligence` describing the kind, the
///   pattern, the confidence, the evidence, and the route the
///   UI should take.
///
/// The `image_based_detection` argument is `Some(image)` when
/// the caller has access to the decoded image (P9's review step
/// can pass it in); otherwise we rely on metadata + camera
/// signatures + heuristic channels-from-dimensions alone.
pub fn analyze(
    metadata: &ExtractedMetadata,
    image_based_detection: Option<bayer_detection::BayerDetectionResult>,
) -> BayerIntelligence {
    let mut observations: Vec<BayerObservation> = Vec::new();

    // 1. Explicit metadata: FITS BAYERPAT keyword.
    if let Some(bayerpat) = metadata.bayerpat.as_deref() {
        if let Some(pattern) = BayerPattern::parse(bayerpat) {
            observations.push(BayerObservation::new("fits_bayerpat", bayerpat, 1.0, 1.0));
            return BayerIntelligence {
                kind: BayerInferenceKind::ExplicitMetadata,
                pattern: Some(pattern),
                confidence: 1.0,
                observations,
                route: BayerRoute::AutoProceed,
            };
        } else {
            // Unknown pattern value — record it but don't trust it.
            observations.push(BayerObservation::new(
                "fits_bayerpat_unknown",
                bayerpat,
                0.3,
                0.2,
            ));
        }
    }

    // 2. Camera signature match: if dimensions match a known
    //    camera in the database, take the pattern from there.
    if let (Some(w), Some(h)) = (metadata.width, metadata.height) {
        if let Some(sig) = bayer_detection::match_camera_signature(w as usize, h as usize) {
            observations.push(BayerObservation::new(
                "camera_signature",
                format!("{} ({}) {}x{}", sig.name, sig.sensor, sig.width, sig.height),
                0.9,
                0.85,
            ));
            return BayerIntelligence {
                kind: BayerInferenceKind::StrongInferred,
                pattern: Some(sig.pattern),
                confidence: 0.85,
                observations,
                route: BayerRoute::AutoProceed,
            };
        } else {
            observations.push(BayerObservation::new(
                "camera_signature",
                format!("no_match_for_{}x{}", w, h),
                0.1,
                0.2,
            ));
        }
    }

    // 3. Statistical detection: only available if the caller has
    //    decoded it via bayer_detection::detect_bayer. We trust
    //    the existing confidence score but reshape it into the
    //    four-state taxonomy.
    if let Some(detection) = image_based_detection {
        let weight = match detection.method {
            DetectionMethod::Metadata => 1.0,
            DetectionMethod::Statistical => 0.7,
            DetectionMethod::CameraSignature => 0.9,
            DetectionMethod::AssumeRGB => 0.5,
        };
        observations.push(BayerObservation::new(
            "statistical_detection",
            format!(
                "{:?} confidence={:.2}",
                detection.method, detection.confidence
            ),
            weight,
            detection.confidence,
        ));

        if !detection.is_bayer {
            return BayerIntelligence {
                kind: BayerInferenceKind::NoBayerPattern,
                pattern: None,
                confidence: detection.confidence,
                observations,
                route: BayerRoute::NoDebayerNeeded,
            };
        }

        if detection.confidence > 0.7 {
            return BayerIntelligence {
                kind: BayerInferenceKind::StrongInferred,
                pattern: detection.pattern,
                confidence: detection.confidence,
                observations,
                route: BayerRoute::AutoProceed,
            };
        }

        return BayerIntelligence {
            kind: BayerInferenceKind::WeakInference,
            pattern: detection.pattern,
            confidence: detection.confidence,
            observations,
            route: BayerRoute::PromptUser,
        };
    }

    // 4. No statistical detection was performed. If the file
    //    declares 3+ channels (RGB already), it's NoBayerPattern.
    //    If it declares 1 channel (monochrome), it's WeakInference
    //    with no pattern (the camera signature or the user must
    //    supply one).
    if let Some(c) = channel_count_from_metadata(metadata) {
        if c >= 3 {
            observations.push(BayerObservation::new("channels", format!("{c}"), 0.5, 1.0));
            return BayerIntelligence {
                kind: BayerInferenceKind::NoBayerPattern,
                pattern: None,
                confidence: 1.0,
                observations,
                route: BayerRoute::NoDebayerNeeded,
            };
        }
        if c == 1 {
            observations.push(BayerObservation::new("channels", "1", 0.4, 0.5));
            return BayerIntelligence {
                kind: BayerInferenceKind::WeakInference,
                pattern: None,
                confidence: 0.4,
                observations,
                route: BayerRoute::PromptUser,
            };
        }
    }

    // 5. Fall back: no signal at all. Tell the UI to prompt.
    observations.push(BayerObservation::new(
        "no_signal",
        "no_bayerpat_no_camera_no_channels",
        0.1,
        0.1,
    ));
    BayerIntelligence {
        kind: BayerInferenceKind::WeakInference,
        pattern: None,
        confidence: 0.2,
        observations,
        route: BayerRoute::PromptUser,
    }
}

/// Best-effort inference of channel count from FITS NAXIS.
///
/// FITS doesn't have a canonical "channels" keyword the way
/// modern image formats do, but NAXIS=3 with NAXIS3 in {1,3,4}
/// is the typical case. We also fall back to the file size
/// heuristic for PNG/JPEG (1 = grayscale, 3 = RGB, 4 = RGBA)
/// once real EXIF parsing lands in P9.
fn channel_count_from_metadata(metadata: &ExtractedMetadata) -> Option<u8> {
    // P1's extract_metadata doesn't yet populate channels. The
    // heuristic below is a placeholder; P9 can extend it once
    // EXIF parsing lands.
    let _ = metadata;
    None
}

/// CR-04 §7 — apply the existing bayer_detection::route_by_confidence
/// to a `BayerIntelligence`. Wraps the existing routing logic
/// with the four-state taxonomy.
pub fn reroute(intelligence: BayerIntelligence) -> BayerIntelligence {
    let route = match (intelligence.kind, intelligence.confidence) {
        (BayerInferenceKind::ExplicitMetadata, _) => BayerRoute::AutoProceed,
        (BayerInferenceKind::StrongInferred, _) => BayerRoute::AutoProceed,
        (BayerInferenceKind::NoBayerPattern, _) => BayerRoute::NoDebayerNeeded,
        (BayerInferenceKind::WeakInference, c) if c > 0.5 => BayerRoute::PromptUser,
        (BayerInferenceKind::WeakInference, _) => BayerRoute::PromptUser,
    };
    BayerIntelligence {
        route,
        ..intelligence
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_scan::ExtractedMetadata;

    fn metadata_with_bayerpat(bayerpat: &str) -> ExtractedMetadata {
        ExtractedMetadata {
            bayerpat: Some(bayerpat.into()),
            ..Default::default()
        }
    }

    fn metadata_with_dimensions(w: u32, h: u32) -> ExtractedMetadata {
        ExtractedMetadata {
            width: Some(w),
            height: Some(h),
            ..Default::default()
        }
    }

    #[test]
    fn explicit_bayerpat_yields_explicit_metadata() {
        let md = metadata_with_bayerpat("RGGB");
        let intel = analyze(&md, None);
        assert_eq!(intel.kind, BayerInferenceKind::ExplicitMetadata);
        assert_eq!(intel.pattern, Some(BayerPattern::RGGB));
        assert_eq!(intel.confidence, 1.0);
        assert_eq!(intel.route, BayerRoute::AutoProceed);
    }

    #[test]
    fn unknown_bayerpat_falls_through_to_prompt() {
        let md = metadata_with_bayerpat("XYZ");
        let intel = analyze(&md, None);
        // Falls through to the no-signal fallback.
        assert_eq!(intel.kind, BayerInferenceKind::WeakInference);
        assert_eq!(intel.route, BayerRoute::PromptUser);
    }

    #[test]
    fn camera_signature_match_yields_strong_inferred() {
        // Seestar S50: 1920x1080 in the camera DB.
        let md = metadata_with_dimensions(1920, 1080);
        let intel = analyze(&md, None);
        assert_eq!(intel.kind, BayerInferenceKind::StrongInferred);
        assert_eq!(intel.pattern, Some(BayerPattern::RGGB));
        assert_eq!(intel.route, BayerRoute::AutoProceed);
    }

    #[test]
    fn statistical_detection_strong_yields_strong_inferred() {
        let md = ExtractedMetadata::default();
        let detection = bayer_detection::BayerDetectionResult {
            is_bayer: true,
            confidence: 0.95,
            pattern: Some(BayerPattern::GRBG),
            method: DetectionMethod::Statistical,
        };
        let intel = analyze(&md, Some(detection));
        assert_eq!(intel.kind, BayerInferenceKind::StrongInferred);
        assert_eq!(intel.pattern, Some(BayerPattern::GRBG));
        assert!(intel.confidence >= 0.7);
        assert_eq!(intel.route, BayerRoute::AutoProceed);
    }

    #[test]
    fn statistical_detection_weak_yields_weak_inference() {
        let md = ExtractedMetadata::default();
        let detection = bayer_detection::BayerDetectionResult {
            is_bayer: true,
            confidence: 0.45,
            pattern: Some(BayerPattern::BGGR),
            method: DetectionMethod::Statistical,
        };
        let intel = analyze(&md, Some(detection));
        assert_eq!(intel.kind, BayerInferenceKind::WeakInference);
        assert_eq!(intel.route, BayerRoute::PromptUser);
    }

    #[test]
    fn statistical_detection_not_bayer_yields_no_bayer_pattern() {
        let md = ExtractedMetadata::default();
        let detection = bayer_detection::BayerDetectionResult {
            is_bayer: false,
            confidence: 0.95,
            pattern: None,
            method: DetectionMethod::AssumeRGB,
        };
        let intel = analyze(&md, Some(detection));
        assert_eq!(intel.kind, BayerInferenceKind::NoBayerPattern);
        assert_eq!(intel.pattern, None);
        assert_eq!(intel.route, BayerRoute::NoDebayerNeeded);
    }

    #[test]
    fn no_signal_yields_weak_inference_with_low_confidence() {
        let md = ExtractedMetadata::default();
        let intel = analyze(&md, None);
        assert_eq!(intel.kind, BayerInferenceKind::WeakInference);
        assert!(intel.confidence < 0.5);
        assert_eq!(intel.route, BayerRoute::PromptUser);
    }

    #[test]
    fn reroute_preserves_strong_inferred() {
        let intel = BayerIntelligence {
            kind: BayerInferenceKind::StrongInferred,
            pattern: Some(BayerPattern::RGGB),
            confidence: 0.85,
            observations: vec![],
            route: BayerRoute::PromptUser, // mis-routed; reroute fixes it
        };
        let rerouted = reroute(intel);
        assert_eq!(rerouted.route, BayerRoute::AutoProceed);
    }

    #[test]
    fn observations_are_recorded_for_audit() {
        let md = metadata_with_bayerpat("RGGB");
        let intel = analyze(&md, None);
        let signals: Vec<&str> = intel
            .observations
            .iter()
            .map(|o| o.signal.as_str())
            .collect();
        assert!(signals.contains(&"fits_bayerpat"));
    }
}
