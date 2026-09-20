//! CR-07 §8.4: SNR metric tests.
//!
//! The `estimated_snr` function computes the global
//! signal-to-noise ratio in dB: 20 × log10(mean / sigma).
//! The `local_snr` function computes the mean per-tile SNR
//! across the 4×4 regional grid.
//!
//! These tests pin the contract: a uniform noise-free image
//! has very high SNR; a noisy image has lower SNR; a
//! noise-dominated image has low SNR.

use astroforge_core::image::F32Image;
use astroforge_core::image_analysis::metrics::{estimated_snr, local_snr, Confidence};

fn uniform_image(size: usize, value: f32) -> F32Image {
    let mut img = F32Image::new(size, size, 1);
    for y in 0..size {
        for x in 0..size {
            img[(0, y, x)] = value;
        }
    }
    img
}

fn noisy_image(size: usize, mean: f32, sigma: f32) -> F32Image {
    // Deterministic pseudo-noise: high-frequency sinusoid
    // (period ~6 pixels) so the 3x3 MAD estimator sees it
    // as noise rather than smooth signal.
    let mut img = F32Image::new(size, size, 1);
    for y in 0..size {
        for x in 0..size {
            let v = mean + sigma * ((x as f32).sin() * (y as f32).cos());
            img[(0, y, x)] = v.max(0.0);
        }
    }
    img
}

#[test]
fn estimated_snr_uniform_image_has_high_snr() {
    let img = uniform_image(256, 0.5);
    let sample = estimated_snr(&img);
    // A uniform image has sigma ≈ 0, so SNR should be very
    // high (the sigma is clamped to epsilon, giving a large
    // positive dB value).
    assert!(
        sample.value > 30.0,
        "uniform image SNR {} should be > 30 dB",
        sample.value
    );
}

#[test]
fn estimated_snr_noisy_image_has_lower_snr_than_uniform() {
    let uniform = uniform_image(256, 0.5);
    let noisy = noisy_image(256, 0.5, 0.1);
    let uniform_snr = estimated_snr(&uniform).value;
    let noisy_snr = estimated_snr(&noisy).value;
    // The uniform image has sigma ≈ 0, so its SNR is very
    // high. The noisy image has a non-zero sigma, so its
    // SNR is lower. The exact value depends on the MAD
    // estimator and the noise pattern, but the ordering
    // must hold: noisy < uniform.
    assert!(
        noisy_snr < uniform_snr,
        "noisy SNR {} should be < uniform SNR {}",
        noisy_snr,
        uniform_snr
    );
}

#[test]
fn estimated_snr_value_is_finite() {
    let img = noisy_image(256, 0.5, 0.1);
    let sample = estimated_snr(&img);
    assert!(sample.value.is_finite());
}

#[test]
fn local_snr_uniform_image_has_high_snr() {
    let img = uniform_image(256, 0.5);
    let sample = local_snr(&img);
    // Uniform image: every tile has sigma ≈ 0, so every
    // tile SNR is very high. The mean is also very high.
    assert!(
        sample.value > 30.0,
        "uniform image local SNR {} should be > 30 dB",
        sample.value
    );
}

#[test]
fn local_snr_noisy_image_has_lower_snr_than_uniform() {
    let uniform = uniform_image(256, 0.5);
    let noisy = noisy_image(256, 0.5, 0.1);
    let uniform_snr = local_snr(&uniform).value;
    let noisy_snr = local_snr(&noisy).value;
    assert!(
        noisy_snr < uniform_snr,
        "noisy local SNR {} should be < uniform local SNR {}",
        noisy_snr,
        uniform_snr
    );
}

#[test]
fn local_snr_too_small_image_returns_zero() {
    let img = uniform_image(16, 0.5);
    let sample = local_snr(&img);
    assert_eq!(sample.value, 0.0);
    assert_eq!(sample.confidence, Confidence::Low);
}

#[test]
fn local_snr_value_is_finite() {
    let img = noisy_image(256, 0.5, 0.1);
    let sample = local_snr(&img);
    assert!(sample.value.is_finite());
}

#[test]
fn snr_deterministic_for_same_input() {
    let img = noisy_image(256, 0.5, 0.1);
    let a = estimated_snr(&img);
    let b = estimated_snr(&img);
    assert!((a.value - b.value).abs() < 1e-10);
    let la = local_snr(&img);
    let lb = local_snr(&img);
    assert!((la.value - lb.value).abs() < 1e-10);
}
