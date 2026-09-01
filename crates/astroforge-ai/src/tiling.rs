use astroforge_core::image::F32Image;
use serde::{Deserialize, Serialize};

pub const DEFAULT_TILE_SIZE: u32 = 512;
pub const DEFAULT_OVERLAP: u32 = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileConfig {
    pub tile_size: u32,
    pub overlap: u32,
}

impl Default for TileConfig {
    fn default() -> Self {
        Self {
            tile_size: DEFAULT_TILE_SIZE,
            overlap: DEFAULT_OVERLAP,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn generate_tiles(image_width: u32, image_height: u32, config: &TileConfig) -> Vec<Tile> {
    let mut tiles = Vec::new();
    let step = config.tile_size - config.overlap;

    let mut y = 0u32;
    while y < image_height {
        let mut x = 0u32;
        while x < image_width {
            let tile_w = config.tile_size.min(image_width - x);
            let tile_h = config.tile_size.min(image_height - y);
            tiles.push(Tile {
                x,
                y,
                width: tile_w,
                height: tile_h,
            });
            x += step;
        }
        y += step;
    }

    tiles
}

pub fn extract_tile(image: &F32Image, tile: &Tile) -> F32Image {
    let channels = image.channels();
    let mut result = F32Image::new(tile.width as usize, tile.height as usize, channels);

    for c in 0..channels {
        for y in 0..tile.height as usize {
            for x in 0..tile.width as usize {
                let src_y = (tile.y as usize + y).min(image.height() - 1);
                let src_x = (tile.x as usize + x).min(image.width() - 1);
                result[(c, y, x)] = image[(c, src_y, src_x)];
            }
        }
    }

    result
}

pub fn blend_tile(
    output: &mut F32Image,
    tile_image: &F32Image,
    tile: &Tile,
    weight_map: &mut F32Image,
) {
    let channels = output.channels();
    let tile_h = tile.height as usize;
    let tile_w = tile.width as usize;

    for c in 0..channels {
        for y in 0..tile_h {
            for x in 0..tile_w {
                let dst_y = tile.y as usize + y;
                let dst_x = tile.x as usize + x;
                if dst_y < output.height() && dst_x < output.width() {
                    let weight = cosine_blend_weight(x, y, tile_w, tile_h);
                    let current_weight = weight_map[(c, dst_y, dst_x)];
                    let new_weight = current_weight + weight;

                    if new_weight > 0.0 {
                        output[(c, dst_y, dst_x)] = (output[(c, dst_y, dst_x)] * current_weight
                            + tile_image[(c, y, x)] * weight)
                            / new_weight;
                        weight_map[(c, dst_y, dst_x)] = new_weight;
                    }
                }
            }
        }
    }
}

fn cosine_blend_weight(x: usize, y: usize, width: usize, height: usize) -> f32 {
    let wx = if width > 1 {
        let t = x as f32 / (width - 1) as f32;
        0.5 * (1.0 - (std::f32::consts::PI * t).cos())
    } else {
        1.0
    };
    let wy = if height > 1 {
        let t = y as f32 / (height - 1) as f32;
        0.5 * (1.0 - (std::f32::consts::PI * t).cos())
    } else {
        1.0
    };
    wx * wy
}

pub fn run_tiled_inference(
    image: &F32Image,
    config: &TileConfig,
    infer_fn: impl Fn(&F32Image) -> F32Image,
) -> F32Image {
    let width = image.width();
    let height = image.height();
    let channels = image.channels();

    let mut output = F32Image::new(width, height, channels);
    let mut weight_map = F32Image::new(width, height, channels);

    let tiles = generate_tiles(width as u32, height as u32, config);

    for tile in &tiles {
        let tile_input = extract_tile(image, tile);
        let tile_output = infer_fn(&tile_input);
        blend_tile(&mut output, &tile_output, tile, &mut weight_map);
    }

    for c in 0..channels {
        for y in 0..height {
            for x in 0..width {
                if weight_map[(c, y, x)] == 0.0 {
                    output[(c, y, x)] = image[(c, y, x)];
                }
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_tiles() {
        let config = TileConfig {
            tile_size: 256,
            overlap: 64,
        };
        let tiles = generate_tiles(512, 512, &config);
        assert!(tiles.len() >= 4);
    }

    #[test]
    fn test_generate_tiles_small_image() {
        let config = TileConfig {
            tile_size: 512,
            overlap: 64,
        };
        let tiles = generate_tiles(100, 100, &config);
        assert_eq!(tiles.len(), 1);
        assert_eq!(tiles[0].width, 100);
        assert_eq!(tiles[0].height, 100);
    }

    #[test]
    fn test_extract_and_blend_tile() {
        let mut image = F32Image::new(16, 16, 1);
        for y in 0..16 {
            for x in 0..16 {
                image[(0, y, x)] = (x + y) as f32;
            }
        }
        let tile = Tile {
            x: 0,
            y: 0,
            width: 8,
            height: 8,
        };
        let extracted = extract_tile(&image, &tile);
        assert_eq!(extracted.width(), 8);
        assert_eq!(extracted.height(), 8);

        let mut output = F32Image::new(16, 16, 1);
        let mut weight = F32Image::new(16, 16, 1);
        blend_tile(&mut output, &extracted, &tile, &mut weight);
        assert!(weight[(0, 0, 0)] > 0.0);
    }

    #[test]
    fn test_run_tiled_inference_identity() {
        let mut image = F32Image::new(32, 32, 1);
        for y in 0..32 {
            for x in 0..32 {
                image[(0, y, x)] = (x + y) as f32;
            }
        }
        let config = TileConfig {
            tile_size: 16,
            overlap: 4,
        };
        let result = run_tiled_inference(&image, &config, |tile| tile.clone());
        for y in 0..32 {
            for x in 0..32 {
                assert!((result[(0, y, x)] - image[(0, y, x)]).abs() < 1.0);
            }
        }
    }

    #[test]
    fn test_cosine_blend_weight() {
        let w_center = cosine_blend_weight(4, 4, 8, 8);
        let w_corner = cosine_blend_weight(0, 0, 8, 8);
        assert!(w_center > w_corner);
    }
}
