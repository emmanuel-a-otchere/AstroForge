//! CR-06 P2 — `ImageAnalysisReport`.
//!
//! Aggregates metrics + structures + defects into a single
//! report with per-observation evidence and confidence.
//! The recommendation engine (P3) reads this report to
//! drive the "Observation → Evidence → Confidence →
//! Recommendation" contract from CR-06 §6 / §7.

use serde::{Deserialize, Serialize};

use crate::image::F32Image;

use super::defects::{hot_pixels, trail_artifacts, DefectSample};
use super::metrics::{
    background_gradient, chromatic_noise, highlight_clipping, local_contrast, luminance_noise,
    Confidence, MetricsSample,
};
use super::structures::{bright_core, faint_structures, star_count, StructureSample};

/// One observation: the named characteristic, the
/// measurement, the supporting evidence region, and the
/// confidence score.
///
/// The wire shape matches CR-06 §5.1 / §6: every report
/// field carries observation + value + confidence, plus an
/// optional evidence region so the UI can highlight the
/// source of the measurement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub name: String,
    /// Binned value (e.g. `"high"`, `"moderate"`,
    /// `"gradient"`). The numeric value lives in the
    /// sibling `value` field so the UI can render either.
    pub label: String,
    pub value: f64,
    pub evidence_region: Option<[u32; 4]>,
    pub confidence: Confidence,
}

impl Observation {
    pub fn confidence_score(&self) -> f32 {
        self.confidence.as_score()
    }
}

/// The full report — what P3 reads.
///
/// The shape is a flat list of `observations` plus a few
/// structured sub-fields (`star_count`, `nebula_count`)
/// that the recommendation engine keys on. The flat list
/// is the UI's source of truth; the structured fields are
/// optimisation shortcuts for the recommendation engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageAnalysisReport {
    pub image_version_id: String,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub observations: Vec<Observation>,
    pub star_count: u32,
    pub faint_structure_count: u32,
    pub has_bright_core: bool,
    pub hot_pixel_count: u32,
    pub trail_artifact_count: u32,
    /// Engine version that produced this report.
    pub engine_version: String,
    /// Generated-at ISO timestamp.
    pub created_at: String,
}

impl ImageAnalysisReport {
    /// Render the report as a JSON string suitable for
    /// the `image_analyses.profile_json` column. The
    /// schema version is `1` for CR-06 P2; future
    /// iterations will bump this string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }
}

fn obs(name: &str, sample: &MetricsSample, label: &str) -> Observation {
    Observation {
        name: name.into(),
        label: label.into(),
        value: sample.value,
        evidence_region: sample.evidence_region,
        confidence: sample.confidence,
    }
}

fn structure_obs(name: &str, sample: &StructureSample, label: &str) -> Observation {
    Observation {
        name: name.into(),
        label: label.into(),
        value: sample.count as f64,
        evidence_region: sample
            .centroid
            .map(|[x, y]| [x.saturating_sub(16), y.saturating_sub(16), 32, 32]),
        confidence: sample.confidence,
    }
}

fn defect_obs(name: &str, sample: &DefectSample, label: &str) -> Observation {
    Observation {
        name: name.into(),
        label: label.into(),
        value: sample.count as f64,
        evidence_region: None,
        confidence: sample.confidence,
    }
}

/// The analyzer entry point. Runs every metric, structure,
/// and defect detector over the input and aggregates them
/// into an `ImageAnalysisReport`.
///
/// Deterministic: two calls with identical inputs produce
/// identical reports (no clock / thread state). The
/// `engine_version` and `created_at` fields are the only
/// non-deterministic fields; they are caller-controlled
/// via [`analyze_at`] if a stable timestamp is needed for
/// reproducibility tests.
pub fn analyze(image: &F32Image, image_version_id: &str) -> ImageAnalysisReport {
    let now = current_iso_timestamp();
    analyze_at(image, image_version_id, env!("CARGO_PKG_VERSION"), &now)
}

pub fn analyze_at(
    image: &F32Image,
    image_version_id: &str,
    engine_version: &str,
    created_at: &str,
) -> ImageAnalysisReport {
    let noise = luminance_noise(image);
    let bg = background_gradient(image);
    let clip = highlight_clipping(image);
    let chroma = chromatic_noise(image);
    let contrast = local_contrast(image);

    let stars = star_count(image);
    let faint = faint_structures(image);
    let core = bright_core(image);

    let hot = hot_pixels(image);
    let trails = trail_artifacts(image);

    let observations = vec![
        obs("noise", &noise, &noise_label(noise.value, noise.confidence)),
        obs(
            "background",
            &bg,
            &background_label(bg.value, bg.confidence),
        ),
        obs("clipping", &clip, &clipping_label(clip.value)),
        obs("chromatic_noise", &chroma, &chroma_label(chroma.value)),
        obs("local_contrast", &contrast, &contrast_label(contrast.value)),
        structure_obs("stars", &stars, &format!("{} stars detected", stars.count)),
        structure_obs(
            "nebula",
            &faint,
            &format!("{} faint-structure components", faint.count),
        ),
        structure_obs(
            "bright_core",
            &core,
            if core.count > 0 {
                "Bright core present"
            } else {
                "No bright core"
            },
        ),
        defect_obs("hot_pixels", &hot, &format!("{} hot pixels", hot.count)),
        defect_obs(
            "trails",
            &trails,
            &format!("{} trail artifacts", trails.count),
        ),
    ];

    ImageAnalysisReport {
        image_version_id: image_version_id.to_string(),
        width: image.width() as u32,
        height: image.height() as u32,
        channels: image.channels() as u32,
        observations,
        star_count: stars.count,
        faint_structure_count: faint.count,
        has_bright_core: core.count > 0,
        hot_pixel_count: hot.count,
        trail_artifact_count: trails.count,
        engine_version: engine_version.into(),
        created_at: created_at.into(),
    }
}

