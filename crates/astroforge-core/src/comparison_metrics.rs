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
}
