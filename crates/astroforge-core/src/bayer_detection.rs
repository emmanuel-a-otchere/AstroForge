use crate::image::F32Image;
use crate::debayer::BayerPattern;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayerDetectionResult {
    pub is_bayer: bool,
    pub confidence: f64,
    pub pattern: Option<BayerPattern>,
    pub method: DetectionMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectionMethod {
    Metadata,
    Statistical,
    CameraSignature,
    AssumeRGB,
}

pub fn detect_bayer(image: &F32Image) -> BayerDetectionResult {
    if image.channels() >= 3 {
        return BayerDetectionResult {
            is_bayer: false,
            confidence: 1.0,
            pattern: None,
            method: DetectionMethod::AssumeRGB,
        };
    }

    let autocorr = autocorrelation_2x2(image);
    let green_var = green_variance_test(image);
    let is_grayscale = image.channels() == 1;

    if !is_grayscale {
        return BayerDetectionResult {
            is_bayer: false,
            confidence: 1.0,
            pattern: None,
            method: DetectionMethod::AssumeRGB,
        };
    }

    let mut confidence = 0.0;
    confidence += autocorr * 0.5;
    confidence += green_var * 0.5;
    confidence = confidence.min(1.0);

    let is_bayer = confidence > 0.5;
    let pattern = if is_bayer {
        Some(detect_pattern(image))
    } else {
        None
    };

    let method = if confidence > 0.85 {
        DetectionMethod::Statistical
    } else if confidence > 0.5 {
        DetectionMethod::Statistical
    } else {
        DetectionMethod::AssumeRGB
    };

    BayerDetectionResult {
        is_bayer,
        confidence,
        pattern,
        method,
    }
}

fn autocorrelation_2x2(image: &F32Image) -> f64 {
    let c = 0;
    let width = image.width();
    let height = image.height();

    if width < 4 || height < 4 {
        return 0.0;
    }

    let mut sum_offset1 = 0.0f64;
    let mut sum_offset2 = 0.0f64;
    let mut count = 0u64;

    for y in 0..height - 2 {
        for x in 0..width - 2 {
            let v = image[(c, y, x)] as f64;
            let v_x1 = image[(c, y, x + 1)] as f64;
            let v_x2 = image[(c, y, x + 2)] as f64;
            let v_y1 = image[(c, y + 1, x)] as f64;
            let v_y2 = image[(c, y + 2, x)] as f64;

            sum_offset1 += (v - v_x1).abs();
            sum_offset1 += (v - v_y1).abs();
            sum_offset2 += (v - v_x2).abs();
            sum_offset2 += (v - v_y2).abs();
            count += 2;
        }
    }

    if count == 0 {
        return 0.0;
    }

    let avg_offset1 = sum_offset1 / count as f64;
    let avg_offset2 = sum_offset2 / count as f64;

    if avg_offset2 > 0.0 {
        let ratio = avg_offset1 / avg_offset2;
        if ratio < 0.5 {
            return 0.9;
        } else if ratio < 0.7 {
            return 0.7;
        }
    }

    0.3
}

fn green_variance_test(image: &F32Image) -> f64 {
    let c = 0;
    let width = image.width();
    let height = image.height();

    if width < 4 || height < 4 {
        return 0.0;
    }

    let mut green_positions = [0.0f64; 4];
    let mut green_counts = [0u64; 4];

    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let pos = ((y % 2) * 2 + (x % 2)) as usize;
            green_positions[pos] += image[(c, y, x)] as f64;
            green_counts[pos] += 1;
        }
    }

    let mut means = [0.0f64; 4];
    for i in 0..4 {
        if green_counts[i] > 0 {
            means[i] = green_positions[i] / green_counts[i] as f64;
        }
    }

    let overall_mean = means.iter().sum::<f64>() / 4.0;
    let variance: f64 = means.iter().map(|m| (m - overall_mean).powi(2)).sum::<f64>() / 4.0;

    let g0_g2_diff = (means[0] - means[2]).abs();
    let g1_g3_diff = (means[1] - means[3]).abs();

    if g0_g2_diff < overall_mean * 0.05 && g1_g3_diff < overall_mean * 0.05 {
        0.85
    } else if variance < overall_mean * 0.01 {
        0.6
    } else {
        0.2
    }
}

fn detect_pattern(image: &F32Image) -> BayerPattern {
    let c = 0;
    let width = image.width();
    let height = image.height();

    let mut corner_means = [0.0f64; 4];
    let mut counts = [0u64; 4];

    let sample_size = 50.min(width / 4).min(height / 4).max(1);

    for y in 0..sample_size {
        for x in 0..sample_size {
            let pos = ((y % 2) * 2 + (x % 2)) as usize;
            corner_means[pos] += image[(c, y, x)] as f64;
            counts[pos] += 1;
        }
    }

    for i in 0..4 {
        if counts[i] > 0 {
            corner_means[i] /= counts[i] as f64;
        }
    }

    let max_idx = corner_means
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);

    match max_idx {
        0 => BayerPattern::RGGB,
        1 => BayerPattern::GRBG,
        2 => BayerPattern::BGGR,
        3 => BayerPattern::GBRG,
        _ => BayerPattern::RGGB,
    }
}

