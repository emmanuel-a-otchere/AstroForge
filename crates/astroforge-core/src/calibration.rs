use crate::image::F32Image;
use ndarray::s;

pub fn build_master_dark(
    frames: &[F32Image],
    exptime: f64,
    ccd_temp: Option<f64>,
) -> Result<F32Image, CalibrationError> {
    if frames.is_empty() {
        return Err(CalibrationError::NoFrames);
    }
    let sigma = 3.0;
    let max_iters = 5;
    sigma_clipped_median_combine(frames, sigma, max_iters)
}

pub fn build_master_flat(frames: &[F32Image]) -> Result<F32Image, CalibrationError> {
    if frames.is_empty() {
        return Err(CalibrationError::NoFrames);
    }
    let mut combined = sigma_clipped_median_combine(frames, 3.0, 5)?;
    normalize_flat(&mut combined);
    Ok(combined)
}

pub fn build_master_bias(frames: &[F32Image]) -> Result<F32Image, CalibrationError> {
    if frames.is_empty() {
        return Err(CalibrationError::NoFrames);
    }
    sigma_clipped_median_combine(frames, 3.0, 5)
}

pub fn apply_calibration(
    light: &F32Image,
    master_dark: Option<&F32Image>,
    master_flat: Option<&F32Image>,
    master_bias: Option<&F32Image>,
) -> F32Image {
    let mut result = light.clone();

    if let Some(dark) = master_dark {
        if let Some(bias) = master_bias {
            let dark_minus_bias = dark - bias;
            result = &result - &dark_minus_bias;
        } else {
            result = &result - dark;
        }
    } else if let Some(bias) = master_bias {
        result = &result - bias;
    }

    if let Some(flat) = master_flat {
        let median = flat.mean().unwrap_or(1.0).max(1e-10);
        for c in 0..result.channels() {
            for y in 0..result.height() {
                for x in 0..result.width() {
                    let f = flat[(c.min(flat.channels() - 1), y.min(flat.height() - 1), x.min(flat.width() - 1))];
                    if f.abs() > 1e-10 {
                        result[(c, y, x)] /= f / median as f32;
                    }
                }
            }
        }
    }

    result
}

pub fn apply_calibration_lights_only(
    light: &F32Image,
    master_flat: Option<&F32Image>,
) -> F32Image {
    apply_calibration(light, None, master_flat, None)
}

pub struct StreamingCalibrator {
    master_dark: Option<F32Image>,
    master_flat: Option<F32Image>,
    master_bias: Option<F32Image>,
}

impl StreamingCalibrator {
    pub fn new(
        master_dark: Option<F32Image>,
        master_flat: Option<F32Image>,
        master_bias: Option<F32Image>,
    ) -> Self {
        Self {
            master_dark,
            master_flat,
            master_bias,
        }
    }

    pub fn calibrate_frame(&self, frame: &F32Image) -> F32Image {
        apply_calibration(
            frame,
            self.master_dark.as_ref(),
            self.master_flat.as_ref(),
            self.master_bias.as_ref(),
        )
    }
}

fn sigma_clipped_median_combine(
    frames: &[F32Image],
    sigma: f64,
    max_iters: u32,
) -> Result<F32Image, CalibrationError> {
    let n = frames.len();
    let channels = frames[0].channels();
    let height = frames[0].height();
    let width = frames[0].width();

    let mut result = F32Image::new(width, height, channels);

    for c in 0..channels {
        for y in 0..height {
            for x in 0..width {
                let mut values: Vec<f32> = frames
                    .iter()
                    .map(|f| f[(c, y, x)])
                    .collect();

                for _ in 0..max_iters {
                    let mean = values.iter().sum::<f32>() / values.len() as f32;
                    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32;
                    let std = var.sqrt();
                    let threshold = sigma as f32 * std;

                    let before = values.len();
                    values.retain(|v| (v - mean).abs() <= threshold);
                    if values.len() == before || values.len() < 2 {
                        break;
                    }
                }

                values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mid = values.len() / 2;
                let median = if values.len() % 2 == 0 {
                    (values[mid - 1] + values[mid]) / 2.0
                } else {
                    values[mid]
                };
                result[(c, y, x)] = median;
            }
        }
    }

    Ok(result)
}

