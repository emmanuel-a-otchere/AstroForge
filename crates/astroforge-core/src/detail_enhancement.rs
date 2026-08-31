use crate::image::F32Image;

pub fn multi_scale_unsharp_mask(
    image: &F32Image,
    scales: &[u32],
    amount: f32,
) -> F32Image {
    let mut result = image.clone();

    for &scale in scales {
        let blurred = box_blur(image, scale as usize);
        let high_freq = subtract(image, &blurred);
        add_scaled(&mut result, &high_freq, amount);
    }

    result
}

pub fn local_contrast_enhancement(
    image: &F32Image,
    radius: u32,
    amount: f32,
) -> F32Image {
    let blurred = box_blur(image, radius as usize);
    let mut result = image.clone();

    let mean = image.iter().sum::<f32>() / image.len() as f32;

    for c in 0..result.channels() {
        for y in 0..result.height() {
            for x in 0..result.width() {
                let orig = image[(c, y, x)];
                let blur_val = blurred[(c, y, x)];
                let diff = orig - blur_val;
                result[(c, y, x)] = blur_val + diff * amount;
                if result[(c, y, x)] < 0.0 {
                    result[(c, y, x)] = 0.0;
                }
            }
        }
    }

    result
}

pub fn structure_transfer(
    target: &F32Image,
    source: &F32Image,
    blend: f32,
) -> F32Image {
    let mut result = target.clone();
    let source_blur = box_blur(source, 3);
    let target_blur = box_blur(target, 3);

    for c in 0..result.channels().min(source.channels()) {
        for y in 0..result.height() {
            for x in 0..result.width() {
                let high_freq = source[(c, y, x)] - source_blur[(c, y, x)];
                let base = target_blur[(c, y, x)];
                result[(c, y, x)] = base + high_freq * blend;
                if result[(c, y, x)] < 0.0 {
                    result[(c, y, x)] = 0.0;
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

    let channels = image.channels();
    let width = image.width();
    let height = image.height();
    let mut result = F32Image::new(width, height, channels);

    for c in 0..channels {
        for y in 0..height {
            for x in 0..width {
                let mut sum = 0.0f32;
                let mut count = 0;
                for dy in -(radius as i32)..=(radius as i32) {
                    for dx in -(radius as i32)..=(radius as i32) {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                            sum += image[(c, ny as usize, nx as usize)];
                            count += 1;
                        }
                    }
                }
                result[(c, y, x)] = sum / count as f32;
            }
        }
    }

    result
}

fn subtract(a: &F32Image, b: &F32Image) -> F32Image {
    let mut result = a.clone();
    for c in 0..result.channels().min(b.channels()) {
        for y in 0..result.height() {
            for x in 0..result.width() {
                result[(c, y, x)] -= b[(c, y, x)];
            }
        }
    }
    result
}

fn add_scaled(target: &mut F32Image, source: &F32Image, scale: f32) {
    for c in 0..target.channels().min(source.channels()) {
        for y in 0..target.height() {
            for x in 0..target.width() {
                target[(c, y, x)] += source[(c, y, x)] * scale;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_blur() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(100.0);
        let blurred = box_blur(&img, 1);
        assert!((blurred[(0, 4, 4)] - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_multi_scale_unsharp_mask() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(100.0);
        let result = multi_scale_unsharp_mask(&img, &[1, 2], 0.5);
        assert_eq!(result.width(), 8);
        assert_eq!(result.height(), 8);
    }

    #[test]
    fn test_local_contrast_enhancement() {
        let mut img = F32Image::new(8, 8, 1);
        for y in 0..8 {
            for x in 0..8 {
                img[(0, y, x)] = (x + y) as f32;
            }
        }
        let result = local_contrast_enhancement(&img, 2, 1.5);
        assert_eq!(result.width(), 8);
    }

    #[test]
    fn test_structure_transfer() {
        let mut target = F32Image::new(4, 4, 1);
        target.fill(50.0);
        let mut source = F32Image::new(4, 4, 1);
        source.fill(100.0);
        let result = structure_transfer(&target, &source, 0.5);
        assert_eq!(result.width(), 4);
    }
}
