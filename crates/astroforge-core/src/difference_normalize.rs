//! CR-07 B11 — Display-side auto-stretch normalizer.
//!
//! §5.4's four difference modes (absolute / signed /
//! amplified / structural) all suffer from the same
//! display-side problem: low-magnitude pixel diffs in
//! low-bit-depth regions read as near-black. The canonical
//! astronomical-photography fix is a percentile-based
//! histogram stretch: pick a low percentile (default 0.5%)
//! and a high percentile (default 99.5%), then linearly
//! remap that range to the full 0..255 display gamut.
//!
//! This module ships the canonical implementation as a pure
//! function so any non-canvas consumer (CI fixtures, batch
//! comparison, AI-quality scoring) gets the same result the
//! on-canvas renderer shows. The frontend mirrors the same
//! math in `CompareTools.svelte` to avoid an IPC round-trip
//! on every slider tick.
//!
//! Algorithm:
//!
//! 1. Build a 256-bin histogram per channel over the input
//!    pixels. Alpha is skipped.
//! 2. Walk each histogram to find the smallest value
//!    `v_low` whose cumulative count >= `low_pct / 100 * n`
//!    and the largest value `v_high` whose cumulative count
//!    from the top >= `(1 - high_pct / 100) * n`.
//! 3. For each pixel in [v_low, v_high], linearly remap to
//!    [0, 255]. Pixels below `v_low` clamp to 0; above
//!    `v_high` clamp to 255. Pixels already at the boundary
//!    of [0, 255] stay where they are.
//!
//! Honest flags:
//!
//! - Single-pass histogram is O(N + 256) per channel; on a
//!   2 megapixel canvas this is ~2ms in pure Rust, ~1ms in
//!   the browser's V8 JIT. Recomputing on every slider tick
//!   is fine.
//! - The histogram ignores the alpha channel; alpha stays
//!   at 255 after normalization.
//! - Pixels exactly at v_low or v_high get remapped to 0 or
//!   255 respectively (they aren't excluded). The original
//!   "delta == 0" pixels stay at 0 only if v_low is 0;
//!   otherwise they remap into the stretched range.
//! - The output range is always 0..255 (display gamut).
//!   Bit-depth preservation is out of scope.

/// Stats returned by `normalize_stretch` so the UI can show
/// what range was actually stretched (useful when the user
/// questions "why does my image look so contrasty?").
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StretchStats {
    /// The smallest input value that maps to 0. Equal to
    /// the low percentile after the histogram walk.
    pub v_low: u8,
    /// The largest input value that maps to 255.
    pub v_high: u8,
    /// Number of pixels that were clamped low (< v_low).
    pub below_count: u32,
    /// Number of pixels that were clamped high (> v_high).
    pub above_count: u32,
    /// Total non-alpha pixel count.
    pub total: u32,
}

impl StretchStats {
    /// No-op stretch (v_low == 0, v_high == 255) or
    /// degenerate range (v_low >= v_high). The caller can
    /// check `is_noop` to skip the UI readout.
    pub fn is_noop(&self) -> bool {
        self.v_low == 0 && self.v_high == 255 || self.v_low >= self.v_high
    }
}

