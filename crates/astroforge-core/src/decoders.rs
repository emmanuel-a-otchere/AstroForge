//! CR-05 P4 slice 5.1 — real TIFF + FITS pixel decoders.
//!
//! Replaces the aspirational `from_tiff_bytes` / `from_fits_bytes`
//! references with working implementations using the `tiff` and
//! `fitsrs` crates. Both produce `F32Image` in the crate's
//! `(channels, height, width)` layout with values normalized to
//! the `[0.0, 1.0]` range (integer formats) or left as-is
//! (floating-point formats, already in whatever range the source
//! data carries).
//!
//! **Error handling**: every decode failure maps to a descriptive
//! `ImageDecodeError`; nothing panics.

use crate::image::F32Image;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageDecodeError {
    #[error("TIFF decode: {0}")]
    Tiff(String),
    #[error("FITS decode: {0}")]
    Fits(String),
    #[error("unsupported image format: {0}")]
    Unsupported(String),
    #[error("I/O: {0}")]
    Io(String),
}

impl F32Image {
    /// CR-05 P4 slice 5.1 — decode a TIFF image from bytes.
    ///
    /// Supports 8/16/32-bit unsigned integer, 32/64-bit float, and
    /// multi-channel (RGB) images. Integer formats are normalized to
    /// [0.0, 1.0] by dividing by the format's maximum; floats are
    /// passed through unchanged.
    pub fn from_tiff_bytes(bytes: &[u8]) -> Result<Self, ImageDecodeError> {
        use tiff::decoder::Decoder;
        use tiff::ColorType;

        let cursor = std::io::Cursor::new(bytes);
        let mut decoder =
            Decoder::new(cursor).map_err(|e| ImageDecodeError::Tiff(format!("open: {e}")))?;

        let (width, height) = decoder
            .dimensions()
            .map_err(|e| ImageDecodeError::Tiff(format!("dimensions: {e}")))?;

        let color_type = decoder
            .colortype()
            .map_err(|e| ImageDecodeError::Tiff(format!("colortype: {e}")))?;

        // Determine channel count from color type.
        let channels = match color_type {
            ColorType::Gray(_) => 1,
            ColorType::RGB(_) | ColorType::YCbCr(_) => 3,
            ColorType::RGBA(_) | ColorType::CMYK(_) => 4,
            ColorType::GrayA(_) => 2,
            other => {
                return Err(ImageDecodeError::Tiff(format!(
                    "unsupported color type: {other:?}"
                )))
            }
        };

        // Read the image into raw samples.
        let raw = decoder
            .read_image()
            .map_err(|e| ImageDecodeError::Tiff(format!("read: {e}")))?;

        // Convert to f32 with normalization.
        let mut data = ndarray::Array3::<f32>::zeros((channels, height as usize, width as usize));

        match raw {
            tiff::decoder::DecodingResult::U8(samples) => {
                let inv_max = 1.0 / 255.0;
                let stride = channels; // samples per pixel
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        let base = (y * width as usize + x) * stride;
                        for c in 0..channels {
                            data[(c, y, x)] = samples[base + c] as f32 * inv_max;
                        }
                    }
                }
            }
            tiff::decoder::DecodingResult::U16(samples) => {
                let inv_max = 1.0 / 65535.0;
                let stride = channels;
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        let base = (y * width as usize + x) * stride;
                        for c in 0..channels {
                            data[(c, y, x)] = samples[base + c] as f32 * inv_max;
                        }
                    }
                }
            }
            tiff::decoder::DecodingResult::U32(samples) => {
                let inv_max = 1.0 / u32::MAX as f64;
                let stride = channels;
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        let base = (y * width as usize + x) * stride;
                        for c in 0..channels {
                            data[(c, y, x)] = (samples[base + c] as f64 * inv_max) as f32;
                        }
                    }
                }
            }
            tiff::decoder::DecodingResult::F32(samples) => {
                let stride = channels;
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        let base = (y * width as usize + x) * stride;
                        for c in 0..channels {
                            data[(c, y, x)] = samples[base + c];
                        }
                    }
                }
            }
            tiff::decoder::DecodingResult::F64(samples) => {
                let stride = channels;
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        let base = (y * width as usize + x) * stride;
                        for c in 0..channels {
                            data[(c, y, x)] = samples[base + c] as f32;
                        }
                    }
                }
            }
            other => {
                return Err(ImageDecodeError::Tiff(format!(
                    "unsupported sample type: {other:?}"
                )))
            }
        }

        Ok(F32Image::from(data))
    }

    /// CR-05 P4 slice 5.1 — decode a FITS image from bytes.
    ///
    /// Supports BITPIX = -64 (f64), -32 (f32), 16 (i16), 8 (u8) and
    /// NAXIS = 2 (mono) or NAXIS = 3 (multi-channel).
    ///
    /// FITS data is big-endian; `fitsrs` handles the byte swap
    /// internally. Integer formats are normalized to [0.0, 1.0].
    pub fn from_fits_bytes(bytes: &[u8]) -> Result<Self, ImageDecodeError> {
        use fitsrs::hdu::HDU;
        use fitsrs::Fits;

        let cursor = std::io::Cursor::new(bytes);
        let mut fits = Fits::from_reader(cursor);

        // Walk HDUs; the first one with NAXIS >= 2 is our image.
        while let Some(hdu_result) = fits.next() {
            let hdu = hdu_result.map_err(|e| ImageDecodeError::Fits(format!("hdu parse: {e}")))?;

            // We only care about image HDUs (primary or extension).
            let image_hdu = match hdu {
                HDU::Primary(h) => h,
                HDU::XImage(h) => h,
                _ => continue,
            };

            let header = image_hdu.get_header();
            let xtension = header.get_xtension();

            let naxis = xtension.get_naxis();
            if naxis.len() < 2 {
                continue;
            }

            let _bitpix = xtension.get_bitpix(); // unused: we infer from the data variant
            let naxis1 = naxis[0] as usize; // width
            let naxis2 = naxis[1] as usize; // height
            let channels = if naxis.len() >= 3 {
                naxis[2] as usize
            } else {
                1
            };

            // Read the data.
            let data = fits.get_data(&image_hdu);
            let pixels = data.pixels();

            let n_pixels = naxis1 * naxis2 * channels;
            let mut flat: Vec<f64> = Vec::with_capacity(n_pixels);

            match pixels {
                fitsrs::hdu::data::image::Pixels::U8(it) => {
                    let inv = 1.0 / u8::MAX as f64;
                    for v in it {
                        flat.push(v as f64 * inv);
                    }
                }
                fitsrs::hdu::data::image::Pixels::I16(it) => {
                    let inv = 1.0 / i16::MAX as f64;
                    for v in it {
                        flat.push(v as f64 * inv);
                    }
                }
                fitsrs::hdu::data::image::Pixels::I32(it) => {
                    let inv = 1.0 / i32::MAX as f64;
                    for v in it {
                        flat.push(v as f64 * inv);
                    }
                }
                fitsrs::hdu::data::image::Pixels::I64(it) => {
                    let inv = 1.0 / i64::MAX as f64;
                    for v in it {
                        flat.push(v as f64 * inv);
                    }
                }
                fitsrs::hdu::data::image::Pixels::F32(it) => {
                    for v in it {
                        flat.push(v as f64);
                    }
                }
                fitsrs::hdu::data::image::Pixels::F64(it) => {
                    for v in it {
                        flat.push(v);
                    }
                }
            }

            if flat.len() != n_pixels {
                return Err(ImageDecodeError::Fits(format!(
                    "pixel count mismatch: header says {n_pixels}, data has {}",
                    flat.len()
                )));
            }

            // FITS stores data in row-major order: for NAXIS=3,
            // axis order is (NAXIS1=width, NAXIS2=height, NAXIS3=channel).
            // We need (channels, height, width).
            let mut out = ndarray::Array3::<f32>::zeros((channels, naxis2, naxis1));
            for c in 0..channels {
                for y in 0..naxis2 {
                    for x in 0..naxis1 {
                        let src_idx = c * (naxis1 * naxis2) + y * naxis1 + x;
                        out[(c, y, x)] = flat[src_idx] as f32;
                    }
                }
            }

            return Ok(F32Image::from(out));
        }

        Err(ImageDecodeError::Fits(
            "no HDU with NAXIS >= 2 found".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal grayscale TIFF in memory using the `tiff`
    /// crate's encoder, then decode it. Asserts dimensions and
    /// normalized values.
    #[test]
    fn tiff_round_trip_u8_gray() {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut encoder = tiff::encoder::TiffEncoder::new(&mut buf).unwrap();
            let img_data: Vec<u8> = vec![0, 127, 200, 255];
            encoder
                .write_image::<tiff::encoder::colortype::Gray8>(2, 2, &img_data)
                .unwrap();
        }
        let img = F32Image::from_tiff_bytes(buf.get_ref()).unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
        assert_eq!(img.channels(), 1);
        assert!((img[(0, 0, 0)] - 0.0).abs() < 1e-6);
        assert!((img[(0, 0, 1)] - 127.0 / 255.0).abs() < 1e-6);
        assert!((img[(0, 1, 0)] - 200.0 / 255.0).abs() < 1e-6);
        assert!((img[(0, 1, 1)] - 1.0).abs() < 1e-6);
    }

    /// Build a minimal RGB TIFF (8-bit, 3 channels) via the encoder.
    #[test]
    fn tiff_round_trip_u8_rgb() {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut encoder = tiff::encoder::TiffEncoder::new(&mut buf).unwrap();
            let img_data: Vec<u8> = vec![
                255, 0, 0, // red
                0, 255, 0, // green
                0, 0, 255, // blue
                255, 255, 255, // white
            ];
            encoder
                .write_image::<tiff::encoder::colortype::RGB8>(2, 2, &img_data)
                .unwrap();
        }
        let img = F32Image::from_tiff_bytes(buf.get_ref()).unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
        assert_eq!(img.channels(), 3);
        // First pixel: red = 1.0, green = 0.0, blue = 0.0
        assert!((img[(0, 0, 0)] - 1.0).abs() < 1e-6);
        assert!((img[(1, 0, 0)] - 0.0).abs() < 1e-6);
        assert!((img[(2, 0, 0)] - 0.0).abs() < 1e-6);
    }

    /// Build a minimal FITS file (BITPIX=-32, NAXIS=2) and decode.
    #[test]
    fn fits_round_trip_f32_mono() {
        let width = 2;
        let height = 2;
        let n_pixels = width * height;
        let data_bytes = n_pixels * 4; // f32 = 4 bytes

        // Build header cards (80 chars each).
        let mut cards = String::new();
        cards.push_str(&format!("{:80}", "SIMPLE  =                    T"));
        cards.push_str(&format!("{:80}", "BITPIX  =                  -32"));
        cards.push_str(&format!("{:80}", "NAXIS   =                    2"));
        cards.push_str(&format!("{:80}", format!("NAXIS1  = {:>20}", width)));
        cards.push_str(&format!("{:80}", format!("NAXIS2  = {:>20}", height)));
        cards.push_str(&format!("{:80}", "END"));
        // Pad header to 2880 bytes.
        while !cards.len().is_multiple_of(2880) {
            cards.push(' ');
        }

        let mut buf = cards.into_bytes();
        // Data: big-endian f32 values [1.0, 0.5, 0.25, 0.75]
        let values: [f32; 4] = [1.0, 0.5, 0.25, 0.75];
        for v in values {
            buf.extend_from_slice(&v.to_be_bytes());
        }
        // Pad data block to 2880 bytes.
        let header_blocks = (buf.len() - data_bytes) / 2880;
        let total_needed = (header_blocks + 1) * 2880;
        while buf.len() < total_needed {
            buf.push(0);
        }

        let img = F32Image::from_fits_bytes(&buf).unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
        assert_eq!(img.channels(), 1);
        assert!((img[(0, 0, 0)] - 1.0).abs() < 1e-6);
        assert!((img[(0, 0, 1)] - 0.5).abs() < 1e-6);
        assert!((img[(0, 1, 0)] - 0.25).abs() < 1e-6);
        assert!((img[(0, 1, 1)] - 0.75).abs() < 1e-6);
    }
}
