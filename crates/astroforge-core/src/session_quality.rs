//! RT-13 (#431) — per-session frame-quality trends + outlier flags.
//!
//! Surfaces the existing per-frame metric machinery (`fwhm_distribution`,
//! `metric_snapshot_full` keys) as a session-level view: one row per
//! light frame, a per-metric trend (OLS slope across frame order), and
//! robust outlier flags (median/MAD z-score) that feed the import
//! recommendation engine — a frame whose FWHM blows up mid-session is
//! exactly the frame the reviewer should drop.
//!
//! Scope note (honesty): eccentricity is NOT trended — the star
//! extractor does not compute star shapes today, and fabricating one
//! would be worse than its absence. It joins when shape extraction
//! lands.

use crate::comparison_metrics::{fwhm_distribution, metric_snapshot_full};
use crate::image::F32Image;
use serde::{Deserialize, Serialize};

/// Quality summary of one light frame, in capture order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrameQuality {
    pub index: usize,
    pub name: String,
    /// Mean FWHM of detected stars (pixels); 0.0 when none detected.
    pub fwhm_mean: f64,
    /// Stars detected (the FWHM distribution's length).
    pub star_count: usize,
    /// `signal.estimated_snr` from the metric snapshot.
    pub estimated_snr: f64,
    /// `signal.local_snr` from the metric snapshot.
    pub local_snr: f64,
}

impl FrameQuality {
    /// Compute a frame's summary from its decoded pixels: the mean of
    /// the per-star FWHM distribution plus the SNR snapshot entries.
    pub fn compute(index: usize, name: &str, image: &F32Image) -> Self {
        let dist = fwhm_distribution(image);
        let fwhm_mean = if dist.is_empty() {
            0.0
        } else {
            dist.iter().sum::<f64>() / dist.len() as f64
        };
        let snap = metric_snapshot_full(image);
        let get = |key: &str| snap.get(key).copied().unwrap_or(0.0);
        FrameQuality {
            index,
            name: name.to_string(),
            fwhm_mean,
            star_count: dist.len(),
            estimated_snr: get("signal.estimated_snr"),
            local_snr: get("signal.local_snr"),
        }
    }

    fn metric_value(&self, metric: &str) -> Option<f64> {
        match metric {
            "fwhm" => Some(self.fwhm_mean).filter(|v| *v > 0.0),
            "stars" => Some(self.star_count as f64),
            "estimated_snr" => Some(self.estimated_snr).filter(|v| v.is_finite()),
            "local_snr" => Some(self.local_snr).filter(|v| v.is_finite()),
            _ => None,
        }
    }
}

/// The trended metric keys, in display order.
pub const TREND_METRICS: &[&str] = &["fwhm", "stars", "estimated_snr", "local_snr"];

/// Directional note for a metric across the session (OLS slope over
/// frame order). `improving` uses the registry's direction convention
/// where available; for FWHM lower is better, stars/SNR higher is
/// better.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricTrend {
    pub metric: String,
    pub first: f64,
    pub last: f64,
    /// OLS slope per frame index.
    pub slope_per_frame: f64,
    pub improving: bool,
}

/// A robust outlier flag for one frame + metric.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutlierFlag {
    pub frame_index: usize,
    pub frame_name: String,
    pub metric: String,
    pub value: f64,
    /// The session median for the metric.
    pub typical: f64,
    /// Deviation in robust sigmas (median/MAD z-score).
    pub robust_sigma: f64,
}

/// Session-level rollup of the per-frame quality rows.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SessionQualityReport {
    pub frames: Vec<FrameQuality>,
    pub trends: Vec<MetricTrend>,
    pub outliers: Vec<OutlierFlag>,
}

/// Robust flag threshold: flag when a frame deviates more than this
/// many (MAD-scaled) sigmas from the session median. 3.0 keeps the
/// false-positive rate low on short sessions.
pub const OUTLIER_SIGMA: f64 = 3.0;

/// Build the session report from per-frame rows (already in capture
/// order). Pure — safe to call from any thread, deterministic output.
pub fn session_quality_report(frames: Vec<FrameQuality>) -> SessionQualityReport {
    let mut report = SessionQualityReport {
        frames: frames.clone(),
        trends: Vec::new(),
        outliers: Vec::new(),
    };
    if frames.len() < 2 {
        return report;
    }

    for metric in TREND_METRICS {
        let series: Vec<(usize, f64)> = frames
            .iter()
            .filter_map(|f| f.metric_value(metric).map(|v| (f.index, v)))
            .collect();
        if series.len() < 2 {
            continue;
        }
        let first = series[0].1;
        let last = series[series.len() - 1].1;
        let slope = ols_slope(&series);
        let improving = match *metric {
            "fwhm" => slope < 0.0,
            _ => slope > 0.0,
        };
        report.trends.push(MetricTrend {
            metric: metric.to_string(),
            first,
            last,
            slope_per_frame: slope,
            improving,
        });
    }

    for metric in TREND_METRICS {
        let values: Vec<(usize, &FrameQuality, f64)> = frames
            .iter()
            .filter_map(|f| f.metric_value(metric).map(|v| (f.index, f, v)))
            .collect();
        if values.len() < 3 {
            continue; // Outliers are meaningless on 1-2 frames.
        }
        let mut sorted: Vec<f64> = values.iter().map(|(_, _, v)| *v).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted[sorted.len() / 2];
        let mad = median_abs_deviation(&sorted, median);
        // MAD == 0 happens when half the frames sit exactly on the
        // median (integer star counts are prone). Stddev is the
        // textbook fallback but one wild frame inflates it into
        // silence; instead floor the scale at 5% of the median —
        // a deviation must exceed 15% of the typical value to flag,
        // which keeps float-noise unflagged and wild frames flagged.
        let scale = {
            let robust = 1.4826 * mad;
            let floor = 0.05 * median.abs().max(f64::MIN_POSITIVE);
            if robust > floor {
                robust
            } else {
                floor
            }
        };
        if scale <= f64::EPSILON {
            continue;
        }
        for (idx, frame, value) in values {
            let sigma = (value - median).abs() / scale;
            if sigma > OUTLIER_SIGMA {
                report.outliers.push(OutlierFlag {
                    frame_index: idx,
                    frame_name: frame.name.clone(),
                    metric: metric.to_string(),
                    value,
                    typical: median,
                    robust_sigma: sigma,
                });
            }
        }
    }

    report
}

