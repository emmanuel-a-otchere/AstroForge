use crate::image::F32Image;

pub fn correct_cosmetics(image: &F32Image, sigma_threshold: f64) -> F32Image {
    let mut result = image.clone();
    let _c = 0;
    let height = result.height();
    let width = result.width();

    let mean = result.iter().sum::<f32>() / result.len() as f32;
    let var = result.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / result.len() as f32;
    let std = var.sqrt();
    let hot_threshold = mean + sigma_threshold as f32 * std;
    let cold_threshold = mean - sigma_threshold as f32 * std;

    for ch in 0..result.channels() {
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let val = result[(ch, y, x)];
                if val > hot_threshold || val < cold_threshold {
                    let mut sum = 0.0f32;
                    let mut count = 0;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let nx = (x as i32 + dx) as usize;
                            let ny = (y as i32 + dy) as usize;
                            sum += result[(ch, ny, nx)];
                            count += 1;
                        }
                    }
                    result[(ch, y, x)] = sum / count as f32;
                }
            }
        }
    }

    result
}

pub fn detect_hot_pixels(image: &F32Image, sigma_threshold: f64) -> Vec<(usize, usize)> {
    let mut hot = Vec::new();
    let mean = image.iter().sum::<f32>() / image.len() as f32;
    let var = image.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / image.len() as f32;
    let std = var.sqrt();
    let threshold = mean + sigma_threshold as f32 * std;

    for c in 0..image.channels().min(1) {
        for y in 0..image.height() {
            for x in 0..image.width() {
                if image[(c, y, x)] > threshold {
                    hot.push((x, y));
                }
            }
        }
    }
    hot
}

pub fn detect_cold_pixels(image: &F32Image, sigma_threshold: f64) -> Vec<(usize, usize)> {
    let mut cold = Vec::new();
    let mean = image.iter().sum::<f32>() / image.len() as f32;
    let var = image.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / image.len() as f32;
    let std = var.sqrt();
    let threshold = mean - sigma_threshold as f32 * std;

    for c in 0..image.channels().min(1) {
        for y in 0..image.height() {
            for x in 0..image.width() {
                if image[(c, y, x)] < threshold {
                    cold.push((x, y));
                }
            }
        }
    }
    cold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correct_cosmetics_removes_hot_pixel() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(100.0);
        img[(0, 4, 4)] = 5000.0;
        let result = correct_cosmetics(&img, 3.0);
        assert!((result[(0, 4, 4)] - 100.0).abs() < 50.0);
    }

    #[test]
    fn test_correct_cosmetics_removes_cold_pixel() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(100.0);
        img[(0, 4, 4)] = 0.0;
        let result = correct_cosmetics(&img, 3.0);
        assert!((result[(0, 4, 4)] - 100.0).abs() < 50.0);
    }

    #[test]
    fn test_detect_hot_pixels() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(100.0);
        img[(0, 2, 3)] = 5000.0;
        let hot = detect_hot_pixels(&img, 3.0);
        assert!(hot.contains(&(3, 2)));
    }

    #[test]
    fn test_detect_cold_pixels() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(100.0);
        img[(0, 2, 3)] = 0.0;
        let cold = detect_cold_pixels(&img, 3.0);
        assert!(cold.contains(&(3, 2)));
    }
}
