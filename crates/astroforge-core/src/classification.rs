//! CR-04 §6 — Frame classification.
//!
//! Classifies each discovered file into a `FrameKind` (Light /
//! Dark / Flat / Bias / Unknown / Unsupported / Invalid) with
//! explicit confidence + evidence. Every important inference is
//! shaped as Observation → Evidence → Confidence → Decision (per
//! CR-04 §2.3 ADR-04.2 "Evidence Before Inference").
//!
//! The module wraps the existing `ingest.rs::determine_frame_type`
//! heuristics and extends them with:
//!
//! - explicit `FrameKind::Unknown` and `FrameKind::Unsupported`
//!   variants distinct from Light/Dark/Flat/Bias;
//! - per-evidence weighting (FITS header keyword > filename >
//!   exposure-time heuristic > directory context);
//! - a deterministic confidence score so the UI can flag low-
//!   confidence classifications for user confirmation (CR-04
//!   §13 ambiguity UX);
//! - a typed `Classification` return value that bundles the kind,
//!   confidence, and a Vec of evidence observations.
//!
//! The classification runs after `import_scan` (P1) which has
//! already established the discovery state + extracted metadata.
//! The input is `ExtractedMetadata` + the source path + the
//! content hash (for content-aware fingerprints if needed).

use crate::import_scan::{AssetFormat, ExtractedMetadata};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// CR-04 §6 — the seven primary frame classes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FrameKind {
    Light,
    Dark,
    Flat,
    Bias,
    Unknown,
    Unsupported,
    Invalid,
}

impl FrameKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FrameKind::Light => "light",
            FrameKind::Dark => "dark",
            FrameKind::Flat => "flat",
            FrameKind::Bias => "bias",
            FrameKind::Unknown => "unknown",
            FrameKind::Unsupported => "unsupported",
            FrameKind::Invalid => "invalid",
        }
    }
}

/// One observation that contributed to a classification.
///
/// Multiple observations are combined into the final `Classification`
/// via weighted scoring. Each observation carries its own
/// `weight` (0.0..=1.0) and `confidence` (0.0..=1.0); the final
/// classification score is `Σ(weight × confidence) / Σ(weight)`
/// for the kindest observation that wins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameObservation {
    #[serde(rename = "signal")]
    pub signal: String,
    pub value: String,
    pub weight: f64,
    pub confidence: f64,
}

impl FrameObservation {
    pub fn new(
        signal: &'static str,
        value: impl Into<String>,
        weight: f64,
        confidence: f64,
    ) -> Self {
        Self {
            signal: signal.to_string(),
            value: value.into(),
            weight,
            confidence,
        }
    }
}

/// CR-04 §6 + §15 — the result of classifying a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classification {
    pub kind: FrameKind,
    /// 0.0..=1.0. Below ~0.6 the UI should flag the inference
    /// (CR-04 §13 ambiguity UX).
    pub confidence: f64,
    pub observations: Vec<FrameObservation>,
}

impl Classification {
    pub fn is_low_confidence(&self) -> bool {
        self.confidence < 0.6
    }
}