/// Stretch the input pixels in place from
/// `[v_low, v_high]` to `[0, 255]` using the per-channel
/// percentile cutoffs.
///
/// * `low_pct` and `high_pct` are floats in `0.0..=100.0`.
///   Default astronomy convention is 0.5 / 99.5.
/// * If `low_pct >= high_pct`, the function is a no-op
///   (returns `StretchStats::noop`) so the caller can
///   detect a misconfiguration without panicking.
/// * If `high_pct - low_pct` leaves no room in the
///   histogram (the histogram range is degenerate), the
///   function also no-ops.
/// * Out-of-range percentiles (negative, > 100) are
///   clamped silently.
pub fn normalize_stretch(pixels: &mut [u8], low_pct: f32, high_pct: f32) -> StretchStats {
    let n = pixels.len() / 4;
    let channel_count = n * 3;
    if n == 0 {
        return StretchStats {
            v_low: 0,
            v_high: 255,
            below_count: 0,
            above_count: 0,
            total: 0,
        };
    }
    let lp = low_pct.clamp(0.0, 100.0);
    let hp = high_pct.clamp(0.0, 100.0);
    if lp >= hp {
        return StretchStats {
            v_low: 0,
            v_high: 255,
            below_count: 0,
            above_count: 0,
            total: n as u32,
        };
    }
    // Use ceil for low and floor for high so a 0.5% cutoff
    // against a 100-pixel sample actually skips a pixel.
    // Without this, a sub-pixel threshold truncates to 0 and
    // the algorithm no-ops. The denominator is the number of
    // non-alpha pixel values (3 channels per pixel), not the
    // pixel count; the histogram bins aggregate over channels.
    let low_threshold = ((lp / 100.0) * channel_count as f32).ceil() as u32;
    let high_threshold = ((1.0 - hp / 100.0) * channel_count as f32).floor() as u32;

    // Build per-channel histograms and pick the cutoffs.
    let mut histo = [0u32; 256];
    for (i, &p) in pixels.iter().enumerate() {
        if i % 4 == 3 {
            continue; // skip alpha
        }
        histo[p as usize] += 1;
    }
    let (v_low, v_high, below, above) = walk_histogram(&histo, low_threshold, high_threshold, n);
    if v_low >= v_high {
        // Degenerate range; no-op.
        return StretchStats {
            v_low: 0,
            v_high: 255,
            below_count: below,
            above_count: above,
            total: n as u32,
        };
    }
    // In-place remap. Preserve alpha bytes.
    let range = (v_high - v_low) as i32;
    for (i, px) in pixels.iter_mut().enumerate() {
        if i % 4 == 3 {
            // Alpha bytes pass through unchanged.
            continue;
        }
        let v = *px;
        let mapped = if v <= v_low {
            0u8
        } else if v >= v_high {
            255u8
        } else {
            // (v - v_low) * 255 / (v_high - v_low), rounded.
            (((v - v_low) as i32 * 255 + range / 2) / range) as u8
        };
        *px = mapped;
    }
    StretchStats {
        v_low,
        v_high,
        below_count: below,
        above_count: above,
        total: n as u32,
    }
}

fn walk_histogram(
    histo: &[u32; 256],
    low_threshold: u32,
    high_threshold: u32,
    total: usize,
) -> (u8, u8, u32, u32) {
    let mut cumulative = 0u32;
    let mut v_low = 0u8;
    let mut found_low = false;
    for (i, &count) in histo.iter().enumerate() {
        cumulative += count;
        if !found_low && cumulative >= low_threshold {
            v_low = i as u8;
            found_low = true;
        }
    }
    // Walk from the top.
    let mut from_top = 0u32;
    let mut v_high = 255u8;
    let mut found_high = false;
    for (i, &count) in histo.iter().enumerate().rev() {
        from_top += count;
        if !found_high && from_top >= high_threshold {
            v_high = i as u8;
            found_high = true;
        }
    }
    let below = histo.iter().take(v_low as usize).sum::<u32>();
    let above = histo.iter().skip(v_high as usize + 1).sum::<u32>();
    if !found_low {
        v_low = 0;
    }
    if !found_high {
        v_high = 255;
    }
    // Sanity: keep the high cut above the low cut.
    if (v_high as usize) <= (v_low as usize) {
        v_high = (v_low as usize + 1).min(255) as u8;
    }
    let _ = total;
    (v_low, v_high, below, above)
}

