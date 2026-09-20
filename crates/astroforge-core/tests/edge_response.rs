//! CR-07 §8.2: edge response metric tests.
//!
//! The `edge_response` function computes the mean Sobel
//! gradient magnitude across the image preview. These
//! tests pin the contract: a uniform image produces zero
//! edges, an image with a sharp vertical edge produces
//! strong edges, and the function is deterministic.

use astroforge_core::image::F32Image;
use astroforge_core::image_analysis::metrics::edge_response;

fn make_uniform_image(w: usize, h: usize, value: f32) -> F32Image {
    let mut img = F32Image::new(w, h, 1);
    for y in 0..h {
        for x in 0..w {
            img[(0, y, x)] = value;
        }
    }
    img
}

/// An image with a sharp vertical edge at column `w / 2`:
/// left half is dark, right half is bright. The Sobel
/// filter should pick up the vertical edge strongly.
fn make_sharp_vertical_edge_image(w: usize, h: usize) -> F32Image {
    let mut img = F32Image::new(w, h, 1);
    let mid = w / 2;
    for y in 0..h {
        for x in 0..w {
            img[(0, y, x)] = if x < mid { 0.0 } else { 1.0 };
        }
    }
    img
}

/// A gently-graded image: brightness ramps linearly from
/// 0 at the left to 1 at the right. Sobel detects a
/// gradient but it's a smooth gradient (not a sharp
/// edge), so the mean magnitude is lower than for a
/// sharp edge.
fn make_gradient_image(w: usize, h: usize) -> F32Image {
    let mut img = F32Image::new(w, h, 1);
    for y in 0..h {
        for x in 0..w {
            img[(0, y, x)] = x as f32 / (w - 1) as f32;
        }
    }
    img
}

#[test]
fn edge_response_uniform_image_has_zero_magnitude() {
    let img = make_uniform_image(256, 256, 0.5);
    let sample = edge_response(&img);
    // A perfectly uniform image has no gradient; every Sobel
    // magnitude is 0, so the mean is 0.
    assert_eq!(sample.value, 0.0);
    assert!(sample.label.is_some());
}

#[test]
fn edge_response_sharp_edge_image_has_high_magnitude() {
    let img = make_sharp_vertical_edge_image(256, 256);
    let sample = edge_response(&img);
    // A sharp edge produces strong Sobel magnitudes. The
    // mean across the preview should be substantially > 0.
    assert!(sample.value > 0.1);
}

#[test]
fn edge_response_sharp_edge_stronger_than_gradient() {
    let sharp = make_sharp_vertical_edge_image(256, 256);
    let gradient = make_gradient_image(256, 256);
    let sharp_sample = edge_response(&sharp);
    let gradient_sample = edge_response(&gradient);
    // The sharp edge has higher Sobel magnitudes than the
    // smooth gradient (more energy concentrated at the
    // boundary).
    assert!(sharp_sample.value > gradient_sample.value);
}

#[test]
fn edge_response_too_small_image_returns_zero() {
    let img = make_uniform_image(2, 2, 0.5);
    let sample = edge_response(&img);
    assert_eq!(sample.value, 0.0);
}

#[test]
fn edge_response_deterministic_for_same_input() {
    let img = make_sharp_vertical_edge_image(128, 128);
    let a = edge_response(&img);
    let b = edge_response(&img);
    assert_eq!(a.value, b.value);
}

#[test]
fn edge_response_value_is_non_negative() {
    let img = make_sharp_vertical_edge_image(64, 64);
    let sample = edge_response(&img);
    assert!(sample.value >= 0.0);
    assert!(sample.value.is_finite());
}
