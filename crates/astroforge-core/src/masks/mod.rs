//! CR-06 P5 — Region-aware mask system.
//!
//! Per CR-06 §12, every AI operation that targets a
//! semantic region consumes a `Mask` (a 2D raster of
//! `[0, 1]` weights). The mask system has four kinds:
//!
//! - `Auto` — generated from analysis (stars / nebula /
//!   galaxy / core / background thresholds).
//! - `Parametric` — derived from astrophotography
//!   properties (star brightness, size, luminance,
//!   saturation).
//! - `User` — hand-painted by the user (brush, polygon,
//!   gradient, radial).
//! - `Composite` — boolean composition of any of the
//!   above (union / intersect / difference).
//!
//! All four kinds share one canonical wire shape: a
//! flat `Vec<f32>` of length `width * height` carrying
//! per-pixel weights, plus the kind + provenance
//! metadata. The encoding is JSON-friendly so the
//! `AiMask::mask_json` column can store it without
//! schema pinning.
//!
//! The module is pure-Rust. No GPU. Pixel-level
//! operations use `Mask::from_pixels` / `to_vec` /
//! `apply` / `union` / `intersect` / `difference`; the
//! renderer (CR-07) reads the raster and draws it onto
//! the Zone B canvas with adjustable opacity.

pub mod auto;
pub mod composite;
pub mod encoding;
pub mod parametric;
pub mod user;

use serde::{Deserialize, Serialize};

/// Mask kind. Each kind corresponds to a `Mask::from_*`
/// constructor in the matching submodule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskKind {
    Auto,
    Parametric,
    User,
    Composite,
}

impl MaskKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaskKind::Auto => "auto",
            MaskKind::Parametric => "parametric",
            MaskKind::User => "user",
            MaskKind::Composite => "composite",
        }
    }
}

/// A 2D mask raster. Per-pixel weights in `[0, 1]`,
/// where `1.0` means "fully included" and `0.0` means
/// "fully excluded". Operations that need soft masks
/// (e.g. a feathered brush) can use any value in
/// between.
///
/// Storage is row-major: `pixels[y * width + x]`.
/// The `Mask` is `Clone`-able but the clone is
/// `O(width * height)` — large rasters should be
/// reference-counted at the application boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mask {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<f32>,
    pub kind: MaskKind,
    pub provenance: String,
}

impl Mask {
    /// Build a mask from a flat row-major pixel buffer.
    /// Pixel values outside `[0, 1]` are clamped (defensive
    /// — the analyzer + user brushes both work in
    /// `[0, 1]`).
    pub fn from_pixels(
        width: u32,
        height: u32,
        kind: MaskKind,
        provenance: String,
        raw: &[f32],
    ) -> Self {
        let expected = (width as usize) * (height as usize);
        let mut pixels = vec![0.0_f32; expected];
        for (i, &v) in raw.iter().take(expected).enumerate() {
            pixels[i] = v.clamp(0.0, 1.0);
        }
        Self {
            width,
            height,
            pixels,
            kind,
            provenance,
        }
    }

    /// Build an empty (all-zeros) mask of the given size.
    /// Useful for `Mask::union`/`intersect` operations
    /// where one operand is a fresh composite.
    pub fn zeros(width: u32, height: u32, kind: MaskKind, provenance: String) -> Self {
        let n = (width as usize) * (height as usize);
        Self {
            width,
            height,
            pixels: vec![0.0; n],
            kind,
            provenance,
        }
    }

    /// Number of pixels in the raster.
    pub fn len(&self) -> usize {
        self.pixels.len()
    }

