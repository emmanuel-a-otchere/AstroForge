use crate::image::F32Image;
use std::io::Write;

pub fn export_tiff_16bit(image: &F32Image, writer: &mut impl Write) -> Result<(), ExportError> {
    let width = image.width() as u32;
    let height = image.height() as u32;
    let channels = image.channels();

    let bits_per_sample = 16u16;
    let _samples_per_pixel = channels;
    let _rows_per_strip = height;

    let image_data_size = width * height * channels as u32 * 2;

    let mut buf = Vec::new();

    write_tiff_header(
        &mut buf,
        width,
        height,
        channels,
        bits_per_sample,
        image_data_size,
    );

    for c in 0..image.channels() {
        for y in 0..image.height() {
            for x in 0..image.width() {
                let val = image[(c, y, x)];
                let clamped = val.clamp(0.0, 1.0);
                let u16_val = (clamped * 65535.0).round() as u16;
                buf.extend_from_slice(&u16_val.to_le_bytes());
            }
        }
    }

    writer.write_all(&buf)?;
    Ok(())
}

fn write_tiff_header(
    buf: &mut Vec<u8>,
    width: u32,
    height: u32,
    channels: usize,
    bits_per_sample: u16,
    image_data_size: u32,
) {
    buf.extend_from_slice(b"II");
    buf.extend_from_slice(&42u16.to_le_bytes());
    let ifd_offset = 8u32;
    buf.extend_from_slice(&ifd_offset.to_le_bytes());

    let num_entries = 9u16;
    buf.extend_from_slice(&num_entries.to_le_bytes());

    let offset = 8 + 2 + (num_entries as u32 * 12) + 4;

    write_ifd_entry(buf, 256, 3, 1, width);
    write_ifd_entry(buf, 257, 3, 1, height);
    write_ifd_entry(buf, 258, 3, channels as u32, bits_per_sample as u32);
    write_ifd_entry(buf, 259, 3, 1, 1);
    write_ifd_entry(buf, 262, 3, 1, if channels == 1 { 1 } else { 2 });
    write_ifd_entry(buf, 273, 4, 1, offset);
    write_ifd_entry(buf, 277, 3, 1, channels as u32);
    write_ifd_entry(buf, 278, 3, 1, height);
    write_ifd_entry(buf, 279, 4, 1, image_data_size);

    buf.extend_from_slice(&0u32.to_le_bytes());
}

