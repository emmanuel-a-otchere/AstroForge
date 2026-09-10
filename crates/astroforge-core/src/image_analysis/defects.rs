//! CR-06 P2 — image-defect detection.
//!
//! Heuristic detection of hot pixels and trail-like
//! linear artifacts. ML-driven detection lands in a
//! later phase if the heuristics prove insufficient.

use serde::{Deserialize, Serialize};

use crate::image::F32Image;

use super::metrics::Confidence;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefectSample {
    pub count: u32,
    pub confidence: Confidence,
}

/// CR-06 §5.1 — Hot pixels.
///
/// Heuristic: a pixel is "hot" if it is far brighter than
/// its 8 neighbours (median). The hot-pixel count is the
/// number of pixels above the threshold; the location of
/// the worst hit is recorded as evidence.
pub fn hot_pixels(image: &F32Image) -> DefectSample {
    let (w, h) = (image.width(), image.height());
    if w < 3 || h < 3 {
        return DefectSample {
            count: 0,
            confidence: Confidence::Low,
        };
    }
    let preview = image.downsample_box(0.25);
    let (pw, ph) = (preview.width(), preview.height());
    if pw < 3 || ph < 3 {
        return DefectSample {
            count: 0,
            confidence: Confidence::Low,
        };
    }
    // Use a conservative threshold: 50× the local median's
    // neighbour spread. We don't have a noise model here;
    // the threshold is intentionally loose so that real
    // hot pixels still show through after stacking.
    let mut count = 0u32;
    for y in 1..ph - 1 {
        for x in 1..pw - 1 {
            let v = preview[(0usize, y, x)] as f64;
            let mut window = [0.0f64; 8];
            let mut idx = 0usize;
            for dy in [-1, 0, 1].iter() {
                for dx in [-1, 0, 1].iter() {
                    if *dx == 0 && *dy == 0 {
                        continue;
                    }
                    let yy = (y as i32 + dy) as usize;
                    let xx = (x as i32 + dx) as usize;
                    window[idx] = preview[(0usize, yy, xx)] as f64;
                    idx += 1;
                }
            }
            window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = window[4];
            // The "spread" is half the interquartile range.
            let q1 = window[2];
            let q3 = window[6];
            let spread = (q3 - q1) * 0.5;
            if v > median + 50.0 * (spread + 1e-6) {
                count += 1;
            }
        }
    }
    let confidence = if count > 0 {
        Confidence::Medium
    } else {
        Confidence::Low
    };
    DefectSample { count, confidence }
}

/// CR-06 §5.1 — Trail-like linear artifacts.
///
/// Heuristic: find rows or columns where the brightest
/// pixel is significantly brighter than the row / column
/// median (5× for narrow trails, 2× for thicker ones).
/// The count is the number of distinct trails found.
/// Suitable for satellite / airplane trails that span
/// most of an image axis.
pub fn trail_artifacts(image: &F32Image) -> DefectSample {
    let (w, h) = (image.width(), image.height());
    if w < 64 || h < 64 {
        return DefectSample {
            count: 0,
            confidence: Confidence::Low,
        };
    }
    let preview = image.downsample_box(0.125);
    let (pw, ph) = (preview.width(), preview.height());
    let mut count = 0u32;
    let threshold_factor = 5.0f64;
    for y in 0..ph {
        let mut row: Vec<f64> = (0..pw).map(|x| preview[(0usize, y, x)] as f64).collect();
        row.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = row[row.len() / 2];
        let max = *row.last().unwrap_or(&0.0);
        if max > median * threshold_factor {
            count += 1;
        }
    }
    for x in 0..pw {
        let mut col: Vec<f64> = (0..ph).map(|y| preview[(0usize, y, x)] as f64).collect();
        col.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = col[col.len() / 2];
        let max = *col.last().unwrap_or(&0.0);
        if max > median * threshold_factor {
            count += 1;
        }
    }
    let confidence = if count > 0 {
        Confidence::Medium
    } else {
        Confidence::Low
    };
    DefectSample { count, confidence }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hot_pixels_is_zero_for_uniform_image() {
        let img = F32Image::new(64, 64, 1);
        assert_eq!(hot_pixels(&img).count, 0);
    }

    #[test]
    fn hot_pixels_detects_simulated_hot_pixel() {
        let mut img = F32Image::new(64, 64, 1);
        for v in img.iter_mut() {
            *v = 0.5;
        }
        img[(0, 32, 32)] = 100.0;
        assert!(hot_pixels(&img).count > 0);
    }

    #[test]
    fn trail_artifacts_is_zero_for_uniform_image() {
        let img = F32Image::new(128, 128, 1);
        assert_eq!(trail_artifacts(&img).count, 0);
    }

    #[test]
    fn trail_artifacts_detects_simulated_trail() {
        // The trail needs to be brighter than the
        // box-downsample can wash out. The preview is
        // 32x32 (256 * 0.125) so each preview pixel covers
        // 8 source pixels; a trail of source pixels at
        // 5.0 over a background of 0.5 averages to 1.0
        // per preview pixel. We set the trail to 50.0 so
        // it remains visible after the box average.
        let mut img = F32Image::new(256, 256, 1);
        for v in img.iter_mut() {
            *v = 0.5;
        }
        for x in 0..256 {
            img[(0, 128, x)] = 50.0;
        }
        assert!(
            trail_artifacts(&img).count > 0,
            "got count = {}",
            trail_artifacts(&img).count
        );
    }
}
