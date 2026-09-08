use std::ops::{Deref, DerefMut, Sub};

use ndarray::Array3;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 3-channel float image stored as `(channels, height, width)`.
///
/// This is a newtype wrapper around [`Array3<f32>`] so the crate can define
/// inherent constructor / accessor methods without violating Rust's orphan
/// rule (which forbids `impl` on a foreign type, even via a type alias).
///
/// All `Array3` methods remain accessible through `Deref`/`DerefMut`, so
/// call sites that index with `img[(c, y, x)]`, call `.iter()`, `.sum()`,
/// `.len()`, `.mean()`, etc. work unchanged. Pixel-wise arithmetic
/// (`a - b`) is supported via the `Sub` impls below.
#[derive(Debug, Clone)]
pub struct F32Image(Array3<f32>);

impl F32Image {
    /// Create a zero-filled image with `channels × height × width` elements.
    /// The underlying `Array3` stores shape as `(channels, height, width)`.
    pub fn new(width: usize, height: usize, channels: usize) -> Self {
        Self(Array3::zeros((channels, height, width)))
    }

    pub fn width(&self) -> usize {
        self.0.shape()[2]
    }

    pub fn height(&self) -> usize {
        self.0.shape()[1]
    }

    pub fn channels(&self) -> usize {
        self.0.shape()[0]
    }

    /// CR-05 P4 slice 5 — box-sampling downsample for preview
    /// generation (CR-05 §11). Each output pixel is the mean of the
    /// source pixels in its footprint, so the preview preserves
    /// global signal level (unlike nearest-neighbour subsampling,
    /// which would alias gradients).
    ///
    /// `scale` is clamped to `(0.0, 1.0]`; a scale of 1.0 returns a
    /// clone. Output dimensions are `max(1, round(dim * scale))` so
    /// even tiny scales produce a viewable 1×1 image.
    pub fn downsample_box(&self, scale: f64) -> F32Image {
        let scale = scale.clamp(f64::EPSILON, 1.0);
        if (scale - 1.0).abs() < f64::EPSILON {
            return self.clone();
        }
        let (c, h, w) = (self.channels(), self.height(), self.width());
        let out_w = ((w as f64 * scale).round() as usize).max(1);
        let out_h = ((h as f64 * scale).round() as usize).max(1);
        let mut out = Array3::<f32>::zeros((c, out_h, out_w));
        // Per-output-pixel source footprint via the box-sampling
        // mapping: out pixel (ox, oy) covers source rows
        // [oy*h/out_h, (oy+1)*h/out_h) and cols likewise.
        for oy in 0..out_h {
            let y0 = (oy * h) / out_h;
            let y1 = (((oy + 1) * h) / out_h).max(y0 + 1).min(h);
            for ox in 0..out_w {
                let x0 = (ox * w) / out_w;
                let x1 = (((ox + 1) * w) / out_w).max(x0 + 1).min(w);
                let count = ((y1 - y0) * (x1 - x0)) as f32;
                for ch in 0..c {
                    let mut acc = 0.0f32;
                    for sy in y0..y1 {
                        for sx in x0..x1 {
                            acc += self.0[(ch, sy, sx)];
                        }
                    }
                    out[(ch, oy, ox)] = acc / count;
                }
            }
        }
        F32Image(out)
    }
}

impl Deref for F32Image {
    type Target = Array3<f32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for F32Image {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Array3<f32>> for F32Image {
    fn from(arr: Array3<f32>) -> Self {
        Self(arr)
    }
}

impl From<F32Image> for Array3<f32> {
    fn from(img: F32Image) -> Self {
        img.0
    }
}

// Pixel-wise subtraction. Without these, `dark - bias` and `&result - &dark`
// (which the rest of the crate relies on) fail to compile because `Sub`
// is only implemented for `Array3<f32>`, not for our newtype wrapper.

impl Sub for &F32Image {
    type Output = F32Image;

    fn sub(self, rhs: &F32Image) -> F32Image {
        F32Image(&self.0 - &rhs.0)
    }
}

impl Sub for F32Image {
    type Output = F32Image;

    fn sub(self, rhs: F32Image) -> F32Image {
        F32Image(self.0 - rhs.0)
    }
}

// Forward serde through the wrapped Array3. The derive macro can't reach
// into a private field through the orphan rule, so we hand-roll.

impl Serialize for F32Image {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for F32Image {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Array3::<f32>::deserialize(deserializer).map(F32Image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CR-05 P4 slice 5 — 4x4 → 2x2 at scale 0.5: each output pixel
    /// is the mean of its 2x2 source block.
    #[test]
    fn downsample_box_halves_dimensions_and_averages() {
        let mut img = F32Image::new(4, 4, 1);
        // Fill row-major with 0..16 so block means are easy to
        // verify by hand:
        //   [ 0  1 |  2  3 ]
        //   [ 4  5 |  6  7 ]
        //   -------+-------
        //   [ 8  9 | 10 11 ]
        //   [12 13 | 14 15 ]
        for y in 0..4 {
            for x in 0..4 {
                img[(0, y, x)] = (y * 4 + x) as f32;
            }
        }
        let out = img.downsample_box(0.5);
        assert_eq!(out.width(), 2);
        assert_eq!(out.height(), 2);
        assert_eq!(out.channels(), 1);
        // Block means: (0+1+4+5)/4 = 2.5, (2+3+6+7)/4 = 4.5,
        // (8+9+12+13)/4 = 10.5, (10+11+14+15)/4 = 12.5
        assert!((out[(0, 0, 0)] - 2.5).abs() < 1e-6);
        assert!((out[(0, 0, 1)] - 4.5).abs() < 1e-6);
        assert!((out[(0, 1, 0)] - 10.5).abs() < 1e-6);
        assert!((out[(0, 1, 1)] - 12.5).abs() < 1e-6);
    }

    /// CR-05 P4 slice 5 — scale 1.0 returns a same-size clone.
    #[test]
    fn downsample_box_scale_one_is_identity() {
        let mut img = F32Image::new(3, 2, 1);
        img[(0, 1, 2)] = 7.0;
        let out = img.downsample_box(1.0);
        assert_eq!(out.width(), 3);
        assert_eq!(out.height(), 2);
        assert!((out[(0, 1, 2)] - 7.0).abs() < 1e-6);
    }

    /// CR-05 P4 slice 5 — a very small scale still yields a
    /// viewable 1×1 image whose pixel is the global mean.
    #[test]
    fn downsample_box_tiny_scale_floors_to_1x1() {
        let mut img = F32Image::new(8, 8, 1);
        for y in 0..8 {
            for x in 0..8 {
                img[(0, y, x)] = 1.0;
            }
        }
        let out = img.downsample_box(0.01);
        assert_eq!(out.width(), 1);
        assert_eq!(out.height(), 1);
        assert!((out[(0, 0, 0)] - 1.0).abs() < 1e-6);
    }

    /// CR-05 P4 slice 5 — multi-channel images downsample each
    /// channel independently.
    #[test]
    fn downsample_box_handles_multi_channel() {
        let mut img = F32Image::new(4, 4, 3);
        for c in 0..3 {
            for y in 0..4 {
                for x in 0..4 {
                    img[(c, y, x)] = c as f32;
                }
            }
        }
        let out = img.downsample_box(0.5);
        assert_eq!(out.channels(), 3);
        for c in 0..3 {
            assert!((out[(c, 0, 0)] - c as f32).abs() < 1e-6);
        }
    }
}
