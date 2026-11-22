//! Color conversion + display preparation (CPU path).

use astroforge_core::image::F32Image;
use egui::Color32;

/// Convert a display-ready `F32Image` (values in [0, 1] after tone
/// mapping) into an egui texture. Mono images are replicated across
/// RGB; 3-channel images map channel-plane data to interleaved
/// pixels; other channel counts use the first three planes.
///
/// Values are scaled linearly to 0..255, matching the web canvas
/// behavior (the stretch math already produced display-referred
/// values; no extra sRGB encode on top).
pub fn to_color_image(image: &F32Image) -> egui::ColorImage {
    let (w, h, c) = (image.width(), image.height(), image.channels());
    let mut pixels = Vec::with_capacity(w * h);
    let at = |x: usize, y: usize, ch: usize| -> u8 {
        let v = image[(ch.min(c - 1), y, x)].clamp(0.0, 1.0);
        (v * 255.0).round() as u8
    };
    for y in 0..h {
        for x in 0..w {
            match c {
                1 => {
                    let l = at(x, y, 0);
                    pixels.push(Color32::from_rgb(l, l, l));
                }
                _ => pixels.push(Color32::from_rgb(at(x, y, 0), at(x, y, 1), at(x, y, 2))),
            }
        }
    }
    egui::ColorImage::new([w, h], pixels)
}

/// Downsample for display if the source exceeds `max_dim` on either
/// axis (box-sampling via core, preserving signal level). The canvas
/// zooms the display image; panning/zooming at full sensor
/// resolution lands with the wgpu slice.
pub fn prepare_display(image: &F32Image, max_dim: u32) -> F32Image {
    let largest = image.width().max(image.height()) as f64;
    let max_dim = max_dim as f64;
    if largest > max_dim {
        image.downsample_box(max_dim / largest)
    } else {
        image.clone()
    }
}
