//! CR-06 P5 — Parametric masks.
//!
//! Per CR-06 §12, parametric masks derive a 2D weight
//! raster from astrophotography properties (star
//! brightness, star size, luminance, saturation). P5
//! ships the four canonical kinds:
//!
//! - `StarBrightness` — high luma pixels with a soft
//!   threshold (catches faint stars).
//! - `StarSize` — pixels whose 3×3 neighbourhood has
//!   high contrast (catches star cores).
//! - `Luminance` — plain luma > threshold (the user
//!   picks the threshold via the editor slider).
//! - `Saturation` — pixels where `(max - min) > t`
//!   (catches colourful stars + nebular emission).
//!
//! The result is composited with the user's existing
//! mask via `composite::union` / `intersect` to build
//! the parametric recipe. The kind enum is small so a
//! future slice can extend without breaking the wire
//! shape.

use crate::image::F32Image;

use super::{Mask, MaskKind};

/// The four canonical parametric kinds from CR-06 §12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParametricKind {
    StarBrightness,
    StarSize,
    Luminance,
    Saturation,
}

impl ParametricKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ParametricKind::StarBrightness => "star_brightness",
            ParametricKind::StarSize => "star_size",
            ParametricKind::Luminance => "luminance",
            ParametricKind::Saturation => "saturation",
        }
    }
}

/// Build a parametric mask.
///
/// `threshold` is the per-kind activation threshold
/// (a float in `[0, 1]`). Pixels above the threshold
/// contribute `1.0`; pixels below contribute `0.0`;
/// pixels near the threshold get a soft falloff so the
/// mask edge is anti-aliased.
pub fn build(image: &F32Image, kind: ParametricKind, threshold: f32) -> Mask {
    let width = image.width() as u32;
    let height = image.height() as u32;
    let n = (width as usize) * (height as usize);
    let mut raw = vec![0.0_f32; n];
    let threshold = threshold.clamp(0.0, 1.0);

    for y in 0..height as usize {
        for x in 0..width as usize {
            let r = image[(0, y, x)];
            let g = if image.channels() > 1 {
                image[(1, y, x)]
            } else {
                r
            };
            let b = if image.channels() > 2 {
                image[(2, y, x)]
            } else {
                r
            };
            let raw_pixel = match kind {
                ParametricKind::StarBrightness | ParametricKind::Luminance => (r + g + b) / 3.0,
                ParametricKind::Saturation => {
                    let max = r.max(g).max(b);
                    let min = r.min(g).min(b);
                    max - min
                }
                ParametricKind::StarSize => {
                    // 3×3 neighbourhood contrast (max - min
                    // across the 3×3 window). The P2
                    // analysis engine has a richer local
                    // contrast metric; this is a fast
                    // approximation good enough for
                    // parametric seeds.
                    let mut lo = f32::INFINITY;
                    let mut hi = f32::NEG_INFINITY;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                                continue;
                            }
                            let v = image[(0, ny as usize, nx as usize)];
                            if v < lo {
                                lo = v;
                            }
                            if v > hi {
                                hi = v;
                            }
                        }
                    }
                    if lo.is_finite() {
                        hi - lo
                    } else {
                        0.0
                    }
                }
            };
            // Soft falloff: weights between `threshold`
            // and `threshold + 0.1` ramp linearly from
            // 0 to 1. Outside that window the weight is
            // either 0 (below) or 1 (above).
            let w = if raw_pixel <= threshold {
                0.0
            } else if raw_pixel >= threshold + 0.1 {
                1.0
            } else {
                (raw_pixel - threshold) / 0.1
            };
            raw[y * width as usize + x] = w;
        }
    }

    Mask::from_pixels(
        width,
        height,
        MaskKind::Parametric,
        kind.as_str().to_string(),
        &raw,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

    fn image_with(rows: Vec<Vec<[f32; 3]>>) -> F32Image {
        let h = rows.len();
        let w = rows[0].len();
        let mut arr = Array3::<f32>::zeros((3, h, w));
        for (y, row) in rows.iter().enumerate() {
            for (x, px) in row.iter().enumerate() {
                arr[(0, y, x)] = px[0];
                arr[(1, y, x)] = px[1];
                arr[(2, y, x)] = px[2];
            }
        }
        F32Image::from(arr)
    }

    #[test]
    fn star_brightness_mask_uses_luma() {
        let img = image_with(vec![
            vec![[0.1, 0.1, 0.1], [0.9, 0.9, 0.9]],
            vec![[0.1, 0.1, 0.1], [0.1, 0.1, 0.1]],
        ]);
        let m = build(&img, ParametricKind::StarBrightness, 0.5);
        // The single bright pixel gets a weight close to 1.
        assert!(m.get(1, 0) > 0.9);
        // The four dim pixels stay at 0 (well below the
        // threshold).
        assert_eq!(m.get(0, 0), 0.0);
        assert_eq!(m.get(0, 1), 0.0);
        assert_eq!(m.get(1, 1), 0.0);
    }

    #[test]
    fn saturation_mask_picks_colorful_pixels() {
        let img = image_with(vec![
            vec![[0.5, 0.5, 0.5], [0.9, 0.1, 0.5]],
            vec![[0.5, 0.5, 0.5], [0.1, 0.9, 0.5]],
        ]);
        let m = build(&img, ParametricKind::Saturation, 0.5);
        // (1, 0): (0.9, 0.1, 0.5) — saturation = 0.8
        assert!(m.get(1, 0) > 0.9);
        // (0, 0): (0.5, 0.5, 0.5) — saturation = 0
        assert_eq!(m.get(0, 0), 0.0);
    }

    #[test]
    fn threshold_clamps_to_range() {
        let img = image_with(vec![vec![[0.5, 0.5, 0.5], [0.5, 0.5, 0.5]]]);
        let _ = build(&img, ParametricKind::Luminance, 2.0); // above 1
        let _ = build(&img, ParametricKind::Luminance, -1.0); // below 0
    }

    #[test]
    fn kind_strings_match_documentation() {
        assert_eq!(ParametricKind::StarBrightness.as_str(), "star_brightness");
        assert_eq!(ParametricKind::StarSize.as_str(), "star_size");
        assert_eq!(ParametricKind::Luminance.as_str(), "luminance");
        assert_eq!(ParametricKind::Saturation.as_str(), "saturation");
    }
}
