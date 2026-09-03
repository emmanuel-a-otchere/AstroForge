//! Integration test for the Phase 9 vertical slice's Tauri commands:
//!
//!   scan_directory → classify_frame → fits::read_f32_image →
//!   mvp_pipeline::run_pipeline
//!
//! Mirrors the logic inside `pipeline_run_session` (src-tauri/src/main.rs)
//! so the command body can stay declarative while the heavy lifting
//! has a real test behind it. Runs end-to-end with synthetic FITS
//! files written to a tempdir — no external data, no network.

use astroforge_core::fits::{self, FitsHeader};
use astroforge_core::ingest::{self, FrameInfo, FrameType};
use astroforge_core::mvp_pipeline::{self, PipelineConfig, Verbosity};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn write_minimal_fits(path: &PathBuf, width: usize, height: usize, exptime: f64) {
    // Minimal valid FITS file: header (2880 bytes) + zeroed image data
    // rounded up to the FITS record size. Uses fits::write_header for
    // the header so the format matches the parser byte-for-byte.
    let mut header = FitsHeader::new();
    header.set("IMAGETYP", "LIGHT");
    header.set("EXPTIME", &exptime.to_string());
    header.set("NAXIS1", &width.to_string());
    header.set("NAXIS2", &height.to_string());

    let mut file = File::create(path).expect("create fits");
    fits::write_header(&header, &mut file).expect("write header");

    // Data: width * height f32 big-endian floats; zeroes are fine for
    // this test — we just want the pipeline to walk every stage.
    let pixel_count = width * height;
    let mut data = Vec::with_capacity(pixel_count * 4);
    for _ in 0..pixel_count {
        data.extend_from_slice(&0.0_f32.to_be_bytes());
    }
    // Round up to a FITS block boundary.
    let block_size: usize = 2880;
    let total = data.len();
    if total % block_size != 0 {
        data.resize(total + (block_size - total % block_size), 0);
    }
    file.write_all(&data).expect("data");
}

fn tmp_workspace(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    p.push(format!(
        "astroforge-pipeline-{name}-{}-{stamp}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("mkdir tmp");
    p
}

#[test]
fn ingest_classify_finds_lights_only() {
    let dir = tmp_workspace("classify");
    write_minimal_fits(&dir.join("light_001.fits"), 16, 16, 120.0);
    write_minimal_fits(&dir.join("light_002.fits"), 16, 16, 120.0);
    write_minimal_fits(&dir.join("dark_001.fits"), 16, 16, 120.0);

    let paths = ingest::scan_directory(&dir).expect("scan");
    assert_eq!(paths.len(), 3);

    let mut frames: Vec<FrameInfo> = Vec::new();
    for p in paths {
        let bytes = std::fs::read(&p).expect("read");
        frames.push(ingest::classify_frame(&p, &bytes));
    }
    let lights = frames
        .iter()
        .filter(|f| f.frame_type == FrameType::Light)
        .count();
    let darks = frames
        .iter()
        .filter(|f| f.frame_type == FrameType::Dark)
        .count();
    assert_eq!(lights, 2);
    assert_eq!(darks, 1);
}

#[test]
fn end_to_end_pipeline_walks_every_stage() {
    let dir = tmp_workspace("e2e");
    write_minimal_fits(&dir.join("light_001.fits"), 32, 32, 60.0);
    write_minimal_fits(&dir.join("light_002.fits"), 32, 32, 60.0);
    write_minimal_fits(&dir.join("light_003.fits"), 32, 32, 60.0);

    let paths = ingest::scan_directory(&dir).expect("scan");
    let mut frames = Vec::new();
    let mut light_paths = Vec::new();
    for p in &paths {
        let bytes = std::fs::read(p).expect("read");
        let f = ingest::classify_frame(p, &bytes);
        if f.frame_type == FrameType::Light {
            light_paths.push(p.clone());
        }
        frames.push(f);
    }
    assert_eq!(light_paths.len(), 3);

    // Mirror the Tauri command: read every light frame via the FITS
    // layer so we exercise the same path the IPC handler does.
    let mut calibrated = Vec::new();
    for p in &light_paths {
        let bytes = std::fs::read(p).expect("read fits");
        let header = fits::parse_header(&bytes).expect("parse header");
        let img = fits::read_f32_image(&bytes, &header).expect("read image");
        calibrated.push(img);
    }
    assert_eq!(calibrated.len(), 3);

    let manifest = ingest::build_manifest("test-session", dir.to_str().unwrap(), frames);
    let config = PipelineConfig {
        verbosity: Verbosity::Beginner,
        lights_only: true,
        ..PipelineConfig::default()
    };
    let result = mvp_pipeline::run_pipeline(&manifest, calibrated, &config);

    assert!(result.success);
    assert_eq!(result.report.frame_stats.lights, 3);
    // Three stages should be recorded: registration, stacking, stretching
    let stage_ids: Vec<String> = result
        .report
        .stage_parameters
        .iter()
        .map(|s| s.stage_id.clone())
        .collect();
    assert!(stage_ids.contains(&"registration".into()));
    assert!(stage_ids.contains(&"stacking".into()));
    assert!(stage_ids.contains(&"stretching".into()));
}
