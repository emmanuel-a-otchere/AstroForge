use crate::image::F32Image;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturePoint {
    pub x: f64,
    pub y: f64,
    pub response: f64,
}

pub fn detect_features(
    image: &F32Image,
    max_corners: usize,
    quality_threshold: f64,
) -> Vec<FeaturePoint> {
    let c = 0;
    let width = image.width();
    let height = image.height();
    let mut features = Vec::new();

    let mean = image.iter().sum::<f32>() / image.len() as f32;
    let threshold = mean * quality_threshold as f32;

    let block_size = 16;
    for by in (0..height).step_by(block_size) {
        for bx in (0..width).step_by(block_size) {
            let end_y = (by + block_size).min(height);
            let end_x = (bx + block_size).min(width);

            let mut best_x = bx;
            let mut best_y = by;
            let mut best_response = 0.0f32;

            for y in by..end_y {
                for x in bx..end_x {
                    let response = harris_response(image, c, x, y);
                    if response > best_response && response > threshold {
                        best_response = response;
                        best_x = x;
                        best_y = y;
                    }
                }
            }

            if best_response > threshold {
                features.push(FeaturePoint {
                    x: best_x as f64,
                    y: best_y as f64,
                    response: best_response as f64,
                });
            }
        }
    }

    features.sort_by(|a, b| {
        b.response
            .partial_cmp(&a.response)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    features.truncate(max_corners);
    features
}

fn harris_response(image: &F32Image, channel: usize, x: usize, y: usize) -> f32 {
    let mut ix = 0.0f32;
    let mut iy = 0.0f32;

    if x > 0 && x < image.width() - 1 {
        ix = image[(channel, y, x + 1)] - image[(channel, y, x - 1)];
    }
    if y > 0 && y < image.height() - 1 {
        iy = image[(channel, y + 1, x)] - image[(channel, y - 1, x)];
    }

    ix * ix + iy * iy
}

pub fn detect_limb(image: &F32Image) -> Option<LimbInfo> {
    let c = 0;
    let width = image.width();
    let height = image.height();

    let mean = image.iter().sum::<f32>() / image.len() as f32;

    let mut min_x = width;
    let mut max_x = 0usize;
    let mut min_y = height;
    let mut max_y = 0usize;
    let mut found = false;

    let threshold = mean * 1.2;

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

    if !found {
        return None;
    }

    let cx = (min_x + max_x) as f64 / 2.0;
    let cy = (min_y + max_y) as f64 / 2.0;
    let radius = ((max_x - min_x).max(max_y - min_y) as f64) / 2.0;

    Some(LimbInfo {
        center_x: cx,
        center_y: cy,
        radius,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimbInfo {
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
}

pub fn track_features(
    ref_features: &[FeaturePoint],
    frame: &F32Image,
    search_radius: usize,
) -> Vec<(FeaturePoint, FeaturePoint)> {
    let c = 0;
    let mut matches = Vec::new();

    for ref_pt in ref_features {
        let rx = ref_pt.x as usize;
        let ry = ref_pt.y as usize;
        let mut best_x = rx;
        let mut best_y = ry;
        let mut best_response = 0.0f32;

        for dy in -(search_radius as i32)..=(search_radius as i32) {
            for dx in -(search_radius as i32)..=(search_radius as i32) {
                let nx = (rx as i32 + dx) as usize;
                let ny = (ry as i32 + dy) as usize;
                if nx < frame.width() && ny < frame.height() {
                    let response = harris_response(frame, c, nx, ny);
                    if response > best_response {
                        best_response = response;
                        best_x = nx;
                        best_y = ny;
                    }
                }
            }
        }

        matches.push((
            ref_pt.clone(),
            FeaturePoint {
                x: best_x as f64,
                y: best_y as f64,
                response: best_response as f64,
            },
        ));
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(w: usize, h: usize) -> F32Image {
        let mut img = F32Image::new(w, h, 1);
        for y in 0..h {
            for x in 0..w {
                img[(0, y, x)] = ((x + y) % 64) as f32;
            }
        }
        img
    }

    #[test]
    fn test_detect_features() {
        let img = make_test_image(64, 64);
        let features = detect_features(&img, 20, 0.5);
        assert!(features.len() <= 20);
    }

    #[test]
    fn test_detect_limb() {
        let mut img = F32Image::new(64, 64, 1);
        img.fill(10.0);
        for y in 20..44 {
            for x in 20..44 {
                img[(0, y, x)] = 200.0;
            }
        }
        let limb = detect_limb(&img);
        assert!(limb.is_some());
        let l = limb.unwrap();
        assert!(l.radius > 0.0);
    }

    #[test]
    fn test_detect_limb_none() {
        let img = F32Image::new(32, 32, 1);
        let limb = detect_limb(&img);
        assert!(limb.is_none());
    }

    #[test]
    fn test_track_features() {
        let img = make_test_image(64, 64);
        let features = detect_features(&img, 10, 0.3);
        if !features.is_empty() {
            let matches = track_features(&features, &img, 3);
            assert_eq!(matches.len(), features.len());
        }
    }
}
