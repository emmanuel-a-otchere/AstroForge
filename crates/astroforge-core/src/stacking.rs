use crate::image::F32Image;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackResult {
    pub image: F32Image,
    pub weight_map: F32Image,
    pub frame_count: usize,
    pub rejected_count: usize,
}

pub fn kappa_sigma_stack(
    frames: &[F32Image],
    kappa: f64,
    max_iterations: u32,
) -> Result<StackResult, StackError> {
    if frames.is_empty() {
        return Err(StackError::NoFrames);
    }

    let channels = frames[0].channels();
    let height = frames[0].height();
    let width = frames[0].width();
    let n = frames.len();

    let mut result = F32Image::new(width, height, channels);
    let mut weight_map = F32Image::new(width, height, channels);
    let mut rejected_count = 0usize;

    for c in 0..channels {
        for y in 0..height {
            for x in 0..width {
                let values: Vec<f32> = frames.iter().map(|f| f[(c, y, x)]).collect();
                let mut mask: Vec<bool> = vec![true; n];

                for _ in 0..max_iterations {
                    let active: Vec<f32> = values
                        .iter()
                        .zip(&mask)
                        .filter(|(_, &m)| m)
                        .map(|(&v, _)| v)
                        .collect();
                    if active.len() < 3 {
                        break;
                    }
                    let mean = active.iter().sum::<f32>() / active.len() as f32;
                    let var = active.iter().map(|v| (v - mean).powi(2)).sum::<f32>()
                        / active.len() as f32;
                    let std = var.sqrt();
                    let threshold = kappa as f32 * std;

                    let mut changed = false;
                    for i in 0..n {
                        if mask[i] && (values[i] - mean).abs() > threshold {
                            mask[i] = false;
                            changed = true;
                            rejected_count += 1;
                        }
                    }
                    if !changed {
                        break;
                    }
                }

                let active: Vec<f32> = values
                    .iter()
                    .zip(&mask)
                    .filter(|(_, &m)| m)
                    .map(|(&v, _)| v)
                    .collect();
                if active.is_empty() {
                    result[(c, y, x)] = 0.0;
                } else {
                    result[(c, y, x)] = active.iter().sum::<f32>() / active.len() as f32;
                }
                weight_map[(c, y, x)] = active.len() as f32;
            }
        }
    }

    Ok(StackResult {
        image: result,
        weight_map,
        frame_count: n,
        rejected_count,
    })
}

pub struct StreamingStacker {
    sum: F32Image,
    sum_sq: F32Image,
    count: F32Image,
    frame_count: usize,
}

impl StreamingStacker {
    pub fn new(width: usize, height: usize, channels: usize) -> Self {
        Self {
            sum: F32Image::new(width, height, channels),
            sum_sq: F32Image::new(width, height, channels),
            count: F32Image::new(width, height, channels),
            frame_count: 0,
        }
    }

    pub fn add_frame(&mut self, frame: &F32Image) {
        for c in 0..self.sum.channels() {
            for y in 0..self.sum.height() {
                for x in 0..self.sum.width() {
                    let val = frame[(c, y, x)];
                    self.sum[(c, y, x)] += val;
                    self.sum_sq[(c, y, x)] += val * val;
                    self.count[(c, y, x)] += 1.0;
                }
            }
        }
        self.frame_count += 1;
    }

    pub fn result(&self) -> StackResult {
        let mut image = F32Image::new(self.sum.width(), self.sum.height(), self.sum.channels());
        let mut weight_map =
            F32Image::new(self.sum.width(), self.sum.height(), self.sum.channels());

        for c in 0..image.channels() {
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let cnt = self.count[(c, y, x)];
                    if cnt > 0.0 {
                        image[(c, y, x)] = self.sum[(c, y, x)] / cnt;
                        weight_map[(c, y, x)] = cnt;
                    }
                }
            }
        }

        StackResult {
            image,
            weight_map,
            frame_count: self.frame_count,
            rejected_count: 0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StackError {
    #[error("No frames provided")]
    NoFrames,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_uniform_frame(width: usize, height: usize, value: f32) -> F32Image {
        let mut img = F32Image::new(width, height, 1);
        img.fill(value);
        img
    }

    #[test]
    fn test_kappa_sigma_stack_uniform() {
        let frames = vec![
            make_uniform_frame(4, 4, 100.0),
            make_uniform_frame(4, 4, 100.0),
            make_uniform_frame(4, 4, 100.0),
        ];
        let result = kappa_sigma_stack(&frames, 3.0, 5).unwrap();
        assert!((result.image[(0, 0, 0)] - 100.0).abs() < 0.01);
        assert_eq!(result.frame_count, 3);
        assert_eq!(result.rejected_count, 0);
    }

    #[test]
    fn test_kappa_sigma_rejects_outlier() {
        let mut frames = vec![
            make_uniform_frame(4, 4, 100.0),
            make_uniform_frame(4, 4, 100.0),
            make_uniform_frame(4, 4, 100.0),
            make_uniform_frame(4, 4, 100.0),
            make_uniform_frame(4, 4, 100.0),
        ];
        let mut outlier = make_uniform_frame(4, 4, 100.0);
        outlier[(0, 0, 0)] = 5000.0;
        frames.push(outlier);
        let result = kappa_sigma_stack(&frames, 3.0, 5).unwrap();
        assert!((result.image[(0, 0, 0)] - 100.0).abs() < 10.0);
    }

    #[test]
    fn test_streaming_stacker() {
        let mut stacker = StreamingStacker::new(4, 4, 1);
        stacker.add_frame(&make_uniform_frame(4, 4, 100.0));
        stacker.add_frame(&make_uniform_frame(4, 4, 200.0));
        let result = stacker.result();
        assert!((result.image[(0, 0, 0)] - 150.0).abs() < 0.01);
        assert_eq!(result.frame_count, 2);
        assert!((result.weight_map[(0, 0, 0)] - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_streaming_stacker_memory_bounded() {
        let mut stacker = StreamingStacker::new(8, 8, 1);
        for i in 0..30 {
            stacker.add_frame(&make_uniform_frame(8, 8, i as f32));
        }
        let result = stacker.result();
        assert_eq!(result.frame_count, 30);
        assert!((result.weight_map[(0, 0, 0)] - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_no_frames_error() {
        let result = kappa_sigma_stack(&[], 3.0, 5);
        assert!(result.is_err());
    }
}
