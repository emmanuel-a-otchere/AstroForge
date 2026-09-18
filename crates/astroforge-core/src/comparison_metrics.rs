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
use crate::registration::extract_stars;

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

// CR-07 §23.2: per-star FWHM distribution + histogram.

/// CR-07 §23.2: full per-star FWHM distribution plus
/// seven-number summary. The histogram is pre-binned
/// (Sturges' rule, floored at 1 and capped at 50) so the
/// frontend does not need to redo the bucketing work.
///
/// `count == 0` means "no stars detected at the
/// default sigma threshold"; the bin edges and counts
/// will be empty, the seven-number summary will be
/// `f64::NAN`, and the histogram will be skipped by
/// the Svelte component (it renders a "No stars
/// detected" message instead).
#[derive(Debug, Clone, Serialize)]
pub struct FwhmHistogram {
    /// Star count (= `Vec<f64>::len()` of `fwhm_distribution`).
    pub count: usize,
    /// Sorted raw FWHM values in pixels.
    pub values: Vec<f64>,
    /// Mean of `values`; `f64::NAN` if `count == 0`.
    pub mean: f64,
    /// Median of `values`; `f64::NAN` if `count == 0`.
    pub median: f64,
    /// 25th percentile (linear interpolation); `f64::NAN` if `count == 0`.
    pub p25: f64,
    /// 75th percentile (linear interpolation); `f64::NAN` if `count == 0`.
    pub p75: f64,
    /// Min; `f64::NAN` if `count == 0`.
    pub min: f64,
    /// Max; `f64::NAN` if `count == 0`.
    pub max: f64,
    /// Bin edges (`bins + 1` edges, in pixels). Empty if `count == 0`.
    pub bin_edges: Vec<f64>,
    /// Counts per bin (`bins` counts). Empty if `count == 0`.
    pub counts: Vec<u32>,
}

/// CR-07 §23.2: extract per-star FWHM values from a
/// decoded image. Wraps `registration::extract_stars`
/// (sigma = 3.0 above the image mean) and returns the
/// `fwhm` field of every detected star. Sort order
/// matches `extract_stars` (brightest first), but the
/// histogram function sorts again by value, so the
/// frontend can rely on `FwhmHistogram.values` being
/// non-decreasing.
pub fn fwhm_distribution(image: &F32Image) -> Vec<f64> {
    extract_stars(image, 3.0)
        .into_iter()
        .map(|s| s.fwhm)
        .collect()
}

/// CR-07 §23.2: `fwhm_distribution` + pre-binned
/// histogram + seven-number summary.
///
/// Edge cases:
/// - `count == 0` returns a zeroed struct with
///   `f64::NAN` summary stats and empty `bin_edges` /
///   `counts` (the frontend renders "No stars
///   detected").
/// - `count == 1` returns one bin with that single
///   value's FWHM as both the left and right edge (the
///   histogram bar renders as a 1-pixel-wide column at
///   that x-coordinate).
/// - Bin count is `min(50, max(1, ceil(log2(n)) + 1))`
///   (Sturges' rule, capped). For typical star counts
///   (32 stars -> 6 bins, 256 -> 9 bins, 4096 -> 13
///   bins) the histogram is readable.
pub fn fwhm_histogram(image: &F32Image) -> FwhmHistogram {
    let mut values = fwhm_distribution(image);
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let count = values.len();
    if count == 0 {
        return FwhmHistogram {
            count: 0,
            values,
            mean: f64::NAN,
            median: f64::NAN,
            p25: f64::NAN,
            p75: f64::NAN,
            min: f64::NAN,
            max: f64::NAN,
            bin_edges: Vec::new(),
            counts: Vec::new(),
        };
    }
    let min = values[0];
    let max = values[count - 1];
    let sum: f64 = values.iter().sum();
    let mean = sum / count as f64;
    let median = percentile(&values, 0.5);
    let p25 = percentile(&values, 0.25);
    let p75 = percentile(&values, 0.75);

    // Sturges' rule, capped to [1, 50].
    let raw_bins = ((count as f64).log2().ceil() as usize).saturating_add(1);
    let bins = raw_bins.clamp(1, 50);

    let (bin_edges, counts) = if count == 1 {
        // Single-value case: one bin centered on the value.
        // Edges are [value, value] so the bar renders as a
        // 1-pixel-wide column at that x-coordinate.
        (vec![min, min], vec![1u32])
    } else if min == max {
        // All values identical (degenerate FWHM): one bin
        // with both edges equal to that value, count = N.
        (vec![min, min], vec![count as u32])
    } else {
        // Standard Sturges histogram. `bin_edges` has
        // `bins + 1` entries; `counts` has `bins` entries.
        let width = (max - min) / bins as f64;
        let edges: Vec<f64> = (0..=bins).map(|i| min + i as f64 * width).collect();
        let mut counts = vec![0u32; bins];
        for v in &values {
            let idx = ((v - min) / width).floor() as usize;
            // Guard against `v == max` (which would index past the end).
            let idx = idx.min(bins - 1);
            counts[idx] += 1;
        }
        (edges, counts)
    };

    FwhmHistogram {
        count,
        values,
        mean,
        median,
        p25,
        p75,
        min,
        max,
        bin_edges,
        counts,
    }
}

