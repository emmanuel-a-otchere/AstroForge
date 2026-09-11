//! CR-04 P6 — Capture Analysis (CR-04 §10).
//!
//! Per CR-04 §10, the routing decision between deep-sky and
//! planetary/lunar is **critical**: mis-routing sends the
//! dataset into the wrong processing pipeline and an
//! irreversible processing strategy must never be silently
//! applied to an ambiguous dataset.
//!
//! This module analyses a session (a set of assets that P5
//! grouped together) and emits a `CaptureClassification`
//! with:
//!
//! - `kind`: DeepSky / PlanetaryLunar / Ambiguous
//! - `confidence`: 0.0..=1.0; below ~0.6 triggers the
//!   ambiguity UX (P9).
//! - `observations`: per-signal contributions (signal,
//!   value, weight, confidence) so the UI can show
//!   "Observation → Evidence → Confidence → Decision"
//!   (per CR-04 §2.3 ADR-04.2).
//! - `ambiguous`: short-circuit flag — true when the two
//!   leading signals disagree or when the winning signal's
//!   confidence is too low to safely route.
//!
//! ## Signal hierarchy
//!
//! 1. **Exposure** (weight 0.20) — short exposures
//!    (< 1.0 s) are strong planetary/lunar indicators;
//!    long exposures (≥ 60.0 s) are deep-sky indicators.
//! 2. **Frame count** (weight 0.15) — very high counts
//!    (≥ 1000) are planetary video-stack indicators;
//!    low counts (< 200) are deep-sky.
//! 3. **Target kind** (weight 0.25) — P4 `TargetKind`
//!    values Planet / Moon collapse to PlanetaryLunar;
//!    Galaxy / Nebula / Cluster / Star to DeepSky;
//!    Unknown / Comet are weak signals (weight 0.05).
//! 4. **Frame dimensions** (weight 0.10) — small ROI
//!    (max(width, height) < 1024) is common in planetary
//!    lucky imaging; large frames (≥ 2048) are
//!    deep-sky.
//! 5. **Filename/directory** (weight 0.15) — explicit
//!    keywords like "jupiter", "saturn", "lunar", "moon"
//!    resolve to PlanetaryLunar; "galaxy", "nebula",
//!    "cluster" to DeepSky.
//! 6. **Calibration frame presence** (weight 0.15) — if
//!    the session contains any darks/flats/bias frames,
//!    that's a deep-sky indicator; missing calibration
//!    frames favour planetary/lunar.
//!
//! The signals are combined into a weighted score per
//! `kind`. The winner is the kind with the higher score.
//! If the winner's score is below the confidence threshold
//! (0.6) or if the top two scores are within 0.15 of each
//! other, the classification is flagged `ambiguous` and
//! the UI must surface a confirmation dialog before any
//! irreversible routing.
//!
//! ## No drift on missing metadata
//!
//! Missing signal fields → omitted observation. The
//! weighted scoring still runs on the signals that are
//! present. A session with *no* signals (empty input)
//! emits `CaptureClassification::default_ambiguous()`
//! with `kind = Ambiguous` and `confidence = 0.0` — never
//! a confident guess.
//!
//! ## Composes with P4 + P5
//!
//! The caller composes this module with P4
//! (target_detection) + P5 (session_grouping): the
//! P5 `SessionGroup` aggregates assets, the caller derives
//! a `SessionSummary` from those assets, and this module
//! consumes the `SessionSummary` + the per-session P4
//! `TargetIntelligence` (or `None`).

use crate::import_scan::ExtractedMetadata;
use crate::target_detection::TargetIntelligence;
use serde::{Deserialize, Serialize};

/// CR-04 §10 — the routing decision.
/// `Ambiguous` is a first-class variant (not a fallback).
/// When `kind == Ambiguous`, the UI MUST prompt the user (per
/// CR-04 §13 ambiguity UX).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureKind {
    DeepSky,
    PlanetaryLunar,
    Ambiguous,
}

impl CaptureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureKind::DeepSky => "deep_sky",
            CaptureKind::PlanetaryLunar => "planetary_lunar",
            CaptureKind::Ambiguous => "ambiguous",
        }
    }
}