fn noise_label(value: f64, confidence: Confidence) -> String {
    let _ = confidence;
    if value < 0.005 {
        "Low".into()
    } else if value < 0.02 {
        "Moderate".into()
    } else if value < 0.05 {
        "High".into()
    } else {
        "Extreme".into()
    }
}

fn background_label(value: f64, confidence: Confidence) -> String {
    let _ = confidence;
    if value < 5.0 {
        "Flat background".into()
    } else if value < 20.0 {
        "Mild gradient".into()
    } else if value < 50.0 {
        "Moderate gradient".into()
    } else {
        "Strong gradient".into()
    }
}

fn clipping_label(value: f64) -> String {
    if value < 0.001 {
        "No clipping".into()
    } else if value < 0.01 {
        "Mild clipping".into()
    } else if value < 0.05 {
        "Moderate clipping".into()
    } else {
        "Heavy clipping".into()
    }
}

fn chroma_label(value: f64) -> String {
    if value < 0.02 {
        "Balanced colour".into()
    } else if value < 0.05 {
        "Slight chromatic noise".into()
    } else if value < 0.10 {
        "Moderate chromatic noise".into()
    } else {
        "Strong chromatic noise".into()
    }
}

fn contrast_label(value: f64) -> String {
    if value < 0.01 {
        "Flat".into()
    } else if value < 0.05 {
        "Low contrast".into()
    } else if value < 0.10 {
        "Good contrast".into()
    } else {
        "High contrast".into()
    }
}

/// Best-effort ISO-8601 UTC timestamp without external
/// dependencies. Returns "1970-01-01T00:00:00Z" on
/// platforms where `time` is unavailable (e.g. wasm32);
/// callers needing strict accuracy should pass an
/// explicit timestamp via [`analyze_at`].
fn current_iso_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // UTC date conversion without external deps. The
    // algorithm is the standard "days-since-1970 → Y-M-D"
    // from Howard Hinnant's date library.
    let s = secs as i64;
    let z = s / 86400;
    let s2 = s - z * 86400;
    let h = s2 / 3600;
    let m = (s2 - h * 3600) / 60;
    let sec = s2 - h * 3600 - m * 60;
    let z2 = z + 719468;
    let era = if z2 >= 0 { z2 } else { z2 - 146096 } / 146097;
    let doe = (z2 - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mo = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, m, sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_uniform_image_is_well_formed() {
        let img = F32Image::new(128, 128, 1);
        let report = analyze_at(&img, "ver_uniform", "test-0.1.0", "2026-09-10T00:00:00Z");
        assert_eq!(report.width, 128);
        assert_eq!(report.height, 128);
        assert_eq!(report.star_count, 0);
        assert_eq!(report.faint_structure_count, 0);
        assert!(!report.has_bright_core);
        // 10 observations: noise, background, clipping,
        // chromatic_noise, local_contrast, stars, nebula,
        // bright_core, hot_pixels, trails.
        assert_eq!(report.observations.len(), 10);
    }

    #[test]
    fn analyze_serializes_to_json_and_round_trips() {
        let img = F32Image::new(64, 64, 1);
        let report = analyze_at(&img, "ver_1", "test-0.1.0", "2026-09-10T00:00:00Z");
        let json = report.to_json().expect("to_json");
        let back: ImageAnalysisReport = ImageAnalysisReport::from_json(&json).expect("from_json");
        assert_eq!(report, back);
    }

    #[test]
    fn analyze_detects_simulated_stars() {
        let mut img = F32Image::new(128, 128, 1);
        img[(0, 32, 32)] = 1.0;
        img[(0, 64, 64)] = 1.0;
        img[(0, 96, 96)] = 1.0;
        let report = analyze_at(&img, "ver_stars", "test-0.1.0", "2026-09-10T00:00:00Z");
        assert!(report.star_count >= 1, "expected at least one star");
    }

    #[test]
    fn analyze_detects_bright_core() {
        // The blob is large enough that, after box
        // downsampling, the blob pixels remain far above
        // the rest of the image's `mean + 8σ` threshold.
        let mut img = F32Image::new(256, 256, 1);
        for y in 80..176 {
            for x in 80..176 {
                img[(0, y, x)] = 1.0;
            }
        }
        let report = analyze_at(&img, "ver_core", "test-0.1.0", "2026-09-10T00:00:00Z");
        assert!(report.has_bright_core, "{:?}", report);
    }

    #[test]
    fn analyze_is_deterministic() {
        let mut img = F32Image::new(128, 128, 1);
        img[(0, 32, 32)] = 1.0;
        img[(0, 64, 64)] = 1.0;
        let r1 = analyze_at(&img, "v", "test-0.1.0", "2026-09-10T00:00:00Z");
        let r2 = analyze_at(&img, "v", "test-0.1.0", "2026-09-10T00:00:00Z");
        // Ignore the `created_at` field; everything else
        // must match for a fixed input.
        let mut r1_no_ts = r1.clone();
        r1_no_ts.created_at.clear();
        let mut r2_no_ts = r2.clone();
        r2_no_ts.created_at.clear();
        assert_eq!(r1_no_ts, r2_no_ts);
    }
}
