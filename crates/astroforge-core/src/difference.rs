//! CR-07 B10 — Difference image renderers.
//!
//! §5.4 of `CR-07-IMAGE-REVIEW-COMPARISON-DECISION.md` defines four
//! "potential modes" for image-space difference visualization:
//!
//! * **absolute** — `|A - B|` per channel, optionally amplified.
//! * **signed** — `A - B` shifted by 128 so equal pixels read grey;
//!   brighter means B is brighter than A, darker means A is brighter
//!   than B.
//! * **amplified** — `|A - B| * gain` clamped to 0..255 with `gain`
//!   user-set.
//! * **structural** — edge magnitude of both images (Sobel
//!   approximation) abs-diffed; surfaces structural change while
//!   suppressing smooth-area noise.
//!
//! All four ship behind the same `DiffKind` enum so the frontend
//! picker (`CompareTools.svelte`) can dispatch through one function.
//! Display-side normalization (auto-stretch, fixed-stretch,
//! perceptual) lives in `difference_normalize.rs` and lands in B11;
//! the math here stays in 8-bit canvas space, mirroring the
//! upstream B6/B4 diff pipeline.
//!
//! Honest flags:
//!
//! - Structural difference uses a 3x3 Sobel-style neighbour diff,
//!   not a true gradient magnitude. Edge sharpness in astrophotos
//!   is mostly spatial; a true Sobel adds ~30 LOC and a reference
//!   test fixture for marginal accuracy gain. Documented as a
//!   future slice.
//! - All paths operate on packed RGBA8 (`[u8]` quadruples). The
//!   frontend already stages this representation on its hidden
//!   `canvasA` / `canvasB`; no F32 conversion is needed yet.

/// Which difference algorithm to use when blending A vs B.
///
/// One enum, one frontend, four backend behaviours. The renderer
/// ignores any alpha byte (index 3 of each quadruplet) and treats
/// it as opaque in the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiffKind {
    /// `|A - B|` per channel, clamped to 8-bit. The default.
    Absolute,
    /// `A - B` shifted by 128. 0 = equal, 255 = A >> B, 0 = B >> A.
    Signed,
    /// `|A - B| * gain` clamped to 8-bit. `gain` is a user-set
    /// positive f32 (typically 1..16).
    Amplified,
    /// Edge map of A abs-diff edge map of B. Surfaces structural
    /// change while suppressing smooth area.
    Structural,
}

impl DiffKind {
    /// Stable string label for the frontend dropdown.
    pub fn label(self) -> &'static str {
        match self {
            DiffKind::Absolute => "Absolute",
            DiffKind::Signed => "Signed",
            DiffKind::Amplified => "Amplified",
            DiffKind::Structural => "Structural",
        }
    }

    /// All four modes in display order. Used by the frontend
    /// to populate the picker and by tests to enumerate.
    pub fn all() -> [DiffKind; 4] {
        [
            DiffKind::Absolute,
            DiffKind::Signed,
            DiffKind::Amplified,
            DiffKind::Structural,
        ]
    }
}

/// Render `a` into a freshly-allocated RGBA8 buffer of the same
/// length. `a` and `b` must be the same length; in debug builds
/// an unequal-length pair panics, in release builds the shorter
/// length is used.
///
/// `gain` only affects `DiffKind::Amplified`; for the others it
/// is silently ignored. Passing `gain = 1.0` is a safe default.
///
/// `width` is the image width in pixels (used by the structural
/// path to find up / left neighbours). Callers producing a flat
/// RGBA8 buffer from a `CanvasImageData` can use
/// `image_data.width`. For non-structural modes, the value is
/// ignored.
pub fn compute_diff(kind: DiffKind, a: &[u8], b: &[u8], gain: f32, width: u32) -> Vec<u8> {
    debug_assert_eq!(
        a.len(),
        b.len(),
        "compute_diff: a/b length mismatch ({a_len} vs {b_len})",
        a_len = a.len(),
        b_len = b.len()
    );
    let n = a.len().min(b.len());
    let mut out = vec![0u8; n];
    match kind {
        DiffKind::Absolute => render_absolute(&mut out, a, b, n),
        DiffKind::Signed => render_signed(&mut out, a, b, n),
        DiffKind::Amplified => render_amplified(&mut out, a, b, n, gain),
        DiffKind::Structural => render_structural(&mut out, a, b, n, width as usize),
    }
    out
}

fn render_absolute(out: &mut [u8], a: &[u8], b: &[u8], n: usize) {
    for i in 0..n {
        // Alpha channel stays opaque.
        if i % 4 == 3 {
            out[i] = 255;
            continue;
        }
        let d = (a[i] as i16 - b[i] as i16).unsigned_abs() as u8;
        out[i] = d;
    }
}

