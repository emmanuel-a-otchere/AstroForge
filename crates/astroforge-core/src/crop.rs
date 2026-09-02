use crate::image::F32Image;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropRegion {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

pub fn crop(image: &F32Image, region: &CropRegion) -> F32Image {
    let channels = image.channels();
    let mut result = F32Image::new(region.width, region.height, channels);

    for c in 0..channels {
        for y in 0..region.height {
            for x in 0..region.width {
                let src_y = region.y + y;
                let src_x = region.x + x;
                if src_y < image.height() && src_x < image.width() {
                    result[(c, y, x)] = image[(c, src_y, src_x)];
                }
            }
        }
    }

    result
}

pub fn auto_crop_to_subject(image: &F32Image, border_percent: f64) -> F32Image {
    let width = image.width();
    let height = image.height();
    let channels = image.channels();

    let mut min_x = width;
    let mut max_x = 0usize;
    let mut min_y = height;
    let mut max_y = 0usize;

    let mean = image.iter().sum::<f32>() / image.len() as f32;
    let threshold = mean * 1.1;

    let mut found = false;
    for c in 0..channels.min(1) {
        for y in 0..height {
            for x in 0..width {
                if image[(c, y, x)] > threshold {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    found = true;
                }
            }
        }
    }

    if !found {
        return image.clone();
    }

    let border_x = (width as f64 * border_percent / 100.0) as usize;
    let border_y = (height as f64 * border_percent / 100.0) as usize;

    let region = CropRegion {
        x: min_x.saturating_sub(border_x),
        y: min_y.saturating_sub(border_y),
        width: (max_x - min_x + 2 * border_x).min(width),
        height: (max_y - min_y + 2 * border_y).min(height),
    };

    crop(image, &region)
}

pub fn rotate_90(image: &F32Image, clockwise: bool) -> F32Image {
    let width = image.width();
    let height = image.height();
    let channels = image.channels();
    let mut result = F32Image::new(height, width, channels);

    for c in 0..channels {
        for y in 0..height {
            for x in 0..width {
                let (nx, ny) = if clockwise {
                    (height - 1 - y, x)
                } else {
                    (y, width - 1 - x)
                };
                result[(c, ny, nx)] = image[(c, y, x)];
            }
        }
    }

    result
}

pub fn remove_borders(image: &F32Image, border_width: usize) -> F32Image {
    let width = image.width();
    let height = image.height();
    let _channels = image.channels();

    if border_width == 0 {
        return image.clone();
    }

    let new_w = width.saturating_sub(2 * border_width);
    let new_h = height.saturating_sub(2 * border_width);

    if new_w == 0 || new_h == 0 {
        return image.clone();
    }

    let region = CropRegion {
        x: border_width,
        y: border_width,
        width: new_w,
        height: new_h,
    };

    crop(image, &region)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crop() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(100.0);
        let region = CropRegion {
            x: 2,
            y: 2,
            width: 4,
            height: 4,
        };
        let result = crop(&img, &region);
        assert_eq!(result.width(), 4);
        assert_eq!(result.height(), 4);
    }

    #[test]
    fn test_auto_crop() {
        let mut img = F32Image::new(16, 16, 1);
        img.fill(10.0);
        img[(0, 5, 5)] = 1000.0;
        img[(0, 10, 10)] = 1000.0;
        let result = auto_crop_to_subject(&img, 5.0);
        assert!(result.width() <= 16);
        assert!(result.height() <= 16);
    }

    #[test]
    fn test_rotate_90_clockwise() {
        let mut img = F32Image::new(4, 2, 1);
        img[(0, 0, 0)] = 1.0;
        img[(0, 0, 1)] = 2.0;
        let result = rotate_90(&img, true);
        assert_eq!(result.width(), 2);
        assert_eq!(result.height(), 4);
    }

    #[test]
    fn test_remove_borders() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(100.0);
        let result = remove_borders(&img, 1);
        assert_eq!(result.width(), 6);
        assert_eq!(result.height(), 6);
    }
}