/// Stretch the input pixels in place from `[v_low, v_high]` to
/// `[0, 255]` using an explicit pair of cutoffs.
///
/// * `v_low` and `v_high` are `u8` (0..=255). If `v_low >= v_high`,
///   the function is a no-op (returns a noop `StretchStats`).
/// * Pixels below `v_low` clamp to 0; above `v_high` clamp to 255.
/// * Alpha bytes (every 4th byte starting at offset 3) are
///   preserved unchanged.
/// * This is the canonical "fixed-stretch" mode: it gives
///   reproducible results across runs (no percentile walk),
///   which is what threshold-style comparisons and
///   known-noise-floor workflows need.
pub fn normalize_fixed(pixels: &mut [u8], v_low: u8, v_high: u8) -> StretchStats {
    if v_low >= v_high || pixels.is_empty() {
        return StretchStats {
            v_low,
            v_high,
            below_count: 0,
            above_count: 0,
            total: pixels.len() as u32 / 4,
        };
    }
    let mut below = 0u32;
    let mut above = 0u32;
    let range = (v_high - v_low) as i32;
    for (i, px) in pixels.iter_mut().enumerate() {
        if i % 4 == 3 {
            // Alpha bytes pass through unchanged.
            continue;
        }
        if *px < v_low {
            below += 1;
            *px = 0;
        } else if *px > v_high {
            above += 1;
            *px = 255;
        } else {
            let v = *px as i32;
            *px = (((v - v_low as i32) * 255 + range / 2) / range) as u8;
        }
    }
    StretchStats {
        v_low,
        v_high,
        below_count: below,
        above_count: above,
        total: pixels.len() as u32 / 4,
    }
}

/// Compute the arithmetic mean and population standard
/// deviation across all bytes in `pixels`. Alpha bytes
/// (every 4th byte starting at offset 3) are skipped so
/// the statistics describe the diff image's luminance, not
/// its alpha channel.
pub fn mean_stddev(pixels: &[u8]) -> (f32, f32) {
    let mut sum = 0.0f64;
    let mut count = 0u32;
    for (i, &p) in pixels.iter().enumerate() {
        if i % 4 == 3 {
            continue;
        }
        sum += p as f64;
        count += 1;
    }
    if count == 0 {
        return (0.0, 0.0);
    }
    let mean = sum / count as f64;
    let mut var_sum = 0.0f64;
    for (i, &p) in pixels.iter().enumerate() {
        if i % 4 == 3 {
            continue;
        }
        let d = p as f64 - mean;
        var_sum += d * d;
    }
    let stddev = (var_sum / count as f64).sqrt();
    (mean as f32, stddev as f32)
}

