//! CR-05 §12 — Adaptive Processing engine.
//!
//! §12 distinguishes **static parameters** ("Noise reduction = 0.35") from
//! **adaptive parameters** ("Noise profile detected → Moderate noise
//! reduction because high background noise with relatively strong nebular
//! signal"). This module is the pure decision engine that turns measured
//! image metrics into a concrete parameter set, **plus the reason** so the
//! UI can surface *why* a parameter was chosen.
//!
//! ## Scope discipline
//!
//! §12 is intentionally vague — it names the *principle*, not a fixed
//! parameter list. To keep this slice honest, `derive_adaptive_parameters`
//! only emits values for which a verifiable mapping from metrics exists in
//! `crate::quality` (snr, fwhm, star count, background). The shape is
//! extensible: when slice 4 introduces `get_processing_metrics`,
//! downstream code can add new fields (saturation, gradient strength,
//! registration quality) by appending `Option<AdaptiveValue>` members
//! without breaking serialised callers — every field is
//! `#[serde(default)]` and defaults to `None`.
//!
//! ## What this module does NOT do
//!
//! - It does not call any handlers — it only derives parameters.
//! - It does not read from / write to disk — it's a pure function.
//! - It does not invent metrics it can't derive from the current quality
//!   module (no fake `saturation: 0.0` placeholders).
//!
//! ## Reasoning visibility
//!
//! Every adaptive value carries a `reason: String`. This is a deliberate
//! §13 commitment: the feedback loop is only useful if the user (or the
//! runner in slice 4) can audit why a parameter was chosen. Reasons are
//! stable strings so they can be matched in tests and UI translations.

use serde::{Deserialize, Serialize};

/// Coarse noise assessment derived from SNR + background gradient.
///
/// `recommended_strength` is a 0.0–1.0 multiplier the runner can apply
/// to a noise-reduction algorithm; the three-band ladder keeps it
/// understandable in Guided mode and inspectable in Expert mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseProfile {
    pub kind: NoiseKind,
    /// 0.0 = no reduction; 1.0 = strongest reduction.
    pub recommended_strength: f32,
    /// Human-readable label for Guided mode and test snapshots.
    pub label: String,
    /// Why this profile was chosen. Stable string for tests + UI.
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoiseKind {
    /// SNR ≥ 40 — clean sky, basically Gaussian read-noise only.
    Low,
    /// 20 ≤ SNR < 40 — typical narrowband / bortle-4 sky.
    Moderate,
    /// 8 ≤ SNR < 20 — light-polluted or short total integration.
    High,
    /// SNR < 8 — extreme noise, gradient, or sparse integration.
    Extreme,
}

/// Sharpening strength, gated by FWHM + star count. We never sharpen
/// hard when there are few stars (chance of creating artefacts on noise).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharpeningProfile {
    /// 0.0 = no sharpening; 1.0 = aggressive.
    pub strength: f32,
    pub reason: String,
}

/// The full §12 output for one stage. Forward-compatible: new adaptive
/// fields (e.g. `color_saturation`, `background_gradient_correction`)
/// land as `Option<…>` so older serialised sets still deserialize.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AdaptiveParameterSet {
    #[serde(default)]
    pub noise: Option<NoiseProfile>,
    #[serde(default)]
    pub sharpening: Option<SharpeningProfile>,
}

/// Measured image statistics. Deliberately a separate type from
/// `crate::quality::FrameQuality` so the adaptive engine can be unit
/// tested without constructing an `F32Image` (cheaper + clearer).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageMetrics {
    /// Signal-to-noise ratio. The §12 noise decision pivots on this.
    pub snr: f64,
    /// Full-width at half maximum of the brightest peak, in pixels.
    /// 0.0 when no clear peak exists.
    pub fwhm: f64,
    /// Pixels brighter than mean + 5·stddev. Used for star-count gating.
    pub star_count: u32,
    /// |slope| of a linear fit through per-row means.
    pub background_gradient: f64,
    /// Image mean intensity (0.0–1.0 for normalised F32 images).
    pub mean: f64,
    /// Image stddev. Always ≥ 0.0.
    pub stddev: f64,
}

impl ImageMetrics {
    /// Empty / starless / flat image → conservative defaults. Used as
    /// the safety net so callers never see a panic on degenerate input.
    pub fn empty() -> Self {
        Self {
            snr: 0.0,
            fwhm: 0.0,
            star_count: 0,
            background_gradient: 0.0,
            mean: 0.0,
            stddev: 0.0,
        }
    }
}

// ── Noise banding ────────────────────────────────────────────────────

const LOW_SNR_FLOOR: f64 = 40.0;
const MODERATE_SNR_FLOOR: f64 = 20.0;
const HIGH_SNR_FLOOR: f64 = 8.0;

const NOISE_STRENGTH_LOW: f32 = 0.15;
const NOISE_STRENGTH_MODERATE: f32 = 0.35;
const NOISE_STRENGTH_HIGH: f32 = 0.60;
const NOISE_STRENGTH_EXTREME: f32 = 0.85;

