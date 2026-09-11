//! CR-06 P4 — AI Enhancement Operations registry.
//!
//! P4 ships the operation layer that:
//!
//! - defines the registry of supported operations
//!   (`denoise_luminance`, `denoise_chrominance`,
//!   `deconv`, `star_refine`, `star_reduce`, `detail_enhance`,
//!   `super_resolution`, `inpaint`, `background_cleanup`,
//!   `hot_pixel_clean`, `trail_clean`);
//! - pins the canonical `OperationParameter` schema for
//!   each;
//! - carries the `SafetyClassification` and human-readable
//!   metadata the Studio UI surfaces;
//! - exposes a pure `OperationOutcome` type the apply
//!   round persists into the `AiOperation` row.
//!
//! The actual pixel transformation for the model-backed
//! operations (`denoise`, `sr`, `inpaint`, etc.) lands in
//! P5 with the real ONNX dispatch. P4 deliberately
//! exposes a `dispatch()` function whose body returns a
//! `PassthroughOutcome` for every operation, so the
//! end-to-end apply + branch + stack-reorder flow is
//! exercisable on a CI runner without a GPU. The
//! substitution is local: P5 replaces the body, the
//! signatures + state machine stay.
//!
//! This module does NOT live in `astroforge-core` because
//! the operations registry is the AI crate's domain —
//! `astroforge-core` carries the data model + stack engine
//! but the operations themselves are AI-side.

use serde::{Deserialize, Serialize};

/// One operation's canonical metadata. The Studio UI
/// reads this list to render the per-operation card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationInfo {
    pub operation_id: String,
    pub display_name: String,
    pub category: OperationCategory,
    pub safety_classification: SafetyClassification,
    pub description: String,
    pub default_parameters_json: String,
    /// Operations marked `requires_region` cannot run
    /// without a mask (`inpaint` is the canonical
    /// example). The Studio UI greys the Apply button
    /// out when no mask is selected.
    #[serde(default)]
    pub requires_region: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationCategory {
    /// Pre-denoise cleanups (deterministic).
    Cleanup,
    /// Noise / chromatic reduction.
    Denoise,
    /// Spatial / deconvolution restoration.
    Restoration,
    /// Star / astronomical structure refinement.
    Star,
    /// Detail / structure recovery.
    Detail,
    /// Super-resolution / generative upscale.
    Upscale,
    /// Inpainting / region-restricted replacement.
    Inpaint,
    /// Background / sky gradient operations.
    Background,
}

impl OperationCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationCategory::Cleanup => "cleanup",
            OperationCategory::Denoise => "denoise",
            OperationCategory::Restoration => "restoration",
            OperationCategory::Star => "star",
            OperationCategory::Detail => "detail",
            OperationCategory::Upscale => "upscale",
            OperationCategory::Inpaint => "inpaint",
            OperationCategory::Background => "background",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyClassification {
    /// No generative step; output is reproducible.
    Deterministic,
    /// Perceptual enhancement (sharpening, denoise);
    /// pixel values are modified but no new structure
    /// is invented.
    Perceptual,
    /// Generative step (SR, inpaint, trail fill);
    /// output may include synthesised structure.
    Generative,
}

impl SafetyClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyClassification::Deterministic => "deterministic",
            SafetyClassification::Perceptual => "perceptual",
            SafetyClassification::Generative => "generative",
        }
    }
}