    /// True when the raster has zero pixels (an empty
    /// mask shape).
    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }

    /// Read a single pixel value at (x, y). Returns
    /// `0.0` when the coordinates are out of bounds
    /// (defensive — out-of-range sampling produces no
    /// contribution).
    pub fn get(&self, x: u32, y: u32) -> f32 {
        if x >= self.width || y >= self.height {
            return 0.0;
        }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        self.pixels[idx]
    }

    /// Write a single pixel value at (x, y). No-op when
    /// the coordinates are out of bounds.
    pub fn set(&mut self, x: u32, y: u32, value: f32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        self.pixels[idx] = value.clamp(0.0, 1.0);
    }

    /// Iterate (x, y, weight) triples. Useful for the
    /// renderer (`MaskOverlay.svelte`) and for tests
    /// that inspect per-pixel state without copying
    /// the buffer.
    pub fn iter_pixels(&self) -> impl Iterator<Item = (u32, u32, f32)> + '_ {
        let w = self.width;
        self.pixels
            .iter()
            .enumerate()
            .map(move |(i, &v)| ((i % w as usize) as u32, (i / w as usize) as u32, v))
    }

    /// Apply this mask to a per-pixel weight from another
    /// source (a user brush stroke, a parametric
    /// generator, etc.). The result is `min(self, other)`
    /// within the mask region — the mask is the
    /// restrictive end of the composition. Outside the
    /// mask region (out-of-bounds coordinates) the
    /// mask has no opinion, so the source passes
    /// through unmolested.
    pub fn apply(&self, source: f32, x: u32, y: u32) -> f32 {
        if x >= self.width || y >= self.height {
            return source;
        }
        source.min(self.get(x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_pixels_clamps_out_of_range() {
        let m = Mask::from_pixels(2, 2, MaskKind::Auto, "test".into(), &[2.0, -1.0, 0.5, 0.7]);
        assert_eq!(m.get(0, 0), 1.0);
        assert_eq!(m.get(1, 0), 0.0);
        assert_eq!(m.get(0, 1), 0.5);
        assert_eq!(m.get(1, 1), 0.7);
    }

    #[test]
    fn get_set_round_trip() {
        let mut m = Mask::zeros(4, 4, MaskKind::User, "test".into());
        m.set(2, 2, 0.6);
        assert_eq!(m.get(2, 2), 0.6);
        assert_eq!(m.get(0, 0), 0.0);
    }

    #[test]
    fn get_out_of_bounds_returns_zero() {
        let m = Mask::zeros(2, 2, MaskKind::User, "test".into());
        assert_eq!(m.get(99, 99), 0.0);
    }

    #[test]
    fn zeros_mask_is_all_zero() {
        let m = Mask::zeros(3, 3, MaskKind::Composite, "test".into());
        assert_eq!(m.len(), 9);
        assert!(m.iter_pixels().all(|(_, _, v)| v == 0.0));
    }

    #[test]
    fn apply_takes_min_with_source() {
        let mut m = Mask::zeros(3, 3, MaskKind::Composite, "test".into());
        m.set(1, 1, 0.4);
        // Inside the mask region at a pixel where the
        // mask is zero: source is clamped to zero (the
        // mask is the restrictive end of the
        // composition).
        assert_eq!(m.apply(0.8, 0, 0), 0.0);
        // Inside the mask region at a pixel where the
        // mask is 0.4: source is clamped to the mask
        // weight (mask wins).
        assert_eq!(m.apply(0.8, 1, 1), 0.4);
        // Source values below the mask are unchanged.
        assert_eq!(m.apply(0.2, 1, 1), 0.2);
        // Out-of-bounds coordinates: the mask has no
        // opinion, so the source passes through.
        assert_eq!(m.apply(0.8, 99, 99), 0.8);
    }

    #[test]
    fn iter_pixels_yields_row_major() {
        let m = Mask::from_pixels(2, 2, MaskKind::Auto, "test".into(), &[0.1, 0.2, 0.3, 0.4]);
        let collected: Vec<_> = m.iter_pixels().collect();
        assert_eq!(collected[0], (0, 0, 0.1));
        assert_eq!(collected[1], (1, 0, 0.2));
        assert_eq!(collected[2], (0, 1, 0.3));
        assert_eq!(collected[3], (1, 1, 0.4));
    }

    #[test]
    fn mask_kind_round_trips_string_form() {
        for k in [
            MaskKind::Auto,
            MaskKind::Parametric,
            MaskKind::User,
            MaskKind::Composite,
        ] {
            assert_eq!(k.as_str(), format!("{:?}", k).to_lowercase());
        }
    }
}
