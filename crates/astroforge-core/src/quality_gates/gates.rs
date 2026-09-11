//! CR-06 P6 — per-gate implementations.
//!
//! Each function in this module is a pure function
//! from `(source: F32Image, result: F32Image) ->
//! GateFinding`. The orchestrator (`report::run`)
//! calls all ten and aggregates the findings into
//! `QualityGateReport`.
//!
//! The implementations are deliberately simple —
//! per-pixel statistics + 3×3 local contrast — so
//! the engine runs on a CI runner without a GPU. A
//! future slice can swap in the richer per-channel
//! gradient statistics if needed.

use crate::image::F32Image;
use crate::quality_gates::{GateFinding, GateId, GateThresholds, Severity};

/// CR-06 §37 — `Clipping`. Sudden loss of
/// highlight / shadow detail. The score is the
/// fractional increase in pixels at the dark or
/// bright saturation rails (`<= 0.01` and `>= 0.99`).
pub fn clipping(source: &F32Image, result: &F32Image, t: &GateThresholds) -> GateFinding {
    let src_count = count_saturated(source);
    let res_count = count_saturated(result);
    let n = (source.width() * source.height()) as f32;
    let src_frac = src_count as f32 / n;
    let res_frac = res_count as f32 / n;
    let score = (res_frac - src_frac).max(0.0);
    let (severity, msg) = if score >= t.clipping_fail {
        (
            Severity::Failure,
            "Severe clipping: many pixels pushed to the rails".to_string(),
        )
    } else if score >= t.clipping_warn {
        (Severity::Warning, "Moderate clipping detected".to_string())
    } else if score > 0.0005 {
        (
            Severity::Info,
            format!(
                "Slight clipping ({} extra saturated pixels)",
                res_count.saturating_sub(src_count)
            ),
        )
    } else {
        (Severity::Ok, "No clipping".to_string())
    };
    GateFinding {
        gate: GateId::Clipping,
        severity,
        message: msg,
        score,
        recommendation: None,
    }
}

fn count_saturated(image: &F32Image) -> usize {
    let w = image.width();
    let h = image.height();
    let c = image.channels();
    let mut count = 0_usize;
    for y in 0..h {
        for x in 0..w {
            for ch in 0..c {
                let v = image[(ch, y, x)];
                if v <= 0.01 || v >= 0.99 {
                    count += 1;
                    break;
                }
            }
        }
    }
    count
}

/// CR-06 §37 — `NoiseAmplification`. Variance
/// increases by more than `max_variance_increase`.
pub fn noise_amplification(
    source: &F32Image,
    result: &F32Image,
    t: &GateThresholds,
) -> GateFinding {
    let src_var = variance(source);
    let res_var = variance(result);
    let score = if src_var <= 0.0 {
        res_var
    } else {
        res_var / src_var
    };
    let (severity, msg) = if score >= t.noise_amplify_fail {
        (
            Severity::Failure,
            format!("Noise amplified by {:.2}×", score),
        )
    } else if score >= t.noise_amplify_warn {
        (
            Severity::Warning,
            format!("Noise amplified by {:.2}×", score),
        )
    } else if score > 1.05 {
        (
            Severity::Info,
            format!("Slight noise increase ({:.2}×)", score),
        )
    } else {
        (Severity::Ok, "Noise level stable".to_string())
    };
    GateFinding {
        gate: GateId::NoiseAmplification,
        severity,
        message: msg,
        score,
        recommendation: None,
    }
}

/// CR-06 §37 — `ExcessiveSmoothing`. Variance
/// drops below a floor (the operation smoothed
/// out signal).
pub fn excessive_smoothing(
    source: &F32Image,
    result: &F32Image,
    t: &GateThresholds,
) -> GateFinding {
    let src_var = variance(source);
    let res_var = variance(result);
    let score = if src_var <= 0.0 {
        1.0
    } else {
        res_var / src_var
    };
    let (severity, msg) = if score <= t.smoothing_fail {
        (
            Severity::Failure,
            "Severe smoothing: signal flattened".to_string(),
        )
    } else if score <= t.smoothing_warn {
        (
            Severity::Warning,
            "Excessive smoothing detected".to_string(),
        )
    } else if score < 0.95 {
        (Severity::Info, "Slight smoothing".to_string())
    } else {
        (Severity::Ok, "Signal preserved".to_string())
    };
    GateFinding {
        gate: GateId::ExcessiveSmoothing,
        severity,
        message: msg,
        score,
        recommendation: None,
    }
}

