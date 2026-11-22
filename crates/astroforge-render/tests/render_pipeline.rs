//! P1 CPU-path tests: decode dispatch, tone mapping, color
//! conversion, and display preparation — over the synthetic frame.

use astroforge_core::image::F32Image;
use astroforge_render::{decode_image, demo, prepare_display, to_color_image, ToneMap};

#[test]
fn synthetic_frame_is_linear_and_dark() {
    let img = demo::synthetic_deep_sky(320, 240);
    assert_eq!((img.width(), img.height(), img.channels()), (320, 240, 3));
    let mean: f64 = img.iter().map(|v| *v as f64).sum::<f64>() / img.len() as f64;
    assert!(
        mean < 0.2,
        "synthetic field should be mostly dark so AutoStretch matters (mean={mean})"
    );
}

#[test]
fn autostretch_lifts_the_field_into_display_range() {
    let img = demo::synthetic_deep_sky(160, 120);
    let (stretched, params) = ToneMap::AutoStretch.apply(&img);
    // Star fields keep a low MEAN even after stretch (most pixels are
    // sky); assert on the distribution instead: highlights lift well
    // into display range and a visible fraction clears background.
    let max = stretched.iter().copied().fold(0.0_f32, f32::max);
    let lifted = stretched.iter().filter(|v| **v > 0.1).count();
    assert!(
        max > 0.4,
        "auto stretch should lift highlights into display range (max={max})"
    );
    assert!(
        lifted > stretched.len() / 100,
        "stretched field should have visible structure (>1% of pixels over 0.1, got {lifted})"
    );
    // Params application reproduces the same result bit-for-bit-ish.
    let (reapplied, _) = ToneMap::Params(params).apply(&img);
    let max_diff: f32 = stretched
        .iter()
        .zip(reapplied.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max);
    assert!(
        max_diff < 1e-4,
        "stored-params reapplication diverged (max_diff={max_diff})"
    );
}

/// Encode a valid 16-bit RGB TIFF with the `tiff` crate.
///
/// astroforge-core's `export_tiff_16bit` writes an invalid
/// BitsPerSample IFD entry for RGB (inline value instead of an offset
/// array), which its own `from_tiff_bytes` rejects — tracked as a
/// core finding, not fixed here. This test therefore verifies the
/// decoder against standards-compliant TIFFs.
fn encode_valid_tiff16_rgb(width: u32, height: u32, shading: f32) -> Vec<u8> {
    let mut data = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            for c in 0..3u32 {
                let v = ((x + c * 7) as f32 / (width as f32 + 14.0) * shading).clamp(0.0, 1.0);
                data.push((v * 65535.0) as u16);
            }
        }
    }
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut enc = tiff::encoder::TiffEncoder::new(&mut out).expect("encoder");
        let mut img = enc
            .new_image::<tiff::encoder::colortype::RGB16>(width, height)
            .expect("image");
        img.write_data(&data).expect("write data");
    }
    out.into_inner()
}

#[test]
fn decodes_valid_tiff16_rgb_via_magic_bytes() {
    let bytes = encode_valid_tiff16_rgb(32, 24, 1.0);
    let decoded = decode_image(&bytes).expect("decode via magic bytes");
    assert_eq!(
        (decoded.width(), decoded.height(), decoded.channels()),
        (32, 24, 3)
    );

    // Gradient endpoint: pixel (31, y, channel 0) should be near
    // 31/46 of full scale.
    let v = decoded[(0, 0, 31)];
    let expected = 31.0 / (32.0 + 14.0);
    assert!(
        (v - expected).abs() < 0.01,
        "pixel value {v} vs expected {expected}"
    );
}

#[test]
fn decode_rejects_unknown_magic() {
    let err = decode_image(b"not an image at all").expect_err("must reject");
    assert!(matches!(
        err,
        astroforge_core::decoders::ImageDecodeError::Unsupported(_)
    ));
}

#[test]
fn color_image_handles_mono_and_rgb() {
    let mut mono = F32Image::new(4, 2, 1);
    for y in 0..2 {
        for x in 0..4 {
            mono[(0, y, x)] = (x as f32) / 4.0;
        }
    }
    let ci = to_color_image(&mono);
    assert_eq!(ci.size, [4, 2]);
    let px = ci.pixels[3]; // x=3 -> 0.75 -> 191
    assert_eq!(px, egui::Color32::from_rgb(191, 191, 191));

    let rgb = demo::synthetic_deep_sky(6, 4);
    let ci = to_color_image(&rgb);
    assert_eq!(ci.size, [6, 4]);
    assert_eq!(ci.pixels.len(), 24);
}

#[test]
fn prepare_display_downsamples_only_when_needed() {
    let img = demo::synthetic_deep_sky(200, 100);
    let kept = prepare_display(&img, 512);
    assert_eq!((kept.width(), kept.height()), (200, 100));

    let shrunk = prepare_display(&img, 64);
    assert!(shrunk.width() <= 64 && shrunk.height() <= 64);
    assert!(shrunk.width() > 0 && shrunk.height() > 0);
}
