//! CR-07 B4 — metric snapshots + A/B comparison reports over real
//! pixels.
//!
//! B2 shipped the metric registry (`metric_registry.rs`), the delta
//! computation (`compute_deltas`), and the §11 natural-language
//! summary (`assessment.rs`) — but nothing produced the per-version
//! metric values those functions consume. This module closes that
//! gap: it derives a `ComparisonMetric::values`-shaped snapshot from
//! a decoded `F32Image` using the shipped `image_analysis::metrics`
//! detectors, then assembles the full delta table + summary for a
//! version pair.
//!
//! Only the metrics with shipped detectors produce values; every
//! other registry kind surfaces as a `None`-valued
//! (`Inconclusive`) row, per B2's `compute_deltas` contract. The
//! UI renders those as "—" so the table is honest about what is
//! and is not measured.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::assessment::natural_language_summary;
use crate::comparison::{ComparisonItem, ComparisonSlot};
use crate::domain_store::{DomainStore, DomainStoreError};
use crate::image::F32Image;
use crate::image_analysis::metrics;
use crate::metric_registry::{compute_deltas, MetricDeltaRow, MetricKind};

/// Compute the CR-07 §8 metric snapshot for one decoded image.
///
/// Keys are `MetricKind::as_str()` strings, matching the
/// `ComparisonMetric::values` shape from B1. Only metrics with a
/// shipped `image_analysis::metrics` detector are populated:
///
/// | Key | Detector |
/// |---|---|
/// | `noise.luminance` | `luminance_noise` |
/// | `noise.chrominance` | `chromatic_noise` |
/// | `sharpness.local` | `local_contrast` |
/// | `background.gradient` | `background_gradient` |
/// | `dynamic_range.highlight_clipping` | `highlight_clipping` |
///
/// Snapshot values are the detectors' raw `MetricsSample::value`;
/// confidence and evidence regions are preview-report concerns and
/// are not part of the delta table.
pub fn metric_snapshot(image: &F32Image) -> BTreeMap<String, f64> {
    let mut values = BTreeMap::new();
    values.insert(
        MetricKind::NoiseLuminance.as_str().to_string(),
        metrics::luminance_noise(image).value,
    );
    values.insert(
        MetricKind::NoiseChrominance.as_str().to_string(),
        metrics::chromatic_noise(image).value,
    );
    values.insert(
        MetricKind::SharpnessLocal.as_str().to_string(),
        metrics::local_contrast(image).value,
    );
    values.insert(
        MetricKind::BackgroundGradient.as_str().to_string(),
        metrics::background_gradient(image).value,
    );
    values.insert(
        MetricKind::DynamicRangeHighlightClipping
            .as_str()
            .to_string(),
        metrics::highlight_clipping(image).value,
    );
    values
}