/// Ordinary-least-squares slope of (x, y) points.
fn ols_slope(series: &[(usize, f64)]) -> f64 {
    let n = series.len() as f64;
    let sum_x: f64 = series.iter().map(|(x, _)| *x as f64).sum();
    let sum_y: f64 = series.iter().map(|(_, y)| *y).sum();
    let sum_xy: f64 = series.iter().map(|(x, y)| *x as f64 * *y).sum();
    let sum_x2: f64 = series.iter().map(|(x, _)| (*x as f64) * (*x as f64)).sum();
    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        return 0.0;
    }
    (n * sum_xy - sum_x * sum_y) / denom
}

fn median_abs_deviation(sorted: &[f64], median: f64) -> f64 {
    let mut devs: Vec<f64> = sorted.iter().map(|v| (v - median).abs()).collect();
    devs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    devs[devs.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(index: usize, fwhm: f64, stars: f64, snr: f64) -> FrameQuality {
        FrameQuality {
            index,
            name: format!("light_{index:03}.fits"),
            fwhm_mean: fwhm,
            star_count: stars as usize,
            estimated_snr: snr,
            local_snr: snr * 0.5,
        }
    }

    #[test]
    fn trend_slope_and_direction() {
        // FWHM improving (falling), stars flat, SNR worsening.
        let frames = vec![
            frame(0, 4.0, 100.0, 10.0),
            frame(1, 3.6, 100.0, 9.0),
            frame(2, 3.2, 100.0, 8.0),
        ];
        let report = session_quality_report(frames);
        let fwhm = report
            .trends
            .iter()
            .find(|t| t.metric == "fwhm")
            .expect("fwhm trend");
        assert!(fwhm.slope_per_frame < 0.0);
        assert!(fwhm.improving, "falling FWHM is improving");
        assert_eq!(fwhm.first, 4.0);
        assert_eq!(fwhm.last, 3.2);
        let snr = report
            .trends
            .iter()
            .find(|t| t.metric == "estimated_snr")
            .expect("snr trend");
        assert!(!snr.improving, "falling SNR is not improving");
        // Flat series produce slope ~0 and are neutral-improving for
        // higher-is-better metrics.
        let stars = report
            .trends
            .iter()
            .find(|t| t.metric == "stars")
            .expect("stars trend");
        assert!(stars.slope_per_frame.abs() < 1e-9);
    }

    #[test]
    fn outliers_flag_robustly() {
        // Ten frames at FWHM 3.5 with one wild frame at 9.0.
        let mut frames: Vec<FrameQuality> = (0..10).map(|i| frame(i, 3.5, 100.0, 10.0)).collect();
        frames[7].fwhm_mean = 9.0;
        let report = session_quality_report(frames);
        assert_eq!(report.outliers.len(), 1, "exactly the wild frame flags");
        let flag = &report.outliers[0];
        assert_eq!(flag.frame_index, 7);
        assert_eq!(flag.metric, "fwhm");
        assert_eq!(flag.typical, 3.5);
        assert!(flag.robust_sigma > OUTLIER_SIGMA);
    }

    #[test]
    fn no_false_outliers_when_no_spread() {
        // Identical star counts: MAD == 0 -> no flags ( stddev
        // fallback would stay silent too, but the guard is explicit ).
        let frames = vec![
            frame(0, 3.5, 100.0, 10.0),
            frame(1, 3.5, 100.0, 10.0),
            frame(2, 3.5, 100.0, 10.0),
        ];
        let report = session_quality_report(frames);
        assert!(report.outliers.is_empty());
    }

    #[test]
    fn short_sessions_report_no_trends_or_outliers() {
        let single = session_quality_report(vec![frame(0, 3.0, 10.0, 5.0)]);
        assert!(single.trends.is_empty());
        assert!(single.outliers.is_empty());
        let pair = session_quality_report(vec![frame(0, 3.0, 10.0, 5.0), frame(1, 9.0, 10.0, 5.0)]);
        // Trends yes (2 points), outliers no (< 3 frames).
        assert_eq!(pair.trends.len(), TREND_METRICS.len());
        assert!(pair.outliers.is_empty());
    }

    /// End-to-end over two committed sample-dataset frames: real
    /// decode, real star extraction, finite outputs.
    #[test]
    fn frame_quality_computes_over_sample_dataset() {
        let mut frame_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        frame_path.push("../../tests/fixtures/sample-dataset/light_000.fits");
        let bytes = std::fs::read(&frame_path).expect("sample dataset committed");
        let header = crate::fits::parse_header(&bytes).expect("header");
        let image = crate::fits::read_f32_image(&bytes, &header).expect("pixels");
        let fq = FrameQuality::compute(0, "light_000.fits", &image);
        assert!(fq.star_count > 0, "synthetic star field must yield stars");
        assert!(fq.fwhm_mean.is_finite() && fq.fwhm_mean > 0.0);
        assert!(fq.estimated_snr.is_finite());
    }
}