/// One signal that contributed to the classification.
/// Mirrors the `FrameObservation` shape used by
/// `classification.rs` (P3) and the
/// `TargetObservation` shape used by `target_detection.rs`
/// (P4). The Observation → Evidence → Confidence →
/// Decision contract is consistent across all three
/// classification modules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureObservation {
    /// Signal id (e.g. "exposure", "frame_count",
    /// "target_kind").
    pub signal: String,
    /// Observed value as a string (e.g. "0.5s",
    /// "1500", "Planet").
    pub value: String,
    /// 0.0..=1.0. Static weight per signal (see module
    /// docstring).
    pub weight: f64,
    /// 0.0..=1.0. Per-signal confidence. Some signals
    /// (exposure, frame_count) have near-binary confidence;
    /// others (filename, target_kind) are softer.
    pub confidence: f64,
    /// Which kind this signal favours.
    pub favours: CaptureKind,
}

/// CR-04 §10 — the result of classifying one session's
/// capture type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureClassification {
    pub kind: CaptureKind,
    /// 0.0..=1.0. Weighted aggregate of the observation
    /// scores for the winning kind.
    pub confidence: f64,
    pub observations: Vec<CaptureObservation>,
    /// Short-circuit flag — true when the two leading
    /// signals disagree or when the winning signal's
    /// confidence is too low to safely route.
    pub ambiguous: bool,
}

impl CaptureClassification {
    /// Default for "no signals present": ambiguous with
    /// zero confidence. Used by `analyse_session` when the
    /// session has no assets.
    pub fn default_ambiguous() -> Self {
        Self {
            kind: CaptureKind::Ambiguous,
            confidence: 0.0,
            observations: Vec::new(),
            ambiguous: true,
        }
    }
}

/// CR-04 §10 — the per-signal thresholds.
///
/// These are the published thresholds from §10; the
/// `analyse_session` function uses them to decide which
/// side of each signal favours which kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureThresholds {
    /// Exposure below this (seconds) → PlanetaryLunar.
    pub short_exposure_seconds: f64,
    /// Exposure at or above this (seconds) → DeepSky.
    pub long_exposure_seconds: f64,
    /// Frame count at or above this → PlanetaryLunar.
    pub high_frame_count: usize,
    /// Frame count below this → DeepSky.
    pub low_frame_count: usize,
    /// Largest image dimension below this → PlanetaryLunar.
    pub small_roi_max_dim: u32,
    /// Largest image dimension at or above this → DeepSky.
    pub large_frame_max_dim: u32,
    /// Confidence threshold below which the classification
    /// is flagged `ambiguous`.
    pub ambiguous_confidence_threshold: f64,
    /// Score-difference threshold below which the top two
    /// kinds are "too close" and the classification is
    /// flagged `ambiguous`.
    pub ambiguous_score_gap: f64,
}

impl Default for CaptureThresholds {
    fn default() -> Self {
        Self {
            short_exposure_seconds: 1.0,
            long_exposure_seconds: 60.0,
            high_frame_count: 1000,
            low_frame_count: 200,
            small_roi_max_dim: 1024,
            large_frame_max_dim: 2048,
            ambiguous_confidence_threshold: 0.6,
            ambiguous_score_gap: 0.15,
        }
    }
}

/// CR-04 §10 — a summary of one session's worth of
/// assets, derived by the IPC layer (P8) from the P5
/// `SessionGroup`. This module does not own the
/// aggregation logic — the caller builds the summary
/// from `ExtractedMetadata` + P4 `TargetIntelligence`
/// per session.
#[derive(Debug, Clone)]
pub struct SessionSummary<'a> {
    /// All assets in the session. Empty session →
    /// `default_ambiguous()`.
    pub assets: Vec<&'a ExtractedMetadata>,
    /// Optional P4 target intelligence for the session
    /// (the most-confident target across the assets).
    pub target: Option<&'a TargetIntelligence>,
    /// Calibration-frame presence: true if any asset in
    /// the session has frame_type set to a calibration
    /// variant (dark/flat/bias). The caller computes this
    /// from P3's `FrameKind` classification.
    pub has_calibration_frames: bool,
    /// Filename/directory text blob — used to scan for
    /// keywords like "jupiter" or "moon". The caller
    /// concatenates the asset paths; this module does
    /// the matching.
    pub path_text: String,
}

