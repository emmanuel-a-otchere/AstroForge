use crate::image::F32Image;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameQuality {
    pub fwhm: f64,
    pub eccentricity: f64,
    pub star_count: usize,
    pub snr: f64,
    pub background: f64,
    pub cloud_score: f64,
}

pub fn compute_frame_quality(image: &F32Image) -> FrameQuality {
    let stars = crate::registration::extract_stars(image, 3.0);

    let fwhm = if stars.is_empty() {
        0.0
    } else {
        let sum: f64 = stars.iter().map(|s| s.fwhm).sum();
        sum / stars.len() as f64
    };

    let star_count = stars.len();

    let mean = image.iter().sum::<f32>() / image.len() as f32;
    let var = image.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / image.len() as f32;
    let std = var.sqrt();
    let snr = if std > 0.0 {
        mean as f64 / std as f64
    } else {
        0.0
    };
    let background = mean as f64;
    let cloud_score = (std as f64 / mean.max(1e-10) as f64).min(1.0);

    let eccentricity = if !stars.is_empty() {
        let avg_brightness: f64 =
            stars.iter().map(|s| s.brightness).sum::<f64>() / stars.len() as f64;
        (1.0 - (fwhm / (avg_brightness.max(1.0)))).clamp(0.0, 1.0)
    } else {
        0.0
    };

    FrameQuality {
        fwhm,
        eccentricity,
        star_count,
        snr,
        background,
        cloud_score,
    }
}

pub fn filter_frames(qualities: &[FrameQuality], reject_percentile: f64) -> Vec<usize> {
    let n = qualities.len();
    if n == 0 {
        return vec![];
    }

    let reject_count = ((n as f64) * reject_percentile / 100.0).round() as usize;
    if reject_count == 0 {
        return (0..n).collect();
    }

    let mut indexed: Vec<(usize, f64)> = qualities
        .iter()
        .enumerate()
        .map(|(i, q)| (i, q.fwhm))
        .collect();

    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let rejected: std::collections::HashSet<usize> =
        indexed.iter().take(reject_count).map(|(i, _)| *i).collect();

    (0..n).filter(|i| !rejected.contains(i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_uniform_image(w: usize, h: usize, val: f32) -> F32Image {
        let mut img = F32Image::new(w, h, 1);
        img.fill(val);
        img
    }

    #[test]
    fn test_compute_frame_quality() {
        let img = make_uniform_image(16, 16, 100.0);
        let q = compute_frame_quality(&img);
        assert!(q.background > 0.0);
        assert!(q.cloud_score >= 0.0 && q.cloud_score <= 1.0);
    }

    #[test]
    fn test_filter_frames_no_rejection() {
        let qualities = vec![
            FrameQuality {
                fwhm: 2.0,
                eccentricity: 0.5,
                star_count: 100,
                snr: 50.0,
                background: 100.0,
                cloud_score: 0.1,
            },
            FrameQuality {
                fwhm: 3.0,
                eccentricity: 0.6,
                star_count: 80,
                snr: 40.0,
                background: 100.0,
                cloud_score: 0.2,
            },
        ];
        let accepted = filter_frames(&qualities, 0.0);
        assert_eq!(accepted.len(), 2);
    }

    #[test]
    fn test_filter_frames_reject_worst_15pct() {
        let qualities: Vec<FrameQuality> = (0..20)
            .map(|i| FrameQuality {
                fwhm: i as f64,
                eccentricity: 0.5,
                star_count: 100,
                snr: 50.0,
                background: 100.0,
                cloud_score: 0.1,
            })
            .collect();
        let accepted = filter_frames(&qualities, 15.0);
        assert!(accepted.len() < 20);
        assert!(accepted.len() >= 17);
    }
}

// ─── CR-05 P3 slice 1 — QualityMetricSnapshot (deterministic metrics) ──

/// CR-05 P3 slice 1 — deterministic image-quality metrics for a
/// single `F32Image`. Serialised to JSON by the runner and stored
/// in `stage_executions.metric_snapshot_json`.
///
/// Distinct from `FrameQuality` (above), which targets multi-frame
/// selection for stacking. `QualityMetricSnapshot` is the per-stage
/// metric row that the recommendation engine (P3 slice 2) reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityMetricSnapshot {
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub mean: f64,
    pub stddev: f64,
    pub snr_db: f64,
    pub fwhm: f64,
    pub star_count: u32,
    pub background_gradient: f64,
}

impl QualityMetricSnapshot {
    /// CR-05 P3 slice 1 — serialise the snapshot to a JSON string
    /// suitable for the `metric_snapshot_json` column.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// CR-05 P3 slice 1 — deserialise a snapshot from JSON. Used
    /// by the recommendation engine (P3 slice 2) to read prior
    /// stage metrics.
    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }
}