fn normalize_flat(flat: &mut F32Image) {
    let median = flat.mean().unwrap_or(1.0).max(1e-10);
    for val in flat.iter_mut() {
        *val = *val / median as f32;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CalibrationError {
    #[error("No frames provided")]
    NoFrames,
    #[error("Frame dimension mismatch")]
    DimensionMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(width: usize, height: usize, value: f32) -> F32Image {
        let mut img = F32Image::new(width, height, 1);
        img.fill(value);
        img
    }

    #[test]
    fn test_master_bias_build() {
        let frames = vec![
            make_test_image(4, 4, 100.0),
            make_test_image(4, 4, 100.0),
            make_test_image(4, 4, 100.0),
        ];
        let master = build_master_bias(&frames).unwrap();
        assert_eq!(master.width(), 4);
        assert_eq!(master.height(), 4);
        assert!((master[(0, 0, 0)] - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_master_dark_build() {
        let frames = vec![
            make_test_image(4, 4, 500.0),
            make_test_image(4, 4, 500.0),
        ];
        let master = build_master_dark(&frames, 300.0, Some(-10.0)).unwrap();
        assert!((master[(0, 0, 0)] - 500.0).abs() < 0.01);
    }

    #[test]
    fn test_master_flat_normalize() {
        let frames = vec![
            make_test_image(4, 4, 2000.0),
            make_test_image(4, 4, 2000.0),
        ];
        let master = build_master_flat(&frames).unwrap();
        let mean = master.mean().unwrap_or(0.0);
        assert!((mean - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calibration_application() {
        let light = make_test_image(4, 4, 1000.0);
        let dark = make_test_image(4, 4, 100.0);
        let flat = {
            let mut f = make_test_image(4, 4, 2000.0);
            normalize_flat(&mut f);
            f
        };
        let result = apply_calibration(&light, Some(&dark), Some(&flat), None);
        let expected = (1000.0 - 100.0) / 1.0;
        assert!((result[(0, 0, 0)] - expected).abs() < 1.0);
    }

    #[test]
    fn test_lights_only_path() {
        let light = make_test_image(4, 4, 1000.0);
        let flat = {
            let mut f = make_test_image(4, 4, 2000.0);
            normalize_flat(&mut f);
            f
        };
        let result = apply_calibration_lights_only(&light, Some(&flat));
        let expected = 1000.0 / 1.0;
        assert!((result[(0, 0, 0)] - expected).abs() < 1.0);
    }

    #[test]
    fn test_lights_only_no_flat() {
        let light = make_test_image(4, 4, 1000.0);
        let result = apply_calibration_lights_only(&light, None);
        assert!((result[(0, 0, 0)] - 1000.0).abs() < 0.01);
    }

    #[test]
    fn test_streaming_calibrator() {
        let dark = make_test_image(4, 4, 100.0);
        let calibrator = StreamingCalibrator::new(Some(dark), None, None);
        let frame = make_test_image(4, 4, 1000.0);
        let result = calibrator.calibrate_frame(&frame);
        assert!((result[(0, 0, 0)] - 900.0).abs() < 0.01);
    }

    #[test]
    fn test_sigma_clip_rejects_outlier() {
        let mut frames = vec![
            make_test_image(2, 2, 100.0),
            make_test_image(2, 2, 100.0),
            make_test_image(2, 2, 100.0),
            make_test_image(2, 2, 100.0),
            make_test_image(2, 2, 10000.0),
        ];
        let master = sigma_clipped_median_combine(&frames, 3.0, 5).unwrap();
        assert!((master[(0, 0, 0)] - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_no_frames_error() {
        let result = build_master_bias(&[]);
        assert!(result.is_err());
    }
}
