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
use astroforge_core::ingest::{self, FrameInfo, FrameType, SessionManifest};
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
    let mut frames: Vec<FrameInfo> = Vec::new();
    let entries = fs::read_dir(source_dir)
        .with_context(|| format!("failed to read source directory {}", source_dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map_or(true, |e| {
                !(e.eq_ignore_ascii_case("fits") || e.eq_ignore_ascii_case("fit"))
            })
        {
            continue;
        }
        // Best-effort classify using IMAGETYP; fall back to filename.
        let frame = ingest::classify_frame(&path, fits::parse_header)
            .with_context(|| format!("classify_frame failed for {}", path.display()))?;
        frames.push(frame);
    }

    let lights = frames
        .iter()
        .filter(|f| f.frame_type == FrameType::Light)
        .count();
    if lights == 0 {
        bail!("no light frames in {}", source_dir.display());
    }

    let manifest = build_manifest(frames.clone());

    // 2. Read all light frames into F32Images. Calibration is currently
    //    lights-only (no darks/flats yet); the MVP pipeline document
    //    makes that explicit and the CLI mirrors it.
    let mut calibrated: Vec<astroforge_core::image::F32Image> = Vec::new();
    for frame in &frames {
        if frame.frame_type != FrameType::Light {
            continue;
        }
        let img = fits::read_f32_image(&frame.path)
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
            params: s.params,
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

fn build_manifest(frames: Vec<FrameInfo>) -> SessionManifest {
    // Light groups: group lights by filter+binning so the orchestrator
    // can stack each group independently. For the MVP smoke test this
    // is just one bucket, but the wire shape stays correct.
    let mut light_groups: Vec<astroforge_core::ingest::LightGroup> = Vec::new();
    for frame in &frames {
        if frame.frame_type != FrameType::Light {
            continue;
        }
        let filter = frame.filter.clone().unwrap_or_else(|| "L".into());
        let binning = frame.binning.unwrap_or(1);
        if let Some(group) = light_groups
            .iter_mut()
            .find(|g| g.filter == filter && g.binning == binning)
        {
            group.frame_paths.push(frame.path.clone());
        } else {
            light_groups.push(astroforge_core::ingest::LightGroup {
                filter,
                binning,
                frame_paths: vec![frame.path.clone()],
            });
        }
    }

    SessionManifest {
        session_id: format!(
            "cli-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ),
        source_dir: ".".into(),
        frames,
        light_groups,
    }
}
