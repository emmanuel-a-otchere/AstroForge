//! AstroForge MVP CLI — runs the end-to-end deep-sky pipeline on a
//! folder of FITS frames and exports a 16-bit TIFF.
//!
//! Phase 7 MVP close-out (issue #42). The Tauri UI shell (`src-tauri/`)
//! remains the primary user-facing path; this binary exists so the
//! pipeline can be driven headlessly and exercised by scripted smoke
//! tests (issue #43, #44) and the 30-frame memory test (issue #45).
//!
//! Usage:
//!   astroforge <source_dir> <output.tif>
//!
//! Exit codes:
//!   0  pipeline ran successfully and the output file exists
//!   1  bad arguments / IO failure
//!   2  no light frames were found in the source folder
//!   3  pipeline ran but did not produce a preview (rare)
//!   4  output TIFF could not be written

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use astroforge_core::export::export_tiff_16bit;
use astroforge_core::fits;
use astroforge_core::ingest::{self, FrameInfo, FrameType};
use astroforge_core::mvp_pipeline::{self, PipelineConfig};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: astroforge <source_dir> <output.tif>");
        std::process::exit(1);
    }

    let source_dir = PathBuf::from(&args[1]);
    let output_path = PathBuf::from(&args[2]);

    match run(&source_dir, &output_path) {
        Ok(report) => {
            println!("OK {}", serde_json::to_string(&report).unwrap_or_default());
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("FAIL: {:#}", e);
            // Map errors to the documented exit codes.
            let msg = format!("{}", e);
            let code = if msg.contains("no light frames") {
                2
            } else if msg.contains("no preview") {
                3
            } else if msg.contains("output") {
                4
            } else {
                1
            };
            std::process::exit(code);
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct CliReport {
    source_dir: String,
    output_path: String,
    elapsed_ms: u64,
    lights: usize,
    darks: usize,
    flats: usize,
    biases: usize,
    total_exposure_s: f64,
    stages: Vec<StageInfo>,
    preview_width: usize,
    preview_height: usize,
    output_bytes: u64,
}

#[derive(Debug, serde::Serialize)]
struct StageInfo {
    id: String,
    params: std::collections::BTreeMap<String, String>,
}

fn run(source_dir: &PathBuf, output_path: &PathBuf) -> Result<CliReport> {
    let started = Instant::now();

    // 1. Ingest: scan the directory and classify every FITS frame.
    log::info!("scanning {}", source_dir.display());
    let paths = ingest::scan_directory(source_dir)
        .with_context(|| format!("scan_directory failed for {}", source_dir.display()))?;
    let mut frames: Vec<FrameInfo> = Vec::new();
    for path in paths {
        let bytes = fs::read(&path)
            .with_context(|| format!("read failed for {}", path.display()))?;
        let frame = ingest::classify_frame(&path, &bytes);
        frames.push(frame);
    }

    let lights = frames
        .iter()
        .filter(|f| f.frame_type == FrameType::Light)
        .count();
    if lights == 0 {
        bail!("no light frames in {}", source_dir.display());
    }

    let session_id = format!(
        "cli-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let manifest = ingest::build_manifest(&session_id, &source_dir.display().to_string(), frames);

    // 2. Read all light frames into F32Images. Calibration is currently
    //    lights-only (no darks/flats yet); the MVP pipeline document
    //    makes that explicit and the CLI mirrors it.
    let mut calibrated: Vec<astroforge_core::image::F32Image> = Vec::new();
    for frame in &manifest.frames {
        if frame.frame_type != FrameType::Light {
            continue;
        }
        let bytes = fs::read(&frame.path)
            .with_context(|| format!("read failed for {}", frame.path.display()))?;
        let header = fits::parse_header(&bytes)
            .with_context(|| format!("parse_header failed for {}", frame.path.display()))?;
        let img = fits::read_f32_image(&bytes, &header)
            .with_context(|| format!("read_f32_image failed for {}", frame.path.display()))?;
        calibrated.push(img);
    }

    // 3. Run the MVP pipeline end-to-end.
    let config = PipelineConfig::default();
    let result = mvp_pipeline::run_pipeline(&manifest, calibrated, &config);

    if !result.success {
        bail!(
            "pipeline failed: {}",
            result.error.unwrap_or_else(|| "unknown".into())
        );
    }

    let preview = result
        .preview
        .clone()
        .context("pipeline returned no preview")?;

    let stretched = result
        .stretched
        .clone()
        .context("pipeline returned no stretched image")?;

    // 4. Export the stretched image as a 16-bit TIFF.
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create output directory {}", parent.display())
            })?;
        }
    }
    let file = File::create(output_path)
        .with_context(|| format!("failed to create output file {}", output_path.display()))?;
    let mut writer = BufWriter::new(file);
    export_tiff_16bit(&stretched, &mut writer).context("export_tiff_16bit failed")?;
    drop(writer);

    let output_bytes = fs::metadata(output_path)
        .with_context(|| format!("failed to stat output {}", output_path.display()))?
        .len();

    let darks = frames
        .iter()
        .filter(|f| f.frame_type == FrameType::Dark)
        .count();
    let flats = frames
        .iter()
        .filter(|f| f.frame_type == FrameType::Flat)
        .count();
    let biases = frames
        .iter()
        .filter(|f| f.frame_type == FrameType::Bias)
        .count();
    let total_exposure_s: f64 = frames
        .iter()
        .filter(|f| f.frame_type == FrameType::Light)
        .filter_map(|f| f.exptime)
        .sum();

    let stages: Vec<StageInfo> = result
        .report
        .stage_parameters
        .into_iter()
        .map(|s| StageInfo {
            id: s.stage_id,
            params: s.params.into_iter().collect(),
        })
        .collect();

    Ok(CliReport {
        source_dir: source_dir.display().to_string(),
        output_path: output_path.display().to_string(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        lights,
        darks,
        flats,
        biases,
        total_exposure_s,
        stages,
        preview_width: preview.width,
        preview_height: preview.height,
        output_bytes,
    })
}
