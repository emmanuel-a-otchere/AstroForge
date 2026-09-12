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

pub fn enhance_star_layer(
    star_layer: &F32Image,
    color_boost: f32,
    _size_reduction: f32,
) -> F32Image {
    let mut result = star_layer.clone();
    for val in result.iter_mut() {
        *val *= color_boost;
    }
    result
}

pub fn enhance_background_layer(
    background_layer: &F32Image,
    contrast: f32,
    _saturation: f32,
) -> F32Image {
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

/// P1.5-M7-T5 — exact inverse of [`segment_stars`]. The forward path
/// partitions the image into `star_layer` + `background_layer` such
/// that every pixel is in exactly one layer (the other is 0).
/// Summing the two layers recovers the original pixel values.
///
/// Panics if the layers have different geometry — the forward path
/// always produces matched-shape layers, so a mismatch signals a
/// caller bug, not a recoverable runtime error.
pub fn replace_stars(star_layer: &F32Image, background_layer: &F32Image) -> F32Image {
    assert_eq!(
        star_layer.width(),
        background_layer.width(),
        "star_layer width ({}) != background_layer width ({})",
        star_layer.width(),
        background_layer.width(),
    );
    assert_eq!(
        star_layer.height(),
        background_layer.height(),
        "star_layer height ({}) != background_layer height ({})",
        star_layer.height(),
        background_layer.height(),
    );
    assert_eq!(
        star_layer.channels(),
        background_layer.channels(),
        "star_layer channels ({}) != background_layer channels ({})",
        star_layer.channels(),
        background_layer.channels(),
    );
    recombine_layers(star_layer, background_layer)
}

/// P1.5-M7-T5 — exact inverse of [`enhance_star_layer`]. The forward
/// path multiplies each pixel by `color_boost`; the inverse divides
/// by the same factor. Caller is responsible for ensuring
/// `color_boost != 0.0` (the forward path produces all-zero output
/// for a 0.0 boost, which is not invertible; the function asserts
/// to surface the contract violation).
pub fn inverse_star_enhancement(enhanced: &F32Image, color_boost: f32) -> F32Image {
    assert!(
        color_boost.abs() > f32::EPSILON,
        "color_boost must be non-zero (got {color_boost})",
    );
    let mut result = enhanced.clone();
    for val in result.iter_mut() {
        *val /= color_boost;
    }
    result
}

/// P1.5-M7-T5 — inverse of [`enhance_background_layer`]. The forward
/// path shifts the mean to itself (`mean + diff * contrast`); the
/// inverse applies `mean + diff / contrast`. As with
/// [`inverse_star_enhancement`], this is exact in float arithmetic
/// modulo the round-trip noise from the mean pre-computation.
pub fn inverse_background_enhancement(enhanced: &F32Image, contrast: f32) -> F32Image {
    assert!(
        contrast.abs() > f32::EPSILON,
        "contrast must be non-zero (got {contrast})",
    );
    let mut result = enhanced.clone();
    let mean = result.iter().sum::<f32>() / result.len() as f32;
    for val in result.iter_mut() {
        let diff = *val - mean;
        *val = mean + diff / contrast;
        if *val < 0.0 {
            *val = 0.0;
        }
    }
    result
}

pub fn remove_satellite_trails(image: &F32Image, trail_mask: &F32Image) -> F32Image {
    let mut result = image.clone();
    for c in 0..result.channels() {
        for y in 0..result.height() {
            for x in 0..result.width() {
                if trail_mask[(
                    c.min(trail_mask.channels() - 1),
                    y.min(trail_mask.height() - 1),
                    x.min(trail_mask.width() - 1),
                )] > 0.5
                {
                    let mut sum = 0.0f32;
                    let mut count = 0;
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0
                            && nx < result.width() as i32
                            && ny >= 0
                            && ny < result.height() as i32
                        {
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

    // ---- P1.5-M7-T5 reversibility tests ----

    #[test]
    fn replace_stars_round_trips_segment_stars() {
        let mut img = F32Image::new(8, 8, 1);
        for i in 0..64 {
            img[(0, i / 8, i % 8)] = (i as f32) * 0.7 + 0.1;
        }
        img[(0, 4, 4)] = 100.0;
        let split = segment_stars(&img, 2.0);
        let recovered = replace_stars(&split.star_layer, &split.background_layer);
        // Every pixel must match the original within float tolerance.
        for y in 0..8 {
            for x in 0..8 {
                assert!(
                    (recovered[(0, y, x)] - img[(0, y, x)]).abs() < 1e-4,
                    "pixel ({y}, {x}) drift: original={} recovered={}",
                    img[(0, y, x)],
                    recovered[(0, y, x)],
                );
            }
        }
    }

    #[test]
    fn inverse_star_enhancement_round_trips() {
        let mut star = F32Image::new(4, 4, 1);
        for i in 0..16 {
            star[(0, i / 4, i % 4)] = (i as f32) * 0.3;
        }
        let enhanced = enhance_star_layer(&star, 2.5, 0.0);
        let recovered = inverse_star_enhancement(&enhanced, 2.5);
        for y in 0..4 {
            for x in 0..4 {
                assert!(
                    (recovered[(0, y, x)] - star[(0, y, x)]).abs() < 1e-4,
                    "pixel ({y}, {x}) drift: original={} recovered={}",
                    star[(0, y, x)],
                    recovered[(0, y, x)],
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "color_boost must be non-zero")]
    fn inverse_star_enhancement_panics_on_zero_boost() {
        let star = F32Image::new(4, 4, 1);
        let _ = inverse_star_enhancement(&star, 0.0);
    }

    #[test]
    fn inverse_background_enhancement_round_trips() {
        let mut bg = F32Image::new(4, 4, 1);
        for i in 0..16 {
            bg[(0, i / 4, i % 4)] = (i as f32) * 0.4 + 10.0;
        }
        let enhanced = enhance_background_layer(&bg, 1.7, 0.0);
        let recovered = inverse_background_enhancement(&enhanced, 1.7);
        for y in 0..4 {
            for x in 0..4 {
                // The forward path clamps negative results to 0.0;
                // any value that survives the round-trip is
                // expected to be exact.
                assert!(
                    (recovered[(0, y, x)] - bg[(0, y, x)]).abs() < 1e-4,
                    "pixel ({y}, {x}) drift: original={} recovered={}",
                    bg[(0, y, x)],
                    recovered[(0, y, x)],
                );
            }
        }
    }
}
