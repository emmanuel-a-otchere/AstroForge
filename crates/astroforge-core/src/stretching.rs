use crate::image::F32Image;

pub fn auto_stretch(image: &F32Image) -> F32Image {
    let mut result = image.clone();

    let min = result.iter().copied().fold(f32::INFINITY, f32::min);
    let max = result.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(1e-10);

    let midtones = compute_midtones(&result);

    for val in result.iter_mut() {
        let normalized = (*val - min) / range;
        *val = arcsinh_stretch(f64::from(normalized), midtones) as f32;
    }

    result
}

pub fn arcsinh_stretch(value: f64, midtones: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    let beta = midtones.max(1e-10);
    let stretched = beta * (value / beta).asinh() / (1.0 / beta).asinh();
    stretched.clamp(0.0, 1.0)
}

fn compute_midtones(image: &F32Image) -> f64 {
    let mut values: Vec<f32> = image.iter().copied().collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median = values[values.len() / 2] as f64;
    let mean = (values.iter().sum::<f32>() / values.len() as f32) as f64;

    if mean > 0.0 && median > 0.0 {
        let log_mean = mean.ln();
        let log_median = median.ln();
        (log_mean - log_median).exp()
    } else {
        0.25
    }
}

pub fn histogram_stretch(
    image: &F32Image,
    shadows: f64,
    highlights: f64,
    midtones: f64,
) -> F32Image {
    let mut result = image.clone();
    let range = (highlights - shadows).max(1e-10);

    for val in result.iter_mut() {
        let normalized = (*val as f64 - shadows) / range;
        let stretched = midtone_transfer(normalized.clamp(0.0, 1.0), midtones);
        *val = stretched as f32;
    }

    result
}

fn midtone_transfer(value: f64, midtones: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    if value >= 1.0 {
        return 1.0;
    }
    let m = midtones.clamp(0.001, 0.999);
    let result = ((m - 1.0) * value) / ((m - 1.0) * value - m * value + m);
    result.clamp(0.0, 1.0)
}

pub fn compute_histogram(image: &F32Image, bins: usize) -> Vec<u32> {
    let mut hist = vec![0u32; bins];
    let min = image.iter().copied().fold(f32::INFINITY, f32::min);
    let max = image.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(1e-10);

    for &val in image.iter() {
        let normalized = ((val - min) / range) as f64;
        let bin = (normalized * bins as f64) as usize;
        let bin = bin.min(bins - 1);
        hist[bin] += 1;
    }

    hist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_stretch() {
        let mut img = F32Image::new(4, 4, 1);
        for i in 0..16 {
            img[(0, i / 4, i % 4)] = i as f32 * 100.0;
        }
        let stretched = auto_stretch(&img);
        let max = stretched
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::min)
            .max(0.0);
        let min = stretched.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(max <= 1.0);
        assert!(min >= 0.0);
    }

    #[test]
    fn test_arcsinh_stretch() {
        assert!((arcsinh_stretch(0.0, 0.25) - 0.0).abs() < 0.01);
        assert!((arcsinh_stretch(1.0, 0.25) - 1.0).abs() < 0.01);
        let mid = arcsinh_stretch(0.5, 0.25);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn test_histogram_stretch() {
        let mut img = F32Image::new(4, 4, 1);
        img.fill(0.5);
        let stretched = histogram_stretch(&img, 0.0, 1.0, 0.25);
        let val = stretched[(0, 0, 0)];
        assert!(val > 0.0 && val < 1.0);
    }

    #[test]
    fn test_compute_histogram() {
        let mut img = F32Image::new(4, 4, 1);
        img.fill(0.5);
        let hist = compute_histogram(&img, 256);
        assert_eq!(hist[128], 16);
    }
}