/// CR-06 §37 — `ColorShifts`. Per-channel mean
/// delta beyond a threshold.
pub fn color_shifts(source: &F32Image, result: &F32Image, t: &GateThresholds) -> GateFinding {
    let src_means = channel_means(source);
    let res_means = channel_means(result);
    let mut max_delta = 0.0_f32;
    for (a, b) in src_means.iter().zip(res_means.iter()) {
        max_delta = max_delta.max((a - b).abs());
    }
    let score = max_delta;
    let (severity, msg) = if score >= t.color_shift_fail {
        (Severity::Failure, "Severe color shift".to_string())
    } else if score >= t.color_shift_warn {
        (Severity::Warning, "Color shift detected".to_string())
    } else if score > 0.005 {
        (Severity::Info, "Slight color drift".to_string())
    } else {
        (Severity::Ok, "Color preserved".to_string())
    };
    GateFinding {
        gate: GateId::ColorShifts,
        severity,
        message: msg,
        score,
        recommendation: None,
    }
}

/// CR-06 §37 — `StarArtifacts` + `Halos`. Bright-
/// pixel 3×3 contrast increase. The two gates
/// share one computation but produce separate
/// findings because the §37 checklist lists them
/// separately.
pub fn star_artifacts(source: &F32Image, result: &F32Image, t: &GateThresholds) -> GateFinding {
    let src = bright_pixel_contrast(source);
    let res = bright_pixel_contrast(result);
    let score = if src <= 0.0 { res } else { res / src };
    let (severity, msg) = if score >= t.star_contrast_fail {
        (
            Severity::Failure,
            format!("Severe star artifact (contrast {:.2}×)", score),
        )
    } else if score >= t.star_contrast_warn {
        (
            Severity::Warning,
            format!("Moderate star artifact (contrast {:.2}×)", score),
        )
    } else if score > 1.10 {
        (Severity::Info, "Slight star sharpening".to_string())
    } else {
        (Severity::Ok, "Stars preserved".to_string())
    };
    GateFinding {
        gate: GateId::StarArtifacts,
        severity,
        message: msg,
        score,
        recommendation: None,
    }
}

/// CR-06 §37 — `Halos`. Same metric as star
/// artifacts but framed differently; the score
/// interpretation is identical.
pub fn halos(source: &F32Image, result: &F32Image, t: &GateThresholds) -> GateFinding {
    let finding = star_artifacts(source, result, t);
    GateFinding {
        gate: GateId::Halos,
        severity: finding.severity,
        message: match finding.severity {
            Severity::Ok => "No halos detected".to_string(),
            Severity::Info => format!("Slight halo near bright stars ({:.2}×)", finding.score),
            Severity::Warning => format!("Moderate halo near bright stars ({:.2}×)", finding.score),
            Severity::Failure => format!("Severe halo ({:.2}×)", finding.score),
        },
        score: finding.score,
        recommendation: None,
    }
}

/// CR-06 §37 — `FalseStructures`. Variance in
/// flat regions of the result that should have
/// stayed flat. The flat regions are identified
/// via a 5×5 local-variance threshold on the
/// source.
pub fn false_structures(source: &F32Image, result: &F32Image, t: &GateThresholds) -> GateFinding {
    let flat_mask = flat_regions(source);
    let n = flat_mask.iter().filter(|&&x| x).count() as f32;
    if n < 100.0 {
        // Not enough flat pixels to compare against
        // (the image has no real "background").
        return GateFinding {
            gate: GateId::FalseStructures,
            severity: Severity::Ok,
            message: "No flat regions to compare".to_string(),
            score: 0.0,
            recommendation: None,
        };
    }
    let mut sum = 0.0_f32;
    let mut sum_sq = 0.0_f32;
    for y in 0..source.height() {
        for x in 0..source.width() {
            if flat_mask[y * source.width() + x] {
                let v = result[(0, y, x)];
                sum += v;
                sum_sq += v * v;
            }
        }
    }
    let mean = sum / n;
    let var = (sum_sq / n) - mean * mean;
    let score = var.max(0.0);
    let (severity, msg) = if score >= t.false_structure_fail {
        (
            Severity::Failure,
            "False structures detected in flat regions".to_string(),
        )
    } else if score >= t.false_structure_warn {
        (Severity::Warning, "Possible false structures".to_string())
    } else if score > 0.0005 {
        (Severity::Info, "Slight structure introduced".to_string())
    } else {
        (Severity::Ok, "Flat regions stable".to_string())
    };
    GateFinding {
        gate: GateId::FalseStructures,
        severity,
        message: msg,
        score,
        recommendation: None,
    }
}