/// CR-05 P3 slice 1 — compute all slice 1 metrics for an image.
/// Deterministic: two calls with identical inputs produce identical
/// outputs (no clock / thread state).
pub fn compute_metrics(image: &F32Image) -> QualityMetricSnapshot {
    let width = image.width() as u32;
    let height = image.height() as u32;
    let channels = image.channels() as u32;
    let (mean, stddev) = mean_stddev(image);
    let snr_db = if stddev > 1e-12 {
        20.0 * (mean.max(1e-12) / stddev).log10()
    } else {
        0.0
    };
    let fwhm = estimate_fwhm(image, mean, stddev);
    let star_count = count_stars(image, mean, stddev);
    let background_gradient = estimate_background_gradient(image);

    QualityMetricSnapshot {
        width,
        height,
        channels,
        mean,
        stddev,
        snr_db,
        fwhm,
        star_count,
        background_gradient,
    }
}

/// CR-05 P3 slice 1 — mean + population stddev over all pixels in
/// all channels. Returns `(0.0, 0.0)` for an empty image.
fn mean_stddev(image: &F32Image) -> (f64, f64) {
    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut n = 0usize;
    for v in image.iter() {
        let x = *v as f64;
        sum += x;
        sum_sq += x * x;
        n += 1;
    }
    if n == 0 {
        return (0.0, 0.0);
    }
    let mean = sum / n as f64;
    let variance = (sum_sq / n as f64) - mean * mean;
    (mean, variance.max(0.0).sqrt())
}

/// CR-05 P3 slice 1 — estimate FWHM of the brightest star-like
/// peak in the image. Strategy: find the max pixel; sample a 5x5
/// horizontal window through it; take the half-max radius in
/// pixels. Returns 0.0 when no clear peak is found.
fn estimate_fwhm(image: &F32Image, mean: f64, stddev: f64) -> f64 {
    if stddev < 1e-9 || image.width() < 5 || image.height() < 5 {
        return 0.0;
    }
    let max_val = image
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |acc, v| acc.max(f64::from(v)));
    if (max_val - mean).abs() < 1.0 {
        return 0.0;
    }
    let mut peak: Option<(usize, usize, usize)> = None;
    'outer: for c in 0..image.channels() {
        for y in 0..image.height() {
            for x in 0..image.width() {
                let v = image[(c, y, x)] as f64;
                if (v - max_val).abs() < 1e-6 {
                    peak = Some((c, y, x));
                    break 'outer;
                }
            }
        }
    }
    let (_, py, px) = match peak {
        Some(p) => p,
        None => return 0.0,
    };

    let c = 0usize;
    let half_max = max_val * 0.5;
    let w = image.width();
    let mut left = px as f64;
    let mut right = px as f64;
    let mut xl = px as i64;
    while xl > 0 {
        let v = image[(c, py, xl as usize)] as f64;
        if v <= half_max {
            left = xl as f64;
            let prev = image[(c, py, (xl + 1) as usize)] as f64;
            if prev > half_max && (prev - v).abs() > 1e-9 {
                let frac = (prev - half_max) / (prev - v);
                left = (xl as f64) + frac;
            }
            break;
        }
        left = xl as f64;
        xl -= 1;
    }
    let mut xr = px as i64;
    while (xr as usize) < w.saturating_sub(1) {
        let v = image[(c, py, xr as usize)] as f64;
        if v <= half_max {
            right = xr as f64;
            let prev = image[(c, py, (xr - 1).max(0) as usize)] as f64;
            if prev > half_max && (prev - v).abs() > 1e-9 {
                let frac = (prev - half_max) / (prev - v);
                right = (xr as f64) - frac;
            }
            break;
        }
        right = xr as f64;
        xr += 1;
    }
    (right - left).max(0.0)
}

/// CR-05 P3 slice 1 — count pixels brighter than
/// `mean + 5 * stddev`. Threshold derived from sigma-clipping
/// conventions used in astronomical image analysis.
fn count_stars(image: &F32Image, mean: f64, stddev: f64) -> u32 {
    let threshold = mean + 5.0 * stddev;
    let mut count = 0u32;
    for v in image.iter() {
        if (*v as f64) > threshold {
            count = count.saturating_add(1);
        }
    }
    count
}

