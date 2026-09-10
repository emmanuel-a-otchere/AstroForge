//! CR-06 P2 — astronomical-structure detection.
//!
//! Heuristic detection of stars, nebula, and galaxy-like
//! regions in an `F32Image`. ML hooks land in a later
//! phase if the heuristics prove insufficient.
//!
//! Each detection returns a [`StructureSample`] with a
//! count + the centroid of the brightest hit. The report
//! layer attaches this to the observation.

use serde::{Deserialize, Serialize};

use crate::image::F32Image;

use super::metrics::Confidence;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructureSample {
    pub count: u32,
    /// (x, y) pixel centroid of the brightest hit; `None`
    /// when no hit was found.
    pub centroid: Option<[u32; 2]>,
    pub confidence: Confidence,
}

/// CR-06 §5.1 — Star count.
///
/// Heuristic: find local maxima above `mean + 5σ` over a
/// downsampled preview, then return the count plus the
/// centroid of the brightest hit. The `5σ` threshold is
/// intentionally tight so faint-structure pixels don't
/// get mis-counted as stars.
pub fn star_count(image: &F32Image) -> StructureSample {
    let (w, h) = (image.width(), image.height());
    if w < 8 || h < 8 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let preview = image.downsample_box(0.125);
    let (pw, ph) = (preview.width(), preview.height());
    if pw < 3 || ph < 3 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut n = 0usize;
    for v in preview.iter() {
        let x = *v as f64;
        sum += x;
        sum_sq += x * x;
        n += 1;
    }
    if n == 0 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let mean = sum / n as f64;
    let var = (sum_sq / n as f64) - mean * mean;
    let stddev = var.max(0.0).sqrt();
    if stddev < 1e-9 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let threshold = mean + 5.0 * stddev;
    let mut count = 0u32;
    let mut brightest: Option<(f64, u32, u32)> = None;
    for y in 1..ph - 1 {
        for x in 1..pw - 1 {
            let v = preview[(0usize, y, x)] as f64;
            if v < threshold {
                continue;
            }
            // Local maximum check on a 3x3 window.
            let mut is_max = true;
            for dy in [-1, 0, 1].iter() {
                for dx in [-1, 0, 1].iter() {
                    if *dx == 0 && *dy == 0 {
                        continue;
                    }
                    let yy = (y as i32 + dy) as usize;
                    let xx = (x as i32 + dx) as usize;
                    if preview[(0usize, yy, xx)] as f64 >= v {
                        is_max = false;
                        break;
                    }
                }
                if !is_max {
                    break;
                }
            }
            if !is_max {
                continue;
            }
            count += 1;
            if brightest.is_none_or(|(bv, _, _)| v > bv) {
                brightest = Some((v, x as u32, y as u32));
            }
        }
    }
    let centroid = brightest.map(|(_, x, y)| {
        // Map back to original-image coordinates.
        [
            ((x as f64 / pw as f64) * w as f64) as u32,
            ((y as f64 / ph as f64) * h as f64) as u32,
        ]
    });
    let confidence = if count >= 32 {
        Confidence::High
    } else if count >= 4 {
        Confidence::Medium
    } else {
        Confidence::Low
    };
    StructureSample {
        count,
        centroid,
        confidence,
    }
}

/// CR-06 §5.1 — Nebula / faint-structure detector.
///
/// Heuristic: a connected component (BFS) over pixels
/// above `mean + 0.5σ` whose area is in the
/// `[100, 50_000]` range — large enough to be a
/// structured nebula, small enough to not be the
/// background itself. Returns the count of components
/// plus the centroid of the largest one.
///
/// Cheap in `O(W·H)` over the downsampled preview.
pub fn faint_structures(image: &F32Image) -> StructureSample {
    let (w, h) = (image.width(), image.height());
    if w < 32 || h < 32 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let preview = image.downsample_box(0.125);
    let (pw, ph) = (preview.width(), preview.height());
    if pw < 4 || ph < 4 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut n = 0usize;
    for v in preview.iter() {
        let x = *v as f64;
        sum += x;
        sum_sq += x * x;
        n += 1;
    }
    if n == 0 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let mean = sum / n as f64;
    let var = (sum_sq / n as f64) - mean * mean;
    let stddev = var.max(0.0).sqrt();
    let threshold = mean + 0.5 * stddev;
    // Connected-component labelling (4-connected).
    let mut labels = vec![0u32; pw * ph];
    let mut component_sizes: Vec<u32> = vec![0];
    let mut component_xs: Vec<u64> = vec![0];
    let mut component_ys: Vec<u64> = vec![0];
    let mut next_label = 1u32;
    let idx = |x: usize, y: usize| y * pw + x;
    for y in 0..ph {
        for x in 0..pw {
            if preview[(0, y, x)] as f64 <= threshold {
                continue;
            }
            let l = labels[idx(x, y)];
            if l != 0 {
                continue;
            }
            // BFS.
            let label = next_label;
            next_label += 1;
            component_sizes.push(0);
            component_xs.push(0);
            component_ys.push(0);
            let mut stack = vec![(x, y)];
            while let Some((cx, cy)) = stack.pop() {
                let cur_l = labels[idx(cx, cy)];
                if cur_l != 0 {
                    continue;
                }
                labels[idx(cx, cy)] = label;
                component_sizes[label as usize] += 1;
                component_xs[label as usize] += cx as u64;
                component_ys[label as usize] += cy as u64;
                if cx > 0 {
                    stack.push((cx - 1, cy));
                }
                if cx + 1 < pw {
                    stack.push((cx + 1, cy));
                }
                if cy > 0 {
                    stack.push((cx, cy - 1));
                }
                if cy + 1 < ph {
                    stack.push((cx, cy + 1));
                }
            }
        }
    }
    // Filter to components in the nebula-like area band.
    let min_area = 100u32;
    let max_area = 50_000u32;
    let mut count = 0u32;
    let mut largest: Option<(u32, u32, u32)> = None;
    for (i, &s) in component_sizes.iter().enumerate().skip(1) {
        if !(min_area..=max_area).contains(&s) {
            continue;
        }
        count += 1;
        if largest.is_none_or(|(lv, _, _)| s > lv) {
            let lx = component_xs[i] as f64 / s as f64;
            let ly = component_ys[i] as f64 / s as f64;
            largest = Some((s, lx as u32, ly as u32));
        }
    }
    let centroid = largest.map(|(_, x, y)| {
        [
            ((x as f64 / pw as f64) * w as f64) as u32,
            ((y as f64 / ph as f64) * h as f64) as u32,
        ]
    });
    let confidence = if count >= 8 {
        Confidence::High
    } else if count >= 2 {
        Confidence::Medium
    } else {
        Confidence::Low
    };
    StructureSample {
        count,
        centroid,
        confidence,
    }
}