/// Return the full operations registry. The list is the
/// authoritative one — every UI card, every apply
/// round, every recommendation-to-operation mapping
/// reads from it.
pub fn registry() -> Vec<OperationInfo> {
    vec![
        OperationInfo {
            operation_id: "background_cleanup".into(),
            display_name: "Background Cleanup".into(),
            category: OperationCategory::Background,
            safety_classification: SafetyClassification::Deterministic,
            description: "Remove the sky gradient so subsequent operations read a flat background.".into(),
            default_parameters_json: r#"{"strength":1.0}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "hot_pixel_clean".into(),
            display_name: "Hot Pixel Cleanup".into(),
            category: OperationCategory::Cleanup,
            safety_classification: SafetyClassification::Deterministic,
            description: "Remove single-pixel defects detected by the analyzer.".into(),
            default_parameters_json: r#"{"threshold":0.95}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "trail_clean".into(),
            display_name: "Trail Cleanup".into(),
            category: OperationCategory::Inpaint,
            safety_classification: SafetyClassification::Generative,
            description: "Inpaint satellite / aeroplane trails. Generative — the trail pixels are synthesised.".into(),
            default_parameters_json: r#"{"strength":1.0}"#.into(),
            requires_region: true,
        },
        OperationInfo {
            operation_id: "denoise_luminance".into(),
            display_name: "Luminance Denoise".into(),
            category: OperationCategory::Denoise,
            safety_classification: SafetyClassification::Perceptual,
            description: "Reduce luminance noise while preserving fine structure.".into(),
            default_parameters_json: r#"{"strength":0.7,"detail_preservation":0.6,"tile_size":512}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "denoise_chrominance".into(),
            display_name: "Chrominance Denoise".into(),
            category: OperationCategory::Denoise,
            safety_classification: SafetyClassification::Perceptual,
            description: "Reduce color speckle while preserving saturation on stars.".into(),
            default_parameters_json: r#"{"strength":0.6,"tile_size":512}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "deconv".into(),
            display_name: "Deconvolution".into(),
            category: OperationCategory::Restoration,
            safety_classification: SafetyClassification::Deterministic,
            description: "Restore spatial detail via iterative PSF-aware deconvolution.".into(),
            default_parameters_json: r#"{"iterations":20,"psf_kernels":3}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "star_refine".into(),
            display_name: "Star Refinement".into(),
            category: OperationCategory::Star,
            safety_classification: SafetyClassification::Perceptual,
            description: "Sharpen star profiles and tighten FWHM without amplifying noise.".into(),
            default_parameters_json: r#"{"strength":0.5,"protect_color":true}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "star_reduce".into(),
            display_name: "Star Reduction".into(),
            category: OperationCategory::Star,
            safety_classification: SafetyClassification::Perceptual,
            description: "Reduce star size and brightness to expose faint nebular structure.".into(),
            default_parameters_json: r#"{"strength":0.4}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "detail_enhance".into(),
            display_name: "Detail Enhancement".into(),
            category: OperationCategory::Detail,
            safety_classification: SafetyClassification::Perceptual,
            description: "Recover local contrast in faint structures. Should run after denoise.".into(),
            default_parameters_json: r#"{"strength":0.5,"tile_size":512}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "super_resolution".into(),
            display_name: "2× Super Resolution".into(),
            category: OperationCategory::Upscale,
            safety_classification: SafetyClassification::Generative,
            description: "2× linear super-resolution. Perceptual — syntheses high-frequency detail.".into(),
            default_parameters_json: r#"{"scale":2}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "inpaint".into(),
            display_name: "Region Inpaint".into(),
            category: OperationCategory::Inpaint,
            safety_classification: SafetyClassification::Generative,
            description: "Replace pixels inside a region-restricted mask with synthesised content.".into(),
            default_parameters_json: r#"{"region_mask_required":true}"#.into(),
            requires_region: true,
        },
    ]
}

pub fn get_operation(operation_id: &str) -> Option<OperationInfo> {
    registry()
        .into_iter()
        .find(|op| op.operation_id == operation_id)
}

/// The outcome of dispatching an operation. P4 ships a
/// `Passthrough` variant for every operation; the
/// pixels are unchanged (the source version is returned
/// verbatim). P5 replaces this with model-backed
/// variants (`Denoise`, `SuperResolution`, etc.) per
/// the per-operation handlers in `astroforge-ai`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationOutcome {
    pub operation_id: String,
    pub safety_classification: SafetyClassification,
    pub parameters_hash: String,
    pub source_image_version_id: String,
    pub result_image_version_id: String,
    pub preview_id: String,
    pub note: String,
}

/// FNV-1a 64-bit hex. Stable across runs; cheap to
/// compute; sufficient as a UI digest.
fn short_hash(input: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut h = FNV_OFFSET;
    for byte in input.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    format!("{:016x}", h)
}

