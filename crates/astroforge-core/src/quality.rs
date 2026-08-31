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
    let snr = if std > 0.0 { mean as f64 / std as f64 } else { 0.0 };
    let background = mean as f64;
    let cloud_score = (std as f64 / mean.max(1e-10) as f64).min(1.0);

    let eccentricity = if !stars.is_empty() {
        let avg_brightness: f64 = stars.iter().map(|s| s.brightness).sum::<f64>() / stars.len() as f64;
        (1.0 - (fwhm / (avg_brightness.max(1.0)))).max(0.0).min(1.0)
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

pub fn filter_frames(
    qualities: &[FrameQuality],
    reject_percentile: f64,
) -> Vec<usize> {
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

    let rejected: std::collections::HashSet<usize> = indexed
        .iter()
        .take(reject_count)
        .map(|(i, _)| *i)
        .collect();

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
            FrameQuality { fwhm: 2.0, eccentricity: 0.5, star_count: 100, snr: 50.0, background: 100.0, cloud_score: 0.1 },
            FrameQuality { fwhm: 3.0, eccentricity: 0.6, star_count: 80, snr: 40.0, background: 100.0, cloud_score: 0.2 },
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