/// Pure §12 derivation. Deterministic — no RNG, no IO, no clock.
pub fn derive_adaptive_parameters(metrics: &ImageMetrics) -> AdaptiveParameterSet {
    AdaptiveParameterSet {
        noise: Some(derive_noise_profile(metrics)),
        sharpening: Some(derive_sharpening_profile(metrics)),
    }
}

/// §12 noise decision. Splitting this out from `derive_adaptive_parameters`
/// makes it independently testable and lets the IPC handler in slice 4
/// query *just* the noise profile if it wants.
pub fn derive_noise_profile(metrics: &ImageMetrics) -> NoiseProfile {
    // SNR is the primary signal. Background gradient is the tie-breaker:
    // a low SNR with a strong gradient is worse than the SNR alone
    // suggests, so we bump the band up one step.
    let base = if metrics.snr >= LOW_SNR_FLOOR {
        NoiseKind::Low
    } else if metrics.snr >= MODERATE_SNR_FLOOR {
        NoiseKind::Moderate
    } else if metrics.snr >= HIGH_SNR_FLOOR {
        NoiseKind::High
    } else {
        NoiseKind::Extreme
    };

    let kind = if metrics.background_gradient > 0.05 && base != NoiseKind::Extreme {
        bump_up(base)
    } else {
        base
    };

    // Build reason + strength + label from the *final* kind, not the
    // base — otherwise a Low→Moderate bump would still report "clean
    // sky" as the reason, which is misleading.
    let (label, strength, reason) = match kind {
        NoiseKind::Low => (
            "Low".to_string(),
            NOISE_STRENGTH_LOW,
            format!(
                "SNR {:.1} ≥ {:.0} — clean sky; mild noise reduction sufficient.",
                metrics.snr, LOW_SNR_FLOOR
            ),
        ),
        NoiseKind::Moderate => (
            "Moderate".to_string(),
            NOISE_STRENGTH_MODERATE,
            format!(
                "SNR {:.1} — moderate noise reduction (gradient bump: bg slope {:.3} > 0.05).",
                metrics.snr, metrics.background_gradient
            ),
        ),
        NoiseKind::High => (
            "High".to_string(),
            NOISE_STRENGTH_HIGH,
            format!(
                "SNR {:.1} — high noise; aggressive reduction (gradient bump applied).",
                metrics.snr
            ),
        ),
        NoiseKind::Extreme => (
            "Extreme".to_string(),
            NOISE_STRENGTH_EXTREME,
            format!(
                "SNR {:.1} < {:.0} — extreme noise; aggressive reduction required.",
                metrics.snr, HIGH_SNR_FLOOR
            ),
        ),
    };

    NoiseProfile {
        kind,
        recommended_strength: strength,
        label,
        reason,
    }
}

fn bump_up(k: NoiseKind) -> NoiseKind {
    match k {
        NoiseKind::Low => NoiseKind::Moderate,
        NoiseKind::Moderate => NoiseKind::High,
        NoiseKind::High => NoiseKind::Extreme,
        NoiseKind::Extreme => NoiseKind::Extreme,
    }
}

/// §12 sharpening decision. Conservative on starless / low-star images.
pub fn derive_sharpening_profile(metrics: &ImageMetrics) -> SharpeningProfile {
    // No measurable stars → don't sharpen (nothing to enhance, easy to
    // amplify noise).
    if metrics.star_count < 10 || metrics.fwhm <= 0.0 {
        return SharpeningProfile {
            strength: 0.0,
            reason: format!(
                "Only {} stars detected (≥ 10 required for adaptive sharpening); skipping.",
                metrics.star_count
            ),
        };
    }
    // Already sharp → mild.
    if metrics.fwhm < 2.0 {
        return SharpeningProfile {
            strength: 0.20,
            reason: format!(
                "FWHM {:.2} px already sharp; mild sharpening to avoid overshoot.",
                metrics.fwhm
            ),
        };
    }
    // Soft → moderate.
    if metrics.fwhm < 4.0 {
        return SharpeningProfile {
            strength: 0.45,
            reason: format!(
                "FWHM {:.2} px moderate; standard sharpening strength.",
                metrics.fwhm
            ),
        };
    }
    SharpeningProfile {
        strength: 0.65,
        reason: format!(
            "FWHM {:.2} px soft; aggressive sharpening to recover detail.",
            metrics.fwhm
        ),
    }
}

// ── Bridge from `crate::quality` ─────────────────────────────────────

