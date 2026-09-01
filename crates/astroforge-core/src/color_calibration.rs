use crate::image::F32Image;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorCalibrationResult {
    pub red_gain: f32,
    pub green_gain: f32,
    pub blue_gain: f32,
    pub neutral_r: f32,
    pub neutral_g: f32,
    pub neutral_b: f32,
}

pub fn calibrate_color(image: &F32Image) -> ColorCalibrationResult {
    if image.channels() < 3 {
        return ColorCalibrationResult {
            red_gain: 1.0,
            green_gain: 1.0,
            blue_gain: 1.0,
            neutral_r: 0.0,
            neutral_g: 0.0,
            neutral_b: 0.0,
        };
    }

    let r_mean = channel_mean(image, 0);
    let g_mean = channel_mean(image, 1);
    let b_mean = channel_mean(image, 2);

    let max_mean = r_mean.max(g_mean).max(b_mean).max(1e-10);

    let red_gain = max_mean / r_mean.max(1e-10);
    let green_gain = max_mean / g_mean.max(1e-10);
    let blue_gain = max_mean / b_mean.max(1e-10);

    ColorCalibrationResult {
        red_gain,
        green_gain,
        blue_gain,
        neutral_r: r_mean,
        neutral_g: g_mean,
        neutral_b: b_mean,
    }
}

pub fn apply_color_calibration(image: &F32Image, calibration: &ColorCalibrationResult) -> F32Image {
    let mut result = image.clone();
    let gains = [
        calibration.red_gain,
        calibration.green_gain,
        calibration.blue_gain,
    ];

    for c in 0..result.channels().min(3) {
        let gain = gains[c];
        for val in result.slice_mut(ndarray::s![c..c + 1, .., ..]).iter_mut() {
            *val *= gain;
        }
    }

    result
}

pub fn calibrate_from_neutral_region(
    image: &F32Image,
    region: (usize, usize, usize, usize),
) -> ColorCalibrationResult {
    if image.channels() < 3 {
        return ColorCalibrationResult {
            red_gain: 1.0,
            green_gain: 1.0,
            blue_gain: 1.0,
            neutral_r: 0.0,
            neutral_g: 0.0,
            neutral_b: 0.0,
        };
    }

    let (x0, y0, x1, y1) = region;
    let mut r_sum = 0.0f32;
    let mut g_sum = 0.0f32;
    let mut b_sum = 0.0f32;
    let mut count = 0;

    for y in y0..y1.min(image.height()) {
        for x in x0..x1.min(image.width()) {
            r_sum += image[(0, y, x)];
            g_sum += image[(1, y, x)];
            b_sum += image[(2, y, x)];
            count += 1;
        }
    }

    if count == 0 {
        return ColorCalibrationResult {
            red_gain: 1.0,
            green_gain: 1.0,
            blue_gain: 1.0,
            neutral_r: 0.0,
            neutral_g: 0.0,
            neutral_b: 0.0,
        };
    }

    let r_mean = r_sum / count as f32;
    let g_mean = g_sum / count as f32;
    let b_mean = b_sum / count as f32;
    let max_mean = r_mean.max(g_mean).max(b_mean).max(1e-10);

    ColorCalibrationResult {
        red_gain: max_mean / r_mean.max(1e-10),
        green_gain: max_mean / g_mean.max(1e-10),
        blue_gain: max_mean / b_mean.max(1e-10),
        neutral_r: r_mean,
        neutral_g: g_mean,
        neutral_b: b_mean,
    }
}

fn channel_mean(image: &F32Image, channel: usize) -> f32 {
    let slice = image.slice(ndarray::s![channel..channel + 1, .., ..]);
    let sum: f32 = slice.iter().sum();
    let count = slice.len();
    if count > 0 {
        sum / count as f32
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibrate_color_balanced() {
        let mut img = F32Image::new(4, 4, 3);
        img.fill(100.0);
        let result = calibrate_color(&img);
        assert!((result.red_gain - 1.0).abs() < 0.01);
        assert!((result.green_gain - 1.0).abs() < 0.01);
        assert!((result.blue_gain - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calibrate_color_unbalanced() {
        let mut img = F32Image::new(4, 4, 3);
        for y in 0..4 {
            for x in 0..4 {
                img[(0, y, x)] = 50.0;
                img[(1, y, x)] = 100.0;
                img[(2, y, x)] = 200.0;
            }
        }
        let result = calibrate_color(&img);
        assert!(result.red_gain > 1.0);
        assert!((result.green_gain - 2.0).abs() < 0.01);
        assert!(result.blue_gain < 1.0);
    }

    #[test]
    fn test_apply_color_calibration() {
        let mut img = F32Image::new(4, 4, 3);
        img.fill(100.0);
        let cal = ColorCalibrationResult {
            red_gain: 2.0,
            green_gain: 1.0,
            blue_gain: 0.5,
            neutral_r: 100.0,
            neutral_g: 100.0,
            neutral_b: 100.0,
        };
        let result = apply_color_calibration(&img, &cal);
        assert!((result[(0, 0, 0)] - 200.0).abs() < 0.01);
        assert!((result[(1, 0, 0)] - 100.0).abs() < 0.01);
        assert!((result[(2, 0, 0)] - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_calibrate_from_neutral_region() {
        let mut img = F32Image::new(8, 8, 3);
        for y in 0..8 {
            for x in 0..8 {
                img[(0, y, x)] = 50.0;
                img[(1, y, x)] = 100.0;
                img[(2, y, x)] = 150.0;
            }
        }
        let result = calibrate_from_neutral_region(&img, (0, 0, 4, 4));
        assert!(result.red_gain > 1.0);
        assert!(result.blue_gain < 1.0);
    }
}
