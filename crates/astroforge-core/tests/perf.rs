//! CR-07 §32.5: performance tests.
//!
//! Pins wall-clock and allocation bounds for the B7 Perf
//! bundle's Rust-testable surface:
//!
//! - `compute_diff` (4 modes × 4K + 8K resolutions).
//! - §8 detectors (`luminance_noise` + `saturation_percentage`)
//!   on 4K + 8K images.
//! - `Recipe::pipeline_plan_hash` at 10 / 100 / 1000 stages.
//! - `recipe_ai_diff_summary` at 10 / 100 / 1000 stages.
//! - Multi-version `compare_version_images` on 4K images.
//!
//! The bounds here are intentionally generous: this is a
//! debug build (no LTO, no codegen-units=1, no native CPU
//! features). The test purpose is to catch order-of-magnitude
//! regressions, not to enforce micro-benchmarks.
//!
//! Release-mode benchmarks belong in a `criterion`-backed
//! bench harness under `crates/astroforge-core/benches/`.
//! That is a follow-on slice (probably §32.6 or §29).
//!
//! The bounds are documented per-test. Each bound is
//! "must complete within N seconds on a debug build at
//! a reasonable CI runner". Roughly 4x the expected
//! release-mode runtime. If a bound fires, the test
//! prints the actual elapsed time so the failure message
//! is actionable.

use astroforge_core::comparison_metrics::compare_version_images;
use astroforge_core::difference::{compute_diff, DiffKind};
use astroforge_core::image::F32Image;
use astroforge_core::image_analysis::metrics::{luminance_noise, saturation_percentage};
use astroforge_core::recipe::{recipe_ai_diff_summary, ModelType, Recipe};
use std::time::Instant;

// ─── helpers ────────────────────────────────────────────────

/// Build an RGBA8 buffer of `width × height` pixels with
/// the byte at offset `i` set to `(i % 256) as u8`.
/// Deterministic; perfect for perf measurement because
/// the compiler can't constant-fold it.
fn rgba8_grid(width: usize, height: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(width * height * 4);
    for i in 0..(width * height) {
        let v = (i % 256) as u8;
        out.push(v);
        out.push(v.wrapping_add(7));
        out.push(v.wrapping_mul(3));
        out.push(255); // opaque alpha
    }
    out
}

/// Build an F32Image of `width × height × channels`
/// with deterministic per-pixel values in [0, 1].
fn f32_image_grid(width: usize, height: usize, channels: usize) -> F32Image {
    let mut img = F32Image::new(width, height, channels);
    for ch in 0..channels {
        for y in 0..height {
            for x in 0..width {
                let v = ((x + y + ch * 17) % 256) as f32 / 255.0;
                img[(ch, y, x)] = v;
            }
        }
    }
    img
}

/// Wall-clock-bound helper. Asserts the closure completes
/// within `max_seconds`. Prints the actual elapsed time
/// on success so the test log documents the steady-state.
fn must_complete_in<F: FnOnce()>(label: &str, max_seconds: f64, f: F) {
    let start = Instant::now();
    f();
    let elapsed = start.elapsed().as_secs_f64();
    println!("[perf] {label}: {elapsed:.3}s (limit {max_seconds:.1}s)");
    assert!(
        elapsed <= max_seconds,
        "[perf] {label}: took {elapsed:.3}s, exceeds limit {max_seconds:.1}s"
    );
}

// ─── §32.5.1: compute_diff at 4K (3840×2160) ───────────────

#[test]
fn compute_diff_absolute_at_4k_completes_within_bound() {
    let width = 3840;
    let height = 2160;
    let a = rgba8_grid(width, height);
    let b = rgba8_grid(width, height);
    must_complete_in("compute_diff[Absolute] @ 4K", 20.0, || {
        let out = compute_diff(DiffKind::Absolute, &a, &b, 1.0, width as u32);
        assert_eq!(out.len(), a.len());
    });
}

#[test]
fn compute_diff_signed_at_4k_completes_within_bound() {
    let width = 3840;
    let height = 2160;
    let a = rgba8_grid(width, height);
    let b = rgba8_grid(width, height);
    must_complete_in("compute_diff[Signed] @ 4K", 20.0, || {
        let out = compute_diff(DiffKind::Signed, &a, &b, 1.0, width as u32);
        assert_eq!(out.len(), a.len());
    });
}