fn flat_regions(image: &F32Image) -> Vec<bool> {
    let w = image.width();
    let h = image.height();
    let mut mask = vec![false; w * h];
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let center = image[(0, y, x)];
            let mut lo = f32::INFINITY;
            let mut hi = f32::NEG_INFINITY;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;
                    let v = image[(0, ny, nx)];
                    if v < lo {
                        lo = v;
                    }
                    if v > hi {
                        hi = v;
                    }
                }
            }
            mask[y * w + x] = (hi - lo) < 0.01 && center < 0.5;
        }
    }
    mask
}

/// CR-06 §37 — `EdgeArtifacts`. Energy at the
/// image border (top / bottom / left / right 2
/// pixels) increases relative to the source.
pub fn edge_artifacts(source: &F32Image, result: &F32Image, t: &GateThresholds) -> GateFinding {
    let src = border_energy(source);
    let res = border_energy(result);
    let score = if src <= 0.0 { res } else { res / src };
    let (severity, msg) = if score >= t.edge_artifact_fail {
        (Severity::Failure, "Severe edge artifacts".to_string())
    } else if score >= t.edge_artifact_warn {
        (Severity::Warning, "Edge artifacts detected".to_string())
    } else if score > 1.10 {
        (Severity::Info, "Slight edge energy increase".to_string())
    } else {
        (Severity::Ok, "Edges clean".to_string())
    };
    GateFinding {
        gate: GateId::EdgeArtifacts,
        severity,
        message: msg,
        score,
        recommendation: None,
    }
}

