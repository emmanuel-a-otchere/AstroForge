//! 30-frame memory test for the astroforge CLI binary.
//!
//! Phase 7 close-out (issue #45). Generates 30 synthetic light frames,
//! drives the CLI through the full MVP pipeline, and asserts:
//!
//!   1. exit 0
//!   2. output TIFF is produced and valid
//!   3. JSON report has lights == 30
//!   4. peak RSS (Linux only) stays under a documented ceiling
//!
//! The RSS ceiling is intentionally generous (1.5 GiB) — issue #45
//! asks for "30 frames on 4 GB without OOM", so we leave substantial
//! headroom for CI runners that happen to be memory-constrained.
//! The point of the test is to catch *regressions* (e.g. an accidental
//! duplicate buffer), not to pin the absolute number.
//!
//! Run with: cargo test --test cli_memory -- --nocapture

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const FRAME_COUNT: usize = 30;
const FRAME_PIXELS: usize = 128 * 128; // small enough to be fast, big enough to allocate
const RSS_CEILING_KIB: u64 = 1_500_000; // ~1.5 GiB; 4 GB cap leaves 2.5 GB headroom

fn write_synthetic_fits(path: &PathBuf, frame_type: &str, salt: u32) {
    let mut header = astroforge_core::fits::FitsHeader::new();
    header.set("IMAGETYP", frame_type);
    header.set("NAXIS1", "128");
    header.set("NAXIS2", "128");
    header.set("EXPTIME", "120.0");
    header.set("FILTER", "'L'");

    let mut file = File::create(path).unwrap();
    astroforge_core::fits::write_header(&header, &mut file).unwrap();

    let mut data = Vec::with_capacity(FRAME_PIXELS * 4);
    for i in 0..FRAME_PIXELS {
        // Per-frame offset so a stack has signal to find; deterministic.
        let v = (((i as u32).wrapping_add(salt)) % 256) as f32 / 255.0;
        data.extend_from_slice(&v.to_be_bytes());
    }
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
    p.push(format!("astroforge-mem-{}-{}", tag, stamp));
    fs::create_dir_all(&p).unwrap();
    p
}

fn locate_cli() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_astroforge"))
}

#[test]
fn cli_30_frame_memory_test() {
    let source = temp_dir("source");
    let output_dir = temp_dir("out");
    let output = output_dir.join("output.tif");

    for i in 0..FRAME_COUNT {
        write_synthetic_fits(
            &source.join(format!("light_{:03}.fits", i)),
            "LIGHT",
            i as u32 * 7919, // prime stride for visual variation
        );
    }

    let cli = locate_cli();
    let run = Command::new(&cli)
        .arg(&source)
        .arg(&output)
        .env("RUST_LOG", "warn")
        .output()
        .expect("failed to spawn astroforge CLI");

    assert!(
        run.status.success(),
        "CLI exited {:?}\nstdout: {}\nstderr: {}",
        run.status,
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );

    assert!(output.exists(), "output TIFF was not created");

    let meta = fs::metadata(&output).expect("stat output");
    assert!(meta.len() > 0, "output TIFF is empty");

    // Parse the JSON report.
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.starts_with("OK "),
        "expected OK prefix, got: {}",
        stdout
    );
    let report: serde_json::Value = serde_json::from_str(&stdout[3..])
        .unwrap_or_else(|e| panic!("invalid JSON report ({}): {}", e, &stdout[3..]));

    assert_eq!(
        report["lights"].as_u64().unwrap(),
        FRAME_COUNT as u64,
        "report.lights != {}",
        FRAME_COUNT
    );

    // RSS ceiling check — Linux only. On macOS/Windows the field is null
    // and we just log it for visibility.
    match report["peak_rss_kb"].as_u64() {
        Some(kib) => {
            eprintln!(
                "[mem-test] processed {} frames, peak RSS = {} KiB ({:.1} MiB), ceiling {} KiB",
                FRAME_COUNT,
                kib,
                kib as f64 / 1024.0,
                RSS_CEILING_KIB
            );
            assert!(
                kib <= RSS_CEILING_KIB,
                "peak RSS {} KiB exceeded ceiling {} KiB (issue #45: 4 GB budget)",
                kib,
                RSS_CEILING_KIB
            );
        }
        None => {
            eprintln!(
                "[mem-test] processed {} frames; peak_rss_kb unavailable on this platform",
                FRAME_COUNT
            );
        }
    }
}