/// CR-05 P3 slice 1 — estimate background gradient as the slope of
/// a linear fit through per-row means. Returns the absolute slope
/// in intensity units per row. 0.0 means a flat background.
fn estimate_background_gradient(image: &F32Image) -> f64 {
    if image.height() < 2 {
        return 0.0;
    }
    let w = image.width() as f64;
    let c = image.channels();
    let h = image.height();
    let mut row_means = Vec::with_capacity(h);
    for y in 0..h {
        let mut s = 0.0f64;
        for x in 0..image.width() {
            for ch in 0..c {
                s += image[(ch, y, x)] as f64;
            }
        }
        row_means.push(s / (w * c as f64));
    }
    let n = row_means.len() as f64;
    let sum_y: f64 = (0..row_means.len()).map(|y| y as f64).sum();
    let sum_yy: f64 = (0..row_means.len()).map(|y| (y * y) as f64).sum();
    let sum_v: f64 = row_means.iter().sum();
    let sum_vy: f64 = row_means
        .iter()
        .enumerate()
        .map(|(y, v)| v * y as f64)
        .sum();
    let denom = n * sum_yy - sum_y * sum_y;
    if denom.abs() < 1e-12 {
        return 0.0;
    }
    ((n * sum_vy - sum_y * sum_v) / denom).abs()
}

#[cfg(test)]
mod p31_tests {
    use super::*;

    fn frame_8x8_ones() -> F32Image {
        let mut img = F32Image::new(8, 8, 1);
        for v in img.iter_mut() {
            *v = 1.0;
        }
        img
    }

    #[test]
    fn mean_stddev_for_uniform_image() {
        let img = frame_8x8_ones();
        let (m, s) = mean_stddev(&img);
        assert!((m - 1.0).abs() < 1e-9);
        assert!(s.abs() < 1e-9, "stddev of constant image must be 0");
    }

    #[test]
    fn mean_stddev_for_empty_or_degenerate() {
        let img = F32Image::new(1, 1, 1);
        let (m, s) = mean_stddev(&img);
        assert!((m - 0.0).abs() < 1e-9);
        assert!(s.abs() < 1e-9);
    }

    #[test]
    fn compute_metrics_for_uniform_returns_zero_snr() {
        let img = frame_8x8_ones();
        let snap = compute_metrics(&img);
        assert_eq!(snap.width, 8);
        assert_eq!(snap.height, 8);
        assert_eq!(snap.channels, 1);
        assert!((snap.mean - 1.0).abs() < 1e-9);
        assert_eq!(snap.stddev, 0.0);
        assert_eq!(snap.snr_db, 0.0);
        assert_eq!(snap.star_count, 0);
        assert_eq!(snap.background_gradient, 0.0);
    }

    #[test]
    fn compute_metrics_with_bright_star() {
        let mut img = F32Image::new(8, 8, 1);
        for y in 0..8 {
            for x in 0..8 {
                img[(0, y, x)] = 0.0;
            }
        }
        img[(0, 4, 4)] = 100.0;
        let snap = compute_metrics(&img);
        assert!(snap.mean > 1.0 && snap.mean < 2.0);
        assert!(
            snap.star_count >= 1,
            "star_count must detect the bright pixel"
        );
        assert!(snap.fwhm >= 0.0);
    }

    #[test]
    fn compute_metrics_deterministic_for_same_input() {
        let mut img = F32Image::new(16, 16, 1);
        for y in 0..16 {
            for x in 0..16 {
                img[(0, y, x)] = ((x + y) % 16) as f32 * 0.1;
            }
        }
        let a = compute_metrics(&img);
        let b = compute_metrics(&img);
        assert_eq!(a, b, "compute_metrics must be deterministic");
    }

    #[test]
    fn background_gradient_detects_linear_ramp() {
        let mut img = F32Image::new(8, 8, 1);
        for y in 0..8 {
            for x in 0..8 {
                img[(0, y, x)] = (y as f32) * 0.1;
            }
        }
        let snap = compute_metrics(&img);
        assert!(snap.background_gradient > 0.0);
    }

    #[test]
    fn background_gradient_zero_for_uniform() {
        let img = frame_8x8_ones();
        let snap = compute_metrics(&img);
        assert_eq!(snap.background_gradient, 0.0);
    }

    #[test]
    fn snapshot_json_round_trip() {
        let mut img = F32Image::new(8, 8, 1);
        for v in img.iter_mut() {
            *v = 0.5;
        }
        let snap = compute_metrics(&img);
        let raw = snap.to_json().unwrap();
        let back = QualityMetricSnapshot::from_json(&raw).unwrap();
        assert_eq!(snap, back);
    }
}
