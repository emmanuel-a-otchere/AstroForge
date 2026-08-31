use crate::image::F32Image;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Star {
    pub x: f64,
    pub y: f64,
    pub brightness: f64,
    pub fwhm: f64,
}

pub fn extract_stars(image: &F32Image, threshold_sigma: f64) -> Vec<Star> {
    let c = 0;
    let height = image.height();
    let width = image.width();

    let mean = image.iter().sum::<f32>() / image.len() as f32;
    let var = image
        .iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f32>()
        / image.len() as f32;
    let std = var.sqrt();
    let threshold = mean + threshold_sigma as f32 * std;

    let mut stars = Vec::new();
    let mut visited = vec![false; width * height];

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            if visited[y * width + x] {
                continue;
            }
            let val = image[(c, y, x)];
            if val < threshold {
                continue;
            }

            let mut is_local_max = true;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;
                    if nx < width && ny < height && image[(c, ny, nx)] > val {
                        is_local_max = false;
                        break;
                    }
                }
                if !is_local_max {
                    break;
                }
            }

            if is_local_max {
                let (cx, cy, brightness) = centroid(image, x, y, 5);
                let fwhm = estimate_fwhm(image, cx, cy);
                stars.push(Star {
                    x: cx,
                    y: cy,
                    brightness: brightness as f64,
                    fwhm,
                });
                for dy in -5..=5 {
                    for dx in -5..=5 {
                        let nx = (x as i32 + dx) as usize;
                        let ny = (y as i32 + dy) as usize;
                        if nx < width && ny < height {
                            visited[ny * width + nx] = true;
                        }
                    }
                }
            }
        }
    }

    stars.sort_by(|a, b| b.brightness.partial_cmp(&a.brightness).unwrap_or(std::cmp::Ordering::Equal));
    stars
}

fn centroid(image: &F32Image, x: usize, y: usize, radius: usize) -> (f64, f64, f32) {
    let c = 0;
    let mut sum_x = 0.0f64;
    let mut sum_y = 0.0f64;
    let mut sum_w = 0.0f64;
    let mut sum_val = 0.0f32;

    let r = radius as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            let nx = (x as i32 + dx) as usize;
            let ny = (y as i32 + dy) as usize;
            if nx < image.width() && ny < image.height() {
                let val = image[(c, ny, nx)].max(0.0);
                let w = val as f64;
                sum_x += nx as f64 * w;
                sum_y += ny as f64 * w;
                sum_w += w;
                sum_val += val;
            }
        }
    }

    if sum_w > 0.0 {
        (sum_x / sum_w, sum_y / sum_w, sum_val)
    } else {
        (x as f64, y as f64, 0.0)
    }
}

fn estimate_fwhm(image: &F32Image, cx: f64, cy: f64) -> f64 {
    let c = 0;
    let x = cx as usize;
    let y = cy as usize;
    let peak = image[(c, y, x)].max(0.001);
    let half = peak / 2.0;

    let mut left = x;
    while left > 0 && image[(c, y, left)] > half {
        left -= 1;
    }
    let mut right = x;
    while right < image.width() - 1 && image[(c, y, right)] > half {
        right += 1;
    }

    let fwhm_x = (right - left) as f64;

    let mut top = y;
    while top > 0 && image[(c, top, x)] > half {
        top -= 1;
    }
    let mut bottom = y;
    while bottom < image.height() - 1 && image[(c, bottom, x)] > half {
        bottom += 1;
    }

    let fwhm_y = (bottom - top) as f64;
    (fwhm_x + fwhm_y) / 2.0
}

pub fn select_reference_frame(stars_per_frame: &[Vec<Star>]) -> usize {
    let mut best_idx = 0;
    let mut best_score = f64::MAX;

    for (i, stars) in stars_per_frame.iter().enumerate() {
        let median_fwhm = if stars.is_empty() {
            f64::MAX
        } else {
            let mut fwfms: Vec<f64> = stars.iter().map(|s| s.fwhm).collect();
            fwfms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            fwfms[fwfms.len() / 2]
        };

        if median_fwhm < best_score {
            best_score = median_fwhm;
            best_idx = i;
        }
    }

    best_idx
}

#[derive(Debug, Clone)]
pub struct AffineTransform {
    pub dx: f64,
    pub dy: f64,
    pub rotation: f64,
    pub scale: f64,
}

pub fn compute_transform(
    ref_stars: &[Star],
    frame_stars: &[Star],
) -> Option<AffineTransform> {
    if ref_stars.len() < 3 || frame_stars.len() < 3 {
        return None;
    }

    let n = ref_stars.len().min(frame_stars.len());
    let mut sum_dx = 0.0f64;
    let mut sum_dy = 0.0f64;

    for i in 0..n {
        sum_dx += frame_stars[i].x - ref_stars[i].x;
        sum_dy += frame_stars[i].y - ref_stars[i].y;
    }

    let dx = sum_dx / n as f64;
    let dy = sum_dy / n as f64;

    Some(AffineTransform {
        dx,
        dy,
        rotation: 0.0,
        scale: 1.0,
    })
}

