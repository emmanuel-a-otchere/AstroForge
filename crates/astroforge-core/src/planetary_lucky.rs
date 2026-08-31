use crate::image::F32Image;
use crate::planetary_features::FeaturePoint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSharpness {
    pub index: usize,
    pub sharpness: f64,
}

pub fn compute_sharpness(image: &F32Image) -> f64 {
    let c = 0;
    let width = image.width();
    let height = image.height();
    let mut total_gradient = 0.0f64;
    let mut count = 0u64;

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let gx = (image[(c, y, x + 1)] - image[(c, y, x - 1)]).abs();
            let gy = (image[(c, y + 1, x)] - image[(c, y - 1, x)]).abs();
            total_gradient += (gx + gy) as f64;
            count += 1;
        }
    }

    if count > 0 {
        total_gradient / count as f64
    } else {
        0.0
    }
}

pub fn rank_by_sharpness(sharpness_scores: &[FrameSharpness]) -> Vec<usize> {
    let mut sorted = sharpness_scores.to_vec();
    sorted.sort_by(|a, b| b.sharpness.partial_cmp(&a.sharpness).unwrap_or(std::cmp::Ordering::Equal));
    sorted.iter().map(|s| s.index).collect()
}

pub fn select_best_frames(ranked: &[usize], total_frames: usize, percentile: f64) -> Vec<usize> {
    let count = ((total_frames as f64) * percentile / 100.0).round() as usize;
    ranked.iter().take(count.max(1)).copied().collect()
}

pub fn lucky_imaging_select(
    frames: &[F32Image],
    best_percent: f64,
) -> Vec<usize> {
    let sharpness_scores: Vec<FrameSharpness> = frames
        .iter()
        .enumerate()
        .map(|(i, f)| FrameSharpness {
            index: i,
            sharpness: compute_sharpness(f),
        })
        .collect();

    let ranked = rank_by_sharpness(&sharpness_scores);
    select_best_frames(&ranked, frames.len(), best_percent)
}

pub struct StreamingRanker {
    best_percent: f64,
    scores: Vec<FrameSharpness>,
    frame_count: usize,
    max_memory_frames: usize,
}

impl StreamingRanker {
    pub fn new(best_percent: f64, max_memory_frames: usize) -> Self {
        Self {
            best_percent,
            scores: Vec::new(),
            frame_count: 0,
            max_memory_frames,
        }
    }

    pub fn add_frame(&mut self, sharpness: f64) {
        self.scores.push(FrameSharpness {
            index: self.frame_count,
            sharpness,
        });
        self.frame_count += 1;

        if self.scores.len() > self.max_memory_frames {
            self.scores.sort_by(|a, b| b.sharpness.partial_cmp(&a.sharpness).unwrap_or(std::cmp::Ordering::Equal));
            let keep = (self.max_memory_frames as f64 * self.best_percent / 100.0) as usize;
            self.scores.truncate(keep.max(10));
        }
    }

    pub fn select_best(&self) -> Vec<usize> {
        let mut sorted = self.scores.clone();
        sorted.sort_by(|a, b| b.sharpness.partial_cmp(&a.sharpness).unwrap_or(std::cmp::Ordering::Equal));
        let count = ((self.frame_count as f64) * self.best_percent / 100.0).round() as usize;
        sorted.iter().take(count.max(1)).map(|s| s.index).collect()
    }

    pub fn total_frames(&self) -> usize {
        self.frame_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_uniform(w: usize, h: usize, val: f32) -> F32Image {
        let mut img = F32Image::new(w, h, 1);
        img.fill(val);
        img
    }

    fn make_sharp(w: usize, h: usize) -> F32Image {
        let mut img = F32Image::new(w, h, 1);
        for y in 0..h {
            for x in 0..w {
                img[(0, y, x)] = if (x + y) % 2 == 0 { 200.0 } else { 0.0 };
            }
        }
        img
    }

    #[test]
    fn test_compute_sharpness() {
        let sharp = make_sharp(8, 8);
        let flat = make_uniform(8, 8, 100.0);
        assert!(compute_sharpness(&sharp) > compute_sharpness(&flat));
    }

    #[test]
    fn test_rank_by_sharpness() {
        let scores = vec![
            FrameSharpness { index: 0, sharpness: 10.0 },
            FrameSharpness { index: 1, sharpness: 50.0 },
            FrameSharpness { index: 2, sharpness: 30.0 },
        ];
        let ranked = rank_by_sharpness(&scores);
        assert_eq!(ranked, vec![1, 2, 0]);
    }

    #[test]
    fn test_select_best_frames() {
        let ranked = vec![2, 1, 0, 3, 4];
        let selected = select_best_frames(&ranked, 5, 30.0);
        assert_eq!(selected.len(), 2);
        assert!(selected.contains(&2));
    }

    #[test]
    fn test_lucky_imaging_select() {
        let frames = vec![
            make_uniform(8, 8, 50.0),
            make_sharp(8, 8),
            make_uniform(8, 8, 50.0),
        ];
        let selected = lucky_imaging_select(&frames, 33.0);
        assert_eq!(selected.len(), 1);
        assert!(selected.contains(&1));
    }

    #[test]
    fn test_streaming_ranker() {
        let mut ranker = StreamingRanker::new(20.0, 100);
        for i in 0..1000 {
            let sharpness = (i as f64 % 100.0) + 1.0;
            ranker.add_frame(sharpness);
        }
        assert_eq!(ranker.total_frames(), 1000);
        let best = ranker.select_best();
        assert!(best.len() >= 1);
        assert!(best.len() <= 200);
    }

    #[test]
    fn test_streaming_ranker_50k_frames() {
        let mut ranker = StreamingRanker::new(20.0, 500);
        for i in 0..50_000 {
            let sharpness = ((i * 7) as f64 % 1000.0) + 1.0;
            ranker.add_frame(sharpness);
        }
        assert_eq!(ranker.total_frames(), 50_000);
        let best = ranker.select_best();
        assert!(best.len() >= 1);
        assert!(best.len() <= 10_000);
    }
}