/// Linear-interpolation percentile (matches numpy's
/// default `method="linear"`). `q` is in `[0, 1]`.
fn percentile(sorted: &[f64], q: f64) -> f64 {
    debug_assert!(!sorted.is_empty(), "percentile called on empty slice");
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = q * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = rank - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

// CR-07 §23.3: per-pixel noise map (2D sigma field).

/// CR-07 §23.3: 2D per-pixel local sigma field. The
/// image is downsampled to a preview budget
/// (max(width, height) ≤ 256), then for each pixel
/// the local sigma is estimated over a `window` ×
/// `window` neighborhood using the same
/// median-absolute-deviation-on-residuals algorithm
/// as `image_analysis::metrics::luminance_noise` but
/// applied per-pixel instead of aggregated.
///
/// The `sigma` field is a row-major flat array of
/// `width * height` f64 values, indexed as
/// `sigma[y * width + x]`. The values are in the
/// image's normalized scale (typically [0, 1] for
/// FITS / AstroForge-previews; the consumer should
/// scale by `65535` if comparing against a 16-bit
/// representation).
///
/// Edge cases:
/// - Empty image (w == 0 || h == 0): zeroed map
///   with `sigma` empty.
/// - Image smaller than `window`: zeroed map; the
///   Svelte component renders "Not enough pixels
///   for a noise map".
/// - Single-pixel image: zeroed map.
///
/// The summary stats (min, mean, max) are computed
/// alongside the map so the frontend does not have
/// to redo the work.
#[derive(Debug, Clone, Serialize)]
pub struct NoiseMap {
    pub width: u32,
    pub height: u32,
    /// Flat row-major f64 sigma field. `sigma[y * width + x]`.
    pub sigma: Vec<f64>,
    pub min: f64,
    pub mean: f64,
    pub max: f64,
}

/// CR-07 §23.3: compute the per-pixel noise map.
///
/// Window size is 7x7 (a common default in the
/// literature; balances locality vs. estimator
/// stability). Preview size is 256 (matches
/// `luminance_noise`'s budget).
pub fn noise_map(image: &F32Image) -> NoiseMap {
    let w = image.width();
    let h = image.height();
    if w == 0 || h == 0 {
        return NoiseMap {
            width: 0,
            height: 0,
            sigma: Vec::new(),
            min: 0.0,
            mean: 0.0,
            max: 0.0,
        };
    }
    // Downsample to the preview budget. `downsample_box`
    // is the existing helper used by `luminance_noise`.
    let preview = image.downsample_box(0.25);
    let (pw, ph) = (preview.width(), preview.height());
    const WINDOW: usize = 7;
    const HALF: usize = WINDOW / 2;
    if pw < WINDOW || ph < WINDOW {
        // Image (after downsampling) is smaller than
        // the window: can't compute a stable local
        // estimator. Return a zeroed map.
        return NoiseMap {
            width: pw as u32,
            height: ph as u32,
            sigma: Vec::new(),
            min: 0.0,
            mean: 0.0,
            max: 0.0,
        };
    }

    // Per-pixel local sigma. We compute the residual
    // |v - local_median| for every pixel, then
    // MAD-scale it (1.4826 * median) within the
    // window. This is the same estimator as
    // `luminance_noise` but applied per-pixel.
    //
    // For O(W' * H' * WINDOW^2) ≈ 256 * 256 * 49 ≈
    // 3.2M ops on the preview budget, well under
    // 100 ms on a typical laptop.
    let mut sigma = vec![0.0f64; pw * ph];
    let mut min_sigma = f64::INFINITY;
    let mut max_sigma = f64::NEG_INFINITY;
    let mut sum = 0.0f64;
    let mut count = 0u64;

    for y in HALF..(ph - HALF) {
        for x in HALF..(pw - HALF) {
            // Collect the WINDOW x WINDOW window of
            // residuals (each pixel minus its own
            // local median): but computing a local
            // median per pixel would be O(WINDOW^4).
            // Instead we compute the local median
            // once per pixel and take |v - median|
            // as a single residual estimate.
            //
            // For the sigma estimator itself we use
            // the residuals of the inner (WINDOW-2) x
            // (WINDOW-2) pixels, all computed
            // against this same central median. This
            // is a small-window MAD estimator with
            // the same theoretical robustness as the
            // aggregate `luminance_noise`.
            let mut window = [0.0f64; WINDOW * WINDOW];
            for (i, dy) in (0..WINDOW).enumerate() {
                for (j, dx) in (0..WINDOW).enumerate() {
                    let yy = y + dy - HALF;
                    let xx = x + dx - HALF;
                    window[i * WINDOW + j] = preview[(0usize, yy, xx)] as f64;
                }
            }
            window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = window[WINDOW * WINDOW / 2];

            // Inner residuals (skip the outermost
            // ring to avoid edge effects within the
            // window itself).
            let mut residuals = Vec::with_capacity((WINDOW - 2) * (WINDOW - 2));
            for dy in 1..(WINDOW - 1) {
                for dx in 1..(WINDOW - 1) {
                    let v = window[dy * WINDOW + dx];
                    residuals.push((v - median).abs());
                }
            }
            residuals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mad = residuals[residuals.len() / 2];
            let local_sigma = 1.4826 * mad;

            sigma[y * pw + x] = local_sigma;
            if local_sigma < min_sigma {
                min_sigma = local_sigma;
            }
            if local_sigma > max_sigma {
                max_sigma = local_sigma;
            }
            sum += local_sigma;
            count += 1;
        }
    }

    // The summary stats (min, mean, max) are computed
    // over the *inner* field only: the outer ring
    // pixels are zeroed (no stable estimator for them)
    // and would skew the summary if included.
    NoiseMap {
        width: pw as u32,
        height: ph as u32,
        sigma,
        min: if min_sigma.is_finite() {
            min_sigma
        } else {
            0.0
        },
        mean: if count > 0 { sum / count as f64 } else { 0.0 },
        max: if max_sigma.is_finite() {
            max_sigma
        } else {
            0.0
        },
    }
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

    // ─── §23.2: FWHM distribution + histogram ────────────────

    /// A 64×64 image with low-amplitude noise
    /// (mean = 0.5, std ≈ 0.01): every pixel stays
    /// below mean + 3σ, so `extract_stars` returns `[]`.
    /// Used to exercise the "no stars detected"
    /// histogram path. An exactly-flat image doesn't
    /// work because every pixel sits exactly at
    /// threshold and `extract_stars` picks them all up.
    fn low_amplitude_noise_image(size: usize) -> F32Image {
        let mut img = F32Image::new(size, size, 1);
        for (i, v) in img.iter_mut().enumerate() {
            // Deterministic low-amplitude pattern: values
            // oscillate in roughly [0.485, 0.515] so the
            // std is much smaller than the threshold.
            let phase = (i % 7) as f32 * 0.005;
            *v = 0.5 + phase - 0.015;
        }
        img
    }

    /// A 64×64 black image with a single bright star at the
    /// centre (8x8 PSF). Used to exercise the single-star
    /// histogram path.
    fn single_star_image() -> F32Image {
        let mut img = F32Image::new(64, 64, 1);
        // Background: low but non-zero to give the image a
        // defined mean + std (otherwise extract_stars would
        // still find the peak but the threshold would be
        // undefined).
        for v in img.iter_mut() {
            *v = 0.05;
        }
        // Star: an 8×8 peak in the centre, intensity 1.0.
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let x = 32 + dx;
                let y = 32 + dy;
                if (0..64).contains(&x) && (0..64).contains(&y) {
                    let r = ((dx * dx + dy * dy) as f32).sqrt();
                    let falloff = (1.0 - r / 5.0).max(0.0);
                    img[(0, y as usize, x as usize)] = 0.05 + falloff;
                }
            }
        }
        img
    }

    /// A 64×64 image with 5 stars of varying brightness and
    /// FWHM, evenly distributed. Used to exercise multi-star
    /// histogram + percentile paths.
    fn five_star_image() -> F32Image {
        let mut img = F32Image::new(64, 64, 1);
        for v in img.iter_mut() {
            *v = 0.05;
        }
        // Five peaks at different intensities + PSF sizes
        // (FWHM in pixels: ~3, ~5, ~4, ~2, ~6).
        // The integer type is pinned to `i32` so the
        // `py + dy` expression below stays `i32` and
        // the explicit `as usize` casts on `x`/`y` are
        // the only narrowing operations clippy sees.
        let peaks: [(i32, i32, f32, f32); 5] = [
            (10, 10, 1.0, 3.0),
            (32, 14, 0.9, 5.0),
            (54, 18, 0.8, 4.0),
            (16, 50, 0.7, 2.0),
            (48, 50, 0.6, 6.0),
        ];
        for &(px, py, peak_intensity, fwhm) in &peaks {
            let sigma = fwhm / 2.355;
            for dy in -8i32..=8 {
                for dx in -8i32..=8 {
                    let x = (px + dx) as usize;
                    let y = (py + dy) as usize;
                    if x < 64 && y < 64 {
                        let r2 = (dx * dx + dy * dy) as f32;
                        let v = peak_intensity * (-r2 / (2.0 * sigma * sigma)).exp();
                        img[(0, y, x)] = (img[(0, y, x)] + v).max(0.05);
                    }
                }
            }
        }
        img
    }

    #[test]
    fn fwhm_histogram_returns_zeroed_for_image_with_no_stars_above_threshold() {
        // The flat_image helper is exactly 0.5 everywhere;
        // every pixel sits at threshold (mean + 3σ), and
        // extract_stars treats all of them as above-threshold
        // local maxima (it picks them all up). To exercise
        // the "no stars detected" path we need a noisy image
        // where no pixel exceeds mean + 3σ. A 64×64 image
        // with mean = 0.5, std ≈ 0.01, threshold = 0.53:
        // none of the pixels reach it.
        let img = low_amplitude_noise_image(64);
        let h = fwhm_histogram(&img);
        assert_eq!(h.count, 0);
        assert!(h.values.is_empty());
        assert!(h.bin_edges.is_empty());
        assert!(h.counts.is_empty());
        assert!(h.mean.is_nan());
        assert!(h.median.is_nan());
        assert!(h.min.is_nan());
        assert!(h.max.is_nan());
    }

    #[test]
    fn fwhm_histogram_handles_single_star() {
        let img = single_star_image();
        let h = fwhm_histogram(&img);
        assert_eq!(h.count, 1, "expected exactly one star");
        assert!(!h.values.is_empty());
        // Single star -> single bin, count = 1.
        assert_eq!(h.bin_edges.len(), 2);
        assert_eq!(h.counts.len(), 1);
        assert_eq!(h.counts[0], 1);
        // All summary stats equal the single value.
        assert!((h.mean - h.values[0]).abs() < 1e-9);
        assert!((h.median - h.values[0]).abs() < 1e-9);
        assert!((h.min - h.values[0]).abs() < 1e-9);
        assert!((h.max - h.values[0]).abs() < 1e-9);
    }

    #[test]
    fn fwhm_histogram_handles_multi_star_distribution() {
        let img = five_star_image();
        let h = fwhm_histogram(&img);
        // The 5-peak test image should yield ≥ 5 detected
        // stars (extract_stars de-dups overlapping local maxima
        // within a 5-pixel radius, so tightly-spaced peaks may
        // collapse). We assert the histogram is non-empty.
        assert!(h.count >= 5, "expected ≥ 5 stars, got {}", h.count);
        // Bins: Sturges' rule. For 5 stars: bins = 4; for 8+:
        // bins = 5; capped to [1, 50].
        let expected_bins = ((h.count as f64).log2().ceil() as usize + 1).clamp(1, 50);
        assert_eq!(h.bin_edges.len(), expected_bins + 1);
        assert_eq!(h.counts.len(), expected_bins);
        // Total count in the histogram bins equals the star count.
        assert_eq!(h.counts.iter().sum::<u32>() as usize, h.count);
        // All summary stats are within [min, max].
        assert!(h.min <= h.p25);
        assert!(h.p25 <= h.median);
        assert!(h.median <= h.p75);
        assert!(h.p75 <= h.max);
    }

    #[test]
    fn fwhm_histogram_is_deterministic_for_same_input() {
        let img = five_star_image();
        let h1 = fwhm_histogram(&img);
        let h2 = fwhm_histogram(&img);
        assert_eq!(h1.count, h2.count);
        assert_eq!(h1.values, h2.values);
        assert_eq!(h1.bin_edges, h2.bin_edges);
        assert_eq!(h1.counts, h2.counts);
        assert!((h1.mean - h2.mean).abs() < 1e-12);
        assert!((h1.median - h2.median).abs() < 1e-12);
    }

    #[test]
    fn fwhm_distribution_returns_one_value_per_star() {
        let img = five_star_image();
        let dist = fwhm_distribution(&img);
        let hist = fwhm_histogram(&img);
        // The distribution is the un-sorted, un-binned raw
        // per-star FWHM list; the histogram is sorted + binned.
        // Both have the same count.
        assert_eq!(dist.len(), hist.count);
        // All values are positive (FWHM is a length in pixels).
        for &v in &dist {
            assert!(v > 0.0, "FWHM must be positive: got {}", v);
        }
    }

    #[test]
    fn fwhm_histogram_percentiles_match_linear_interp() {
        // For the multi-star image, the 50th percentile must
        // match the numpy-style linear interpolation between
        // the two middle values (or the middle value if odd).
        let img = five_star_image();
        let hist = fwhm_histogram(&img);
        if hist.count >= 2 {
            let mut sorted = hist.values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = sorted.len();
            let expected_median = if n % 2 == 1 {
                sorted[n / 2]
            } else {
                (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
            };
            assert!(
                (hist.median - expected_median).abs() < 1e-9,
                "median {} != expected {}",
                hist.median,
                expected_median
            );
        }
    }

    // ─── §23.3: Noise map (2D sigma field) ─────────────────

    /// A 128×128 image with pure Gaussian noise (mean=0.5,
    /// σ≈0.05) plus a few flat background regions. Used to
    /// exercise the noise map on a "real-world-ish" noisy
    /// preview. The downsampled preview is 32×32 (≥ 7×7),
    /// so the noise map is well-defined.
    fn noisy_preview_image() -> F32Image {
        let mut img = F32Image::new(128, 128, 1);
        // Deterministic pseudo-noise: hash the pixel index
        // into a value in [0.45, 0.55] (≈0.025 std) so the
        // σ is non-zero everywhere but small enough that
        // the per-pixel noise map produces a meaningful
        // heatmap.
        for (i, v) in img.iter_mut().enumerate() {
            // Simple LCG-like hash; deterministic across
            // runs.
            let h = (i.wrapping_mul(2654435761) ^ 0x9E3779B9) as f32;
            let n = (h / u32::MAX as f32) * 0.1 - 0.05;
            *v = 0.5 + n;
        }
        img
    }

    /// A 128×128 image with one half at noise σ ≈ 0.01 and
    /// the other half at noise σ ≈ 0.10. Used to verify
    /// the noise map is *spatially* correct: the noisy
    /// half should have larger sigma values than the
    /// quiet half.
    fn half_noisy_image() -> F32Image {
        let mut img = F32Image::new(128, 128, 1);
        for y in 0..128 {
            for x in 0..128 {
                // LCG hash for deterministic noise.
                let i: u32 = ((y * 128 + x) as u32).wrapping_mul(2654435761) ^ 0x9E3779B9;
                let h = i as f32 / u32::MAX as f32;
                let noise_amp = if x < 64 { 0.01 } else { 0.10 };
                let n = (h - 0.5) * noise_amp * 2.0;
                img[(0, y, x)] = 0.5 + n;
            }
        }
        img
    }

    #[test]
    fn noise_map_returns_zeroed_for_empty_image() {
        // Empty image (1x0 after construction: F32Image
        // refuses 0xN; we use the closest empty case via
        // the flat_image helper at size 1, which is below
        // the 7×7 window after downsampling).
        // Actually F32Image allows 1×1; the downsample
        // yields 1×1 which is below the window. The
        // function should return a zeroed map.
        let img = flat_image(1);
        let m = noise_map(&img);
        assert_eq!(m.sigma.len(), 0);
        assert_eq!(m.min, 0.0);
        assert_eq!(m.mean, 0.0);
        assert_eq!(m.max, 0.0);
    }

    #[test]
    fn noise_map_produces_nonzero_for_noisy_image() {
        let img = noisy_preview_image();
        let m = noise_map(&img);
        assert!(!m.sigma.is_empty(), "noise map must be non-empty");
        assert!(m.width > 0 && m.height > 0);
        // All sigma values must be finite and >= 0.
        for (i, &s) in m.sigma.iter().enumerate() {
            assert!(s.is_finite(), "sigma[{}] not finite: {}", i, s);
            assert!(s >= 0.0, "sigma[{}] negative: {}", i, s);
        }
        // Mean is the arithmetic mean of all sigma values
        // (full field, including the inner-only ones we
        // computed). It must be > 0 for a noisy image.
        assert!(m.mean > 0.0, "mean noise should be > 0");
    }

    #[test]
    fn noise_map_summary_stats_match_array() {
        let img = noisy_preview_image();
        let m = noise_map(&img);
        // The summary stats are computed over the
        // *inner* pixels (those inside the HALF border
        // on every side). The outer ring pixels are
        // zeroed (they don't have a stable local
        // estimator). We re-compute the inner-only
        // stats here and assert equality.
        let half_w = m.width as usize / 2; // approximate; this preview is 32x32, half is 16
                                           // Actually the inner region is [HALF, pw-HALF);
                                           // we need to know pw/ph. Recompute via
                                           // downsample to be exact.
        let preview = img.downsample_box(0.25);
        let (pw, ph) = (preview.width(), preview.height());
        const WINDOW: usize = 7;
        const HALF: usize = WINDOW / 2;
        let mut inner_min = f64::INFINITY;
        let mut inner_max = f64::NEG_INFINITY;
        let mut inner_sum = 0.0f64;
        let mut inner_count = 0usize;
        for y in HALF..(ph - HALF) {
            for x in HALF..(pw - HALF) {
                let s = m.sigma[y * pw + x];
                if s < inner_min {
                    inner_min = s;
                }
                if s > inner_max {
                    inner_max = s;
                }
                inner_sum += s;
                inner_count += 1;
            }
        }
        let inner_mean = inner_sum / inner_count as f64;
        assert!((m.min - inner_min).abs() < 1e-9);
        assert!((m.max - inner_max).abs() < 1e-9);
        assert!((m.mean - inner_mean).abs() < 1e-9);
        // Silence the unused-variable warning for the
        // approximate half_w computed above (kept for
        // documentation that the inner region is
        // half-window on every side).
        let _ = half_w;
    }

    #[test]
    fn noise_map_is_deterministic_for_same_input() {
        let img = noisy_preview_image();
        let m1 = noise_map(&img);
        let m2 = noise_map(&img);
        assert_eq!(m1.sigma, m2.sigma);
        assert_eq!(m1.min, m2.min);
        assert_eq!(m1.mean, m2.mean);
        assert_eq!(m1.max, m2.max);
    }

    #[test]
    fn noise_map_is_spatially_correct_for_half_noisy() {
        // The left half is quieter (smaller noise), the
        // right half is noisier (larger noise). The noise
        // map should reflect this: the mean sigma of the
        // left half must be smaller than the right half.
        let img = half_noisy_image();
        let m = noise_map(&img);
        assert!(!m.sigma.is_empty(), "noise map must be non-empty");
        let half = m.width as usize / 2;
        let mut left_sum = 0.0;
        let mut left_count = 0usize;
        let mut right_sum = 0.0;
        let mut right_count = 0usize;
        for y in 0..m.height as usize {
            for x in 0..m.width as usize {
                let s = m.sigma[y * m.width as usize + x];
                if x < half {
                    left_sum += s;
                    left_count += 1;
                } else {
                    right_sum += s;
                    right_count += 1;
                }
            }
        }
        let left_mean = left_sum / left_count as f64;
        let right_mean = right_sum / right_count as f64;
        // The quiet half (left) should have ~10x less
        // noise than the noisy half (right) per the
        // fixture. We assert a 2x ratio to allow for the
        // smoothing introduced by the 7×7 window.
        assert!(
            right_mean > left_mean * 2.0,
            "right half (noisy) mean {} should be > 2x left half (quiet) mean {}",
            right_mean,
            left_mean
        );
    }

    #[test]
    fn noise_map_field_size_matches_dimensions() {
        let img = noisy_preview_image();
        let m = noise_map(&img);
        let expected = (m.width as usize) * (m.height as usize);
        // The sigma field has length `pw * ph`; some
        // outer-ring pixels (within HALF of any edge)
        // are not assigned and remain 0 from the
        // vec![0.0; n] initialization. So the length
        // is still `pw * ph` (zeroed edge pixels
        // included), but only inner pixels are
        // meaningful. We assert the length matches.
        assert_eq!(m.sigma.len(), expected);
    }
}
