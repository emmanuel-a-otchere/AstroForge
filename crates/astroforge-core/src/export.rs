use crate::image::F32Image;
use std::io::Write;

pub fn export_tiff_16bit(image: &F32Image, writer: &mut impl Write) -> Result<(), ExportError> {
    let width = image.width() as u32;
    let height = image.height() as u32;
    let channels = image.channels() as u16;

    let bits_per_sample = 16u16;
    let samples_per_pixel = channels;
    let rows_per_strip = height;

    let image_data_size = (width * height * channels as u32 * 2) as u32;

    let mut buf = Vec::new();

    write_tiff_header(&mut buf, width, height, channels, bits_per_sample, image_data_size);

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

    let mut offset = 8 + 2 + (num_entries as u32 * 12) + 4;

    write_ifd_entry(buf, 256, 3, 1, width as u32);
    write_ifd_entry(buf, 257, 3, 1, height as u32);
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
            let rows: String = report
                .rejected_frames
                .iter()
                .map(|r| format!("<tr><td>{}</td><td>{}</td></tr>", r.path, r.reason))
                .collect();
            format!("<h2>Rejected Frames</h2><table><tr><th>File</th><th>Reason</th></tr>{}</table>", rows)
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

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
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