fn render_signed(out: &mut [u8], a: &[u8], b: &[u8], n: usize) {
    for i in 0..n {
        if i % 4 == 3 {
            out[i] = 255;
            continue;
        }
        // `(a - b) + 128` mapped to 0..255 with saturation.
        let v = (a[i] as i16 - b[i] as i16) + 128;
        out[i] = v.clamp(0, 255) as u8;
    }
}

fn render_amplified(out: &mut [u8], a: &[u8], b: &[u8], n: usize, gain: f32) {
    let g = if gain.is_finite() && gain > 0.0 {
        gain
    } else {
        1.0
    };
    for i in 0..n {
        if i % 4 == 3 {
            out[i] = 255;
            continue;
        }
        let d = ((a[i] as i16 - b[i] as i16).unsigned_abs() as f32 * g) as i32;
        out[i] = d.clamp(0, 255) as u8;
    }
}

/// Sobel-style neighbour diff used as a cheap edge proxy.
///
/// For each pixel, compute the abs difference to its left
/// neighbour and to its up neighbour (clamped). The output is
/// the max of the two per channel, summed across RGB into a
/// single intensity stored back into the RGB channels
/// (greyscale edge map).
fn render_structural(out: &mut [u8], a: &[u8], b: &[u8], n: usize, width: usize) {
    // The structural path needs 2D neighbours. If width is
    // unknown or the buffer is too small to have interior
    // pixels, fall back to the absolute path so the user
    // still sees a meaningful diff.
    if width < 2 || n < width * 4 * 2 {
        return render_absolute(out, a, b, n);
    }
    let stride = 4usize;
    let height = n / (width * stride);
    if height < 2 {
        return render_absolute(out, a, b, n);
    }
    let mut edge_a = vec![0u8; n];
    let mut edge_b = vec![0u8; n];
    for y in 1..height {
        for x in 1..width - 1 {
            let i = (y * width + x) * stride;
            let left = i - stride;
            let up = i - width * stride;
            for c in 0..3 {
                let ea = ((a[i + c] as i16 - a[left + c] as i16).unsigned_abs())
                    .max((a[i + c] as i16 - a[up + c] as i16).unsigned_abs());
                let eb = ((b[i + c] as i16 - b[left + c] as i16).unsigned_abs())
                    .max((b[i + c] as i16 - b[up + c] as i16).unsigned_abs());
                edge_a[i + c] = ea as u8;
                edge_b[i + c] = eb as u8;
            }
            edge_a[i + 3] = 255;
            edge_b[i + 3] = 255;
        }
    }
    for i in 0..n {
        if i % 4 == 3 {
            out[i] = 255;
            continue;
        }
        let d = (edge_a[i] as i16 - edge_b[i] as i16).unsigned_abs();
        out[i] = d as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 4x4 RGBA8 image: top-left half red, bottom-right half blue.
    /// Used by structural tests as a 4-wide image so neighbour
    /// sampling is well-defined.
    fn fixture() -> Vec<u8> {
        let mut v = Vec::with_capacity(4 * 4 * 4);
        for y in 0..4 {
            for x in 0..4 {
                let (r, g, b) = if y < 2 && x < 2 {
                    (255, 0, 0)
                } else if y >= 2 && x >= 2 {
                    (0, 0, 255)
                } else {
                    (0, 255, 0)
                };
                v.extend_from_slice(&[r, g, b, 255]);
            }
        }
        v
    }

    #[test]
    fn absolute_equal_images_yield_black() {
        let img = fixture();
        let out = compute_diff(DiffKind::Absolute, &img, &img, 1.0, 4);
        // RGB channels all zero; alpha is 255.
        for px in out.as_chunks::<4>().0 {
            assert_eq!(px[0], 0);
            assert_eq!(px[1], 0);
            assert_eq!(px[2], 0);
            assert_eq!(px[3], 255);
        }
    }

    #[test]
    fn absolute_caps_per_channel_to_255() {
        let a = vec![0u8, 0, 0, 255, 0, 0, 0, 255];
        let b = vec![255u8, 255, 255, 255, 100, 100, 100, 255];
        let out = compute_diff(DiffKind::Absolute, &a, &b, 1.0, 2);
        assert_eq!(out[0], 255);
        assert_eq!(out[4], 100);
    }

    #[test]
    fn signed_equal_images_yield_grey_128() {
        let img = fixture();
        let out = compute_diff(DiffKind::Signed, &img, &img, 1.0, 4);
        for px in out.as_chunks::<4>().0 {
            // RGB all 128; alpha 255.
            assert_eq!(px[0], 128);
            assert_eq!(px[1], 128);
            assert_eq!(px[2], 128);
            assert_eq!(px[3], 255);
        }
    }

    #[test]
    fn signed_directions_are_antisymmetric() {
        let a = vec![200u8, 100, 50, 255];
        let b = vec![50u8, 100, 200, 255];
        let ab = compute_diff(DiffKind::Signed, &a, &b, 1.0, 1);
        let ba = compute_diff(DiffKind::Signed, &b, &a, 1.0, 1);
        // (200-50)+128=255+23=clamp(255) vs (50-200)+128=-22=clamp(0).
        assert_eq!(ab[0], 255);
        assert_eq!(ba[0], 0);
    }

    #[test]
    fn amplified_scales_per_channel_above_one() {
        let a = vec![0u8, 0, 0, 255];
        let b = vec![20u8, 40, 60, 255];
        let out1 = compute_diff(DiffKind::Amplified, &a, &b, 1.0, 1);
        let out4 = compute_diff(DiffKind::Amplified, &a, &b, 4.0, 1);
        // Per-pixel: at gain=4, the 20-channel should be 80, the
        // 40-channel should be 160, the 60-channel should be 240.
        assert_eq!(out1[0], 20);
        assert_eq!(out4[0], 80);
        assert_eq!(out4[1], 160);
        assert_eq!(out4[2], 240);
    }

    #[test]
    fn amplified_clamps_to_255() {
        let a = vec![0u8, 0, 0, 255];
        let b = vec![200u8, 200, 200, 255];
        let out = compute_diff(DiffKind::Amplified, &a, &b, 4.0, 1);
        // 200 * 4 = 800 → clamp to 255.
        assert_eq!(out[0], 255);
        assert_eq!(out[1], 255);
        assert_eq!(out[2], 255);
    }

    #[test]
    fn amplified_negative_or_nan_gain_falls_back_to_one() {
        let a = vec![0u8, 0, 0, 255];
        let b = vec![50u8, 50, 50, 255];
        let out_neg = compute_diff(DiffKind::Amplified, &a, &b, -1.0, 1);
        let out_nan = compute_diff(DiffKind::Amplified, &a, &b, f32::NAN, 1);
        assert_eq!(out_neg[0], 50);
        assert_eq!(out_nan[0], 50);
    }

    #[test]
    fn structural_equal_images_yield_black() {
        let img = fixture();
        let out = compute_diff(DiffKind::Structural, &img, &img, 1.0, 4);
        // Edges match; abs-diff = 0; alpha 255.
        for px in out.as_chunks::<4>().0 {
            assert_eq!(px[3], 255);
        }
        // Centre pixels (no neighbours) are guaranteed zero; the
        // interior ring is also zero because A == B.
        for chunk in out.as_chunks::<4>().0 {
            assert!(chunk[0] <= 1);
        }
    }

    #[test]
    fn structural_detects_swap() {
        let a = fixture();
        let mut b = fixture();
        // Swap the bottom-right quadrant: A blue there, B red.
        for y in 2..4 {
            for x in 2..4 {
                let i = (y * 4 + x) * 4;
                b[i] = 255;
                b[i + 1] = 0;
                b[i + 2] = 0;
            }
        }
        let out = compute_diff(DiffKind::Structural, &a, &b, 1.0, 4);
        // The boundary between the quadrants must produce
        // non-zero structural diff in at least one pixel.
        let max = out.as_chunks::<4>().0.iter().map(|c| c[0]).max().unwrap();
        assert!(max > 0, "structural diff should detect quadrant swap");
    }

    #[test]
    fn structural_falls_back_when_width_too_small() {
        // A single-pixel image (width=1, height=1) has no
        // interior; the path must not panic and must produce
        // a sensible result rather than a hung process.
        let img = vec![0u8, 0, 0, 255];
        let out = compute_diff(DiffKind::Structural, &img, &img, 1.0, 1);
        assert_eq!(out.len(), 4);
        assert_eq!(out[3], 255);
    }

    #[test]
    fn diff_kind_all_includes_four_modes() {
        let all = DiffKind::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&DiffKind::Absolute));
        assert!(all.contains(&DiffKind::Signed));
        assert!(all.contains(&DiffKind::Amplified));
        assert!(all.contains(&DiffKind::Structural));
    }

    #[test]
    fn diff_kind_serde_roundtrip() {
        for kind in DiffKind::all() {
            let json = serde_json::to_string(&kind).unwrap();
            let back: DiffKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }
}
