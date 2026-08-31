use crate::image::F32Image;
use crate::dip::{DipConfig, dip_denoise};

pub fn planetary_stretch(image: &F32Image, aggression: f32) -> F32Image {
    let mut result = image.clone();

    let min = result.iter().copied().fold(f32::INFINITY, f32::min);
    let max = result.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(1e-10);

    for val in result.iter_mut() {
        let normalized = (*val - min) / range;
        let stretched = (normalized.powf(1.0 / (1.0 + aggression))).powf(aggression);
        *val = stretched.clamp(0.0, 1.0) * range + min;
    }

    result
}

pub fn planetary_sharpen(image: &F32Image, radius: u32, amount: f32) -> F32Image {
    crate::detail_enhancement::local_contrast_enhancement(image, radius, amount)
}

pub fn wavelet_sharpen(image: &F32Image, scales: &[u32], gain: f32) -> F32Image {
    let mut result = image.clone();

    for &scale in scales {
        let blurred = box_blur(image, scale as usize);
        for c in 0..result.channels() {
            for y in 0..result.height() {
                for x in 0..result.width() {
                    let detail = image[(c, y, x)] - blurred[(c, y, x)];
                    result[(c, y, x)] += detail * gain;
                    if result[(c, y, x)] < 0.0 {
                        result[(c, y, x)] = 0.0;
                    }
                }
            }
        }
    }

    result
}

fn box_blur(image: &F32Image, radius: usize) -> F32Image {
    if radius == 0 {
        return image.clone();
    }
    crate::detail_enhancement::local_contrast_enhancement(image, radius as u32, 0.0)
}

pub fn lunar_hdr_merge(exposure_sets: &[Vec<F32Image>]) -> F32Image {
    if exposure_sets.is_empty() {
        return F32Image::new(1, 1, 1);
    }

    let first = &exposure_sets[0];
    if first.is_empty() {
        return F32Image::new(1, 1, 1);
    }

    let width = first[0].width();
    let height = first[0].height();
    let channels = first[0].channels();
    let mut result = F32Image::new(width, height, channels);

    let mut total_weight = 0.0f32;

    for exposure_set in exposure_sets {
        if exposure_set.is_empty() {
            continue;
        }

        let avg_brightness: f32 = exposure_set
            .iter()
            .map(|f| f.iter().sum::<f32>() / f.len() as f32)
            .sum::<f32>()
            / exposure_set.len() as f32;

        let weight = 1.0 / (avg_brightness.max(1e-10));

        for frame in exposure_set {
            for c in 0..channels {
                for y in 0..height {
                    for x in 0..width {
                        result[(c, y, x)] += frame[(c, y, x)] * weight;
                    }
                }
            }
        }
        total_weight += weight * exposure_set.len() as f32;
    }

    if total_weight > 0.0 {
        for val in result.iter_mut() {
            *val /= total_weight;
        }
    }

    result
}

pub fn dip_coadd(frames: &[F32Image], config: &DipConfig) -> F32Image {
    if frames.is_empty() {
        return F32Image::new(1, 1, 1);
    }

    let width = frames[0].width();
    let height = frames[0].height();
    let channels = frames[0].channels();

    let mut stacked = F32Image::new(width, height, channels);
    let count = frames.len() as f32;

    for frame in frames {
        for c in 0..channels {
            for y in 0..height {
                for x in 0..width {
                    stacked[(c, y, x)] += frame[(c, y, x)] / count;
                }
            }
        }
    }

    dip_denoise(&stacked, config, 0.7)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_uniform(w: usize, h: usize, val: f32) -> F32Image {
        let mut img = F32Image::new(w, h, 1);
        img.fill(val);
        img
    }

    #[test]
    fn test_planetary_stretch() {
        let mut img = F32Image::new(4, 4, 1);
        for i in 0..16 {
            img[(0, i / 4, i % 4)] = i as f32 * 10.0;
        }
        let result = planetary_stretch(&img, 0.5);
        let max = result.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let min = result.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(max >= min);
    }

    #[test]
    fn test_planetary_sharpen() {
        let img = make_uniform(8, 8, 100.0);
        let result = planetary_sharpen(&img, 2, 1.5);
        assert_eq!(result.width(), 8);
    }

    #[test]
    fn test_wavelet_sharpen() {
        let img = make_uniform(8, 8, 100.0);
        let result = wavelet_sharpen(&img, &[1, 2, 4], 0.5);
        assert_eq!(result.width(), 8);
    }

    #[test]
    fn test_lunar_hdr_merge() {
        let short = make_uniform(4, 4, 50.0);
        let long = make_uniform(4, 4, 200.0);
        let result = lunar_hdr_merge(&[vec![short], vec![long]]);
        assert_eq!(result.width(), 4);
        let mean = result.iter().sum::<f32>() / result.len() as f32;
        assert!(mean > 0.0);
    }

    #[test]
    fn test_dip_coadd() {
        let frames = vec![
            make_uniform(4, 4, 100.0),
            make_uniform(4, 4, 200.0),
        ];
        let config = DipConfig {
            max_iterations: 10,
            ..Default::default()
        };
        let result = dip_coadd(&frames, &config);
        assert_eq!(result.width(), 4);
    }
}