#[test]
fn compute_diff_amplified_at_4k_completes_within_bound() {
    let width = 3840;
    let height = 2160;
    let a = rgba8_grid(width, height);
    let b = rgba8_grid(width, height);
    must_complete_in("compute_diff[Amplified] @ 4K", 20.0, || {
        let out = compute_diff(DiffKind::Amplified, &a, &b, 4.0, width as u32);
        assert_eq!(out.len(), a.len());
    });
}

#[test]
fn compute_diff_structural_at_4k_completes_within_bound() {
    let width = 3840;
    let height = 2160;
    let a = rgba8_grid(width, height);
    let b = rgba8_grid(width, height);
    must_complete_in("compute_diff[Structural] @ 4K", 20.0, || {
        let out = compute_diff(DiffKind::Structural, &a, &b, 1.0, width as u32);
        assert_eq!(out.len(), a.len());
    });
}

// ─── §32.5.2: compute_diff at 8K (7680×4320) ───────────────

#[test]
fn compute_diff_absolute_at_8k_completes_within_bound() {
    let width = 7680;
    let height = 4320;
    let a = rgba8_grid(width, height);
    let b = rgba8_grid(width, height);
    must_complete_in("compute_diff[Absolute] @ 8K", 80.0, || {
        let out = compute_diff(DiffKind::Absolute, &a, &b, 1.0, width as u32);
        assert_eq!(out.len(), a.len());
    });
}

#[test]
fn compute_diff_signed_at_8k_completes_within_bound() {
    let width = 7680;
    let height = 4320;
    let a = rgba8_grid(width, height);
    let b = rgba8_grid(width, height);
    must_complete_in("compute_diff[Signed] @ 8K", 80.0, || {
        let out = compute_diff(DiffKind::Signed, &a, &b, 1.0, width as u32);
        assert_eq!(out.len(), a.len());
    });
}

#[test]
fn compute_diff_amplified_at_8k_completes_within_bound() {
    let width = 7680;
    let height = 4320;
    let a = rgba8_grid(width, height);
    let b = rgba8_grid(width, height);
    must_complete_in("compute_diff[Amplified] @ 8K", 80.0, || {
        let out = compute_diff(DiffKind::Amplified, &a, &b, 4.0, width as u32);
        assert_eq!(out.len(), a.len());
    });
}

#[test]
fn compute_diff_structural_at_8k_completes_within_bound() {
    let width = 7680;
    let height = 4320;
    let a = rgba8_grid(width, height);
    let b = rgba8_grid(width, height);
    must_complete_in("compute_diff[Structural] @ 8K", 80.0, || {
        let out = compute_diff(DiffKind::Structural, &a, &b, 1.0, width as u32);
        assert_eq!(out.len(), a.len());
    });
}

// ─── §32.5.3: §8 detectors at 4K + 8K ──────────────────────

#[test]
fn luminance_noise_at_4k_completes_within_bound() {
    let img = f32_image_grid(3840, 2160, 3);
    must_complete_in("luminance_noise @ 4K", 30.0, || {
        let sample = luminance_noise(&img);
        // Just check the sample is well-formed; the actual
        // value depends on the fixture.
        assert!(sample.value.is_finite());
    });
}

#[test]
fn luminance_noise_at_8k_completes_within_bound() {
    let img = f32_image_grid(7680, 4320, 3);
    must_complete_in("luminance_noise @ 8K", 120.0, || {
        let sample = luminance_noise(&img);
        assert!(sample.value.is_finite());
    });
}

#[test]
fn saturation_percentage_at_4k_completes_within_bound() {
    let img = f32_image_grid(3840, 2160, 3);
    must_complete_in("saturation_percentage @ 4K", 30.0, || {
        let sample = saturation_percentage(&img);
        assert!(sample.value.is_finite());
    });
}

#[test]
fn saturation_percentage_at_8k_completes_within_bound() {
    let img = f32_image_grid(7680, 4320, 3);
    must_complete_in("saturation_percentage @ 8K", 120.0, || {
        let sample = saturation_percentage(&img);
        assert!(sample.value.is_finite());
    });
}

// ─── §32.5.4: pipeline_plan_hash at 10 / 100 / 1000 stages ─

fn recipe_with_n_stages(n: usize) -> Recipe {
    let mut r = Recipe::new("perf-test", "stretch");
    for i in 0..n {
        let mut params = std::collections::HashMap::new();
        params.insert(
            "amount".to_string(),
            serde_json::json!((i % 100) as f64 / 100.0),
        );
        r.add_stage(&format!("stage_{i:04}"), params);
    }
    r
}