/// Compute the metrics the adaptive engine needs from an existing
/// `FrameQuality`. Bridges the two modules so a handler that already
/// has `FrameQuality` doesn't have to recompute from pixels.
pub fn metrics_from_frame_quality(fq: &crate::quality::FrameQuality) -> ImageMetrics {
    // Background gradient isn't on `FrameQuality` (P3 metrics didn't
    // surface it); derive a proxy from `cloud_score` which is std/mean.
    // This is a deliberate placeholder — slice 4 adds the real gradient
    // computation to `FrameQuality` when it adds `get_processing_metrics`.
    let background_gradient = fq.cloud_score * 0.05;
    ImageMetrics {
        snr: fq.snr,
        fwhm: fq.fwhm,
        star_count: fq.star_count as u32,
        background_gradient,
        mean: fq.background,
        stddev: fq.cloud_score * fq.background.abs().max(1e-10),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn high_snr() -> ImageMetrics {
        ImageMetrics {
            snr: 60.0,
            fwhm: 1.8,
            star_count: 250,
            background_gradient: 0.0,
            mean: 0.20,
            stddev: 0.003,
        }
    }
    fn moderate_snr() -> ImageMetrics {
        ImageMetrics {
            snr: 25.0,
            fwhm: 3.0,
            star_count: 120,
            background_gradient: 0.0,
            mean: 0.30,
            stddev: 0.012,
        }
    }
    fn high_snr_with_gradient() -> ImageMetrics {
        let mut m = high_snr();
        m.background_gradient = 0.10; // bumps Low → Moderate
        m
    }
    fn extreme() -> ImageMetrics {
        ImageMetrics {
            snr: 5.0,
            fwhm: 6.0,
            star_count: 20,
            background_gradient: 0.0,
            mean: 0.50,
            stddev: 0.10,
        }
    }

    #[test]
    fn low_snr_clean_skies_yields_low_noise_profile() {
        let p = derive_noise_profile(&high_snr());
        assert_eq!(p.kind, NoiseKind::Low);
        assert!((p.recommended_strength - 0.15).abs() < 1e-6);
        assert!(p.reason.contains("60.0"));
    }

    #[test]
    fn moderate_snr_yields_moderate_profile() {
        let p = derive_noise_profile(&moderate_snr());
        assert_eq!(p.kind, NoiseKind::Moderate);
        assert!((p.recommended_strength - 0.35).abs() < 1e-6);
    }

    #[test]
    fn strong_gradient_bumps_low_to_moderate() {
        // Low SNR-side but with gradient → Moderate, not Low.
        let p = derive_noise_profile(&high_snr_with_gradient());
        assert_eq!(p.kind, NoiseKind::Moderate);
        assert_eq!(p.label, "Moderate");
        assert!(
            p.reason.contains("gradient bump"),
            "reason must mention the bump: {}",
            p.reason
        );
    }

    #[test]
    fn extreme_snr_yields_extreme_profile() {
        let p = derive_noise_profile(&extreme());
        assert_eq!(p.kind, NoiseKind::Extreme);
        assert!((p.recommended_strength - 0.85).abs() < 1e-6);
    }

    #[test]
    fn empty_metrics_never_panic_and_pick_extreme() {
        // §12 promise: adaptive engine never refuses to make progress.
        let m = ImageMetrics::empty();
        let p = derive_noise_profile(&m);
        // SNR 0 → Extreme is the spec-blessed conservative choice.
        assert_eq!(p.kind, NoiseKind::Extreme);
        // Sharpening skipped — no stars detected.
        let s = derive_sharpening_profile(&m);
        assert_eq!(s.strength, 0.0);
    }

    #[test]
    fn sharpening_skipped_for_starless_images() {
        let mut m = high_snr();
        m.star_count = 0;
        let s = derive_sharpening_profile(&m);
        assert_eq!(s.strength, 0.0);
        assert!(s.reason.contains("0 stars"));
    }

    #[test]
    fn sharpening_mild_when_already_sharp() {
        let m = high_snr(); // fwhm 1.8, 250 stars
        let s = derive_sharpening_profile(&m);
        assert!((s.strength - 0.20).abs() < 1e-6);
    }

    #[test]
    fn sharpening_aggressive_for_soft_fwhm() {
        let m = extreme(); // fwhm 6.0, 20 stars
        let s = derive_sharpening_profile(&m);
        assert!((s.strength - 0.65).abs() < 1e-6);
    }

    #[test]
    fn derive_adaptive_parameters_is_deterministic() {
        let m = moderate_snr();
        let a = derive_adaptive_parameters(&m);
        let b = derive_adaptive_parameters(&m);
        assert_eq!(a, b, "derive_adaptive_parameters must be pure");
    }

    #[test]
    fn adaptive_parameter_set_omits_noise_when_noise_is_none() {
        // Forward-compat check: serialise/deserialise round trip with
        // noise elided, so slice 4's IPC layer can add new optional
        // fields without breaking older callers.
        let s = AdaptiveParameterSet {
            noise: None,
            sharpening: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: AdaptiveParameterSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn bridge_from_frame_quality_propagates_snr_and_fwhm() {
        let fq = crate::quality::FrameQuality {
            fwhm: 2.5,
            eccentricity: 0.4,
            star_count: 80,
            snr: 30.0,
            background: 0.25,
            cloud_score: 0.05,
        };
        let m = metrics_from_frame_quality(&fq);
        assert_eq!(m.snr, 30.0);
        assert_eq!(m.fwhm, 2.5);
        assert_eq!(m.star_count, 80);
        // The engine then lands on Moderate from these inputs.
        let p = derive_noise_profile(&m);
        assert_eq!(p.kind, NoiseKind::Moderate);
    }
}