/// Dispatch an operation. P4 returns a passthrough
/// outcome for every operation; the result Image
/// Version is a new id (so the apply round still
/// creates a new Image Version per CR-06 §4 /
/// §22), but the pixel data is unchanged. P5
/// replaces the body per-operation.
pub fn dispatch_operation(
    operation_id: &str,
    parameters_json: &str,
    source_image_version_id: &str,
    result_image_version_id: String,
    preview_id: String,
) -> Result<OperationOutcome, OperationError> {
    let info = get_operation(operation_id)
        .ok_or_else(|| OperationError::UnknownOperation(operation_id.into()))?;
    Ok(OperationOutcome {
        operation_id: info.operation_id,
        safety_classification: info.safety_classification,
        parameters_hash: short_hash(parameters_json),
        source_image_version_id: source_image_version_id.into(),
        result_image_version_id,
        preview_id,
        note: "P4 placeholder: passthrough. Real ONNX dispatch lands in P5.".into(),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationError {
    UnknownOperation(String),
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationError::UnknownOperation(id) => write!(f, "unknown operation: {id}"),
        }
    }
}

impl std::error::Error for OperationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_11_operations() {
        assert_eq!(registry().len(), 11);
    }

    #[test]
    fn get_operation_round_trips() {
        let info = get_operation("denoise_luminance").expect("present");
        assert_eq!(info.category, OperationCategory::Denoise);
        assert_eq!(info.safety_classification, SafetyClassification::Perceptual);
    }

    #[test]
    fn get_operation_unknown_returns_none() {
        assert!(get_operation("not_an_op").is_none());
    }

    #[test]
    fn generative_operations_require_region_or_are_upscale() {
        let trail = get_operation("trail_clean").unwrap();
        let sr = get_operation("super_resolution").unwrap();
        let inpaint = get_operation("inpaint").unwrap();
        let denoise = get_operation("denoise_luminance").unwrap();
        assert_eq!(
            trail.safety_classification,
            SafetyClassification::Generative
        );
        assert_eq!(sr.safety_classification, SafetyClassification::Generative);
        assert_eq!(
            inpaint.safety_classification,
            SafetyClassification::Generative
        );
        assert_ne!(
            denoise.safety_classification,
            SafetyClassification::Generative
        );
        assert!(trail.requires_region);
        assert!(inpaint.requires_region);
        assert!(!sr.requires_region);
    }

    #[test]
    fn dispatch_returns_passthrough_outcome() {
        let out = dispatch_operation(
            "denoise_luminance",
            r#"{"strength":0.7}"#,
            "v1",
            "v2".into(),
            "pv1".into(),
        )
        .unwrap();
        assert_eq!(out.operation_id, "denoise_luminance");
        assert_eq!(out.source_image_version_id, "v1");
        assert_eq!(out.result_image_version_id, "v2");
        assert_eq!(out.parameters_hash.len(), 16);
    }

    #[test]
    fn dispatch_unknown_errors() {
        let err =
            dispatch_operation("missing_op", "{}", "v1", "v2".into(), "pv1".into()).unwrap_err();
        assert_eq!(err, OperationError::UnknownOperation("missing_op".into()));
    }

    #[test]
    fn same_parameters_produce_same_hash() {
        let a = dispatch_operation(
            "denoise_luminance",
            r#"{"strength":0.7}"#,
            "v1",
            "v2".into(),
            "p".into(),
        )
        .unwrap();
        let b = dispatch_operation(
            "denoise_luminance",
            r#"{"strength":0.7}"#,
            "v1",
            "v2".into(),
            "p".into(),
        )
        .unwrap();
        assert_eq!(a.parameters_hash, b.parameters_hash);
    }

    #[test]
    fn safety_classification_round_trips() {
        for c in [
            SafetyClassification::Deterministic,
            SafetyClassification::Perceptual,
            SafetyClassification::Generative,
        ] {
            assert_eq!(parse_classification(c.as_str()), Some(c));
        }
    }

    fn parse_classification(raw: &str) -> Option<SafetyClassification> {
        registry()
            .into_iter()
            .map(|o| o.safety_classification)
            .find(|c| c.as_str() == raw)
    }
}