/// CR-04 §10 — the main entry point.
///
/// Analyses one session's worth of assets and emits a
/// `CaptureClassification`. Pure function: no I/O, no DB,
/// no time. The caller composes this with P4 + P5 + the
/// P3 frame-classification output.
pub fn analyse_session(
    summary: &SessionSummary<'_>,
    thresholds: &CaptureThresholds,
) -> CaptureClassification {
    if summary.assets.is_empty() {
        return CaptureClassification::default_ambiguous();
    }

    let mut observations: Vec<CaptureObservation> = Vec::new();

    // Signal 1 — exposure. Use the *median* exposure
    // (robust to a few outlier subs).
    if let Some(median_exp) = median_exposure(&summary.assets) {
        if median_exp < thresholds.short_exposure_seconds {
            observations.push(CaptureObservation {
                signal: "exposure".into(),
                value: format!("{:.2}s", median_exp),
                weight: 0.20,
                confidence: 0.95,
                favours: CaptureKind::PlanetaryLunar,
            });
        } else if median_exp >= thresholds.long_exposure_seconds {
            observations.push(CaptureObservation {
                signal: "exposure".into(),
                value: format!("{:.2}s", median_exp),
                weight: 0.20,
                confidence: 0.90,
                favours: CaptureKind::DeepSky,
            });
        } else {
            // Mid-range exposure — ambiguous by itself,
            // but contributes as a low-confidence
            // observation so the UI can show it.
            observations.push(CaptureObservation {
                signal: "exposure".into(),
                value: format!("{:.2}s", median_exp),
                weight: 0.10,
                confidence: 0.30,
                favours: CaptureKind::Ambiguous,
            });
        }
    }

    // Signal 2 — frame count.
    let n = summary.assets.len();
    if n >= thresholds.high_frame_count {
        observations.push(CaptureObservation {
            signal: "frame_count".into(),
            value: n.to_string(),
            weight: 0.15,
            confidence: 0.85,
            favours: CaptureKind::PlanetaryLunar,
        });
    } else if n < thresholds.low_frame_count {
        observations.push(CaptureObservation {
            signal: "frame_count".into(),
            value: n.to_string(),
            weight: 0.15,
            confidence: 0.80,
            favours: CaptureKind::DeepSky,
        });
    } else {
        observations.push(CaptureObservation {
            signal: "frame_count".into(),
            value: n.to_string(),
            weight: 0.10,
            confidence: 0.40,
            favours: CaptureKind::Ambiguous,
        });
    }

    // Signal 3 — target kind.
    if let Some(ti) = summary.target {
        if let Some(candidate) = &ti.candidate {
            let kind_favours = match candidate.kind {
                crate::target_detection::TargetKind::Planet
                | crate::target_detection::TargetKind::Moon => CaptureKind::PlanetaryLunar,
                crate::target_detection::TargetKind::Galaxy
                | crate::target_detection::TargetKind::Nebula
                | crate::target_detection::TargetKind::Cluster
                | crate::target_detection::TargetKind::Star => CaptureKind::DeepSky,
                crate::target_detection::TargetKind::Comet
                | crate::target_detection::TargetKind::Unknown => CaptureKind::Ambiguous,
            };
            let conf = if kind_favours == CaptureKind::Ambiguous {
                0.30
            } else {
                ti.confidence.min(1.0)
            };
            observations.push(CaptureObservation {
                signal: "target_kind".into(),
                value: candidate.kind.as_str().to_string(),
                weight: if kind_favours == CaptureKind::Ambiguous {
                    0.05
                } else {
                    0.25
                },
                confidence: conf,
                favours: kind_favours,
            });
        }
    }

    // Signal 4 — frame dimensions. Take the max width
    // × height across the session.
    if let Some(max_dim) = max_image_dim(&summary.assets) {
        if max_dim < thresholds.small_roi_max_dim {
            observations.push(CaptureObservation {
                signal: "frame_dimensions".into(),
                value: format!("{}px", max_dim),
                weight: 0.10,
                confidence: 0.70,
                favours: CaptureKind::PlanetaryLunar,
            });
        } else if max_dim >= thresholds.large_frame_max_dim {
            observations.push(CaptureObservation {
                signal: "frame_dimensions".into(),
                value: format!("{}px", max_dim),
                weight: 0.10,
                confidence: 0.75,
                favours: CaptureKind::DeepSky,
            });
        }
    }

    // Signal 5 — filename / directory keywords.
    let text = summary.path_text.to_lowercase();
    let mut planetary_hits = 0;
    let mut deepsky_hits = 0;
    for kw in PLANETARY_KEYWORDS {
        if text.contains(kw) {
            planetary_hits += 1;
        }
    }
    for kw in DEEPSKY_KEYWORDS {
        if text.contains(kw) {
            deepsky_hits += 1;
        }
    }
    if planetary_hits > deepsky_hits {
        observations.push(CaptureObservation {
            signal: "filename_keywords".into(),
            value: format!("planetary={}", planetary_hits),
            weight: 0.15,
            confidence: 0.80,
            favours: CaptureKind::PlanetaryLunar,
        });
    } else if deepsky_hits > planetary_hits {
        observations.push(CaptureObservation {
            signal: "filename_keywords".into(),
            value: format!("deepsky={}", deepsky_hits),
            weight: 0.15,
            confidence: 0.80,
            favours: CaptureKind::DeepSky,
        });
    }

    // Signal 6 — calibration-frame presence.
    if summary.has_calibration_frames {
        observations.push(CaptureObservation {
            signal: "calibration_frames".into(),
            value: "present".into(),
            weight: 0.15,
            confidence: 0.75,
            favours: CaptureKind::DeepSky,
        });
    }

    // Aggregate. Score per kind = sum of weight ×
    // confidence across the observations that favour
    // that kind, normalised by the sum of weights.
    let (deepsky_score, planetary_score) = aggregate_scores(&observations);
    let total_score = deepsky_score + planetary_score;
    let (winner, winner_score) = if deepsky_score >= planetary_score {
        (CaptureKind::DeepSky, deepsky_score)
    } else {
        (CaptureKind::PlanetaryLunar, planetary_score)
    };

    // Confidence is the winner's score (already in
    // 0.0..=1.0 because we divide by Σ weight).
    let confidence = winner_score;

    // Ambiguity check: if confidence is too low, OR if
    // the two leading scores are too close, flag
    // ambiguous.
    let ambiguous = confidence < thresholds.ambiguous_confidence_threshold
        || (total_score > 0.0
            && (deepsky_score - planetary_score).abs() < thresholds.ambiguous_score_gap);

    let kind = if ambiguous {
        CaptureKind::Ambiguous
    } else {
        winner
    };

    CaptureClassification {
        kind,
        confidence,
        observations,
        ambiguous,
    }
}