pub fn apply_transform(image: &F32Image, transform: &AffineTransform) -> F32Image {
    let c = 0;
    let height = image.height();
    let width = image.width();
    let mut result = F32Image::new(width, height, image.channels());

    for ch in 0..image.channels() {
        for y in 0..height {
            for x in 0..width {
                let src_x = x as f64 - transform.dx;
                let src_y = y as f64 - transform.dy;
                let sx = src_x.round() as i64;
                let sy = src_y.round() as i64;
                if sx >= 0 && sx < width as i64 && sy >= 0 && sy < height as i64 {
                    result[(ch, y, x)] = image[(ch, sy as usize, sx as usize)];
                }
            }
        }
    }

    result
}

pub fn cross_correlate(
    ref_cutout: &F32Image,
    frame_cutout: &F32Image,
) -> (f64, f64) {
    let c = 0;
    let height = ref_cutout.height();
    let width = ref_cutout.width();

    let mut best_dx = 0.0f64;
    let mut best_dy = 0.0f64;
    let mut best_corr = 0.0f64;

    let search_range = 3;
    for dy in -search_range..=search_range {
        for dx in -search_range..=search_range {
            let mut corr = 0.0f64;
            let mut count = 0;
            for y in 0..height {
                for x in 0..width {
                    let fx = (x as i32 + dx) as usize;
                    let fy = (y as i32 + dy) as usize;
                    if fx < width && fy < height {
                        corr += ref_cutout[(c, y, x)] as f64 * frame_cutout[(c, fy, fx)] as f64;
                        count += 1;
                    }
                }
            }
            if count > 0 {
                let avg_corr = corr / count as f64;
                if avg_corr > best_corr {
                    best_corr = avg_corr;
                    best_dx = dx as f64;
                    best_dy = dy as f64;
                }
            }
        }
    }

    (best_dx, best_dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_star_field(width: usize, height: usize) -> F32Image {
        let mut img = F32Image::new(width, height, 1);
        img.fill(10.0);
        img[(0, 10, 10)] = 1000.0;
        img[(0, 10, 11)] = 900.0;
        img[(0, 11, 10)] = 900.0;
        img[(0, 50, 50)] = 800.0;
        img[(0, 50, 51)] = 700.0;
        img[(0, 51, 50)] = 700.0;
        img
    }

    #[test]
    fn test_extract_stars() {
        let img = make_star_field(64, 64);
        let stars = extract_stars(&img, 3.0);
        assert!(stars.len() >= 2);
        assert!(stars[0].brightness > stars[1].brightness);
    }

    #[test]
    fn test_select_reference_frame() {
        let stars_per_frame = vec![
            vec![
                Star { x: 10.0, y: 10.0, brightness: 1000.0, fwhm: 3.0 },
                Star { x: 50.0, y: 50.0, brightness: 800.0, fwhm: 3.5 },
            ],
            vec![
                Star { x: 10.0, y: 10.0, brightness: 1000.0, fwhm: 2.0 },
                Star { x: 50.0, y: 50.0, brightness: 800.0, fwhm: 2.5 },
            ],
        ];
        let ref_idx = select_reference_frame(&stars_per_frame);
        assert_eq!(ref_idx, 1);
    }

    #[test]
    fn test_compute_transform() {
        let ref_stars = vec![
            Star { x: 10.0, y: 10.0, brightness: 1000.0, fwhm: 3.0 },
            Star { x: 50.0, y: 50.0, brightness: 800.0, fwhm: 3.0 },
            Star { x: 30.0, y: 70.0, brightness: 600.0, fwhm: 3.0 },
        ];
        let frame_stars = vec![
            Star { x: 12.0, y: 11.0, brightness: 1000.0, fwhm: 3.0 },
            Star { x: 52.0, y: 51.0, brightness: 800.0, fwhm: 3.0 },
            Star { x: 32.0, y: 71.0, brightness: 600.0, fwhm: 3.0 },
        ];
        let transform = compute_transform(&ref_stars, &frame_stars).unwrap();
        assert!((transform.dx - 2.0).abs() < 0.01);
        assert!((transform.dy - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_apply_transform() {
        let mut img = F32Image::new(8, 8, 1);
        img.fill(0.0);
        img[(0, 4, 4)] = 100.0;
        let transform = AffineTransform { dx: 1.0, dy: 0.0, rotation: 0.0, scale: 1.0 };
        let result = apply_transform(&img, &transform);
        assert_eq!(result[(0, 4, 3)], 100.0);
    }

    #[test]
    fn test_cross_correlate() {
        let mut ref_img = F32Image::new(8, 8, 1);
        ref_img.fill(0.0);
        ref_img[(0, 4, 4)] = 100.0;
        let mut frame_img = F32Image::new(8, 8, 1);
        frame_img.fill(0.0);
        frame_img[(0, 4, 5)] = 100.0;
        let (dx, dy) = cross_correlate(&ref_img, &frame_img);
        assert!((dx - 1.0).abs() < 0.01);
        assert!((dy - 0.0).abs() < 0.01);
    }
}