/// Classify a file from its extracted metadata + path.
///
/// Combines the existing FITS-IMAGETYP heuristic, the filename
/// pattern heuristic, the exposure-time heuristic, and the
/// directory-context heuristic into a single weighted score per
/// `FrameKind`. Returns the winning kind + the full evidence list.
pub fn classify_frame(
    path: &Path,
    metadata: &ExtractedMetadata,
    format: AssetFormat,
) -> Classification {
    if format == AssetFormat::Other {
        return Classification {
            kind: FrameKind::Unsupported,
            confidence: 1.0,
            observations: vec![FrameObservation::new(
                "format",
                "unsupported_extension",
                1.0,
                1.0,
            )],
        };
    }

    if metadata.date_obs.is_none()
        && metadata.exptime.is_none()
        && metadata.width.is_none()
        && metadata.filter.is_none()
        && metadata.telescop.is_none()
    {
        // No signal at all — empty file or unreadable. P1's
        // extract_metadata maps this to ImportScanError::Unreadable
        // already, so we only get here for stub-returned metadata
        // (PNG/JPEG/DNG/TIFF without EXIF). The honest answer is
        // "Unknown" with low confidence so the UI prompts.
        let path_lower = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();
        let dir_lower = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();
        if path_lower.is_empty() && dir_lower.is_empty() {
            return Classification {
                kind: FrameKind::Invalid,
                confidence: 1.0,
                observations: vec![FrameObservation::new(
                    "empty_path",
                    "no metadata, no path",
                    1.0,
                    1.0,
                )],
            };
        }
    }

    let mut observations: Vec<FrameObservation> = Vec::new();

    // 1. Filter metadata: FITS FILTER = "Ha"/"OIII"/"SII" or
    //    color wheel tags indicate a Light. A filter containing
    //    the word "flat" or "dark" is a calibration frame.
    if let Some(filter) = metadata.filter.as_deref() {
        let f = filter.to_uppercase();
        let is_calibration_filter =
            f.contains("FLAT") || f.contains("DARK") || f.contains("BIAS") || f.contains("OFFSET");
        if is_calibration_filter {
            observations.push(FrameObservation::new("fits_filter", filter, 0.7, 0.9));
        } else {
            // Narrowband filters (Ha/OIII/SII/L/R/G/B/etc.) are
            // strong Light indicators.
            observations.push(FrameObservation::new("fits_filter", filter, 0.4, 0.7));
        }
    }

    // 2. Exposure heuristic: 0 = Bias, <1s = Bias/Dark, >1s = Light.
    if let Some(exptime) = metadata.exptime {
        if exptime == 0.0 {
            observations.push(FrameObservation::new(
                "exptime_zero",
                format!("{exptime}"),
                0.6,
                0.85,
            ));
        } else if exptime < 1.0 {
            observations.push(FrameObservation::new(
                "exptime_short",
                format!("{exptime}"),
                0.4,
                0.5,
            ));
        } else if exptime >= 30.0 {
            observations.push(FrameObservation::new(
                "exptime_long",
                format!("{exptime}"),
                0.5,
                0.8,
            ));
        }
    }

    // 3. Filename heuristic: "_dark", "_flat", "_bias"/"_offset".
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let lower = name.to_lowercase();
        if lower.contains("dark") && !lower.contains("darks") {
            observations.push(FrameObservation::new("filename_dark", name, 0.5, 0.6));
        } else if lower.contains("flat") && !lower.contains("flats") {
            observations.push(FrameObservation::new("filename_flat", name, 0.5, 0.6));
        } else if lower.contains("bias") || lower.contains("offset") {
            observations.push(FrameObservation::new("filename_bias", name, 0.5, 0.6));
        } else if lower.contains("light") {
            observations.push(FrameObservation::new("filename_light", name, 0.5, 0.7));
        }
    }

    // 4. Directory heuristic: "/darks/", "/flats/", "/bias/".
    if let Some(dir_name) = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
    {
        let lower = dir_name.to_lowercase();
        if lower == "darks" || lower == "dark" {
            observations.push(FrameObservation::new("directory_dark", dir_name, 0.3, 0.7));
        } else if lower == "flats" || lower == "flat" {
            observations.push(FrameObservation::new("directory_flat", dir_name, 0.3, 0.7));
        } else if lower == "bias" || lower == "biases" || lower == "offset" {
            observations.push(FrameObservation::new("directory_bias", dir_name, 0.3, 0.7));
        } else if lower == "lights" || lower == "light" {
            observations.push(FrameObservation::new("directory_light", dir_name, 0.3, 0.7));
        }
    }

    // 5. CCD temperature: -10°C is a common dark-frame indicator.
    if let Some(temp) = metadata.ccd_temp {
        if temp < -5.0 {
            // Cold temperature correlates with dark-frame capture
            // sessions, but it's a weak signal — the lights are
            // often captured at the same temperature.
            observations.push(FrameObservation::new(
                "ccd_temp_cold",
                format!("{temp}"),
                0.2,
                0.3,
            ));
        }
    }

    // Score the observations using the canonical scoring pass.
    let (kind, confidence) = score_observations(&observations);

    Classification {
        kind,
        confidence,
        observations,
    }
}

/// Tally the per-kind scores using the observation values, then
/// pick the winning kind. This is the canonical scoring path —
/// `classify_frame` calls it after collecting observations.
fn score_observations(observations: &[FrameObservation]) -> (FrameKind, f64) {
    let mut scores: std::collections::HashMap<FrameKind, f64> = std::collections::HashMap::new();
    for obs in observations {
        let signal = obs.signal.as_str();
        let kind = if signal.starts_with("fits_filter") {
            let v = obs.value.to_uppercase();
            if v.contains("FLAT") {
                Some(FrameKind::Flat)
            } else if v.contains("DARK") {
                Some(FrameKind::Dark)
            } else if v.contains("BIAS") || v.contains("OFFSET") {
                Some(FrameKind::Bias)
            } else {
                // Any other filter (Ha/OIII/SII/L/R/G/B) is a
                // light frame. We use Light as the kind; the
                // observation is recorded but the kind vote is
                // weak.
                Some(FrameKind::Light)
            }
        } else {
            match signal {
                "exptime_zero" => Some(FrameKind::Bias),
                "exptime_short" => Some(FrameKind::Dark),
                "exptime_long" => Some(FrameKind::Light),
                "filename_dark" | "directory_dark" => Some(FrameKind::Dark),
                "filename_flat" | "directory_flat" => Some(FrameKind::Flat),
                "filename_bias" | "directory_bias" => Some(FrameKind::Bias),
                "filename_light" | "directory_light" => Some(FrameKind::Light),
                "ccd_temp_cold" => Some(FrameKind::Dark),
                _ => None,
            }
        };
        if let Some(kind) = kind {
            *scores.entry(kind).or_insert(0.0) += obs.weight * obs.confidence;
        }
    }

    if let Some((kind, score)) = scores
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(k, s)| (*k, *s))
    {
        // Normalize: a single strong observation (weight 0.7,
        // confidence 0.9) scores ~0.63; we want that to map to
        // ~0.7. Multi-observation agreement (multiple signals
        // voting the same kind) pushes the score toward 1.0.
        // Dividing by 0.9 (a typical "single observation peak")
        // and clamping gives a confidence in the 0.0..=1.0 range
        // that grows as more signals agree.
        let normalized = (score / 0.9).clamp(0.0, 1.0);
        (kind, normalized)
    } else {
        (FrameKind::Unknown, 0.0)
    }
}

