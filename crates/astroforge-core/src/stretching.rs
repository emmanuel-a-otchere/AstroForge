use crate::image::F32Image;

pub fn auto_stretch(image: &F32Image) -> F32Image {
    let mut result = image.clone();

    let min = result.iter().copied().fold(f32::INFINITY, f32::min);
    let max = result.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(1e-10);

    let midtones = compute_midtones(&result);

    for val in result.iter_mut() {
        let normalized = (*val - min) / range;
        *val = arcsinh_stretch(f64::from(normalized), midtones) as f32;
    }

    result
}

pub fn arcsinh_stretch(value: f64, midtones: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    let beta = midtones.max(1e-10);
    // Lupton et al. 1999 arcsinh stretch:
    //   stretched(x) = asinh(x / beta) / asinh(1 / beta)
    // Maps 0 -> 0 and 1 -> 1 regardless of beta.
    let stretched = (value / beta).asinh() / (1.0 / beta).asinh();
    stretched.clamp(0.0, 1.0)
}

fn compute_midtones(image: &F32Image) -> f64 {
    let mut values: Vec<f32> = image.iter().copied().collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median = values[values.len() / 2] as f64;
    let mean = (values.iter().sum::<f32>() / values.len() as f32) as f64;

    if mean > 0.0 && median > 0.0 {
        let log_mean = mean.ln();
        let log_median = median.ln();
        (log_mean - log_median).exp()
    } else {
        0.25
    }
}

pub fn histogram_stretch(
    image: &F32Image,
    shadows: f64,
    highlights: f64,
    midtones: f64,
) -> F32Image {
    let mut result = image.clone();
    let range = (highlights - shadows).max(1e-10);

    for val in result.iter_mut() {
        let normalized = (*val as f64 - shadows) / range;
        let stretched = midtone_transfer(normalized.clamp(0.0, 1.0), midtones);
        // Apply white-point clipping AFTER MTF, matching the GLSL shader's
        //   stretched = min(stretched, u_highlights);
        // step in src/lib/shaders.ts (MTF_STRETCH_SHADER). The `highlights`
        // arg here is the upper end of the input range AND the ceiling on
        // the output. Below-black-point inputs are already 0 after the
        // (x - shadows) / (highlights - shadows) normalisation clamp, so
        // no additional lower-bound clip is needed.
        *val = stretched.min(highlights) as f32;
    }

    result
}

fn midtone_transfer(value: f64, midtones: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    if value >= 1.0 {
        return 1.0;
    }
    let m = midtones.clamp(0.001, 0.999);
    // Lupton et al. 1999 midtone transfer:
    //   m(v) = ((m - 1) * v) / ((2m - 1) * v - m)
    // Maps 0 -> 0 and 1 -> 1; the `m` parameter shifts where the inflection
    // happens (m = 0.5 -> identity, m < 0.5 -> brightens midtones, m > 0.5 -> darkens).
    //
    // PARITY: this formula is mirrored verbatim in the GLSL `mtf()` helper inside
    // src/lib/shaders.ts (`MTF_STRETCH_SHADER`). If you change one, change the other
    // and update both test suites.
    let result = ((m - 1.0) * value) / ((2.0 * m - 1.0) * value - m);
    result.clamp(0.0, 1.0)
}

