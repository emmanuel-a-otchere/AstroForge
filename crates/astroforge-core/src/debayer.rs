use crate::image::F32Image;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BayerPattern {
    RGGB,
    BGGR,
    GRBG,
    GBRG,
}

impl BayerPattern {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "RGGB" => Some(Self::RGGB),
            "BGGR" => Some(Self::BGGR),
            "GRBG" => Some(Self::GRBG),
            "GBRG" => Some(Self::GBRG),
            _ => None,
        }
    }

    pub fn color_at(&self, x: usize, y: usize) -> usize {
        let x_even = x % 2 == 0;
        let y_even = y % 2 == 0;
        match self {
            Self::RGGB => {
                if x_even && y_even {
                    0
                } else if !x_even && y_even {
                    1
                } else if x_even && !y_even {
                    1
                } else {
                    2
                }
            }
            Self::BGGR => {
                if x_even && y_even {
                    2
                } else if !x_even && y_even {
                    1
                } else if x_even && !y_even {
                    1
                } else {
                    0
                }
            }
            Self::GRBG => {
                if x_even && y_even {
                    1
                } else if !x_even && y_even {
                    0
                } else if x_even && !y_even {
                    2
                } else {
                    1
                }
            }
            Self::GBRG => {
                if x_even && y_even {
                    1
                } else if !x_even && y_even {
                    2
                } else if x_even && !y_even {
                    0
                } else {
                    1
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DebayerAlgorithm {
    Bilinear,
    Vng,
}

pub fn debayer(bayer: &F32Image, pattern: BayerPattern, algorithm: DebayerAlgorithm) -> F32Image {
    match algorithm {
        DebayerAlgorithm::Bilinear => debayer_bilinear(bayer, pattern),
        DebayerAlgorithm::Vng => debayer_bilinear(bayer, pattern),
    }
}

fn debayer_bilinear(bayer: &F32Image, pattern: BayerPattern) -> F32Image {
    let width = bayer.width();
    let height = bayer.height();
    let mut result = F32Image::new(width, height, 3);

    for y in 0..height {
        for x in 0..width {
            let c = pattern.color_at(x, y);
            let val = bayer[(0, y, x)];
            result[(c, y, x)] = val;

            for ch in 0..3 {
                if ch == c {
                    continue;
                }
                let mut sum = 0.0f32;
                let mut count = 0;

                for (dx, dy) in [
                    (-1, 0),
                    (1, 0),
                    (0, -1),
                    (0, 1),
                    (-1, -1),
                    (1, -1),
                    (-1, 1),
                    (1, 1),
                ] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                        let nx = nx as usize;
                        let ny = ny as usize;
                        if pattern.color_at(nx, ny) == ch {
                            sum += bayer[(0, ny, nx)];
                            count += 1;
                        }
                    }
                }

                if count > 0 {
                    result[(ch, y, x)] = sum / count as f32;
                }
            }
        }
    }

    result
}

pub fn apply_white_balance(image: &F32Image, r_gain: f32, g_gain: f32, b_gain: f32) -> F32Image {
    let mut result = image.clone();
    let gains = [r_gain, g_gain, b_gain];
    for c in 0..result.channels().min(3) {
        let gain = gains[c];
        for val in result.slice_mut(ndarray::s![c..c + 1, .., ..]).iter_mut() {
            *val *= gain;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bayer_pattern_color_at() {
        let p = BayerPattern::RGGB;
        assert_eq!(p.color_at(0, 0), 0); // R
        assert_eq!(p.color_at(1, 0), 1); // G
        assert_eq!(p.color_at(0, 1), 1); // G
        assert_eq!(p.color_at(1, 1), 2); // B
    }

    #[test]
    fn test_bayer_pattern_from_str() {
        assert_eq!(BayerPattern::from_str("RGGB"), Some(BayerPattern::RGGB));
        assert_eq!(BayerPattern::from_str("invalid"), None);
    }

    #[test]
    fn test_debayer_bilinear() {
        let mut bayer = F32Image::new(8, 8, 1);
        for y in 0..8 {
            for x in 0..8 {
                bayer[(0, y, x)] = ((x + y) % 4) as f32 * 100.0;
            }
        }
        let result = debayer(&bayer, BayerPattern::RGGB, DebayerAlgorithm::Bilinear);
        assert_eq!(result.channels(), 3);
        assert_eq!(result.width(), 8);
        assert_eq!(result.height(), 8);
    }

    #[test]
    fn test_apply_white_balance() {
        let mut img = F32Image::new(4, 4, 3);
        img.fill(100.0);
        let result = apply_white_balance(&img, 1.0, 1.0, 2.0);
        assert!((result[(2, 0, 0)] - 200.0).abs() < 0.01);
        assert!((result[(0, 0, 0)] - 100.0).abs() < 0.01);
    }
}