pub fn route_by_confidence(result: &BayerDetectionResult) -> ConfidenceRoute {
    if result.confidence > 0.85 {
        ConfidenceRoute::AutoProceed(result.pattern)
    } else if result.confidence > 0.5 {
        ConfidenceRoute::Prompt
    } else {
        ConfidenceRoute::AssumeRGB
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfidenceRoute {
    AutoProceed(Option<BayerPattern>),
    Prompt,
    AssumeRGB,
}

pub struct CameraSignature {
    pub name: &'static str,
    pub sensor: &'static str,
    pub pattern: BayerPattern,
    pub width: usize,
    pub height: usize,
}

pub fn camera_signatures() -> Vec<CameraSignature> {
    vec![
        CameraSignature {
            name: "Seestar S50",
            sensor: "IMX462",
            pattern: BayerPattern::RGGB,
            width: 1920,
            height: 1080,
        },
        CameraSignature {
            name: "Seestar S30",
            sensor: "IMX585",
            pattern: BayerPattern::RGGB,
            width: 1920,
            height: 1080,
        },
        CameraSignature {
            name: "ZWO Seestar",
            sensor: "IMX462",
            pattern: BayerPattern::RGGB,
            width: 1920,
            height: 1080,
        },
    ]
}

pub fn match_camera_signature(width: usize, height: usize) -> Option<CameraSignature> {
    camera_signatures()
        .into_iter()
        .find(|s| s.width == width && s.height == height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bayer_pattern_image(w: usize, h: usize) -> F32Image {
        let mut img = F32Image::new(w, h, 1);
        for y in 0..h {
            for x in 0..w {
                let pos = (y % 2) * 2 + (x % 2);
                img[(0, y, x)] = match pos {
                    0 => 100.0,
                    1 => 200.0,
                    2 => 200.0,
                    3 => 50.0,
                    _ => 0.0,
                };
            }
        }
        img
    }

    fn make_uniform_image(w: usize, h: usize, val: f32) -> F32Image {
        let mut img = F32Image::new(w, h, 1);
        img.fill(val);
        img
    }

    #[test]
    fn test_detect_bayer_rgb() {
        let img = F32Image::new(16, 16, 3);
        let result = detect_bayer(&img);
        assert!(!result.is_bayer);
        assert_eq!(result.method, DetectionMethod::AssumeRGB);
    }

    #[test]
    fn test_detect_bayer_pattern_image() {
        let img = make_bayer_pattern_image(32, 32);
        let result = detect_bayer(&img);
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_detect_bayer_uniform() {
        let img = make_uniform_image(32, 32, 100.0);
        let result = detect_bayer(&img);
        assert!(result.confidence < 0.9);
    }

    #[test]
    fn test_route_by_confidence_high() {
        let result = BayerDetectionResult {
            is_bayer: true,
            confidence: 0.9,
            pattern: Some(BayerPattern::RGGB),
            method: DetectionMethod::Statistical,
        };
        match route_by_confidence(&result) {
            ConfidenceRoute::AutoProceed(Some(BayerPattern::RGGB)) => {}
            _ => panic!("Expected AutoProceed with RGGB"),
        }
    }

    #[test]
    fn test_route_by_confidence_medium() {
        let result = BayerDetectionResult {
            is_bayer: true,
            confidence: 0.6,
            pattern: None,
            method: DetectionMethod::Statistical,
        };
        assert_eq!(route_by_confidence(&result), ConfidenceRoute::Prompt);
    }

    #[test]
    fn test_route_by_confidence_low() {
        let result = BayerDetectionResult {
            is_bayer: false,
            confidence: 0.3,
            pattern: None,
            method: DetectionMethod::AssumeRGB,
        };
        assert_eq!(route_by_confidence(&result), ConfidenceRoute::AssumeRGB);
    }

    #[test]
    fn test_camera_signatures() {
        let sigs = camera_signatures();
        assert!(!sigs.is_empty());
    }

    #[test]
    fn test_match_camera_signature() {
        let sig = match_camera_signature(1920, 1080);
        assert!(sig.is_some());
        assert_eq!(sig.unwrap().name, "Seestar S50");
    }

    #[test]
    fn test_match_camera_signature_none() {
        let sig = match_camera_signature(100, 100);
        assert!(sig.is_none());
    }
}
