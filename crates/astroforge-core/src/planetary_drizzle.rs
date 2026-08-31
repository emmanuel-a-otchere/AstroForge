use crate::image::F32Image;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrizzleConfig {
    pub scale: f32,
    pub pixfrac: f32,
}

impl Default for DrizzleConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            pixfrac: 0.6,
        }
    }
}

pub fn planetary_drizzle(
    frames: &[F32Image],
    offsets: &[(f64, f64)],
    config: &DrizzleConfig,
) -> F32Image {
    if frames.is_empty() {
        return F32Image::new(1, 1, 1);
    }

    let base_width = frames[0].width();
    let base_height = frames[0].height();
    let channels = frames[0].channels();
    let scale = config.scale;

    let out_width = (base_width as f32 * scale) as usize;
    let out_height = (base_height as f32 * scale) as usize;

    let mut output = F32Image::new(out_width, out_height, channels);
    let mut weight_map = F32Image::new(out_width, out_height, channels);

    for (frame_idx, frame) in frames.iter().enumerate() {
        let (dx, dy) = if frame_idx < offsets.len() {
            offsets[frame_idx]
        } else {
            (0.0, 0.0)
        };

        for c in 0..channels {
            for y in 0..frame.height() {
                for x in 0..frame.width() {
                    let val = frame[(c, y, x)];
                    let out_x = ((x as f64 + dx) * scale as f64) as usize;
                    let out_y = ((y as f64 + dy) * scale as f64) as usize;

                    if out_x < out_width && out_y < out_height {
                        let w = config.pixfrac;
                        let current_w = weight_map[(c, out_y, out_x)];
                        let new_w = current_w + w;
                        if new_w > 0.0 {
                            output[(c, out_y, out_x)] = (output[(c, out_y, out_x)] * current_w + val * w) / new_w;
                            weight_map[(c, out_y, out_x)] = new_w;
                        }
                    }
                }
            }
        }
    }

    for c in 0..channels {
        for y in 0..out_height {
            for x in 0..out_width {
                if weight_map[(c, y, x)] == 0.0 {
                    output[(c, y, x)] = 0.0;
                }
            }
        }
    }

    output
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
    fn test_drizzle_default_config() {
        let config = DrizzleConfig::default();
        assert!((config.scale - 1.0).abs() < 0.01);
        assert!((config.pixfrac - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_drizzle_single_frame() {
        let frame = make_uniform(8, 8, 100.0);
        let offsets = vec![(0.0, 0.0)];
        let config = DrizzleConfig::default();
        let result = planetary_drizzle(&[frame], &offsets, &config);
        assert_eq!(result.width(), 8);
        assert_eq!(result.height(), 8);
    }

    #[test]
    fn test_drizzle_2x_scale() {
        let frame = make_uniform(8, 8, 100.0);
        let offsets = vec![(0.0, 0.0)];
        let config = DrizzleConfig { scale: 2.0, pixfrac: 0.6 };
        let result = planetary_drizzle(&[frame], &offsets, &config);
        assert_eq!(result.width(), 16);
        assert_eq!(result.height(), 16);
    }

    #[test]
    fn test_drizzle_multiple_frames_with_offsets() {
        let frame1 = make_uniform(8, 8, 100.0);
        let frame2 = make_uniform(8, 8, 200.0);
        let offsets = vec![(0.0, 0.0), (0.5, 0.5)];
        let config = DrizzleConfig::default();
        let result = planetary_drizzle(&[frame1, frame2], &offsets, &config);
        assert_eq!(result.width(), 8);
        assert_eq!(result.height(), 8);
    }

    #[test]
    fn test_drizzle_empty() {
        let config = DrizzleConfig::default();
        let result = planetary_drizzle(&[], &[], &config);
        assert_eq!(result.width(), 1);
    }
}
