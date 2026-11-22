//! Procedural sample frame — a synthetic deep-sky field used by the
//! shell's sample canvas and the render tests.
//!
//! Explicitly NOT real user data: a dark linear-light field with a
//! nebula gradient and a scatter of stars, generated so the canvas +
//! tone-map pipeline can be exercised end-to-end without artifacts on
//! disk. Values are mostly near zero so AutoStretch visibly matters.

use astroforge_core::image::F32Image;

/// Generate a 3-channel synthetic frame: nebula gradient + stars +
/// sensor-ish noise floor.
pub fn synthetic_deep_sky(width: usize, height: usize) -> F32Image {
    let mut img = F32Image::new(width, height, 3);
    let cx = width as f64 * 0.5;
    let cy = height as f64 * 0.42;
    let radius = (width.min(height) as f64) * 0.38;

    // Deterministic pseudo-random (LCG) so runs are reproducible.
    let mut state: u64 = 0x2A_5C_7E_1B_D3_90_6F_11;

    // Nebula: broad exponential gradient with per-channel offsets
    // (H-alpha red dominant, O-III teal accents), very dim in linear.
    for y in 0..height {
        for x in 0..width {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let d = (dx * dx + dy * dy).sqrt() / radius;
            let falloff = (-2.6 * d).exp();
            let swirl = ((dx / radius + dy / radius * 0.6) * 2.2).sin().abs();
            let r = (falloff * (0.55 + 0.45 * swirl) * 0.22).min(1.0);
            let g = (falloff * 0.13 * (1.2 - swirl)).max(0.0);
            let b = (falloff * (0.10 + 0.10 * (d * 3.0).sin().abs())).max(0.0);
            img[(0, y, x)] = r as f32;
            img[(1, y, x)] = g as f32;
            img[(2, y, x)] = b as f32;
        }
    }

    // Stars: ~140 point sources with soft Gaussian-ish cores spread
    // over the full field (a few bright, most faint).
    for _ in 0..140 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let fx = ((state >> 33) as f64 / (u64::MAX >> 33) as f64) % 1.0;
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let fy = ((state >> 33) as f64 / (u64::MAX >> 33) as f64) % 1.0;
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let bright = 0.25 + 0.75 * (((state >> 33) as f64 / (u64::MAX >> 33) as f64) % 1.0);

        let sx = (fx * width as f64) as isize;
        let sy = (fy * height as f64) as isize;
        let core = (1.0 + bright * 2.5) as isize;
        for oy in -core..=core {
            for ox in -core..=core {
                let px = sx + ox;
                let py = sy + oy;
                if px < 0 || py < 0 || px >= width as isize || py >= height as isize {
                    continue;
                }
                let d2 = (ox * ox + oy * oy) as f64;
                let f = (-d2 / (0.7 + bright * 1.8)).exp() * bright;
                let (px, py) = (px as usize, py as usize);
                img[(0, py, px)] = (img[(0, py, px)] + f as f32).min(1.0);
                img[(1, py, px)] = (img[(1, py, px)] + f as f32).min(1.0);
                img[(2, py, px)] = (img[(2, py, px)] + (f * 1.05) as f32).min(1.0);
            }
        }
    }

    img
}