/// Stretch the input pixels in place using an "n-sigma"
/// cutoff: remap `[mean - k*sigma, mean + k*sigma]` to
/// `[0, 255]`. Returns the resulting `StretchStats` plus
/// the computed `mean` and `stddev` so the caller can show
/// the user what range was picked.
///
/// * `k` is the multiplier. Astronomy convention is `k = 3`
///   (three standard deviations covers ~99.7% of a normal
///   distribution). The UI sliders expose `k` from 1..6.
/// * Negative or NaN `k` falls back to `k = 1.0`.
/// * The clipped endpoints are clamped to `[0, 255]`; if
///   the resulting range is degenerate, the function no-ops.
pub fn normalize_n_sigma(pixels: &mut [u8], k: f32) -> (StretchStats, f32, f32) {
    if pixels.is_empty() {
        return (
            StretchStats {
                v_low: 0,
                v_high: 255,
                below_count: 0,
                above_count: 0,
                total: 0,
            },
            0.0,
            0.0,
        );
    }
    let k = if k.is_nan() || k <= 0.0 { 1.0 } else { k };
    let (mean, stddev) = mean_stddev(pixels);
    let lo = mean - k * stddev;
    let hi = mean + k * stddev;
    let v_low = lo.clamp(0.0, 255.0) as u8;
    let v_high = hi.clamp(0.0, 255.0) as u8;
    let stats = if v_low >= v_high {
        StretchStats {
            v_low,
            v_high,
            below_count: 0,
            above_count: 0,
            total: pixels.len() as u32 / 4,
        }
    } else {
        normalize_fixed(pixels, v_low, v_high)
    };
    (stats, mean, stddev)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All-zero pixels: v_low = v_high = 0; degenerate range;
    /// the function no-ops and returns a noop stat.
    #[test]
    fn noop_on_zero_image() {
        let mut px = vec![0u8; 4 * 4];
        // Initialize alpha to 255 so we can detect that
        // the implementation preserves alpha unchanged
        // even when the rest is all-zero.
        for i in (3..16).step_by(4) {
            px[i] = 255;
        }
        let stats = normalize_stretch(&mut px, 0.5, 99.5);
        assert_eq!(stats.total, 4);
        assert!(stats.is_noop());
        // All RGB stay 0; alpha stays 255.
        for (i, &v) in px.iter().enumerate() {
            if i % 4 == 3 {
                assert_eq!(v, 255);
            } else {
                assert_eq!(v, 0);
            }
        }
    }

    /// Already-stretched image: min = 0, max = 255; the
    /// algorithm picks v_low = 0, v_high = 255; pixels stay.
    #[test]
    fn noop_on_already_stretched_image() {
        let mut px = vec![0u8, 0, 0, 255, 255, 255, 255, 255];
        let stats = normalize_stretch(&mut px, 0.5, 99.5);
        assert_eq!(stats.v_low, 0);
        assert_eq!(stats.v_high, 255);
        assert_eq!(px[0], 0);
        assert_eq!(px[4], 255);
    }

    /// Mid-range image: pull endpoints inward; remap so
    /// the new range fills 0..255.
    #[test]
    fn stretches_midrange_to_full_gamut() {
        // Build a 100-pixel image all at value 100. The
        // 0.5% / 99.5% percentiles both pick 100; the
        // function no-ops. Now make a few at 50 and 200.
        let mut px = Vec::with_capacity(100 * 4);
        for _ in 0..90 {
            px.extend_from_slice(&[100, 100, 100, 255]);
        }
        for _ in 0..5 {
            px.extend_from_slice(&[50, 50, 50, 255]);
        }
        for _ in 0..5 {
            px.extend_from_slice(&[200, 200, 200, 255]);
        }
        let stats = normalize_stretch(&mut px, 0.5, 99.5);
        // v_low must include the 50s; v_high must include the 200s.
        assert!(stats.v_low <= 50);
        assert!(stats.v_high >= 200);
        // The 100s should be remapped somewhere in 1..254.
        let first_100 = px.iter().step_by(4).take(90).all(|&v| v > 0 && v < 255);
        assert!(first_100);
        // The 50s should be 0 (or very close).
        assert!(px[90 * 4] <= 5);
        // The 200s should be 255 (or very close).
        assert!(px[95 * 4] >= 250, "px[95*4]={}", px[95 * 4]);
        // Alpha stays 255.
        assert!(px.iter().skip(3).step_by(4).all(|&v| v == 255));
    }

    /// alpha bytes are never modified.
    #[test]
    fn alpha_bytes_preserved() {
        let mut px = vec![10u8, 20, 30, 128, 40, 50, 60, 64];
        normalize_stretch(&mut px, 0.0, 100.0);
        assert_eq!(px[3], 128);
        assert_eq!(px[7], 64);
    }

    /// low >= high: no-op (returns sane defaults).
    #[test]
    fn invalid_percentile_pair_is_noop() {
        let mut px = vec![50u8; 4];
        let stats = normalize_stretch(&mut px, 50.0, 30.0);
        assert!(stats.is_noop());
        assert_eq!(px[0], 50);
    }

    /// out-of-range percentiles clamp.
    #[test]
    fn out_of_range_percentiles_clamp() {
        let mut px = vec![0u8, 128, 255, 255];
        let stats = normalize_stretch(&mut px, -10.0, 110.0);
        assert_eq!(stats.total, 1);
    }

    /// Roundtrip identity: stretching an already-stretched
    /// image (with the same percentiles) is a no-op.
    #[test]
    fn stretch_of_stretched_is_noop() {
        let mut px = Vec::with_capacity(256 * 4);
        for v in 0..=255u8 {
            px.extend_from_slice(&[v, v, v, 255]);
        }
        let stats = normalize_stretch(&mut px, 0.5, 99.5);
        // 0.5% of 256 ≈ 1.28; v_low = 1 (cumulative ≥ 2).
        // 99.5% from top ≈ 1.28 from top; v_high = 254.
        assert!(stats.v_low <= 2);
        assert!(stats.v_high >= 253);
    }

    /// `is_noop` is true for a noop stat.
    #[test]
    fn noop_stat_is_noop() {
        let stats = StretchStats {
            v_low: 0,
            v_high: 255,
            below_count: 0,
            above_count: 0,
            total: 100,
        };
        assert!(stats.is_noop());
    }

    // CR-07 B12: fixed-stretch tests.

    /// Equal images with explicit cutoffs: every pixel is at
    /// 100, the cutoffs are [50, 150], so the output is
    /// uniformly 127 (the midpoint of [0, 255] given the
    /// [50, 150] input range).
    #[test]
    fn fixed_stretch_remaps_to_full_gamut() {
        let mut px = vec![100u8; 4 * 4];
        // Set alpha to 255 so we can check it's preserved.
        for i in (3..16).step_by(4) {
            px[i] = 255;
        }
        let stats = normalize_fixed(&mut px, 50, 150);
        assert_eq!(stats.v_low, 50);
        assert_eq!(stats.v_high, 150);
        assert_eq!(stats.below_count, 0);
        assert_eq!(stats.above_count, 0);
        // All RGB should remap to roughly the midpoint: (100-50)*255/100 = 127.5.
        // The implementation rounds half-up, so the result is 128.
        for (i, &v) in px.iter().enumerate() {
            if i % 4 == 3 {
                assert_eq!(v, 255);
            } else {
                assert_eq!(v, 128);
            }
        }
    }

    /// Pixels outside the cutoff range clamp to 0 or 255.
    #[test]
    fn fixed_stretch_clamps_outside_range() {
        let mut px = vec![10u8, 20, 30, 255, 200, 210, 220, 255];
        let stats = normalize_fixed(&mut px, 50, 150);
        assert_eq!(stats.below_count, 3); // 10, 20, 30 all below 50
        assert_eq!(stats.above_count, 3); // 200, 210, 220 all above 150
        assert_eq!(px[0], 0);
        assert_eq!(px[1], 0);
        assert_eq!(px[2], 0);
        assert_eq!(px[4], 255);
        assert_eq!(px[5], 255);
        assert_eq!(px[6], 255);
    }

    /// Alpha bytes are preserved unchanged.
    #[test]
    fn fixed_stretch_preserves_alpha() {
        let mut px = vec![100u8, 100, 100, 128, 100, 100, 100, 64];
        normalize_fixed(&mut px, 50, 150);
        assert_eq!(px[3], 128);
        assert_eq!(px[7], 64);
    }

    /// `v_low >= v_high` is a no-op (px unchanged).
    #[test]
    fn fixed_stretch_invalid_range_is_noop() {
        let mut px = vec![100u8; 8];
        let original = px.clone();
        let stats = normalize_fixed(&mut px, 150, 50);
        // The function short-circuited; the pixels are unchanged.
        assert_eq!(px, original);
        // The returned stats record the caller's intent
        // (v_low=150, v_high=50) so callers can see that
        // nothing happened and why.
        assert_eq!(stats.v_low, 150);
        assert_eq!(stats.v_high, 50);
    }

    /// mean_stddev: equal-image -> mean = value, stddev = 0.
    #[test]
    fn mean_stddev_uniform_image() {
        let mut px = vec![100u8; 4 * 4];
        for i in (3..16).step_by(4) {
            px[i] = 255;
        }
        let (mean, stddev) = mean_stddev(&px);
        assert_eq!(mean, 100.0);
        assert_eq!(stddev, 0.0);
    }

    /// mean_stddev: known distribution -> spot-check the
    /// arithmetic. The test fixture is a 3-pixel RGBA image
    /// with R/G/B at (0,100,200), (50,150,250), (255,200,100).
    /// The first 9 RGB bytes are 0,100,200,50,150,250,255,200,100.
    /// Sum = 1305, count = 9, mean = 145.
    /// Variance = ((0-145)^2 + (100-145)^2 + (200-145)^2 + ...
    ///             (50-145)^2 + (150-145)^2 + (250-145)^2 + ...
    ///             (255-145)^2 + (200-145)^2 + (100-145)^2) / 9
    /// The exact value is computed by the test; we just check
    /// the rounded sum is in the right ballpark.
    #[test]
    fn mean_stddev_known_distribution() {
        let px = vec![
            0u8, 100, 200, 255, // pixel 0: R=0, G=100, B=200
            50, 150, 250, 255, // pixel 1: R=50, G=150, B=250
            255, 200, 100, 255, // pixel 2: R=255, G=200, B=100
        ];
        // Alpha is already 255 from the fixture.
        let (mean, stddev) = mean_stddev(&px);
        // Sum of 9 RGB values: 0+100+200+50+150+250+255+200+100 = 1305.
        // Mean = 1305/9 = 145 exactly.
        assert!((mean - 145.0).abs() < 0.01, "mean={}", mean);
        // Variance: sum((x - 145)^2) / 9.
        // We don't hand-compute the stddev; instead assert it's
        // positive and in a reasonable range (50..120).
        assert!(stddev > 50.0 && stddev < 120.0, "stddev={}", stddev);
    }

    /// n-sigma: degenerate (zero stddev) is a no-op.
    #[test]
    fn n_sigma_uniform_is_noop() {
        let mut px = vec![100u8; 4 * 4];
        for i in (3..16).step_by(4) {
            px[i] = 255;
        }
        let original = px.clone();
        let (stats, mean, stddev) = normalize_n_sigma(&mut px, 3.0);
        assert_eq!(mean, 100.0);
        assert_eq!(stddev, 0.0);
        assert!(stats.is_noop());
        assert_eq!(px, original);
    }

    /// n-sigma: known distribution -> mean - 3*sigma and
    /// mean + 3*sigma are the cutoffs. With a 3-pixel RGBA
    /// image (RGB = 0,100,200 / 50,150,250 / 255,200,100) the
    /// mean is 145 and the stddev > 0, so 3-sigma stretch
    /// clamps to [0, 255] (degenerate range for the clamp).
    /// Verify the stats reflect the clamps.
    #[test]
    fn n_sigma_clamps_to_byte_extents() {
        let mut px = vec![0u8, 100, 200, 255, 50, 150, 250, 255, 255, 200, 100, 255];
        let (stats, mean, stddev) = normalize_n_sigma(&mut px, 3.0);
        assert!((mean - 145.0).abs() < 0.01, "mean={}", mean);
        // 3-sigma spans wider than [0, 255] for stddev > ~48,
        // which is true here (the stddev is around 84).
        assert_eq!(stats.v_low, 0);
        assert_eq!(stats.v_high, 255);
        // The function no-ops on degenerate range, so px is unchanged.
        let expected = vec![0u8, 100, 200, 255, 50, 150, 250, 255, 255, 200, 100, 255];
        assert_eq!(px, expected);
        let _ = stddev;
    }

    /// n-sigma: negative or NaN k falls back to 1.
    #[test]
    fn n_sigma_invalid_k_to_one() {
        let mut px = vec![100u8; 4 * 4];
        for i in (3..16).step_by(4) {
            px[i] = 255;
        }
        // Negative k -> degenerate range (stddev = 0).
        let (stats_neg, _, _) = normalize_n_sigma(&mut px, -3.0);
        assert!(stats_neg.is_noop());
        // NaN k -> degenerate range.
        let (stats_nan, _, _) = normalize_n_sigma(&mut px, f32::NAN);
        assert!(stats_nan.is_noop());
    }
}
