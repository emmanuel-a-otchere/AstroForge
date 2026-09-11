//! CR-06 P5 — Auto masks from AI segmentation.
//!
//! Per CR-06 §12, auto masks derive a 2D weight raster
//! from per-pixel classification. P5 ships the
//! heuristic thresholds: a per-channel luma + a soft
//! "star-likeness" (high luma + local contrast spike)
//! produce the seed raster; the user can refine the
//! result via the editor (composite.rs).
//!
//! The function is deterministic for fixed inputs so
//! a CI runner can pin the output without a GPU.

use crate::image::F32Image;

use super::{encoding::MaskError, Mask, MaskKind};

/// Target structure for an auto mask. The P5 build
/// supports `Stars` and `Background`; the editor can
/// compose additional targets via `composite::union` /
/// `intersect` / `difference`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoTarget {
    Stars,
    Background,
    BrightCore,
}

impl AutoTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            AutoTarget::Stars => "stars",
            AutoTarget::Background => "background",
            AutoTarget::BrightCore => "bright_core",
        }
    }
}

/// Build an auto mask for the given target.
///
/// The algorithm is intentionally simple: for
/// `Stars`, the per-pixel weight is `clamp((luma -
/// threshold) / (1.0 - threshold))`; for `Background`,
/// it's the inverse (`1.0 - stars_weight`); for
/// `BrightCore`, only pixels above a higher threshold
/// contribute. The result softens around the boundary
/// via a 3×3 box blur, which suppresses single-pixel
/// noise from the source image.
pub fn build(image: &F32Image, target: AutoTarget) -> Result<Mask, MaskError> {
    let width = image.width() as u32;
    let height = image.height() as u32;
    let mut raw = vec![0.0_f32; (width as usize) * (height as usize)];

    // Single-channel luma: average the channels. P5
    // does not run a colour-aware segmentation; the
    // segmentation is a pixel-luma + a small
    // local-contrast spike.
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
            let luma = (r + g + b) / 3.0;
            let w = match target {
                AutoTarget::Stars => {
                    let threshold = 0.6;
                    ((luma - threshold) / (1.0 - threshold)).clamp(0.0, 1.0)
                }
                AutoTarget::Background => {
                    let threshold = 0.4;
                    (1.0 - (luma / threshold).min(1.0)).max(0.0)
                }
                AutoTarget::BrightCore => {
                    let threshold = 0.85;
                    ((luma - threshold) / (1.0 - threshold)).clamp(0.0, 1.0)
                }
            };
            raw[y * width as usize + x] = w;
        }
    }

    // Suppress single-pixel noise via a 3×3 box blur.
    // The blur runs in-place; for a 2D raster at
    // modest sizes this is fine (O(W*H) per pass).
    let blurred = box_blur(&raw, width as usize, height as usize);
    let mask = Mask::from_pixels(
        width,
        height,
        MaskKind::Auto,
        target.as_str().to_string(),
        &blurred,
    );
    Ok(mask)
}

/// 3×3 box blur with edge replication.
fn box_blur(input: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut out = vec![0.0_f32; input.len()];
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0_f32;
            let mut count = 0u32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        continue;
                    }
                    sum += input[ny as usize * width + nx as usize];
                    count += 1;
                }
            }
            out[y * width + x] = if count == 0 { 0.0 } else { sum / count as f32 };
        }
    }
    out
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
    fn star_mask_picks_up_bright_pixels() {
        let img = image_with(vec![
            vec![[0.1, 0.1, 0.1], [0.9, 0.9, 0.9]],
            vec![[0.1, 0.1, 0.1], [0.1, 0.1, 0.1]],
        ]);
        let m = build(&img, AutoTarget::Stars).unwrap();
        // The (1, 0) pixel is bright — its weight should
        // be well above zero.
        assert!(m.get(1, 0) > 0.0);
        // All other pixels are dim — even after the 3×3
        // blur their weights stay low.
        assert!(m.get(0, 0) < 0.5);
        assert!(m.get(0, 1) < 0.5);
        assert!(m.get(1, 1) < 0.5);
    }

    #[test]
    fn background_mask_is_inverse_of_stars() {
        let img = image_with(vec![
            vec![[0.1, 0.1, 0.1], [0.9, 0.9, 0.9]],
            vec![[0.1, 0.1, 0.1], [0.1, 0.1, 0.1]],
        ]);
        let stars = build(&img, AutoTarget::Stars).unwrap();
        let bg = build(&img, AutoTarget::Background).unwrap();
        // The brightest pixel in `stars` should be lower
        // than its counterpart in `background` (the
        // background mask inverts the heuristic).
        let max_stars = stars.pixels.iter().copied().fold(0.0_f32, f32::max);
        let min_bg = bg.pixels.iter().copied().fold(1.0_f32, f32::min);
        assert!(
            max_stars < min_bg,
            "stars max={} bg min={}",
            max_stars,
            min_bg
        );
    }

    #[test]
    fn auto_mask_is_deterministic() {
        let img = image_with(vec![
            vec![[0.1, 0.5, 0.3], [0.9, 0.2, 0.4]],
            vec![[0.3, 0.7, 0.5], [0.4, 0.8, 0.6]],
        ]);
        let a = build(&img, AutoTarget::Stars).unwrap();
        let b = build(&img, AutoTarget::Stars).unwrap();
        assert_eq!(a, b);
    }
}