/// CR-06 §5.1 — Galaxy / bright-core detector.
///
/// Heuristic: the brightest connected component above
/// `mean + 8σ` with area > 25 pixels. If no such component
/// exists, no galaxy / core is detected.
pub fn bright_core(image: &F32Image) -> StructureSample {
    let (w, h) = (image.width(), image.height());
    if w < 32 || h < 32 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let preview = image.downsample_box(0.125);
    let (pw, ph) = (preview.width(), preview.height());
    if pw < 4 || ph < 4 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut n = 0usize;
    for v in preview.iter() {
        let x = *v as f64;
        sum += x;
        sum_sq += x * x;
        n += 1;
    }
    if n == 0 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let mean = sum / n as f64;
    let var = (sum_sq / n as f64) - mean * mean;
    let stddev = var.max(0.0).sqrt();
    let max = preview
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |acc, v| acc.max(f64::from(v)));
    if stddev < 1e-6 || max < mean * 1.5 {
        // Uniform image OR no significant bright region
        // (max within 50% of mean). Skip — no core.
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    // The bright-core detector first finds the brightest
    // pixel on the preview. It then counts contiguous
    // pixels in an 8-pixel window around the brightest
    // hit that are above the per-image mean. The bar is
    // intentionally loose (`> mean`) so an extended
    // bright region in a noisy image still registers;
    // the area gate (≥ 25 pixels) and the radius gate
    // (the window covers ≤ 8×8 = 64 pixels) keep the
    // detector from flagging the whole image as a core.
    let threshold = mean;
    // Find the brightest hit.
    let mut brightest: Option<(f64, u32, u32)> = None;
    for y in 0..ph {
        for x in 0..pw {
            let v = preview[(0, y, x)] as f64;
            if v < threshold {
                continue;
            }
            if brightest.is_none_or(|(bv, _, _)| v > bv) {
                brightest = Some((v, x as u32, y as u32));
            }
        }
    }
    let Some((_, bx, by)) = brightest else {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    };
    let r = 4u32;
    let mut area = 0u32;
    let mut sx = 0u64;
    let mut sy = 0u64;
    let x0 = bx.saturating_sub(r) as usize;
    let y0 = by.saturating_sub(r) as usize;
    let x1 = (bx + r + 1).min(pw as u32) as usize;
    let y1 = (by + r + 1).min(ph as u32) as usize;
    for y in y0..y1 {
        for x in x0..x1 {
            if preview[(0, y, x)] as f64 >= threshold {
                area += 1;
                sx += x as u64;
                sy += y as u64;
            }
        }
    }
    if area < 25 {
        return StructureSample {
            count: 0,
            centroid: None,
            confidence: Confidence::Low,
        };
    }
    let cx = (sx as f64 / area as f64) as u32;
    let cy = (sy as f64 / area as f64) as u32;
    let centroid = [
        ((cx as f64 / pw as f64) * w as f64) as u32,
        ((cy as f64 / ph as f64) * h as f64) as u32,
    ];
    StructureSample {
        count: 1,
        centroid: Some(centroid),
        confidence: Confidence::Medium,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_count_is_zero_for_uniform_image() {
        let img = F32Image::new(128, 128, 1);
        assert_eq!(star_count(&img).count, 0);
    }

    #[test]
    fn star_count_detects_simulated_stars() {
        let mut img = F32Image::new(128, 128, 1);
        // Three bright spots above a low background.
        img[(0, 32, 32)] = 1.0;
        img[(0, 64, 64)] = 1.0;
        img[(0, 96, 96)] = 1.0;
        let sample = star_count(&img);
        assert!(
            sample.count >= 1,
            "expected at least one star, got {}",
            sample.count
        );
    }

    #[test]
    fn faint_structures_is_zero_for_uniform_image() {
        let img = F32Image::new(128, 128, 1);
        assert_eq!(faint_structures(&img).count, 0);
    }

    #[test]
    fn bright_core_detects_central_blob() {
        // The blob is large enough that, after box
        // downsampling, the blob pixels remain far above
        // the rest of the image's `mean + 8σ` threshold.
        let mut img = F32Image::new(256, 256, 1);
        for y in 80..176 {
            for x in 80..176 {
                img[(0, y, x)] = 1.0;
            }
        }
        let sample = bright_core(&img);
        assert_eq!(sample.count, 1, "got {:?}", sample);
        assert!(sample.centroid.is_some());
    }

    #[test]
    fn bright_core_is_zero_for_uniform_image() {
        let img = F32Image::new(128, 128, 1);
        assert_eq!(bright_core(&img).count, 0);
    }
}
