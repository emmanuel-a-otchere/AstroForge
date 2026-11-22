//! Artifact decoding — magic-byte dispatch over the core decoders.

use astroforge_core::decoders::ImageDecodeError;
use astroforge_core::image::F32Image;

/// Decode artifact bytes (TIFF / PNG / JPEG / FITS) into the crate's
/// linear-light `F32Image` (channels × height × width, values
/// normalized to [0, 1] for integer formats). Format is detected from
/// magic bytes so callers don't need to track extensions — artifact
/// records are the source of truth for *what* a file is, and they
/// don't always carry one.
pub fn decode_image(bytes: &[u8]) -> Result<F32Image, ImageDecodeError> {
    if bytes.len() < 4 {
        return Err(ImageDecodeError::Unsupported(
            "buffer too small to contain an image".into(),
        ));
    }

    if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        F32Image::from_tiff_bytes(bytes)
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        F32Image::from_png_bytes(bytes)
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        F32Image::from_jpeg_bytes(bytes)
    } else if bytes.starts_with(b"SIMPLE") {
        F32Image::from_fits_bytes(bytes)
    } else {
        Err(ImageDecodeError::Unsupported(format!(
            "unrecognized image magic: {:?}",
            &bytes[..4.min(bytes.len())]
        )))
    }
}
