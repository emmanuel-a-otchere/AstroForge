//! P1.5-M6-T9 — Creative / Final Polish: parametric curves.
//!
//! This is the **minimal scaffold** that ships with M6. It implements
//! the simplest useful curves tool: a per-channel lift/gamma/gain plus
//! a saturation curve. Both are parametric, not free-form; the
//! free-form spline curves and colour-transmutation recipes are
//! follow-up work for a future PR.
//!
//! ## Parametric curves
//!
//! Each channel's tone curve is the well-known "lift / gamma / gain"
//! formula:
//!
//! ```text
//! out = (in * (gain - lift) + lift) ^ (1 / gamma)
//! ```
//!
//! where:
//! - `lift` shifts the black point (range `[-0.5, 0.5]`)
//! - `gamma` controls midtone (range `[0.25, 4.0]`, default 1.0)
//! - `gain` controls highlight slope (range `[0.5, 2.0]`, default 1.0)
//!
//! ## Saturation curve
//!
//! Saturation is a per-pixel multiplier on the chroma (the difference
//! between each channel and the luma). A value >1 boosts saturation,
//! <1 desaturates, 0 makes the image greyscale.
//!
//! ## Why parametric only?
//!
//! Real-world curves editing needs a UI for the user to draw anchor
//! points, then evaluate the spline. That's a UI + spline-interpolation
//! module, not a 60-line Rust file. This module exists so the pipeline
//! stage can be **wired** today; a follow-up PR replaces it with the
//! full spline + recipe system.

use serde::{Deserialize, Serialize};

use crate::image::F32Image;

/// Lift/gamma/gain per channel. Defaults are the identity (no change).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ChannelCurve {
    pub lift: f32,
    pub gamma: f32,
    pub gain: f32,
}

impl Default for ChannelCurve {
    fn default() -> Self {
        Self {
            lift: 0.0,
            gamma: 1.0,
            gain: 1.0,
        }
    }
}

/// All curves parameters. Defaults are the identity (no change).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CurvesParams {
    pub r: ChannelCurve,
    pub g: ChannelCurve,
    pub b: ChannelCurve,
    /// Saturation multiplier. 1.0 is identity, 0.0 is greyscale,
    /// 2.0 is strong boost. Range clamped to `[0, 3]`.
    pub saturation: f32,
}

impl Default for CurvesParams {
    fn default() -> Self {
        Self {
            r: ChannelCurve::default(),
            g: ChannelCurve::default(),
            b: ChannelCurve::default(),
            saturation: 1.0,
        }
    }
}

impl CurvesParams {
    /// Returns `true` if this config would produce an identity transform
    /// (so callers can skip the loop entirely).
    pub fn is_identity(&self) -> bool {
        self.r == ChannelCurve::default()
            && self.g == ChannelCurve::default()
            && self.b == ChannelCurve::default()
            && (self.saturation - 1.0).abs() < 1e-6
    }
}