/// CR-07 §23.1: per-channel statistics: mean, stddev, min, max,
/// and clip count for every channel of a decoded image.
///
/// Keys are `channel.<r|g|b|...>.<stat>` (e.g.
/// `channel.r.mean`, `channel.g.stddev`, `channel.b.min`,
/// `channel.b.max`, `channel.r.clip_count`). The function
/// handles any number of channels (mono, RGB, RGBA, multi-NB),
/// but the spec §23 names "channel statistics" specifically for
/// the three RGB channels so the keys are emitted as `r`, `g`,
/// `b` for the first three channels and as `c{n}` for the rest
/// (e.g. `c3` for a 4th channel, e.g. narrowband Ha + OIII +
/// SII + luminance).
///
/// `clip_count` is the number of pixels with a value at or
/// above `1.0` (normalized). For an F32Image decoded from a
/// TIFF, that maps to the highlight-clipping threshold. This
/// is the §23 "clipping masks" stat at the per-channel level;
/// the §23 noise-map / mask visualisations are separate
/// follow-on slices.
///
/// This is a single-pass O(N) computation: one iteration over
/// the `Array3<f32>` (channels × height × width), maintaining
/// running sums and per-channel min/max. The output is sorted
/// (BTreeMap) so JSON consumers and tests can rely on a
/// deterministic key order.
pub fn channel_stats(image: &F32Image) -> BTreeMap<String, f64> {
    let n_channels = image.channels();
    let mut sums = vec![0.0f64; n_channels];
    let mut sum_sq = vec![0.0f64; n_channels];
    let mut mins = vec![f64::INFINITY; n_channels];
    let mut maxs = vec![f64::NEG_INFINITY; n_channels];
    let mut clip_counts = vec![0u64; n_channels];

    let total_pixels = image.width() * image.height();
    // Single pass: every channel's pixel hits each accumulator
    // exactly once. Using `f64` for the accumulators (the
    // underlying storage is f32 but a 4K × 4K frame sums to
    // ~1.6e7 values, which is comfortably within f64 mantissa
    // precision and avoids catastrophic cancellation in the
    // stddev pass).
    for ((ch, _y, _x), &v) in image.indexed_iter() {
        let v_f64 = v as f64;
        sums[ch] += v_f64;
        sum_sq[ch] += v_f64 * v_f64;
        if v_f64 < mins[ch] {
            mins[ch] = v_f64;
        }
        if v_f64 > maxs[ch] {
            maxs[ch] = v_f64;
        }
        if v_f64 >= 1.0 {
            clip_counts[ch] += 1;
        }
    }

    let mut out = BTreeMap::new();
    let n_pix_f = total_pixels.max(1) as f64;
    for ch in 0..n_channels {
        let prefix = if ch < 3 {
            // The first three channels are named R, G, B
            // (the spec's "channel statistics" section calls
            // these out by name). Beyond three, fall back to
            // a numeric suffix.
            let label = match ch {
                0 => "r",
                1 => "g",
                2 => "b",
                _ => unreachable!(),
            };
            format!("channel.{label}")
        } else {
            format!("channel.c{ch}")
        };
        let mean = sums[ch] / n_pix_f;
        // Var = E[x^2] - E[x]^2; for a deterministic detector
        // this matches the textbook definition to within f64
        // precision. Clamp at 0.0 so a single-pixel image
        // (or any case where floating-point noise yields a
        // negative tiny variance) surfaces as 0.0 rather than
        // -0.0.
        let var = (sum_sq[ch] / n_pix_f) - (mean * mean);
        let stddev = var.max(0.0).sqrt();
        out.insert(format!("{prefix}.mean"), mean);
        out.insert(format!("{prefix}.stddev"), stddev);
        out.insert(format!("{prefix}.min"), mins[ch]);
        out.insert(format!("{prefix}.max"), maxs[ch]);
        out.insert(format!("{prefix}.clip_count"), clip_counts[ch] as f64);
    }
    out
}

/// CR-07 §23.1: combined snapshot: the 5 detector-backed
/// metrics from `metric_snapshot` plus the per-channel
/// statistics. The delta table (`compute_deltas`) and the
/// post-comparison recommendation rule (`build_delta_table`)
/// only read the 5 detector-backed keys, so adding the channel
/// keys is additive: nothing in the existing data path breaks,
/// and the expert panel reads the new keys from this same
/// BTreeMap.
pub fn metric_snapshot_full(image: &F32Image) -> BTreeMap<String, f64> {
    let mut out = metric_snapshot(image);
    out.extend(channel_stats(image));
    out
}

/// The B4 payload consumed by `MetricsTable.svelte`: the full §10
/// delta table (one row per registry metric) plus the §11
/// natural-language summary.
#[derive(Debug, Clone, Serialize)]
pub struct VersionComparisonReport {
    pub rows: Vec<MetricDeltaRow>,
    pub summary: String,
}

/// Build the A/B comparison report for two decoded images.
///
/// `baseline` is slot A (the version the delta is measured
/// against), `compared` is slot B (the candidate). The labels are
/// used only for the summary header line; the delta rows carry the
/// registry's own labels.
pub fn compare_version_images(
    baseline: &F32Image,
    compared: &F32Image,
    baseline_version_id: &str,
    compared_version_id: &str,
) -> VersionComparisonReport {
    let baseline_values = metric_snapshot(baseline);
    let compared_values = metric_snapshot(compared);
    let rows = compute_deltas(&baseline_values, &compared_values);
    let baseline_item = ComparisonItem {
        slot: ComparisonSlot::A,
        version_id: baseline_version_id.to_string(),
        label: Some(baseline_version_id.to_string()),
    };
    let compared_item = ComparisonItem {
        slot: ComparisonSlot::B,
        version_id: compared_version_id.to_string(),
        label: Some(compared_version_id.to_string()),
    };
    // Gate findings are a quality-gate concern (CR-07 §12 reuses
    // `quality_gates` in the comparison context, a later bundle).
    // The B4 report summarizes metric deltas only.
    let summary = natural_language_summary(&rows, &[], &baseline_item, &compared_item);
    VersionComparisonReport { rows, summary }
}