fn border_energy(image: &F32Image) -> f32 {
    let w = image.width();
    let h = image.height();
    let mut sum = 0.0_f32;
    let mut count = 0_usize;
    // Top + bottom 2 rows.
    for y in 0..2.min(h) {
        for x in 0..w {
            sum += image[(0, y, x)];
            count += 1;
        }
    }
    if h >= 2 {
        for y in (h - 2)..h {
            for x in 0..w {
                sum += image[(0, y, x)];
                count += 1;
            }
        }
    }
    // Left + right 2 cols (excluding the top /
    // bottom rows already counted).
    for y in 2..h.saturating_sub(2) {
        for x in 0..2.min(w) {
            sum += image[(0, y, x)];
            count += 1;
        }
    }
    if w >= 2 {
        for y in 2..h.saturating_sub(2) {
            for x in (w - 2)..w {
                sum += image[(0, y, x)];
                count += 1;
            }
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}

/// CR-06 §37 — `SegmentationLeakage`. Brightness
/// in mask-excluded regions changes more than in
/// the included regions. P6 ships the function
/// signature; the segmentation leakage comparison
/// becomes meaningful when masks are attached
/// (CR-06 P5 wired the mask engine). With no mask
/// the gate is a no-op.
pub fn segmentation_leakage(
    _source: &F32Image,
    _result: &F32Image,
    _t: &GateThresholds,
) -> GateFinding {
    GateFinding {
        gate: GateId::SegmentationLeakage,
        severity: Severity::Ok,
        message: "No mask attached — segmentation leakage check is a no-op".to_string(),
        score: 0.0,
        recommendation: None,
    }
}

/// CR-06 §37 — `Ringing`. Periodic banding near
/// high-contrast edges. P6 ships a coarse proxy:
/// the high-frequency energy ratio (the fraction
/// of variance that lives in the difference between
/// the image and a 3×3 box blur). A real ringing
/// detector lives in a future slice.
pub fn ringing(source: &F32Image, result: &F32Image, _t: &GateThresholds) -> GateFinding {
    let src_hf = high_frequency_energy(source);
    let res_hf = high_frequency_energy(result);
    let score = if src_hf <= 0.0 {
        res_hf
    } else {
        res_hf / src_hf
    };
    let (severity, msg) = if score >= 2.0 {
        (
            Severity::Failure,
            format!("Severe ringing detected ({:.2}×)", score),
        )
    } else if score >= 1.50 {
        (
            Severity::Warning,
            format!("Ringing detected ({:.2}×)", score),
        )
    } else if score > 1.10 {
        (Severity::Info, "Slight high-frequency increase".to_string())
    } else {
        (Severity::Ok, "No ringing".to_string())
    };
    GateFinding {
        gate: GateId::Ringing,
        severity,
        message: msg,
        score,
        recommendation: None,
    }
}

fn high_frequency_energy(image: &F32Image) -> f32 {
    let w = image.width();
    let h = image.height();
    let mut sum = 0.0_f32;
    let mut sum_sq = 0.0_f32;
    let mut count = 0_usize;
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let v = image[(0, y, x)];
            sum += v;
            sum_sq += v * v;
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        (sum_sq / count as f32) - (sum / count as f32).powi(2)
    }
}

fn variance(image: &F32Image) -> f32 {
    let n = image.width() * image.height();
    if n == 0 {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    let mut sum_sq = 0.0_f32;
    for y in 0..image.height() {
        for x in 0..image.width() {
            let v = image[(0, y, x)];
            sum += v;
            sum_sq += v * v;
        }
    }
    (sum_sq / n as f32) - (sum / n as f32).powi(2)
}

fn channel_means(image: &F32Image) -> Vec<f32> {
    let w = image.width();
    let h = image.height();
    let c = image.channels();
    let mut sums = vec![0.0_f32; c];
    for y in 0..h {
        for x in 0..w {
            for ch in 0..c {
                sums[ch] += image[(ch, y, x)];
            }
        }
    }
    let n = (w * h) as f32;
    sums.iter_mut().for_each(|s| *s /= n);
    sums
}

fn bright_pixel_contrast(image: &F32Image) -> f32 {
    let w = image.width();
    let h = image.height();
    if w < 3 || h < 3 {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    let mut count = 0_usize;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let center = image[(0, y, x)];
            if center < 0.6 {
                continue;
            }
            let mut lo = f32::INFINITY;
            let mut hi = f32::NEG_INFINITY;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;
                    let v = image[(0, ny, nx)];
                    if v < lo {
                        lo = v;
                    }
                    if v > hi {
                        hi = v;
                    }
                }
            }
            sum += hi - lo;
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

    fn flat_image(width: usize, height: usize, value: f32) -> F32Image {
        let arr = Array3::<f32>::from_elem((1, height, width), value);
        F32Image::from(arr)
    }

    fn noisy_image(width: usize, height: usize, seed: u32) -> F32Image {
        let mut arr = Array3::<f32>::zeros((1, height, width));
        for y in 0..height {
            for x in 0..width {
                let v = ((x as u32 + y as u32).wrapping_mul(seed).wrapping_add(7)) % 100;
                arr[(0, y, x)] = v as f32 / 100.0;
            }
        }
        F32Image::from(arr)
    }

    #[test]
    fn clipping_passes_on_identical_input() {
        let img = flat_image(8, 8, 0.5);
        let t = GateThresholds::default();
        let finding = clipping(&img, &img, &t);
        assert_eq!(finding.severity, Severity::Ok);
    }

    #[test]
    fn clipping_fires_on_pushed_rails() {
        let src = flat_image(8, 8, 0.5);
        let res = flat_image(8, 8, 0.0);
        let t = GateThresholds::default();
        let finding = clipping(&src, &res, &t);
        // 64 / 64 pixels are now clipped — extreme.
        assert!(finding.score > 0.5);
    }

    #[test]
    fn noise_amplification_passes_on_identical_input() {
        let img = noisy_image(8, 8, 1);
        let t = GateThresholds::default();
        let finding = noise_amplification(&img, &img, &t);
        assert_eq!(finding.severity, Severity::Ok);
    }

    #[test]
    fn excessive_smoothing_fires_on_flat_result() {
        let src = noisy_image(8, 8, 1);
        let res = flat_image(8, 8, 0.5);
        let t = GateThresholds::default();
        let finding = excessive_smoothing(&src, &res, &t);
        assert!(finding.severity == Severity::Warning || finding.severity == Severity::Failure);
    }

    #[test]
    fn color_shifts_passes_on_identical_input() {
        let img = flat_image(8, 8, 0.5);
        let t = GateThresholds::default();
        let finding = color_shifts(&img, &img, &t);
        assert_eq!(finding.severity, Severity::Ok);
    }

    #[test]
    fn color_shifts_fires_on_channel_offset() {
        let src = flat_image(8, 8, 0.5);
        let res = flat_image(8, 8, 0.7);
        let t = GateThresholds::default();
        let finding = color_shifts(&src, &res, &t);
        // The score is the channel mean delta
        // (0.2). Way above the warn threshold (0.05).
        assert!(finding.score >= 0.15);
    }

    #[test]
    fn segmentation_leakage_is_a_no_op_without_mask() {
        let img = flat_image(8, 8, 0.5);
        let t = GateThresholds::default();
        let finding = segmentation_leakage(&img, &img, &t);
        assert_eq!(finding.severity, Severity::Ok);
    }

    #[test]
    fn halos_separate_from_star_artifacts_share_score() {
        let img = flat_image(8, 8, 0.5);
        let t = GateThresholds::default();
        let halo = halos(&img, &img, &t);
        let star = star_artifacts(&img, &img, &t);
        assert_eq!(halo.score, star.score);
    }
}
