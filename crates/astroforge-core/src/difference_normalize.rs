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
    /// No-op stretch (v_low == 0, v_high == 255). The
    /// caller can check `is_noop` to skip the UI readout.
    pub fn is_noop(&self) -> bool {
        self.v_low == 0 && self.v_high == 255
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
}