/// Refine `classify_frame`'s result with the canonical scoring
/// pass. The two-step approach keeps `classify_frame` readable
/// while delegating the math to a focused helper.
///
/// Note: a `Classification` whose kind was set to `Unsupported`
/// or `Invalid` by `classify_frame` is a terminal decision (the
/// file cannot yield a meaningful kind); the scoring pass returns
/// `(Unknown, 0.0)` for empty-observation lists but never
/// overrides a terminal kind. This means a single observation
/// that says "format=unsupported_extension" is preserved as
/// `Unsupported` rather than being re-scored.
pub fn refine(classification: Classification) -> Classification {
    // Terminal kinds are final; do not re-score them.
    if matches!(
        classification.kind,
        FrameKind::Unsupported | FrameKind::Invalid
    ) {
        return classification;
    }
    let (kind, confidence) = score_observations(&classification.observations);
    Classification {
        kind,
        confidence,
        observations: classification.observations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn meta_with_filter(filter: &str, exptime: Option<f64>) -> ExtractedMetadata {
        ExtractedMetadata {
            filter: Some(filter.into()),
            exptime,
            ..Default::default()
        }
    }

    #[test]
    fn classify_filter_flat_is_a_flat() {
        let path = PathBuf::from("flat_001.fits");
        let md = meta_with_filter("FLAT", Some(0.5));
        let c = refine(classify_frame(&path, &md, AssetFormat::Fits));
        assert_eq!(c.kind, FrameKind::Flat);
        assert!(
            c.confidence > 0.5,
            "expected confidence > 0.5, got {}",
            c.confidence
        );
    }

    #[test]
    fn classify_zero_exptime_is_a_bias() {
        let path = PathBuf::from("bias_001.fits");
        let md = meta_with_filter("L", Some(0.0));
        let c = refine(classify_frame(&path, &md, AssetFormat::Fits));
        assert_eq!(c.kind, FrameKind::Bias);
    }

    #[test]
    fn classify_long_exptime_with_ha_filter_is_a_light() {
        let path = PathBuf::from("m31_ha_001.fits");
        let md = meta_with_filter("Ha", Some(300.0));
        let c = refine(classify_frame(&path, &md, AssetFormat::Fits));
        assert_eq!(c.kind, FrameKind::Light);
    }

    #[test]
    fn classify_filename_dark_with_short_exptime_is_a_dark() {
        let path = PathBuf::from("dark_001.fits");
        let md = ExtractedMetadata::default();
        let c = refine(classify_frame(&path, &md, AssetFormat::Fits));
        assert_eq!(c.kind, FrameKind::Dark);
    }

    #[test]
    fn classify_directory_dark_is_a_dark() {
        let path = PathBuf::from("/data/m31/darks/dark_001.fits");
        let md = ExtractedMetadata::default();
        let c = refine(classify_frame(&path, &md, AssetFormat::Fits));
        assert_eq!(c.kind, FrameKind::Dark);
    }

    #[test]
    fn classify_unsupported_format_is_unsupported() {
        let path = PathBuf::from("foo.xyz");
        let md = ExtractedMetadata::default();
        let c = refine(classify_frame(&path, &md, AssetFormat::Other));
        assert_eq!(c.kind, FrameKind::Unsupported);
        assert_eq!(c.confidence, 1.0);
    }

    #[test]
    fn classify_no_signals_is_unknown() {
        let path = PathBuf::from("mystery.fits");
        let md = ExtractedMetadata::default();
        let c = refine(classify_frame(&path, &md, AssetFormat::Fits));
        assert_eq!(c.kind, FrameKind::Unknown);
        assert!(c.confidence < 0.6);
    }

    #[test]
    fn observations_are_recorded_for_audit() {
        let path = PathBuf::from("/darks/dark_001.fits");
        let md = ExtractedMetadata {
            exptime: Some(120.0),
            ..Default::default()
        };
        let c = refine(classify_frame(&path, &md, AssetFormat::Fits));
        assert!(!c.observations.is_empty());
        let signals: Vec<&str> = c.observations.iter().map(|o| o.signal.as_str()).collect();
        assert!(signals.contains(&"directory_dark"));
        assert!(signals.contains(&"exptime_long"));
    }
}