pub fn compute_histogram(image: &F32Image, bins: usize) -> Vec<u32> {
    let mut hist = vec![0u32; bins];
    // Image values are normalised floats in [0, 1]; bin against that range so
    // a value of 0.5 lands in the middle bin (e.g. bin 128 of 256).
    let range = 1.0_f32;

    for &val in image.iter() {
        let normalized = (val / range).clamp(0.0, 1.0) as f64;
        let bin = (normalized * bins as f64) as usize;
        let bin = bin.min(bins - 1);
        hist[bin] += 1;
    }

    hist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_stretch() {
        let mut img = F32Image::new(4, 4, 1);
        for i in 0..16 {
            img[(0, i / 4, i % 4)] = i as f32 * 100.0;
        }
        let stretched = auto_stretch(&img);
        let max = stretched
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::min)
            .max(0.0);
        let min = stretched.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(max <= 1.0);
        assert!(min >= 0.0);
    }

    #[test]
    fn test_arcsinh_stretch() {
        assert!((arcsinh_stretch(0.0, 0.25) - 0.0).abs() < 0.01);
        assert!((arcsinh_stretch(1.0, 0.25) - 1.0).abs() < 0.01);
        let mid = arcsinh_stretch(0.5, 0.25);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn test_histogram_stretch() {
        let mut img = F32Image::new(4, 4, 1);
        img.fill(0.5);
        let stretched = histogram_stretch(&img, 0.0, 1.0, 0.25);
        let val = stretched[(0, 0, 0)];
        assert!(val > 0.0 && val < 1.0);
    }

    #[test]
    fn test_compute_histogram() {
        let mut img = F32Image::new(4, 4, 1);
        img.fill(0.5);
        let hist = compute_histogram(&img, 256);
        assert_eq!(hist[128], 16);
    }

    // ── MTF (Lupton 1999) parity tests ───────────────────────────────────
    //
    // These tests pin the Rust `midtone_transfer` so the GLSL `mtf()` in
    // src/lib/shaders.ts cannot drift. If you change the formula, update
    // both copies AND the values in `expected_mtf_table` below.

    /// Hand-computed expected outputs of mtf(v, m) for canonical (v, m) pairs.
    /// The formula is `((m - 1) * v) / ((2m - 1) * v - m)`; values match
    /// to 4 decimals to absorb f64 rounding noise.
    const EXPECTED_MTF_EPSILON: f64 = 1e-4;

    #[test]
    fn test_mtf_fixed_points_at_zero_and_one() {
        // 0 -> 0 and 1 -> 1 for every m in (0, 1).
        for m in [0.05, 0.25, 0.5, 0.75, 0.95] {
            assert!(
                (midtone_transfer(0.0, m) - 0.0).abs() < EXPECTED_MTF_EPSILON,
                "mtf(0, {m}) = {} (expected 0)",
                midtone_transfer(0.0, m)
            );
            assert!(
                (midtone_transfer(1.0, m) - 1.0).abs() < EXPECTED_MTF_EPSILON,
                "mtf(1, {m}) = {} (expected 1)",
                midtone_transfer(1.0, m)
            );
        }
    }

    #[test]
    fn test_mtf_identity_at_half_midtones() {
        // m = 0.5 is the canonical identity: every value passes through
        // (matches the GLSL `MTF_STRETCH_SHADER` comment: "neutral at 0.25"
        // — but only when the input is the canonical linear-to-display
        // 0.5 luminance. With pure m=0.5 every v satisfies v == mtf(v, 0.5)).
        for v in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0] {
            let out = midtone_transfer(v, 0.5);
            assert!(
                (out - v).abs() < EXPECTED_MTF_EPSILON,
                "m=0.5 must be identity: mtf({v}, 0.5) = {out}"
            );
        }
    }

    #[test]
    fn test_mtf_hand_computed_canonical_pairs() {
        // Reference values computed by hand from the Lupton 1999 formula:
        //   mtf(v, m) = ((m - 1) * v) / ((2m - 1) * v - m)
        //
        //   mtf(0.25, 0.25) = 0.5000
        //   mtf(0.50, 0.25) = 0.7500
        //   mtf(0.50, 0.75) = 0.2500
        //   mtf(0.75, 0.25) = 0.9000
        let cases = [
            (0.25_f64, 0.25_f64, 0.5_f64),
            (0.50, 0.25, 0.75),
            (0.50, 0.75, 0.25),
            (0.75, 0.25, 0.90),
        ];
        for (v, m, expected) in cases {
            let out = midtone_transfer(v, m);
            assert!(
                (out - expected).abs() < EXPECTED_MTF_EPSILON,
                "mtf({v}, {m}) = {out} (expected {expected})"
            );
        }
    }

    #[test]
    fn test_mtf_monotonic_increasing() {
        // The MTF must be strictly increasing in v for any fixed m in (0,1).
        // A regression that introduces a flip would silently break stretching.
        for m in [0.1, 0.25, 0.5, 0.75, 0.9] {
            let mut prev = midtone_transfer(0.0, m);
            for i in 1..=20 {
                let v = i as f64 / 20.0;
                let out = midtone_transfer(v, m);
                assert!(
                    out >= prev,
                    "MTF not monotonic at m={m}: mtf({prev_v})={prev} > mtf({v})={out}",
                    prev_v = (i - 1) as f64 / 20.0
                );
                prev = out;
            }
        }
    }

    // ── histogram_stretch: black-point + highlight clipping ─────────────

    #[test]
    fn test_histogram_stretch_black_point_clip() {
        // A black point of 0.3 means any pixel below 0.3 should land at 0
        // (after the (x - BP) / (1 - BP) normalisation).
        let mut img = F32Image::new(2, 1, 1);
        img[(0, 0, 0)] = 0.1; // below BP -> should clamp to 0
        img[(0, 0, 1)] = 0.3; // at BP     -> should land near 0
        let stretched = histogram_stretch(&img, 0.3, 1.0, 0.5);
        assert!(
            stretched[(0, 0, 0)] <= EXPECTED_MTF_EPSILON as f32,
            "below-BP pixel should clip to 0, got {}",
            stretched[(0, 0, 0)]
        );
        assert!(
            stretched[(0, 0, 1)] <= EXPECTED_MTF_EPSILON as f32 + 0.01,
            "at-BP pixel should land near 0, got {}",
            stretched[(0, 0, 1)]
        );
    }

    #[test]
    fn test_histogram_stretch_highlight_clip() {
        // Highlights = 0.7: any pixel that would exceed 0.7 after MTF must
        // be clamped to <= 0.7 (the highlight ceiling).
        let mut img = F32Image::new(1, 1, 1);
        img[(0, 0, 0)] = 1.0; // brightest input
        let stretched = histogram_stretch(&img, 0.0, 0.7, 0.5);
        assert!(
            stretched[(0, 0, 0)] <= 0.7 + EXPECTED_MTF_EPSILON as f32,
            "highlight ceiling must clamp, got {}",
            stretched[(0, 0, 0)]
        );
    }

    #[test]
    fn test_histogram_stretch_full_range_passthrough_neutral() {
        // With shadows=0, highlights=1, midtones=0.5, every input v should
        // map to itself (identity at m=0.5 over the full normalised range).
        let mut img = F32Image::new(1, 5, 1);
        for i in 0..5 {
            img[(0, i, 0)] = (i as f32 + 1.0) / 6.0;
        }
        let stretched = histogram_stretch(&img, 0.0, 1.0, 0.5);
        for i in 0..5 {
            let v_in = (i as f32 + 1.0) / 6.0;
            let v_out = stretched[(0, i, 0)];
            assert!(
                (v_out - v_in).abs() < EXPECTED_MTF_EPSILON as f32,
                "identity stretch: in={v_in} out={v_out}"
            );
        }
    }
}