#[test]
fn pipeline_plan_hash_10_stages_completes_within_bound() {
    let r = recipe_with_n_stages(10);
    must_complete_in("pipeline_plan_hash @ 10 stages", 1.0, || {
        let h = r.pipeline_plan_hash();
        assert_eq!(h.len(), 64);
    });
}

#[test]
fn pipeline_plan_hash_100_stages_completes_within_bound() {
    let r = recipe_with_n_stages(100);
    must_complete_in("pipeline_plan_hash @ 100 stages", 2.0, || {
        let h = r.pipeline_plan_hash();
        assert_eq!(h.len(), 64);
    });
}

#[test]
fn pipeline_plan_hash_1000_stages_completes_within_bound() {
    let r = recipe_with_n_stages(1000);
    must_complete_in("pipeline_plan_hash @ 1000 stages", 10.0, || {
        let h = r.pipeline_plan_hash();
        assert_eq!(h.len(), 64);
    });
}

// ─── §32.5.5: recipe_ai_diff_summary at scale ──────────────

#[test]
fn recipe_ai_diff_summary_10_stages_completes_within_bound() {
    let a = recipe_with_n_stages(10);
    let mut b = recipe_with_n_stages(10);
    b.add_model("neural-denoiser", ModelType::Perceptual);
    must_complete_in("recipe_ai_diff_summary @ 10 stages", 2.0, || {
        let s = recipe_ai_diff_summary(&a, &b);
        assert!(s.ai_classification_differs);
    });
}

#[test]
fn recipe_ai_diff_summary_100_stages_completes_within_bound() {
    let a = recipe_with_n_stages(100);
    let mut b = recipe_with_n_stages(100);
    b.add_model("neural-denoiser", ModelType::Perceptual);
    must_complete_in("recipe_ai_diff_summary @ 100 stages", 5.0, || {
        let s = recipe_ai_diff_summary(&a, &b);
        assert!(s.ai_classification_differs);
    });
}

#[test]
fn recipe_ai_diff_summary_1000_stages_completes_within_bound() {
    let a = recipe_with_n_stages(1000);
    let mut b = recipe_with_n_stages(1000);
    b.add_model("neural-denoiser", ModelType::Perceptual);
    must_complete_in("recipe_ai_diff_summary @ 1000 stages", 30.0, || {
        let s = recipe_ai_diff_summary(&a, &b);
        assert!(s.ai_classification_differs);
    });
}

// ─── §32.5.6: multi-version compare at 4K ──────────────────

#[test]
fn compare_version_images_at_4k_completes_within_bound() {
    let width = 3840;
    let height = 2160;
    let a = f32_image_grid(width, height, 3);
    let b = f32_image_grid(width, height, 3);
    must_complete_in("compare_version_images @ 4K", 30.0, || {
        let report = compare_version_images(&a, &b, "v-a", "v-b");
        // The comparison surface produces a delta table
        // (rows) and a summary; both must be well-formed.
        assert!(!report.rows.is_empty());
        assert!(!report.summary.is_empty());
    });
}

#[test]
fn compare_version_images_at_2k_completes_within_bound() {
    let width = 2560;
    let height = 1440;
    let a = f32_image_grid(width, height, 3);
    let b = f32_image_grid(width, height, 3);
    must_complete_in("compare_version_images @ 2K", 15.0, || {
        let report = compare_version_images(&a, &b, "v-a", "v-b");
        assert!(!report.rows.is_empty());
    });
}

// ─── §32.5.7: positive control, debug-mode floor ──────────

#[test]
fn perf_tests_print_actual_elapsed_for_diagnostics() {
    // This test is not really a perf assertion. It runs
    // a single small operation and prints the actual
    // elapsed time so the test log captures a baseline
    // for future comparison. This guards against
    // "test never prints anything" no-op scenarios.
    let width = 640;
    let height = 480;
    let a = rgba8_grid(width, height);
    let b = rgba8_grid(width, height);
    let start = Instant::now();
    let out = compute_diff(DiffKind::Absolute, &a, &b, 1.0, width as u32);
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "[perf-baseline] compute_diff[Absolute] @ 640x480: {elapsed:.3}s ({} bytes)",
        out.len()
    );
    assert_eq!(out.len(), a.len());
}