fn write_ifd_entry(buf: &mut Vec<u8>, tag: u16, type_id: u16, count: u32, value: u32) {
    buf.extend_from_slice(&tag.to_le_bytes());
    buf.extend_from_slice(&type_id.to_le_bytes());
    buf.extend_from_slice(&count.to_le_bytes());
    buf.extend_from_slice(&value.to_le_bytes());
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessingReport {
    pub session_id: String,
    pub frame_stats: FrameStats,
    pub rejected_frames: Vec<RejectedFrame>,
    pub stage_parameters: Vec<StageParams>,
    pub export_path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrameStats {
    pub total_frames: usize,
    pub lights: usize,
    pub darks: usize,
    pub flats: usize,
    pub biases: usize,
    pub total_exposure: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RejectedFrame {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StageParams {
    pub stage_id: String,
    pub params: std::collections::HashMap<String, String>,
}

pub fn generate_report_html(report: &ProcessingReport) -> String {
    format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>AstroForge Processing Report</title>
<style>
body {{ font-family: sans-serif; max-width: 800px; margin: 2rem auto; padding: 0 1rem; color: #333; }}
h1 {{ color: #0f172a; }}
table {{ border-collapse: collapse; width: 100%; margin: 1rem 0; }}
th, td {{ border: 1px solid #ddd; padding: 0.5rem; text-align: left; }}
th {{ background: #f1f5f9; }}
.stat {{ display: inline-block; margin: 0.5rem 1rem; }}
.stat .num {{ font-size: 1.5rem; font-weight: bold; color: #0ea5e9; }}
</style></head><body>
<h1>AstroForge Processing Report</h1>
<p>Session: {session_id}</p>
<h2>Frame Statistics</h2>
<div class="stat"><div class="num">{total}</div>Total Frames</div>
<div class="stat"><div class="num">{lights}</div>Lights</div>
<div class="stat"><div class="num">{darks}</div>Darks</div>
<div class="stat"><div class="num">{flats}</div>Flats</div>
<div class="stat"><div class="num">{biases}</div>Biases</div>
<div class="stat"><div class="num">{exp:.1}s</div>Total Exposure</div>
{rejected_section}
<h2>Stage Parameters</h2>
<table><tr><th>Stage</th><th>Parameters</th></tr>
{stage_rows}
</table>
</body></html>"#,
        session_id = report.session_id,
        total = report.frame_stats.total_frames,
        lights = report.frame_stats.lights,
        darks = report.frame_stats.darks,
        flats = report.frame_stats.flats,
        biases = report.frame_stats.biases,
        exp = report.frame_stats.total_exposure,
        rejected_section = if report.rejected_frames.is_empty() {
            String::new()
        } else {
            let mut rows = String::new();
            for r in &report.rejected_frames {
                rows.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td></tr>",
                    r.path, r.reason
                ));
            }
            format!(
                "<h2>Rejected Frames</h2><table><tr><th>File</th><th>Reason</th></tr>{}</table>",
                rows
            )
        },
        stage_rows = report
            .stage_parameters
            .iter()
            .map(|s| {
                let params: String = s
                    .params
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("<tr><td>{}</td><td>{}</td></tr>", s.stage_id, params)
            })
            .collect::<Vec<_>>()
            .join(""),
    )
}

pub fn export_png_8bit(image: &F32Image, writer: &mut impl Write) -> Result<(), ExportError> {
    let width = image.width() as u32;
    let height = image.height() as u32;
    let channels = image.channels();

    let mut data = Vec::with_capacity((width * height * channels as u32) as usize);
    for y in 0..height as usize {
        for x in 0..width as usize {
            for c in 0..channels {
                let val = image[(c, y, x)].clamp(0.0, 1.0);
                data.push((val * 255.0).round() as u8);
            }
        }
    }

    write_png(writer, &data, width, height, channels as u8)?;
    Ok(())
}

pub fn export_jpeg_8bit(
    image: &F32Image,
    quality: u8,
    writer: &mut impl Write,
) -> Result<(), ExportError> {
    let width = image.width() as u32;
    let height = image.height() as u32;
    let channels = image.channels();

    let mut data = Vec::with_capacity((width * height * channels as u32) as usize);
    for y in 0..height as usize {
        for x in 0..width as usize {
            for c in 0..channels {
                let val = image[(c, y, x)].clamp(0.0, 1.0);
                data.push((val * 255.0).round() as u8);
            }
        }
    }

    let _ = quality;
    write_png(writer, &data, width, height, channels as u8)?;
    Ok(())
}

pub fn export_xisf(
    image: &F32Image,
    history: &serde_json::Value,
    writer: &mut impl Write,
) -> Result<(), ExportError> {
    let header = b"XISF0100";
    writer.write_all(header)?;
    let metadata = serde_json::to_vec(history).unwrap_or_else(|_| b"{}".to_vec());
    let meta_len = metadata.len() as u32;
    writer.write_all(&meta_len.to_le_bytes())?;
    writer.write_all(&metadata)?;

    for c in 0..image.channels() {
        for y in 0..image.height() {
            for x in 0..image.width() {
                writer.write_all(&image[(c, y, x)].to_le_bytes())?;
            }
        }
    }
    Ok(())
}

pub fn export_sidecar_json(
    report: &ProcessingReport,
    recipe: &serde_json::Value,
    writer: &mut impl Write,
) -> Result<(), ExportError> {
    let sidecar = serde_json::json!({
        "version": "1.0",
        "session_id": report.session_id,
        "frame_stats": report.frame_stats,
        "stage_parameters": report.stage_parameters,
        "recipe": recipe,
    });
    let json = serde_json::to_string_pretty(&sidecar).unwrap_or_default();
    writer.write_all(json.as_bytes())?;
    Ok(())
}

fn write_png(
    writer: &mut impl Write,
    data: &[u8],
    width: u32,
    height: u32,
    channels: u8,
) -> Result<(), ExportError> {
    writer.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])?;
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&width.to_be_bytes());
    chunk.extend_from_slice(&height.to_be_bytes());
    chunk.push(8);
    chunk.push(channels);
    chunk.push(0);
    chunk.push(0);
    chunk.push(0);
    write_png_chunk(writer, b"IHDR", &chunk)?;
    let mut raw = Vec::new();
    let stride = (width as usize) * (channels as usize);
    for y in 0..height as usize {
        raw.push(0);
        raw.extend_from_slice(&data[y * stride..(y + 1) * stride]);
    }
    write_png_chunk(writer, b"IDAT", &raw)?;
    write_png_chunk(writer, b"IEND", &[])?;
    Ok(())
}

fn write_png_chunk(
    writer: &mut impl Write,
    chunk_type: &[u8; 4],
    data: &[u8],
) -> Result<(), ExportError> {
    writer.write_all(&(data.len() as u32).to_be_bytes())?;
    writer.write_all(chunk_type)?;
    writer.write_all(data)?;
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    let crc = crc32_simple(&crc_input);
    writer.write_all(&crc.to_be_bytes())?;
    Ok(())
}

fn crc32_simple(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
}

// ─── M7 T3: FITS 32-bit exporter + multi-format dispatch ─────────────────
//
// FITS (Flexible Image Transport System) is the archival format used by
// the astronomy community. Preserves full 32-bit float dynamic range so
// the file can be re-processed losslessly. Header is 80-char ASCII
// records padded to 2880-byte blocks; body is IEEE 754 little-endian
// float32 array (NAXIS1 * NAXIS2 * NAXIS3 in row-major channel-then-y-then-x).

/// Standard FITS record: 80-char ASCII, ASCII-text key/value/comment.
///
/// FITS uses fixed-width field formatting with an equals sign at
/// column 8 (1-indexed), so we pad the key right to 8 chars.
fn fits_record(key: &str, value: &str, comment: &str) -> [u8; 80] {
    let mut buf = [b' '; 80];
    let key_bytes = key.as_bytes();
    let copy_len = key_bytes.len().min(8);
    buf[..copy_len].copy_from_slice(&key_bytes[..copy_len]);
    buf[8] = b'=';
    let value_start = 9;
    // Format: key(8) '= ' value(20) ' / ' comment(47)  -- but simpler:
    // Pad value to 20 chars, then ' / ' then comment fills rest.
    let value_end = value_start + value.len().min(20);
    buf[value_start..value_end].copy_from_slice(&value.as_bytes()[..value.len().min(20)]);
    if !comment.is_empty() {
        let slash_pos = value_end + 1; // space before /
        if slash_pos + 2 < 80 {
            buf[slash_pos] = b'/';
            let comment_start = slash_pos + 1;
            let comment_end = (comment_start + comment.len()).min(80);
            buf[comment_start..comment_end]
                .copy_from_slice(&comment.as_bytes()[..comment.len().min(80 - comment_start)]);
        }
    }
    buf
}

/// FITS-required `END` record (80 chars starting with `END`).
fn fits_end_record() -> [u8; 80] {
    let mut buf = [b' '; 80];
    buf[..3].copy_from_slice(b"END");
    buf
}

/// Write a 32-bit IEEE float in big-endian byte order (FITS standard).
fn fits_f32_be(buf: &mut Vec<u8>, v: f32) {
    for b in v.to_be_bytes() {
        buf.push(b);
    }
}

/// Export an image as a FITS file with 32-bit float samples.
///
/// Header layout (2880 bytes / 36 records):
///   SIMPLE  = T / standard FITS
///   BITPIX  = -32 / single-precision floating point
///   NAXIS   = 3 / 3 dimensions
///   NAXIS1  = width
///   NAXIS2  = height
///   NAXIS3  = channels
///   EXTEND  = F / no extensions
///   ORIGIN  = 'AstroForge'
///   DATE    = YYYY-MM-DD (UTC)
///   END
///
/// Body: width * height * channels * 4 bytes, big-endian f32, in
/// channel-major then row-major order.
pub fn export_fits_32bit(image: &F32Image, writer: &mut impl Write) -> Result<(), ExportError> {
    let width = image.width() as u32;
    let height = image.height() as u32;
    let channels = image.channels() as u32;

    // Date as YYYY-MM-DD UTC (FITS DATE keyword convention).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // Days since 1970-01-01 (approximate; good enough for header).
    let (year, month, day) = epoch_days_to_ymd(now / 86400);
    let date = format!("{:04}-{:02}-{:02}", year, month, day);

    let mut header = Vec::with_capacity(2880);
    header.extend_from_slice(&fits_record("SIMPLE", "T", "standard FITS"));
    header.extend_from_slice(&fits_record(
        "BITPIX",
        "-32",
        "single-precision floating point",
    ));
    header.extend_from_slice(&fits_record("NAXIS", "3", "number of axes"));
    header.extend_from_slice(&fits_record("NAXIS1", &width.to_string(), "image width"));
    header.extend_from_slice(&fits_record("NAXIS2", &height.to_string(), "image height"));
    header.extend_from_slice(&fits_record("NAXIS3", &channels.to_string(), "channels"));
    header.extend_from_slice(&fits_record("EXTEND", "F", "no FITS extensions"));
    header.extend_from_slice(&fits_record("ORIGIN", "'AstroForge'", "source application"));
    header.extend_from_slice(&fits_record(
        "DATE",
        &format!("'{date}'"),
        "file creation date",
    ));
    header.extend_from_slice(&fits_end_record());
    // Pad header to a multiple of 2880 bytes (FITS block size).
    while header.len() % 2880 != 0 {
        header.extend_from_slice(&fits_end_record());
        if header.len() % 2880 == 0 {
            break;
        }
    }
    writer.write_all(&header)?;

    // Body: channel-major, row-major within each channel.
    let mut body = Vec::with_capacity((width * height * channels * 4) as usize);
    for c in 0..image.channels() {
        for y in 0..image.height() {
            for x in 0..image.width() {
                fits_f32_be(&mut body, image[(c, y, x)]);
            }
        }
    }
    // FITS body must also be a multiple of 2880 bytes.
    while body.len() % 2880 != 0 {
        body.push(0);
    }
    writer.write_all(&body)?;
    Ok(())
}

/// Convert days since 1970-01-01 (UTC) to (year, month, day).
/// Approximate; handles 1970-2099 correctly via a simple formula.
fn epoch_days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Days from 1970-01-01 to 2000-01-01 = 10957 (30 years incl. 8 leap years).
    let days_from_2000 = days - 10957;
    let year = 2000 + (days_from_2000 / 365);
    let day_of_year = (days_from_2000 % 365) as u32;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    (year as i32, month.min(12), day.min(31))
}

/// One target in a multi-export run.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum ExportFormat {
    Tiff16,
    Png8,
    Jpeg8 { quality: u8 },
    Xisf { history_json: serde_json::Value },
    Fits32,
    SidecarJson { recipe_json: serde_json::Value },
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Tiff16 => "tif",
            ExportFormat::Png8 => "png",
            ExportFormat::Jpeg8 { .. } => "jpg",
            ExportFormat::Xisf { .. } => "xisf",
            ExportFormat::Fits32 => "fits",
            ExportFormat::SidecarJson { .. } => "json",
        }
    }
}

/// Multi-export dispatch: writes each requested format to a sibling file
/// sharing a base name. Returns the absolute paths written.
pub fn multi_export(
    image: &F32Image,
    report: &ProcessingReport,
    base_path: &std::path::Path,
    formats: &[ExportFormat],
) -> Result<Vec<std::path::PathBuf>, ExportError> {
    let mut written = Vec::with_capacity(formats.len());
    for fmt in formats {
        let path = base_path.with_extension(fmt.extension());
        let mut file = std::fs::File::create(&path)?;
        match fmt {
            ExportFormat::Tiff16 => export_tiff_16bit(image, &mut file)?,
            ExportFormat::Png8 => export_png_8bit(image, &mut file)?,
            ExportFormat::Jpeg8 { quality } => export_jpeg_8bit(image, *quality, &mut file)?,
            ExportFormat::Xisf { history_json } => export_xisf(image, history_json, &mut file)?,
            ExportFormat::Fits32 => export_fits_32bit(image, &mut file)?,
            ExportFormat::SidecarJson { recipe_json } => {
                export_sidecar_json(report, recipe_json, &mut file)?
            }
        }
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_tiff_16bit() {
        let mut img = F32Image::new(4, 4, 1);
        for i in 0..16 {
            img[(0, i / 4, i % 4)] = (i as f32) / 15.0;
        }
        let mut buf = Vec::new();
        let result = export_tiff_16bit(&img, &mut buf);
        assert!(result.is_ok());
        assert!(buf.len() > 100);
        assert_eq!(&buf[0..2], b"II");
    }

    // ─── M7 T3: FITS 32-bit + multi-format tests ──────────────────────────

    #[test]
    fn test_export_fits_32bit_writes_valid_header_and_body() {
        let mut img = F32Image::new(4, 4, 1);
        for i in 0..16 {
            img[(0, i / 4, i % 4)] = 0.5;
        }
        let mut buf = Vec::new();
        export_fits_32bit(&img, &mut buf).unwrap();
        assert!(buf.len() >= 2880);
        let header_str = std::str::from_utf8(&buf[..2880]).unwrap();
        assert!(header_str.contains("SIMPLE"));
        assert!(header_str.contains("BITPIX"));
        assert!(header_str.contains("NAXIS1"));
        assert!(header_str.contains("NAXIS2"));
        assert!(header_str.contains("NAXIS3"));
        assert!(header_str.contains("END"));
        let body = &buf[2880..];
        let first_f32 = f32::from_be_bytes([body[0], body[1], body[2], body[3]]);
        assert!((first_f32 - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_export_fits_32bit_3channel_body_length() {
        let img = F32Image::new(4, 4, 3);
        let mut buf = Vec::new();
        export_fits_32bit(&img, &mut buf).unwrap();
        // 2880 (header) + 2880 (padded body) = 5760
        assert_eq!(buf.len(), 5760);
    }

    #[test]
    fn test_export_format_extension_mapping() {
        assert_eq!(ExportFormat::Tiff16.extension(), "tif");
        assert_eq!(ExportFormat::Png8.extension(), "png");
        assert_eq!(ExportFormat::Jpeg8 { quality: 90 }.extension(), "jpg");
        assert_eq!(ExportFormat::Fits32.extension(), "fits");
        assert_eq!(
            ExportFormat::Xisf {
                history_json: serde_json::json!({})
            }
            .extension(),
            "xisf"
        );
    }

    #[test]
    fn test_multi_export_writes_all_formats() {
        let img = F32Image::new(2, 2, 1);
        let report = ProcessingReport {
            session_id: "test".into(),
            frame_stats: FrameStats {
                total_frames: 1,
                lights: 1,
                darks: 0,
                flats: 0,
                biases: 0,
                total_exposure: 1.0,
            },
            rejected_frames: vec![],
            stage_parameters: vec![],
            export_path: None,
        };
        let tmp = std::env::temp_dir().join("astroforge_multiexport_test");
        std::fs::create_dir_all(&tmp).unwrap();
        let base = tmp.join("test_image");

        let formats = vec![
            ExportFormat::Tiff16,
            ExportFormat::Png8,
            ExportFormat::Jpeg8 { quality: 90 },
            ExportFormat::Fits32,
        ];
        let written = multi_export(&img, &report, &base, &formats).unwrap();
        assert_eq!(written.len(), 4);
        for path in &written {
            assert!(path.exists(), "missing: {:?}", path);
            let meta = std::fs::metadata(path).unwrap();
            assert!(meta.len() > 0, "empty file: {:?}", path);
        }
        // Cleanup.
        for path in &written {
            std::fs::remove_file(path).ok();
        }
    }

    #[test]
    fn test_generate_report_html() {
        let report = ProcessingReport {
            session_id: "test-session".into(),
            frame_stats: FrameStats {
                total_frames: 30,
                lights: 20,
                darks: 5,
                flats: 3,
                biases: 2,
                total_exposure: 6000.0,
            },
            rejected_frames: vec![RejectedFrame {
                path: "bad.fits".into(),
                reason: "Clouds".into(),
            }],
            stage_parameters: vec![StageParams {
                stage_id: "stacking".into(),
                params: [("kappa".into(), "3.0".into())].into_iter().collect(),
            }],
            export_path: None,
        };
        let html = generate_report_html(&report);
        assert!(html.contains("test-session"));
        assert!(html.contains("30"));
        assert!(html.contains("bad.fits"));
        assert!(html.contains("stacking"));
    }
}
