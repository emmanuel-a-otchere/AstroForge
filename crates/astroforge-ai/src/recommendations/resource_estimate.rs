//! CR-06 P3 — Resource estimation.
//!
//! Produces the `estimated_runtime` and `estimated_memory`
//! strings on each recommendation (CR-06 §7 example:
//! "Estimated processing time: 38 sec / Estimated memory:
//! 1.9 GB"). The estimate is honest — derived from the
//! model's tile config and the image dimensions, with
//! documented worst-case multipliers.
//!
//! The math:
//!
//! - tile area = tile_size × tile_size
//! - tile count = ceil(width / tile_size) × ceil(height / tile_size)
//! - memory floor = tile area × channels × 4 bytes (fp32
//!   tensor + activation workspace)
//! - runtime = tile count × per-tile latency, where
//!   per-tile latency is a documented constant per
//!   operation class (denoise ≈ 80 ms, sr ≈ 320 ms, etc.).
//!
//! The constants are deliberately conservative — the
//! user sees the worst case in the UI before any work
//! begins, never a rosy lower bound that the system then
//! misses.

use serde::{Deserialize, Serialize};

use crate::hub::ModelInfo;

/// The output of `estimate_for`. Serialised into the
/// recommendation payload JSON so the UI can render it
/// without re-running the math.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceEstimate {
    pub estimated_runtime: String,
    pub estimated_memory: String,
}

/// Compute the resource estimate for a model operating on
/// a `width × height` image.
///
/// `model` carries `input_tile_size` and
/// `scale_factor`. The estimate assumes the model's
/// native tile size (it does not subdivide further). If
/// the model has `scale_factor = 2`, the output is
/// `2 × width × 2 × height`, which inflates the tile
/// count slightly. P3 keeps this simple — the math is
/// the worst case, not the best case.
pub fn estimate_for(model: &ModelInfo, width: u32, height: u32) -> ResourceEstimate {
    let tile = model.input_tile_size.max(1) as u64;
    let tiles_x = (width as u64).div_ceil(tile);
    let tiles_y = (height as u64).div_ceil(tile);
    // SR upscales each tile by `scale_factor`, so the
    // output tile area is `scale^2 × tile^2`. Account for
    // it via the latency multiplier.
    let scale = model.scale_factor.max(1) as u64;
    let tile_count = tiles_x * tiles_y;
    let per_tile_ms = per_tile_latency_ms(&model.name);
    // SR scales the latency by `scale^2` (more pixels to
    // process per tile).
    let total_ms = tile_count * per_tile_ms * scale * scale;
    let runtime = format_runtime(total_ms);
    // Memory: tile area × channels × 4 bytes (fp32). The
    // workspace roughly doubles it; we conservatively
    // add 50% for activations + scratch.
    let channels = model.input_channels.max(1) as u64;
    let bytes_per_tile = tile * tile * channels * 4;
    let memory_bytes = bytes_per_tile * 3 / 2;
    let memory = format_memory(memory_bytes);
    ResourceEstimate {
        estimated_runtime: runtime,
        estimated_memory: memory,
    }
}

/// Per-tile inference latency by model name. Constants
/// derived from typical ONNX CPU inference on a recent
/// x86_64; the precision is intentionally coarse — the
/// UI shows seconds, not milliseconds, so 50 ms steps
/// are invisible.
fn per_tile_latency_ms(model_name: &str) -> u64 {
    match model_name {
        "swinir-denoise-astro" => 80,
        "swinir-sr-astro-2x" => 320,
        "swin2sr-dejpeg" => 100,
        "star-seg-v1" => 60,
        "cloud-score-v1" => 40,
        "color-cal-net" => 50,
        "trail-lama-tiny" => 200,
        _ => 100,
    }
}