/// Failure modes for loading a version's applied pixels.
#[derive(Debug, thiserror::Error)]
pub enum VersionPixelsError {
    #[error("image version '{0}' not found")]
    VersionNotFound(String),
    #[error("{0}")]
    Store(#[from] DomainStoreError),
    #[error("artifact path is outside the applied directory: {0}")]
    OutsideRoot(String),
    #[error("I/O: {0}")]
    Io(String),
    #[error("decode artifact: {0}")]
    Decode(String),
}

/// Decode a version's primary artifact into an `F32Image`.
///
/// Path confinement mirrors the `read_image_artifact` command: the
/// artifact path must canonicalize inside `applied_root` (the
/// caller's `~/.astroforge/applied/<project_id>/`). Absolute paths
/// outside the root, `..` traversal, and symlinks pointing outside
/// are all refused.
///
/// Lives in core (not `src-tauri`) so the confinement + decode path
/// is exercised by CI's `cargo test --workspace`; the Tauri command
/// is a thin wrapper that resolves `applied_root` and maps the
/// error to a string.
pub fn load_version_pixels(
    store: &DomainStore,
    version_id: &str,
    applied_root: &Path,
) -> Result<F32Image, VersionPixelsError> {
    let version = store
        .get_image_version(version_id)?
        .ok_or_else(|| VersionPixelsError::VersionNotFound(version_id.to_string()))?;
    let artifact = store.get_artifact(&version.primary_artifact_id)?;
    let path = Path::new(&artifact.path);
    let canonical = std::fs::canonicalize(path)
        .map_err(|e| VersionPixelsError::Io(format!("canonicalize artifact path: {e}")))?;
    let canonical_root = std::fs::canonicalize(applied_root)
        .map_err(|e| VersionPixelsError::Io(format!("canonicalize applied dir: {e}")))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(VersionPixelsError::OutsideRoot(
            canonical.display().to_string(),
        ));
    }
    let bytes = std::fs::read(&canonical)
        .map_err(|e| VersionPixelsError::Io(format!("read artifact file: {e}")))?;
    F32Image::from_tiff_bytes(&bytes).map_err(|e| VersionPixelsError::Decode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comparison::DeltaDirection;
    use crate::metric_registry::MetricKind;

    /// A uniform mid-gray image: zero noise, zero gradient, zero
    /// clipping, zero local contrast.
    fn flat_image(size: usize) -> F32Image {
        let mut img = F32Image::new(size, size, 1);
        for v in img.iter_mut() {
            *v = 0.5;
        }
        img
    }

    /// A high-frequency hash-pattern image: strong per-pixel
    /// residuals, so the luminance-noise detector reports a
    /// clearly non-zero sigma.
    fn noisy_image(size: usize) -> F32Image {
        let mut img = F32Image::new(size, size, 1);
        for (i, v) in img.iter_mut().enumerate() {
            let h = (i as u32).wrapping_mul(2654435761);
            *v = ((h % 1000) as f32) / 1000.0;
        }
        img
    }

    fn row(report: &VersionComparisonReport, kind: MetricKind) -> &MetricDeltaRow {
        report
            .rows
            .iter()
            .find(|r| r.kind == kind)
            .expect("registry row present")
    }

    #[test]
    fn snapshot_covers_exactly_the_shipped_detectors() {
        let img = flat_image(64);
        let snap = metric_snapshot(&img);
        assert_eq!(snap.len(), 5);
        for kind in [
            MetricKind::NoiseLuminance,
            MetricKind::NoiseChrominance,
            MetricKind::SharpnessLocal,
            MetricKind::BackgroundGradient,
            MetricKind::DynamicRangeHighlightClipping,
        ] {
            assert!(snap.contains_key(kind.as_str()), "missing {}", kind);
        }
    }

    #[test]
    fn report_covers_every_registry_metric() {
        let a = flat_image(64);
        let b = flat_image(64);
        let report = compare_version_images(&a, &b, "va", "vb");
        assert_eq!(report.rows.len(), MetricKind::ALL.len());
    }

    #[test]
    fn identical_images_report_no_material_deltas() {
        let a = noisy_image(64);
        let b = noisy_image(64);
        let report = compare_version_images(&a, &b, "va", "vb");
        for r in &report.rows {
            assert!(
                matches!(
                    r.direction,
                    DeltaDirection::Unchanged | DeltaDirection::Inconclusive
                ),
                "metric {} unexpectedly {:?}",
                r.kind,
                r.direction
            );
        }
    }

    #[test]
    fn noise_reduction_is_detected_as_improvement() {
        // Baseline (A) is noisy; candidate (B) is flat. Noise is
        // LowerIsBetter, so the delta must classify Improved.
        let baseline = noisy_image(64);
        let compared = flat_image(64);
        let report = compare_version_images(&baseline, &compared, "va", "vb");
        let noise = row(&report, MetricKind::NoiseLuminance);
        assert_eq!(noise.direction, DeltaDirection::Improved);
        assert!(noise.percent_change < 0.0);
        // Unmeasured metrics stay honest: no value, Inconclusive.
        let stars = row(&report, MetricKind::StarCount);
        assert_eq!(stars.direction, DeltaDirection::Inconclusive);
        assert!(stars.baseline_value.is_none());
        assert!(stars.compared_value.is_none());
    }

    #[test]
    fn summary_names_the_slots_and_overall_line() {
        let baseline = noisy_image(64);
        let compared = flat_image(64);
        let report = compare_version_images(&baseline, &compared, "va", "vb");
        assert!(report.summary.starts_with("Comparison:"));
        assert!(report.summary.contains("Overall:"));
    }

    // ─── load_version_pixels ─────────────────────────────────────────

    use crate::domain::{Artifact, ArtifactCategory, ImageVersion};
    use std::path::PathBuf;

    fn test_store() -> DomainStore {
        DomainStore::new(&PathBuf::from(":memory:")).unwrap()
    }

    /// Write a real 16-bit TIFF of `img` into `dir`, register the
    /// artifact + version rows, and return the version id.
    fn register_version_with_pixels(
        store: &DomainStore,
        dir: &Path,
        version_id: &str,
        img: &F32Image,
    ) -> String {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(format!("{version_id}.tif"));
        let mut file = std::fs::File::create(&path).unwrap();
        crate::export::export_tiff_16bit(img, &mut file).unwrap();
        drop(file);
        let artifact = Artifact {
            artifact_id: format!("art_{version_id}"),
            artifact_hash: String::new(),
            artifact_type: ArtifactCategory::Derived,
            format: "tif".into(),
            path: path.to_string_lossy().into_owned(),
            size: 0,
            created_at: "2026-09-15T00:00:00Z".into(),
            producer_stage: Some("test".into()),
            pipeline_run_id: None,
            parent_artifact_ids: vec![],
            width: Some(img.width() as u32),
            height: Some(img.height() as u32),
            channels: Some(img.channels() as u32),
            bit_depth: Some(16),
            color_space: Some("srgb".into()),
            linear_or_nonlinear: Some(false),
        };
        store.record_artifact(&artifact).unwrap();
        store
            .upsert_image_version(&ImageVersion {
                version_id: version_id.into(),
                project_id: "proj".into(),
                label: "test".into(),
                sequence: 1,
                primary_artifact_id: artifact.artifact_id.clone(),
                source_version_id: None,
                created_at: "2026-09-15T00:00:00Z".into(),
                hidden: false,
                recipe_id: None,
            })
            .unwrap();
        artifact.artifact_id
    }

    #[test]
    fn load_version_pixels_round_trips_a_real_tiff() {
        let store = test_store();
        let tmp = std::env::temp_dir().join(format!("af-b4-rt-{}", std::process::id()));
        let root = tmp.join("applied").join("proj");
        let img = noisy_image(32);
        register_version_with_pixels(&store, &root, "ver_rt", &img);
        let loaded = load_version_pixels(&store, "ver_rt", &root).unwrap();
        assert_eq!(loaded.width(), img.width());
        assert_eq!(loaded.height(), img.height());
        assert_eq!(loaded.channels(), img.channels());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_version_pixels_refuses_paths_outside_the_root() {
        let store = test_store();
        let tmp = std::env::temp_dir().join(format!("af-b4-out-{}", std::process::id()));
        // The artifact lives in a sibling directory, NOT under the
        // claimed applied root.
        let elsewhere = tmp.join("elsewhere");
        let img = flat_image(16);
        register_version_with_pixels(&store, &elsewhere, "ver_escape", &img);
        let root = tmp.join("applied").join("proj");
        std::fs::create_dir_all(&root).unwrap();
        let err = load_version_pixels(&store, "ver_escape", &root).unwrap_err();
        assert!(
            matches!(err, VersionPixelsError::OutsideRoot(_)),
            "expected OutsideRoot, got {err}"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_version_pixels_reports_missing_versions() {
        let store = test_store();
        let root = Path::new("/nonexistent");
        let err = load_version_pixels(&store, "ver_missing", root).unwrap_err();
        assert!(matches!(err, VersionPixelsError::VersionNotFound(_)));
    }

    // ─── §23.1 channel_stats tests ───────────────────────────────────

    /// An RGB image where R is uniform 0.2, G is uniform 0.5, B
    /// is uniform 0.8. The expected per-channel stats are
    /// closed-form and the test asserts them to within 1e-9.
    fn uniform_rgb_image() -> F32Image {
        let mut img = F32Image::new(4, 4, 3);
        for ((ch, _y, _x), v) in img.indexed_iter_mut() {
            *v = match ch {
                0 => 0.2,
                1 => 0.5,
                2 => 0.8,
                _ => 0.0,
            };
        }
        img
    }

    /// A 4-channel image (R, G, B, narrowband) to exercise the
    /// `c3`-and-beyond numeric suffix path.
    fn four_channel_image() -> F32Image {
        let mut img = F32Image::new(2, 2, 4);
        for ((ch, _y, _x), v) in img.indexed_iter_mut() {
            *v = match ch {
                0 => 0.1,
                1 => 0.2,
                2 => 0.3,
                3 => 0.4,
                _ => 0.0,
            };
        }
        img
    }

    #[test]
    fn channel_stats_uniform_rgb_matches_closed_form() {
        let img = uniform_rgb_image();
        let stats = channel_stats(&img);
        // 3 channels x 5 stats = 15 keys.
        assert_eq!(stats.len(), 15);
        // R: mean=0.2, stddev=0, min=0.2, max=0.2, clip_count=0.
        // The tolerance is 1e-5 (not 1e-9) because the F32Image
        // stores f32, and 0.2 is not exactly representable in
        // f32 (the round-trip is 0.20000000298023224). The
        // function reads f32 → f64, so the f32 quantization
        // shows up in the mean. A 1e-5 tolerance is well above
        // the f32 ulp at this magnitude (~1.2e-8) and well below
        // any user-visible difference.
        assert!((stats["channel.r.mean"] - 0.2).abs() < 1e-5);
        assert!(stats["channel.r.stddev"].abs() < 1e-5);
        assert!((stats["channel.r.min"] - 0.2).abs() < 1e-5);
        assert!((stats["channel.r.max"] - 0.2).abs() < 1e-5);
        assert!(stats["channel.r.clip_count"].abs() < 1e-5);
        // G: 0.5 (0.5 is exactly representable in f32, so the
        // tolerance could be 1e-9, but using 1e-5 keeps the
        // tolerance uniform across channels).
        assert!((stats["channel.g.mean"] - 0.5).abs() < 1e-5);
        assert!((stats["channel.g.max"] - 0.5).abs() < 1e-5);
        // B: 0.8 (also f32-imprecise).
        assert!((stats["channel.b.mean"] - 0.8).abs() < 1e-5);
    }

    #[test]
    fn channel_stats_stddev_matches_known_distribution() {
        // 2x1 mono image with values {0.0, 1.0}: mean=0.5, var=0.25,
        // stddev=0.5. Mono image has 1 channel, so only the
        // non-prefixed keys appear; the function still emits the
        // r-prefixed slot for the single channel.
        let mut img = F32Image::new(2, 1, 1);
        img[(0, 0, 0)] = 0.0;
        img[(0, 0, 1)] = 1.0;
        let stats = channel_stats(&img);
        assert_eq!(stats.len(), 5);
        assert!((stats["channel.r.mean"] - 0.5).abs() < 1e-9);
        assert!((stats["channel.r.stddev"] - 0.5).abs() < 1e-9);
        assert!((stats["channel.r.min"] - 0.0).abs() < 1e-9);
        assert!((stats["channel.r.max"] - 1.0).abs() < 1e-9);
        // The clip_count threshold is >= 1.0, so 1.0 counts.
        assert!((stats["channel.r.clip_count"] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn channel_stats_clip_threshold_is_one() {
        // All pixels at 0.9999 (just below the threshold) → 0 clips.
        // All pixels at 1.0 (the threshold) → all clips (>= 1.0
        // counts).
        let mut below = F32Image::new(3, 3, 1);
        for v in below.iter_mut() {
            *v = 0.9999;
        }
        let stats_below = channel_stats(&below);
        assert!((stats_below["channel.r.clip_count"]).abs() < 1e-9);

        let mut at = F32Image::new(3, 3, 1);
        for v in at.iter_mut() {
            *v = 1.0;
        }
        let stats_at = channel_stats(&at);
        // 3x3 = 9 pixels all at 1.0 → clip_count = 9.
        assert!((stats_at["channel.r.clip_count"] - 9.0).abs() < 1e-9);
    }

    #[test]
    fn channel_stats_handles_four_channels_with_numeric_suffix() {
        let img = four_channel_image();
        let stats = channel_stats(&img);
        // 4 channels x 5 stats = 20 keys.
        assert_eq!(stats.len(), 20);
        // Tolerance 1e-5 (the test values 0.1, 0.2, 0.3, 0.4
        // are not exactly representable in f32).
        assert!((stats["channel.r.mean"] - 0.1).abs() < 1e-5);
        assert!((stats["channel.g.mean"] - 0.2).abs() < 1e-5);
        assert!((stats["channel.b.mean"] - 0.3).abs() < 1e-5);
        // Fourth channel uses the numeric suffix.
        assert!((stats["channel.c3.mean"] - 0.4).abs() < 1e-5);
        assert!((stats["channel.c3.max"] - 0.4).abs() < 1e-5);
    }

    #[test]
    fn channel_stats_is_deterministic_for_same_input() {
        let img = uniform_rgb_image();
        let a = channel_stats(&img);
        let b = channel_stats(&img);
        assert_eq!(a, b);
    }

    #[test]
    fn channel_stats_handles_single_pixel_image() {
        // Edge case: 1x1 image, 1 channel. The f64-accumulator
        // path must not divide by zero; stddev must be exactly
        // 0.0 (single sample). The 0.42 value is not exactly
        // representable in f32, so the tolerance is 1e-5.
        let mut img = F32Image::new(1, 1, 1);
        img[(0, 0, 0)] = 0.42;
        let stats = channel_stats(&img);
        assert_eq!(stats.len(), 5);
        assert!((stats["channel.r.mean"] - 0.42).abs() < 1e-5);
        assert!(stats["channel.r.stddev"].abs() < 1e-5);
        assert!((stats["channel.r.min"] - 0.42).abs() < 1e-5);
        assert!((stats["channel.r.max"] - 0.42).abs() < 1e-5);
    }

    #[test]
    fn metric_snapshot_full_merges_detector_and_channel_keys() {
        let img = uniform_rgb_image();
        let full = metric_snapshot_full(&img);
        // The detector-backed keys from `metric_snapshot` are
        // present.
        assert!(full.contains_key(MetricKind::NoiseLuminance.as_str()));
        assert!(full.contains_key(MetricKind::DynamicRangeHighlightClipping.as_str()));
        // The new channel keys are present.
        assert!(full.contains_key("channel.r.mean"));
        assert!(full.contains_key("channel.g.stddev"));
        assert!(full.contains_key("channel.b.max"));
        assert!(full.contains_key("channel.b.clip_count"));
        // 5 detector keys + 15 channel keys = 20 total.
        assert_eq!(full.len(), 20);
    }
}