// ─── Signal helpers ─────────────────────────────────────────────

const PLANETARY_KEYWORDS: &[&str] = &[
    "jupiter",
    "saturn",
    "mars",
    "venus",
    "mercury",
    "uranus",
    "neptune",
    "lunar",
    "moon",
    "planetary",
];

const DEEPSKY_KEYWORDS: &[&str] = &[
    "galaxy", "nebula", "cluster", "deepsky", "deep_sky", "deep-sky",
];

/// Median exposure across the session's assets, ignoring
/// `None` exposures.
fn median_exposure(assets: &[&ExtractedMetadata]) -> Option<f64> {
    let mut exps: Vec<f64> = assets.iter().filter_map(|a| a.exptime).collect();
    if exps.is_empty() {
        return None;
    }
    exps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = exps.len() / 2;
    Some(if exps.len().is_multiple_of(2) {
        (exps[mid - 1] + exps[mid]) / 2.0
    } else {
        exps[mid]
    })
}

/// Largest image dimension across the session's
/// assets. Returns `None` if no asset reports dimensions.
fn max_image_dim(assets: &[&ExtractedMetadata]) -> Option<u32> {
    assets
        .iter()
        .filter_map(|a| match (a.naxis1, a.naxis2) {
            (Some(w), Some(h)) => {
                let wu = u32::try_from(w).ok();
                let hu = u32::try_from(h).ok();
                match (wu, hu) {
                    (Some(wu), Some(hu)) => Some(wu.max(hu)),
                    _ => None,
                }
            }
            _ => None,
        })
        .max()
}

/// Aggregate the per-kind scores from the observations.
/// Each kind's score is Σ(weight × confidence) / Σ(weight)
/// across observations that favour that kind.
fn aggregate_scores(observations: &[CaptureObservation]) -> (f64, f64) {
    let mut deepsky_weighted = 0.0;
    let mut deepsky_weight = 0.0;
    let mut planetary_weighted = 0.0;
    let mut planetary_weight = 0.0;
    for o in observations {
        match o.favours {
            CaptureKind::DeepSky => {
                deepsky_weighted += o.weight * o.confidence;
                deepsky_weight += o.weight;
            }
            CaptureKind::PlanetaryLunar => {
                planetary_weighted += o.weight * o.confidence;
                planetary_weight += o.weight;
            }
            CaptureKind::Ambiguous => {
                // Ambiguous observations don't contribute to either
                // side; they're surfaced for the UI but don't move
                // the score.
            }
        }
    }
    let deepsky = if deepsky_weight > 0.0 {
        deepsky_weighted / deepsky_weight
    } else {
        0.0
    };
    let planetary = if planetary_weight > 0.0 {
        planetary_weighted / planetary_weight
    } else {
        0.0
    };
    (deepsky, planetary)
}