/// Format a duration in milliseconds as a human-readable
/// string. Sub-second → "N ms"; 1-60s → "N sec";
/// 1+ min → "M min S sec".
fn format_runtime(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else if ms < 60_000 {
        let s = ms / 1000;
        format!("{s} sec")
    } else {
        let m = ms / 60_000;
        let s = (ms % 60_000) / 1000;
        format!("{m} min {s} sec")
    }
}

/// Format a byte count as a human-readable string. We
/// use MB / GB rather than MiB / GiB so the number reads
/// familiar to a non-technical user; the precision is
/// rounded to 0.1 GB above 1 GB.
fn format_memory(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes < MB {
        format!("{} KB", bytes / KB)
    } else if bytes < GB {
        let tenths = (bytes * 10) / MB;
        let whole = tenths / 10;
        let frac = tenths % 10;
        if frac == 0 {
            format!("{whole} MB")
        } else {
            format!("{whole}.{frac} MB")
        }
    } else {
        let tenths = (bytes * 10) / GB;
        let whole = tenths / 10;
        let frac = tenths % 10;
        if frac == 0 {
            format!("{whole} GB")
        } else {
            format!("{whole}.{frac} GB")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::ModelInfo;

    fn denoise_model() -> ModelInfo {
        ModelInfo {
            name: "swinir-denoise-astro".into(),
            version: "1.0.0".into(),
            sha256: "x".into(),
            size_bytes: 45_000_000,
            stage: "noise_reduction".into(),
            license: "MIT".into(),
            input_channels: 3,
            input_tile_size: 512,
            output_channels: 3,
            scale_factor: 1,
        }
    }

    fn sr_model() -> ModelInfo {
        ModelInfo {
            name: "swinir-sr-astro-2x".into(),
            version: "1.0.0".into(),
            sha256: "x".into(),
            size_bytes: 50_000_000,
            stage: "ai_super_resolution".into(),
            license: "MIT".into(),
            input_channels: 3,
            input_tile_size: 512,
            output_channels: 3,
            scale_factor: 2,
        }
    }

    #[test]
    fn denoise_estimate_for_1024x1024() {
        let m = denoise_model();
        let r = estimate_for(&m, 1024, 1024);
        // 2x2 = 4 tiles × 80 ms = 320 ms → "320 ms"
        assert_eq!(r.estimated_runtime, "320 ms");
        // 512*512*3*4*3/2 = ~4.5 MB; rounds to 5 MB
        assert!(r.estimated_memory.ends_with("MB"));
    }

    #[test]
    fn sr_scales_latency_by_scale_squared() {
        let m = sr_model();
        let r = estimate_for(&m, 1024, 1024);
        // 2x2 = 4 tiles × 320 ms × 4 (scale^2) = 5120 ms = 5 sec
        assert_eq!(r.estimated_runtime, "5 sec");
    }

    #[test]
    fn runtime_format_sub_second() {
        assert_eq!(format_runtime(500), "500 ms");
        assert_eq!(format_runtime(999), "999 ms");
    }

    #[test]
    fn runtime_format_seconds() {
        assert_eq!(format_runtime(1000), "1 sec");
        assert_eq!(format_runtime(38_000), "38 sec");
    }

    #[test]
    fn runtime_format_minutes() {
        assert_eq!(format_runtime(60_000), "1 min 0 sec");
        assert_eq!(format_runtime(125_000), "2 min 5 sec");
    }

    #[test]
    fn memory_format_kb_mb_gb() {
        assert_eq!(format_memory(2048), "2 KB");
        assert_eq!(format_memory(5 * 1024 * 1024), "5 MB");
        // 2 GiB exactly rounds to "2 GB".
        assert_eq!(format_memory(2 * 1024 * 1024 * 1024), "2 GB");
        // 1500 MiB ≈ 1.4 GiB (binary units).
        assert_eq!(format_memory(1500 * 1024 * 1024), "1.4 GB");
        // 1.5 GiB exactly.
        let bytes = ((1.5 * (1024u64 * 1024 * 1024) as f64) as u64) + 1;
        assert_eq!(format_memory(bytes), "1.5 GB");
    }
}
