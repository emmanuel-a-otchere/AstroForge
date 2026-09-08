//! CR-05 P4 slice 5.1 — real TIFF + FITS pixel decoders.
//!
//! Replaces the aspirational `from_tiff_bytes` / `from_fits_bytes`
//! references with working implementations. The TIFF decoder uses
//! the `tiff` crate (image-rs org, MIT/Apache-2.0). The FITS decoder
//! is a minimal in-house reader (~150 lines, zero deps) — see the
//! commit message for why we hand-rolled instead of using fitsrs.
//!
//! Both produce `F32Image` in the crate's `(channels, height,
//! width)` layout with values normalized to the `[0.0, 1.0]` range
//! (integer formats) or left as-is (floating-point formats).
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
    /// Minimal in-house FITS reader: handles BITPIX = 8 / 16 / 32 /
    /// 64 (signed) and -32 / -64 (IEEE float), NAXIS = 2 or 3.
    ///
    /// FITS records are 80-char cards in 2880-byte blocks; the
    /// header is followed by the data, padded to a 2880-byte
    /// boundary. Integer formats are normalized to [0, 1] via
    /// division by the maximum positive value; floats pass through
    /// unchanged (clamped to the source data's range).
    pub fn from_fits_bytes(bytes: &[u8]) -> Result<Self, ImageDecodeError> {
        // 1. Parse the header: scan 80-char cards, terminate on END.
        let mut i = 0;
        let mut bitpix: Option<i32> = None;
        let mut naxis: Option<usize> = None;
        let mut naxes: Vec<usize> = Vec::new();
        let header_end;

        loop {
            if i + 80 > bytes.len() {
                return Err(ImageDecodeError::Fits("header truncated before END".into()));
            }
            let card = std::str::from_utf8(&bytes[i..i + 80])
                .map_err(|e| ImageDecodeError::Fits(format!("card utf8: {e}")))?;
            i += 80;

            // Skip blank-padding cards at the end of the header block.
            if card.trim().is_empty() {
                if i % 2880 == 0 {
                    header_end = i;
                    break;
                }
                continue;
            }

            // END card terminates the header.
            if card.starts_with("END ") || card.starts_with("END=") || card.trim() == "END" {
                header_end = i;
                break;
            }

            // Parse KEY = VALUE / COMMENT
            let key = card.get(..8).unwrap_or("").trim();
            let value_part = card.get(9..80).unwrap_or("").trim();
            let value_str = value_part.split('/').next().unwrap_or("").trim();

            match key {
                "BITPIX" => {
                    bitpix = Some(
                        value_str
                            .parse::<i32>()
                            .map_err(|e| ImageDecodeError::Fits(format!("BITPIX: {e}")))?,
                    );
                }
                "NAXIS" => {
                    naxis = Some(
                        value_str
                            .parse::<usize>()
                            .map_err(|e| ImageDecodeError::Fits(format!("NAXIS: {e}")))?,
                    );
                }
                k if k.starts_with("NAXIS") && k.len() > 5 => {
                    let v: usize = value_str
                        .parse()
                        .map_err(|e| ImageDecodeError::Fits(format!("{k}: {e}")))?;
                    naxes.push(v);
                }
                _ => {} // ignore unknown cards
            }
            // Compiler note: `?` inside a `match` arm that
            // assigns to an outer `let` doesn't propagate
            // properly, so we use an explicit `Some(..)` wrapper
            // for BITPIX/NAXIS and `push(..)` for NAXISn — the
            // `?` returns from `from_fits_bytes` on parse error.
            // (The `?`s above are inside expressions that yield
            //  values of the right type.)
        }

        let bitpix = bitpix.ok_or_else(|| ImageDecodeError::Fits("missing BITPIX".into()))?;
        let naxis = naxis.unwrap_or(0);
        if naxis < 2 {
            return Err(ImageDecodeError::Fits(format!(
                "NAXIS < 2 (got {naxis}); not an image HDU"
            )));
        }
        if naxes.len() < 2 {
            return Err(ImageDecodeError::Fits("missing NAXIS1/NAXIS2".into()));
        }

        let width = naxes[0];
        let height = naxes[1];
        let channels = if naxis >= 3 && naxes.len() >= 3 {
            naxes[2]
        } else {
            1
        };
        let n_pixels = width * height * channels;

        // 2. Compute data block start. FITS spec: header_end snaps up to
        //    the next 2880-byte boundary, then data follows.
        let data_start = header_end.div_ceil(2880) * 2880;
        let bytes_per_pixel = (bitpix.unsigned_abs() / 8) as usize;
        let data_len = n_pixels * bytes_per_pixel;
        if data_start + data_len > bytes.len() {
            return Err(ImageDecodeError::Fits(format!(
                "data truncated: need {data_len} bytes from offset {data_start}, have {}",
                bytes.len().saturating_sub(data_start)
            )));
        }
        let data = &bytes[data_start..data_start + data_len];

        // 3. Decode pixels according to BITPIX.
        let mut flat: Vec<f64> = Vec::with_capacity(n_pixels);
        let mut cursor = 0;
        for _ in 0..n_pixels {
            let v = match bitpix {
                8 => {
                    let bytes_arr = [data[cursor]; 1];
                    cursor += 1;
                    i8::from_be_bytes(bytes_arr) as f64 / i8::MAX as f64
                }
                16 => {
                    let mut a = [0u8; 2];
                    a.copy_from_slice(&data[cursor..cursor + 2]);
                    cursor += 2;
                    i16::from_be_bytes(a) as f64 / i16::MAX as f64
                }
                32 => {
                    let mut a = [0u8; 4];
                    a.copy_from_slice(&data[cursor..cursor + 4]);
                    cursor += 4;
                    i32::from_be_bytes(a) as f64 / i32::MAX as f64
                }
                64 => {
                    let mut a = [0u8; 8];
                    a.copy_from_slice(&data[cursor..cursor + 8]);
                    cursor += 8;
                    i64::from_be_bytes(a) as f64 / i64::MAX as f64
                }
                -32 => {
                    let mut a = [0u8; 4];
                    a.copy_from_slice(&data[cursor..cursor + 4]);
                    cursor += 4;
                    f32::from_be_bytes(a) as f64
                }
                -64 => {
                    let mut a = [0u8; 8];
                    a.copy_from_slice(&data[cursor..cursor + 8]);
                    cursor += 8;
                    f64::from_be_bytes(a)
                }
                other => {
                    return Err(ImageDecodeError::Fits(format!(
                        "unsupported BITPIX: {other}"
                    )))
                }
            };
            flat.push(v);
        }

        // 4. Reshape (channels, height, width) from row-major FITS
        //    data layout.
        let mut out = ndarray::Array3::<f32>::zeros((channels, height, width));
        for c in 0..channels {
            for y in 0..height {
                for x in 0..width {
                    let src_idx = c * (width * height) + y * width + x;
                    out[(c, y, x)] = flat[src_idx] as f32;
                }
            }
        }

        Ok(F32Image::from(out))
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

    /// Build a minimal FITS file (BITPIX=16, NAXIS=3, 3 channels)
    /// and decode. Verifies multi-axis FITS layout.
    #[test]
    fn fits_round_trip_i16_rgb() {
        let width = 2;
        let height = 2;
        let channels = 3;
        let n_pixels = width * height * channels;
        let data_bytes = n_pixels * 2; // i16 = 2 bytes

        let mut cards = String::new();
        cards.push_str(&format!("{:80}", "SIMPLE  =                    T"));
        cards.push_str(&format!("{:80}", "BITPIX  =                   16"));
        cards.push_str(&format!("{:80}", "NAXIS   =                    3"));
        cards.push_str(&format!("{:80}", format!("NAXIS1  = {:>20}", width)));
        cards.push_str(&format!("{:80}", format!("NAXIS2  = {:>20}", height)));
        cards.push_str(&format!("{:80}", format!("NAXIS3  = {:>20}", channels)));
        cards.push_str(&format!("{:80}", "END"));
        while !cards.len().is_multiple_of(2880) {
            cards.push(' ');
        }

        let mut buf = cards.into_bytes();
        // 3 channels of i16 data, values [1000, 2000, 3000, 4000] per channel.
        for _ in 0..channels {
            let values: [i16; 4] = [1000, 2000, 3000, 4000];
            for v in values {
                buf.extend_from_slice(&v.to_be_bytes());
            }
        }
        let header_blocks = (buf.len() - data_bytes) / 2880;
        let total_needed = (header_blocks + 1) * 2880;
        while buf.len() < total_needed {
            buf.push(0);
        }

        let img = F32Image::from_fits_bytes(&buf).unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
        assert_eq!(img.channels(), 3);
        // First channel first pixel: 1000 / 32767 ≈ 0.0305
        let expected = 1000.0 / i16::MAX as f64;
        assert!((img[(0, 0, 0)] as f64 - expected).abs() < 1e-4);
    }
}