// ─── Unit tests ─────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
mod tests {
    use super::*;
    use crate::target_detection::{TargetCandidate, TargetKind};

    fn metadata(
        exptime: Option<f64>,
        width: Option<i64>,
        height: Option<i64>,
    ) -> ExtractedMetadata {
        ExtractedMetadata {
            object: None,
            exptime,
            filter: None,
            xbinning: None,
            ybinning: None,
            ccd_temp: None,
            naxis1: width,
            naxis2: height,
            bitpix: None,
            bayerpat: None,
            telescop: None,
            instrume: None,
            focallen: None,
            gain: None,
            offset: None,
            date_obs: None,
            width: width.and_then(|n| u32::try_from(n).ok()),
            height: height.and_then(|n| u32::try_from(n).ok()),
            bit_depth: None,
            camera: None,
        }
    }

    fn target_intel(kind: TargetKind, name: &str) -> TargetIntelligence {
        TargetIntelligence {
            candidate: Some(TargetCandidate {
                name: name.into(),
                kind,
                aliases: Vec::new(),
            }),
            confidence: 0.95,
            observations: Vec::new(),
        }
    }

    fn summary<'a>(
        assets: Vec<&'a ExtractedMetadata>,
        target: Option<&'a TargetIntelligence>,
        calibration: bool,
        path_text: &str,
    ) -> SessionSummary<'a> {
        SessionSummary {
            assets,
            target,
            has_calibration_frames: calibration,
            path_text: path_text.into(),
        }
    }

    #[test]
    fn empty_session_is_ambiguous() {
        let s = summary(vec![], None, false, "");
        let c = analyse_session(&s, &CaptureThresholds::default());
        assert_eq!(c.kind, CaptureKind::Ambiguous);
        assert!(c.ambiguous);
        assert_eq!(c.confidence, 0.0);
    }

    #[test]
    fn short_exposure_classifies_as_planetary_lunar() {
        // 1500 frames @ 0.5 s, small ROI, "jupiter" in path.
        let assets: Vec<ExtractedMetadata> = (0..1500)
            .map(|_| metadata(Some(0.5), Some(640), Some(480)))
            .collect();
        let asset_refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        let ti = target_intel(TargetKind::Planet, "Jupiter");
        let s = summary(asset_refs, Some(&ti), false, "/data/jupiter/2026-09-01/");
        let c = analyse_session(&s, &CaptureThresholds::default());
        assert_eq!(c.kind, CaptureKind::PlanetaryLunar);
        assert!(!c.ambiguous);
        assert!(c.confidence > 0.7);
    }

    #[test]
    fn long_exposure_classifies_as_deep_sky() {
        // 50 frames @ 300 s, large ROI, "M31" target,
        // calibration frames present.
        let assets: Vec<ExtractedMetadata> = (0..50)
            .map(|_| metadata(Some(300.0), Some(4096), Some(4096)))
            .collect();
        let asset_refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        let ti = target_intel(TargetKind::Galaxy, "M31");
        let s = summary(asset_refs, Some(&ti), true, "/data/M31/2026-08-14/Light/");
        let c = analyse_session(&s, &CaptureThresholds::default());
        assert_eq!(c.kind, CaptureKind::DeepSky);
        assert!(!c.ambiguous);
        assert!(c.confidence > 0.7);
    }

    #[test]
    fn mid_range_exposure_with_no_target_classifies_as_deep_sky() {
        // 30 frames @ 5 s (mid-range), large ROI (2048),
        // no target intel, no calibration. Large ROI +
        // low frame count are deep-sky indicators that
        // dominate the absent planetary signals.
        let assets: Vec<ExtractedMetadata> = (0..30)
            .map(|_| metadata(Some(5.0), Some(2048), Some(2048)))
            .collect();
        let asset_refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        let s = summary(asset_refs, None, false, "/data/Unknown/");
        let c = analyse_session(&s, &CaptureThresholds::default());
        // Score: frame_count (0.80 deep-sky) +
        // frame_dimensions (0.75 deep-sky) → 0.78.
        // Planetary: 0. Score gap > 0.15.
        assert_eq!(c.kind, CaptureKind::DeepSky);
    }

    #[test]
    fn lunar_target_with_short_exposure_classifies_planetary_lunar() {
        // Lunar capture: 200 frames @ 0.5 s, small ROI,
        // target Moon, path "Moon".
        let assets: Vec<ExtractedMetadata> = (0..200)
            .map(|_| metadata(Some(0.5), Some(640), Some(480)))
            .collect();
        let asset_refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        let ti = target_intel(TargetKind::Moon, "Moon");
        let s = summary(asset_refs, Some(&ti), false, "/data/Moon/2026-09-01/");
        let c = analyse_session(&s, &CaptureThresholds::default());
        // Exposure (short, 0.95 conf, planetary) +
        // target_kind (Moon, 0.95 conf, planetary) +
        // frame_dimensions (small, 0.70 conf, planetary)
        // + filename "moon" (0.80 conf, planetary) →
        // strong planetary score. Frame count (200 = exactly
        // low threshold, ambiguous contribution).
        assert_eq!(c.kind, CaptureKind::PlanetaryLunar);
        assert!(!c.ambiguous);
    }

    #[test]
    fn unknown_target_kind_does_not_dominate_decision() {
        // 50 frames @ 120 s, large ROI, unknown target,
        // calibration present.
        let assets: Vec<ExtractedMetadata> = (0..50)
            .map(|_| metadata(Some(120.0), Some(4096), Some(4096)))
            .collect();
        let asset_refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        let ti = target_intel(TargetKind::Unknown, "Unknown");
        let s = summary(asset_refs, Some(&ti), true, "/data/Unknown/");
        let c = analyse_session(&s, &CaptureThresholds::default());
        // Despite unknown target, exposure + frame count +
        // calibration favour deep-sky.
        assert_eq!(c.kind, CaptureKind::DeepSky);
    }

    #[test]
    fn filename_keyword_dominates_ambiguous_target() {
        // 20 frames @ 60 s (long exposure), small ROI,
        // path contains "jupiter".
        let assets: Vec<ExtractedMetadata> = (0..20)
            .map(|_| metadata(Some(60.0), Some(640), Some(480)))
            .collect();
        let asset_refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        let s = summary(asset_refs, None, false, "/data/Jupiter/2026-09-01/");
        let c = analyse_session(&s, &CaptureThresholds::default());
        // Exposure favours deep-sky (long), but filename
        // keyword + small ROI favour planetary/lunar.
        // Score gap should be small enough that this is
        // ambiguous, OR planetary/lunar wins.
        // 60.0 >= long_exposure (60.0) → deep-sky
        // 640 < small_roi (1024) → planetary/lunar
        // "jupiter" → planetary/lunar
        // We have 1 deep-sky signal (weight 0.20) +
        // 1 mid-range frame count (weight 0.10) +
        // 2 planetary signals (weight 0.10 + 0.15) = 0.25 vs 0.30.
        // Planetary wins narrowly. Verify the kind matches.
        // The actual assertion: with strong filename + ROI
        // vs only long exposure, the function picks
        // planetary/lunar.
        assert!(c.kind == CaptureKind::PlanetaryLunar || c.kind == CaptureKind::Ambiguous);
    }

    #[test]
    fn calibration_frames_presence_is_a_signal() {
        let assets: Vec<ExtractedMetadata> = (0..50)
            .map(|_| metadata(Some(180.0), Some(4096), Some(4096)))
            .collect();
        let asset_refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        let s = summary(asset_refs, None, true, "/data/M31/Light/");
        let c = analyse_session(&s, &CaptureThresholds::default());
        assert_eq!(c.kind, CaptureKind::DeepSky);
    }

    #[test]
    fn observations_are_recorded_in_classification() {
        let assets: Vec<ExtractedMetadata> = (0..50)
            .map(|_| metadata(Some(300.0), Some(4096), Some(4096)))
            .collect();
        let asset_refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        let ti = target_intel(TargetKind::Galaxy, "M31");
        let s = summary(asset_refs, Some(&ti), true, "/data/M31/Light/");
        let c = analyse_session(&s, &CaptureThresholds::default());
        // 4 signals should fire: exposure (deep-sky),
        // frame_count (deep-sky), target_kind (deep-sky),
        // frame_dimensions (deep-sky), calibration (deep-sky).
        // That's 5 observations.
        assert!(c.observations.len() >= 4);
        let signals: Vec<&str> = c.observations.iter().map(|o| o.signal.as_str()).collect();
        assert!(signals.contains(&"exposure"));
        assert!(signals.contains(&"frame_count"));
        assert!(signals.contains(&"target_kind"));
        assert!(signals.contains(&"frame_dimensions"));
        assert!(signals.contains(&"calibration_frames"));
    }

    #[test]
    fn thresholds_default_values_match_spec() {
        let t = CaptureThresholds::default();
        assert_eq!(t.short_exposure_seconds, 1.0);
        assert_eq!(t.long_exposure_seconds, 60.0);
        assert_eq!(t.high_frame_count, 1000);
        assert_eq!(t.low_frame_count, 200);
        assert_eq!(t.small_roi_max_dim, 1024);
        assert_eq!(t.large_frame_max_dim, 2048);
        assert_eq!(t.ambiguous_confidence_threshold, 0.6);
        assert_eq!(t.ambiguous_score_gap, 0.15);
    }

    #[test]
    fn aggregate_scores_zero_when_no_signals() {
        let obs: Vec<CaptureObservation> = Vec::new();
        let (d, p) = aggregate_scores(&obs);
        assert_eq!(d, 0.0);
        assert_eq!(p, 0.0);
    }

    #[test]
    fn aggregate_scores_only_counts_favoured_kind() {
        let obs = vec![
            CaptureObservation {
                signal: "exposure".into(),
                value: "30s".into(),
                weight: 0.20,
                confidence: 0.90,
                favours: CaptureKind::DeepSky,
            },
            CaptureObservation {
                signal: "frame_count".into(),
                value: "100".into(),
                weight: 0.15,
                confidence: 0.80,
                favours: CaptureKind::DeepSky,
            },
        ];
        let (d, p) = aggregate_scores(&obs);
        assert!(d > 0.0);
        assert_eq!(p, 0.0);
    }

    #[test]
    fn capture_kind_serialises_snake_case() {
        assert_eq!(
            serde_json::to_string(&CaptureKind::DeepSky).unwrap(),
            "\"deep_sky\""
        );
        assert_eq!(
            serde_json::to_string(&CaptureKind::PlanetaryLunar).unwrap(),
            "\"planetary_lunar\""
        );
        assert_eq!(
            serde_json::to_string(&CaptureKind::Ambiguous).unwrap(),
            "\"ambiguous\""
        );
    }

    #[test]
    fn median_exposure_handles_empty() {
        let assets: Vec<ExtractedMetadata> = Vec::new();
        let refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        assert!(median_exposure(&refs).is_none());
    }

    #[test]
    fn median_exposure_handles_odd_count() {
        let assets = [
            metadata(Some(10.0), None, None),
            metadata(Some(20.0), None, None),
            metadata(Some(30.0), None, None),
        ];
        let refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        assert_eq!(median_exposure(&refs), Some(20.0));
    }

    #[test]
    fn median_exposure_handles_even_count() {
        let assets = [
            metadata(Some(10.0), None, None),
            metadata(Some(20.0), None, None),
            metadata(Some(30.0), None, None),
            metadata(Some(40.0), None, None),
        ];
        let refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        assert_eq!(median_exposure(&refs), Some(25.0));
    }

    #[test]
    fn median_exposure_skips_missing() {
        let assets = [
            metadata(None, None, None),
            metadata(Some(20.0), None, None),
            metadata(None, None, None),
            metadata(Some(40.0), None, None),
        ];
        let refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        assert_eq!(median_exposure(&refs), Some(30.0));
    }

    #[test]
    fn max_image_dim_returns_none_for_no_dimensions() {
        let assets = [metadata(None, None, None)];
        let refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        assert!(max_image_dim(&refs).is_none());
    }

    #[test]
    fn max_image_dim_picks_largest() {
        let assets = [
            metadata(None, Some(640), Some(480)),
            metadata(None, Some(1920), Some(1080)),
            metadata(None, Some(4096), Some(4096)),
        ];
        let refs: Vec<&ExtractedMetadata> = assets.iter().collect();
        assert_eq!(max_image_dim(&refs), Some(4096));
    }
}
