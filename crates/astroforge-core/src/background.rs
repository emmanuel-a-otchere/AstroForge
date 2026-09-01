use crate::image::F32Image;

pub fn extract_background(image: &F32Image, sample_points: &[(f64, f64)]) -> F32Image {
    let width = image.width();
    let height = image.height();
    let channels = image.channels();

    let mut result = image.clone();
    let mut background_model = F32Image::new(width, height, channels);

    for ch in 0..channels {
        let bg_level = estimate_background_level(image, ch, sample_points);
        for y in 0..height {
            for x in 0..width {
                background_model[(ch, y, x)] = bg_level;
            }
        }
    }

    for ch in 0..channels {
        for y in 0..height {
            for x in 0..width {
                result[(ch, y, x)] -= background_model[(ch, y, x)];
                if result[(ch, y, x)] < 0.0 {
                    result[(ch, y, x)] = 0.0;
                }
            }
        }
    }

    result
}

fn estimate_background_level(
    image: &F32Image,
    channel: usize,
    sample_points: &[(f64, f64)],
) -> f32 {
    if sample_points.is_empty() {
        let mut values: Vec<f32> = image
            .slice(ndarray::s![channel..channel + 1, .., ..])
            .iter()
            .copied()
            .collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        return values[values.len() / 4];
    }

    let mut samples = Vec::new();
    for (sx, sy) in sample_points {
        let x = (*sx * image.width() as f64) as usize;
        let y = (*sy * image.height() as f64) as usize;
        if x < image.width() && y < image.height() {
            samples.push(image[(channel, y, x)]);
        }
    }

    if samples.is_empty() {
        return 0.0;
    }

    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    samples[samples.len() / 2]
}

pub fn subtract_gradient(image: &F32Image, gradient: &F32Image) -> F32Image {
    let mut result = image.clone();
    for ch in 0..result.channels() {
        for y in 0..result.height() {
            for x in 0..result.width() {
                let g = if ch < gradient.channels() && y < gradient.height() && x < gradient.width()
                {
                    gradient[(ch, y, x)]
                } else {
                    0.0
                };
                result[(ch, y, x)] -= g;
                if result[(ch, y, x)] < 0.0 {
                    result[(ch, y, x)] = 0.0;
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_background_flat() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(100.0);
        let result = extract_background(&img, &[]);
        let mean = result.iter().sum::<f32>() / result.len() as f32;
        assert!(mean.abs() < 50.0);
    }

    #[test]
    fn test_extract_background_with_gradient() {
        let mut img = F32Image::new(16, 16, 1);
        for y in 0..16 {
            for x in 0..16 {
                img[(0, y, x)] = 100.0 + y as f32 * 10.0;
            }
        }
        let result = extract_background(&img, &[]);
        assert!(result[(0, 0, 0)] < img[(0, 0, 0)]);
    }

    #[test]
    fn test_subtract_gradient() {
        let mut img = F32Image::new(4, 4, 1);
        img.fill(200.0);
        let mut grad = F32Image::new(4, 4, 1);
        grad.fill(50.0);
        let result = subtract_gradient(&img, &grad);
        assert!((result[(0, 0, 0)] - 150.0).abs() < 0.01);
    }
}
