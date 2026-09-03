//! Integration test for the astroforge CLI binary.
//!
//! Phase 7 smoke test (#43, #44). Builds a synthetic FITS folder,
//! invokes the CLI as a subprocess, asserts a valid 16-bit TIFF is
//! produced.
//!
//! Run with: cargo test --test cli_smoke -- --nocapture

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn write_synthetic_fits(path: &PathBuf, width: usize, height: usize, frame_type: &str) {
    let mut header = astroforge_core::fits::FitsHeader::new();
    header.set("IMAGETYP", frame_type);
    header.set("NAXIS1", &width.to_string());
    header.set("NAXIS2", &height.to_string());

    let mut file = File::create(path).unwrap();
    astroforge_core::fits::write_header(&header, &mut file).unwrap();

    // Data: simple gradient so the pipeline has something to stretch.
    let pixels = width * height;
    let mut data = Vec::with_capacity(pixels * 4);
    for i in 0..pixels {
        let v = (i % 256) as f32 / 255.0;
        data.extend_from_slice(&v.to_be_bytes());
    }
    // Pad to FITS record boundary.
    let block = 2880usize;
    if data.len() % block != 0 {
        data.resize(data.len() + (block - data.len() % block), 0);
    }
    file.write_all(&data).unwrap();
}

fn temp_dir(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    p.push(format!("astroforge-cli-{}-{}", tag, stamp));
    fs::create_dir_all(&p).unwrap();
    p
}

fn locate_cli() -> PathBuf {
    // cargo's CARGO_BIN_EXE_<name> env points at the compiled binary.
    PathBuf::from(env!("CARGO_BIN_EXE_astroforge"))
}

#[test]
fn cli_smoke_runs_end_to_end() {
    let source = temp_dir("source");
    let output_dir = temp_dir("out");
    let output = output_dir.join("output.tif");

    // 3 light frames + 1 dark + 1 flat.
    for i in 0..3 {
        write_synthetic_fits(
            &source.join(format!("light_{:03}.fits", i)),
            32,
            32,
            "LIGHT",
        );
    }
    write_synthetic_fits(&source.join("dark_001.fits"), 32, 32, "DARK");
    write_synthetic_fits(&source.join("flat_001.fits"), 32, 32, "FLAT");

    let cli = locate_cli();
    let output_run = Command::new(&cli)
        .arg(&source)
        .arg(&output)
        .env("RUST_LOG", "warn")
        .output()
        .expect("failed to spawn astroforge CLI");

    assert!(
        output_run.status.success(),
        "CLI exited {:?}\nstdout: {}\nstderr: {}",
        output_run.status,
        String::from_utf8_lossy(&output_run.stdout),
        String::from_utf8_lossy(&output_run.stderr),
    );

    assert!(output.exists(), "output TIFF was not created");

    let meta = fs::metadata(&output).expect("stat output");
    assert!(meta.len() > 0, "output TIFF is empty");

    // Validate the TIFF header: first two bytes are 'I' 'I' (little-endian)
    // or 'M' 'M' (big-endian). Both are valid; we only assert the magic.
    let mut f = File::open(&output).unwrap();
    let mut header = [0u8; 4];
    use std::io::Read;
    f.read_exact(&mut header).unwrap();
    let magic = &header[..2];
    assert!(
        magic == b"II" || magic == b"MM",
        "not a TIFF: {:?}",
        magic
    );

    // Parse the CLI's JSON report on stdout — it tells us what ran.
    let stdout = String::from_utf8_lossy(&output_run.stdout);
    assert!(stdout.starts_with("OK "), "expected OK prefix, got: {}", stdout);
    let json = &stdout[3..];
    let report: serde_json::Value = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("invalid JSON report ({}): {}", e, json));
    assert_eq!(report["lights"], 3);
    assert_eq!(report["darks"], 1);
    assert_eq!(report["flats"], 1);
    assert_eq!(report["biases"], 0);
    assert!(report["stages"].as_array().unwrap().len() >= 3);
}

#[test]
fn cli_smoke_exits_2_when_no_lights() {
    let source = temp_dir("empty");
    let output = source.join("output.tif");

    // Only a dark frame — no lights → CLI exits 2.
    write_synthetic_fits(&source.join("dark_001.fits"), 16, 16, "DARK");

    let cli = locate_cli();
    let status = Command::new(&cli)
        .arg(&source)
        .arg(&output)
        .env("RUST_LOG", "warn")
        .status()
        .expect("failed to spawn CLI");

    assert_eq!(
        status.code(),
        Some(2),
        "expected exit code 2 (no lights), got {:?}",
        status.code()
    );
}

#[test]
fn cli_smoke_exits_1_on_bad_args() {
    let cli = locate_cli();
    let status = Command::new(&cli)
        .env("RUST_LOG", "warn")
        .status()
        .expect("failed to spawn CLI");
    assert_eq!(status.code(), Some(1));
}