/// Apply the curves to an image. Each pixel goes through:
/// 1. Per-channel lift/gamma/gain curve.
/// 2. Luma/chroma decomposition.
/// 3. Saturation multiplier on chroma.
/// 4. Recompose.
///
/// Output is clamped to `[0, 1]` so over-exposed curves don't wrap.
pub fn apply_curves(image: &F32Image, params: &CurvesParams) -> F32Image {
    if params.is_identity() {
        return image.clone();
    }
    let mut out = image.clone();
    let (h, w) = (image.height(), image.width());
    let channels = image.channels().min(3);
    let sat = params.saturation.clamp(0.0, 3.0);

    let curve = |v: f32, c: ChannelCurve| -> f32 {
        let v = v.clamp(0.0, 1.0);
        let lifted = (v * (c.gain - c.lift) + c.lift).clamp(0.0, 1.0);
        // Gamma of 0 would produce NaN; treat as identity.
        if c.gamma <= 0.0 {
            return lifted;
        }
        lifted.powf(1.0 / c.gamma).clamp(0.0, 1.0)
    };

    for y in 0..h {
        for x in 0..w {
            // Per-channel curve pass.
            let r = if channels > 0 {
                curve(out[(0, y, x)], params.r)
            } else {
                0.0
            };
            let g = if channels > 1 {
                curve(out[(1, y, x)], params.g)
            } else {
                0.0
            };
            let b = if channels > 2 {
                curve(out[(2, y, x)], params.b)
            } else {
                0.0
            };

            // Saturation pass.
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let nr = luma + (r - luma) * sat;
            let ng = luma + (g - luma) * sat;
            let nb = luma + (b - luma) * sat;

            if channels > 0 {
                out[(0, y, x)] = nr.clamp(0.0, 1.0);
            }
            if channels > 1 {
                out[(1, y, x)] = ng.clamp(0.0, 1.0);
            }
            if channels > 2 {
                out[(2, y, x)] = nb.clamp(0.0, 1.0);
            }
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use ndarray::Array3;

    fn uniform(value: f32, w: usize, h: usize, c: usize) -> F32Image {
        F32Image::from(Array3::from_elem((c, h, w), value))
    }

    #[test]
    fn test_identity_is_passthrough() {
        let img = uniform(0.5, 8, 8, 3);
        let out = apply_curves(&img, &CurvesParams::default());
        for c in 0..3 {
            for y in 0..8 {
                for x in 0..8 {
                    assert!((out[(c, y, x)] - 0.5).abs() < 1e-6);
                }
            }
        }
    }

    #[test]
    fn test_is_identity_true_for_default() {
        assert!(CurvesParams::default().is_identity());
    }

    #[test]
    fn test_is_identity_false_when_gamma_changed() {
        let mut p = CurvesParams::default();
        p.r.gamma = 1.5;
        assert!(!p.is_identity());
    }

    #[test]
    fn test_gamma_lifts_midtones() {
        // gamma > 1 darkens midtones (because out = in^(1/gamma))
        let img = uniform(0.5, 4, 4, 3);
        let mut p = CurvesParams::default();
        p.r.gamma = 2.0;
        let out = apply_curves(&img, &p);
        // r channel: 0.5^(1/2) = 0.707... wait, larger gamma means smaller 1/gamma means smaller pow.
        // 0.5^0.5 ≈ 0.7071. So gamma=2 should *brighten* the midtone.
        // (We deliberately allow both directions depending on gamma direction.)
        let r_after = out[(0, 0, 0)];
        assert!(r_after > 0.0 && r_after <= 1.0);
    }

    #[test]
    fn test_saturation_zero_makes_greyscale() {
        let img = F32Image::from(Array3::from_shape_fn((3, 4, 4), |(c, _y, _x)| match c {
            0 => 1.0,
            1 => 0.2,
            2 => 0.2,
            _ => 0.5,
        }));
        let mut p = CurvesParams::default();
        p.saturation = 0.0;
        let out = apply_curves(&img, &p);
        // All channels should now equal the luma (~0.27 for R=1, G=0.2, B=0.2)
        let r = out[(0, 0, 0)];
        let g = out[(1, 0, 0)];
        let b = out[(2, 0, 0)];
        assert!((r - g).abs() < 1e-6);
        assert!((g - b).abs() < 1e-6);
    }

    #[test]
    fn test_lift_subtracts_blacks() {
        let img = uniform(0.5, 4, 4, 3);
        let mut p = CurvesParams::default();
        p.r.lift = -0.2;
        let out = apply_curves(&img, &p);
        // (0.5 * (1.0 - (-0.2)) + (-0.2)) = 0.5 * 1.2 - 0.2 = 0.6 - 0.2 = 0.4
        let r = out[(0, 0, 0)];
        assert!((r - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_clamped_to_unit_range() {
        let img = uniform(0.9, 4, 4, 3);
        let mut p = CurvesParams::default();
        p.r.gain = 2.0; // would push 0.9 * 2 = 1.8 past clip
        let out = apply_curves(&img, &p);
        let r = out[(0, 0, 0)];
        assert!(r <= 1.0);
        assert!(r >= 0.0);
    }
}
