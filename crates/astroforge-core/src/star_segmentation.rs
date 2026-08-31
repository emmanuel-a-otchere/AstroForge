use crate::image::F32Image;

pub struct StarSegmentationResult {
    pub star_layer: F32Image,
    pub background_layer: F32Image,
}

pub fn segment_stars(image: &F32Image, threshold_sigma: f64) -> StarSegmentationResult {
    let channels = image.channels();
    let width = image.width();
    let height = image.height();

    let mean = image.iter().sum::<f32>() / image.len() as f32;
    let var = image.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / image.len() as f32;
    let std = var.sqrt();
    let threshold = mean + threshold_sigma as f32 * std;

    let mut star_layer = F32Image::new(width, height, channels);
    let mut background_layer = F32Image::new(width, height, channels);

    for c in 0..channels {
        for y in 0..height {
            for x in 0..width {
                let val = image[(c, y, x)];
                if val > threshold {
                    star_layer[(c, y, x)] = val;
                    background_layer[(c, y, x)] = 0.0;
                } else {
                    star_layer[(c, y, x)] = 0.0;
                    background_layer[(c, y, x)] = val;
                }
            }
        }
    }

    StarSegmentationResult {
        star_layer,
        background_layer,
    }
}

pub fn enhance_star_layer(star_layer: &F32Image, color_boost: f32, size_reduction: f32) -> F32Image {
    let mut result = star_layer.clone();
    for val in result.iter_mut() {
        *val *= color_boost;
    }
    result
}

pub fn enhance_background_layer(background_layer: &F32Image, contrast: f32, saturation: f32) -> F32Image {
    let mut result = background_layer.clone();
    let mean = result.iter().sum::<f32>() / result.len() as f32;
    for val in result.iter_mut() {
        let diff = *val - mean;
        *val = mean + diff * contrast;
        if *val < 0.0 {
            *val = 0.0;
        }
    }
    result
}

pub fn recombine_layers(star_layer: &F32Image, background_layer: &F32Image) -> F32Image {
    let channels = star_layer.channels();
    let width = star_layer.width();
    let height = star_layer.height();
    let mut result = F32Image::new(width, height, channels);

    for c in 0..channels {
        for y in 0..height {
            for x in 0..width {
                result[(c, y, x)] = star_layer[(c, y, x)] + background_layer[(c, y, x)];
            }
        }
    }

    result
}

pub fn remove_satellite_trails(image: &F32Image, trail_mask: &F32Image) -> F32Image {
    let mut result = image.clone();
    for c in 0..result.channels() {
        for y in 0..result.height() {
            for x in 0..result.width() {
                if trail_mask[(c.min(trail_mask.channels() - 1), y.min(trail_mask.height() - 1), x.min(trail_mask.width() - 1)] > 0.5 {
                    let mut sum = 0.0f32;
                    let mut count = 0;
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < result.width() as i32 && ny >= 0 && ny < result.height() as i32 {
                            sum += result[(c, ny as usize, nx as usize)];
                            count += 1;
                        }
                    }
                    if count > 0 {
                        result[(c, y, x)] = sum / count as f32;
                    }
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
    fn test_segment_stars() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(10.0);
        img[(0, 4, 4)] = 1000.0;
        let result = segment_stars(&img, 3.0);
        assert!(result.star_layer[(0, 4, 4)] > 0.0);
        assert!((result.background_layer[(0, 4, 4)] - 0.0).abs() < 0.01);
        assert!((result.star_layer[(0, 0, 0)] - 0.0).abs() < 0.01);
        assert!(result.background_layer[(0, 0, 0)] > 0.0);
    }

    #[test]
    fn test_recombine_layers() {
        let mut star = F32Image::new(4, 4, 1);
        star.fill(10.0);
        let mut bg = F32Image::new(4, 4, 1);
        bg.fill(50.0);
        let result = recombine_layers(&star, &bg);
        assert!((result[(0, 0, 0)] - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_enhance_star_layer() {
        let mut star = F32Image::new(4, 4, 1);
        star.fill(100.0);
        let result = enhance_star_layer(&star, 1.5, 0.0);
        assert!((result[(0, 0, 0)] - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_remove_satellite_trails() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(100.0);
        img[(0, 4, 0)] = 5000.0;
        img[(0, 4, 1)] = 5000.0;
        let mut mask = F32Image::new(8, 8, 1);
        mask[(0, 4, 0)] = 1.0;
        mask[(0, 4, 1)] = 1.0;
        let result = remove_satellite_trails(&img, &mask);
        assert!(result[(0, 4, 0)] < 5000.0);
    }
}
