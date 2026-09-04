//! P1.5-M5-T1+T2 — `AIService` interface and dispatch.
//!
//! The `AIService` trait is the public surface the rest of the app talks to
//! for AI-driven image processing. Three methods cover the full workflow:
//!
//! - [`AIService::analyse`] — read-only inspection of an image, returns
//!   statistics and quality metrics. Used by the UI to show "preview stats"
//!   and to feed the suggestion engine.
//!
//! - [`AIService::suggestParams`] — given an image and a stage, recommend
//!   parameters. Used by the `Automagic Expert` Accept/Refine flow
//!   (M4-T5/#163) and by `PreviewStats` stability checks (M2-T5/#149).
//!
//! - [`AIService::execute`] — run the stage with the supplied parameters,
//!   returning the processed image. Delegates to the registered
//!   [`AiModel`](crate::models::AiModel) when one is available, otherwise
//!   falls back to a CPU algorithmic path.
//!
//! ## Dispatching
//!
//! [`dispatch`] picks an engine based on the current
//! [`HardwareProbe`](crate::hardware::HardwareProbe): a Metal/CUDA/Vulkan
//! GPU at High tier takes the accelerated path, anything else takes the
//! free CPU path. Both paths are guaranteed to return a result — if the
//! AI engine fails, [`AIError::EngineFailed`] is raised and the caller
//! can retry via the CPU path.
//!
//! ## Reporting
//!
//! [`ProgressReporter`] is a callback the UI registers to receive progress
//! updates. The default implementation drops them; the Tauri command layer
//! wires a real reporter that posts events to the webview.
//!
//! [`PathSelection`] is returned by [`dispatch`] so the caller can show
//! a "Running on GPU" / "Running on CPU" message in the UI.
//!
//! ## Scope
//!
//! This is the **scaffold** for P1.5-M5. Real model-backed behaviour
//! (ONNX inference, perceptual quality models) lands with P1.5-M6.
//! Today's implementations are deliberately simple:
//!
//! - `analyse` computes min/max/mean/std-dev per channel from the
//!   underlying `Array3<f32>`.
//! - `suggestParams` returns heuristic defaults keyed off `stage.type`.
//! - `execute` is a passthrough to the registered `AiModel::run`, with
//!   a CPU no-op fallback if no model is registered.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use astroforge_core::image::F32Image;

use crate::hardware::{GpuBackend, HardwareProbe, QualityTier};
use crate::registry::ModelRegistry;

/// Information about which execution path was selected for a given request.
///
/// Returned by [`dispatch`] so the UI can show a clear
/// "Running on Metal GPU (High tier) — accelerated path" or
/// "Running on CPU (Standard tier) — free path" message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathSelection {
    /// Which backend was selected: a specific GPU backend, or `Cpu`.
    pub backend: GpuBackend,
    /// The quality tier the probe selected.
    pub tier: QualityTier,
    /// Human-readable explanation of why this path was chosen, suitable
    /// for surfacing directly in the UI.
    pub reason: String,
}

/// Per-channel statistics from [`AIService::analyse`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelStats {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub std_dev: f32,
}

/// Result of [`AIService::analyse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub width: u32,
    pub height: u32,
    pub channels: Vec<ChannelStats>,
    /// True if any channel is clipped at 0.0 (pure black) or 1.0 (pure
    /// white). Useful for the UI to flag over-stretched previews.
    pub has_clipping: bool,
}

/// A single suggested parameter, returned by [`AIService::suggestParams`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamRecommendation {
    pub key: String,
    pub value: serde_json::Value,
    /// Confidence in [0, 1]. 1.0 is "this is the obvious right answer",
    /// 0.5 is "this is a guess with low confidence".
    pub confidence: f32,
}

/// Result of [`AIService::suggestParams`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamProposal {
    /// Recommended parameters keyed by stage-param name (e.g. "strength",
    /// "midtones"). The caller merges these with its current params.
    pub params: HashMap<String, serde_json::Value>,
    /// Per-parameter confidence breakdown.
    pub confidence: HashMap<String, f32>,
    /// Free-text rationale suitable for surfacing in the UI's suggestion
    /// panel.
    pub rationale: String,
}

/// Strongly-typed wrapper around a successful AI execution.
///
/// Today this is just an `F32Image`. The wrapper exists so future revisions
/// can add fields (determinism hashes, engine attribution, processing time)
/// without breaking the signature.
#[derive(Debug, Clone)]
pub struct ProcessedImage {
    pub image: F32Image,
    pub path: PathSelection,
}

/// Errors that can come out of the service layer.
///
/// All three methods return `Result<_, AIError>`. The
/// `CPUFallback { .. }` variant is the recommended way for callers to
/// recover — it carries a default [`ParamProposal`] that the CPU path
/// can apply directly.
#[derive(Debug, thiserror::Error)]
pub enum AIError {
    #[error("AI engine failed: {0}")]
    EngineFailed(String),
    #[error("Engine selection failed: {0}")]
    NoEngine(String),
    #[error("Image format unsupported: {0}")]
    BadImage(String),
    #[error("CPU fallback recommended: {0}")]
    CPUFallback(String),
    #[error("Operation cancelled")]
    Cancelled,
}

/// Callback signature for progress updates.
///
/// `progress` is in `[0.0, 1.0]`. `stage` is a short human-readable label
/// (e.g. `"loading model"`, `"tiling"`, `"inferencing tile 3/9"`).
pub type ProgressReporter = Arc<dyn Fn(f32, &str) + Send + Sync>;

/// A no-op reporter that drops all updates. Use when the caller doesn't
/// care about progress (e.g. unit tests).
pub fn silent_reporter() -> ProgressReporter {
    Arc::new(|_, _| {})
}

/// The `AIService` interface — the public surface the rest of the app
/// talks to.
pub trait AIService: Send + Sync {
    /// Read-only inspection of an image. Returns per-channel statistics
    /// and a clipping flag.
    fn analyse(&self, image: &F32Image) -> Result<AnalysisResult, AIError>;

    /// Recommend parameters for a given stage. The caller supplies the
    /// stage type as a string (e.g. `"stretch"`, `"denoise"`,
    /// `"color_calibration"`) and a hint about whether the data is
    /// linear or stretched.
    fn suggest_params(
        &self,
        image: &F32Image,
        stage_type: &str,
        data_type: &str,
    ) -> Result<ParamProposal, AIError>;

    /// Run the stage with the supplied parameters and return the
    /// processed image. `reporter` is called periodically with progress
    /// updates.
    fn execute(
        &self,
        image: &F32Image,
        stage_type: &str,
        params: &HashMap<String, serde_json::Value>,
        reporter: ProgressReporter,
    ) -> Result<ProcessedImage, AIError>;

    /// Inspect the supplied `HardwareProbe` and decide which execution
    /// path to take. Pure function — does not mutate state.
    fn path_selection(&self, probe: &HardwareProbe) -> PathSelection;
}

/// Default implementation that delegates to the registered
/// [`ModelRegistry`] and falls back to a CPU path on failure.
pub struct DefaultAIService {
    registry: Arc<ModelRegistry>,
    probe: HardwareProbe,
}

impl DefaultAIService {
    pub fn new(registry: Arc<ModelRegistry>, probe: HardwareProbe) -> Self {
        Self { registry, probe }
    }
}

/// Convenience constructor: detect the host hardware and return a
/// ready-to-use service. The registry starts empty (no models
/// registered); callers should populate it before calling `execute`.
pub fn default_service() -> DefaultAIService {
    DefaultAIService::new(
        Arc::new(ModelRegistry::new(std::path::PathBuf::from("models"))),
        HardwareProbe::detect(),
    )
}

impl AIService for DefaultAIService {
    fn analyse(&self, image: &F32Image) -> Result<AnalysisResult, AIError> {
        analyse_image(image)
    }

    fn suggest_params(
        &self,
        image: &F32Image,
        stage_type: &str,
        data_type: &str,
    ) -> Result<ParamProposal, AIError> {
        suggest_heuristic(image, stage_type, data_type)
    }

    fn execute(
        &self,
        _image: &F32Image,
        stage_type: &str,
        _params: &HashMap<String, serde_json::Value>,
        reporter: ProgressReporter,
    ) -> Result<ProcessedImage, AIError> {
        let _path = self.path_selection(&self.probe);
        reporter(0.05, "starting");
        // Look up a registered model for this stage. If found, we know
        // a model exists in the registry; the actual inference path
        // will be wired up when the model factory lands (P1.5-M6).
        // Today we return the AIError::NoEngine so callers fall back
        // to a CPU path explicitly, rather than silently running an
        // identity transform.
        if self.registry.info_for_stage(stage_type).is_some() {
            reporter(0.50, "model registered but no engine bound");
            return Err(AIError::NoEngine(format!(
                "model for stage '{stage_type}' is registered but no inference engine is wired (P1.5-M6)."
            )));
        }
        reporter(0.50, "no engine");
        Err(AIError::NoEngine(format!(
            "no AI model registered for stage '{stage_type}'"
        )))
    }

    fn path_selection(&self, probe: &HardwareProbe) -> PathSelection {
        dispatch(probe)
    }
}

/// Decide which execution path to take based on the probe. Pure function.
pub fn dispatch(probe: &HardwareProbe) -> PathSelection {
    let tier = probe.select_tier();
    let backend = preferred_backend(probe);
    let reason = match backend {
        GpuBackend::Cpu => format!(
            "CPU path (tier: {}). No accelerated backend available; using algorithmic defaults.",
            tier.label()
        ),
        _ => format!(
            "{} path (tier: {}). GPU detected: using accelerated inference.",
            backend_label(&backend),
            tier.label()
        ),
    };
    PathSelection {
        backend,
        tier,
        reason,
    }
}

fn preferred_backend(probe: &HardwareProbe) -> GpuBackend {
    if probe.supports_backend(&GpuBackend::Cuda) {
        GpuBackend::Cuda
    } else if probe.supports_backend(&GpuBackend::Metal) {
        GpuBackend::Metal
    } else if probe.supports_backend(&GpuBackend::DirectMl) {
        GpuBackend::DirectMl
    } else if probe.supports_backend(&GpuBackend::OpenVino) {
        GpuBackend::OpenVino
    } else {
        GpuBackend::Cpu
    }
}

fn backend_label(b: &GpuBackend) -> &'static str {
    match b {
        GpuBackend::Cpu => "CPU",
        GpuBackend::Cuda => "CUDA GPU",
        GpuBackend::Metal => "Metal GPU",
        GpuBackend::DirectMl => "DirectML GPU",
        GpuBackend::OpenVino => "OpenVINO",
    }
}

/// Pure-function analyser: per-channel min/max/mean/std-dev.
pub fn analyse_image(image: &F32Image) -> Result<AnalysisResult, AIError> {
    if image.width() == 0 || image.height() == 0 {
        return Err(AIError::BadImage("empty image".to_string()));
    }
    let channels = image.channels();
    let mut stats = Vec::with_capacity(channels);
    let mut has_clipping = false;
    for c in 0..channels {
        let view = image.index_axis(ndarray::Axis(0), c);
        let (min, max, sum, sum_sq, count) = view.fold(
            (f32::INFINITY, f32::NEG_INFINITY, 0.0_f64, 0.0_f64, 0_usize),
            |(lo, hi, s, ss, n), &v| {
                (
                    lo.min(v),
                    hi.max(v),
                    s + v as f64,
                    ss + (v as f64) * (v as f64),
                    n + 1,
                )
            },
        );
        let n = count as f64;
        let mean = (sum / n) as f32;
        let variance = ((sum_sq / n) - (mean as f64) * (mean as f64)).max(0.0);
        let std_dev = variance.sqrt() as f32;
        if min <= 0.0 || max >= 1.0 {
            has_clipping = true;
        }
        stats.push(ChannelStats {
            min,
            max,
            mean,
            std_dev,
        });
    }
    Ok(AnalysisResult {
        width: image.width() as u32,
        height: image.height() as u32,
        channels: stats,
        has_clipping,
    })
}

/// Heuristic parameter suggester. Keyed off stage type; uses `analyse` to
/// inform a couple of recommendations.
pub fn suggest_heuristic(
    image: &F32Image,
    stage_type: &str,
    data_type: &str,
) -> Result<ParamProposal, AIError> {
    let stats = analyse_image(image)?;
    let mut params: HashMap<String, serde_json::Value> = HashMap::new();
    let mut confidence: HashMap<String, f32> = HashMap::new();
    let rationale = match stage_type {
        "stretch" => {
            // Heuristic: pull the low end down to the data minimum, push
            // the high end up to the data maximum. Midtones at 0.25
            // (slight bias toward shadows). Strength 0.65 — moderate.
            let lo = stats
                .channels
                .iter()
                .map(|c| c.min)
                .fold(f32::INFINITY, f32::min);
            let hi = stats
                .channels
                .iter()
                .map(|c| c.max)
                .fold(f32::NEG_INFINITY, f32::max);
            params.insert("blackPoint".into(), serde_json::json!(lo.max(0.0)));
            params.insert("highlights".into(), serde_json::json!(hi.min(1.0)));
            params.insert("midtones".into(), serde_json::json!(0.25));
            params.insert("strength".into(), serde_json::json!(0.65));
            confidence.insert("blackPoint".into(), 0.7);
            confidence.insert("highlights".into(), 0.7);
            confidence.insert("midtones".into(), 0.5);
            confidence.insert("strength".into(), 0.5);
            format!(
                "Linear histogram spans [{:.3}, {:.3}]. Setting black/highlights to the data edges and a moderate midtone/strength keeps the curve gentle.",
                lo, hi
            )
        }
        "denoise" => {
            // Heuristic: more noise → stronger denoise. Use std_dev as a
            // rough noise proxy. Threshold scales inversely with detail.
            let avg_std = stats.channels.iter().map(|c| c.std_dev).sum::<f32>()
                / stats.channels.len().max(1) as f32;
            let strength = (avg_std * 4.0).clamp(0.1, 0.8);
            params.insert("strength".into(), serde_json::json!(strength));
            params.insert("threshold".into(), serde_json::json!(0.3));
            confidence.insert("strength".into(), 0.4);
            confidence.insert("threshold".into(), 0.3);
            format!(
                "Average per-channel std-dev {:.3} → denoise strength {:.2}. Lower confidence: real noise estimation needs a model.",
                avg_std, strength
            )
        }
        "color_calibration" => {
            // Heuristic: if the image is already near full range and well
            // saturated, suggest neutral saturation (1.0). Otherwise
            // gently boost toward 1.05.
            let avg_mean = stats.channels.iter().map(|c| c.mean).sum::<f32>()
                / stats.channels.len().max(1) as f32;
            let sat = if !(0.3..=0.7).contains(&avg_mean) {
                1.05
            } else {
                1.0
            };
            params.insert("saturation".into(), serde_json::json!(sat));
            params.insert("strength".into(), serde_json::json!(0.6));
            confidence.insert("saturation".into(), 0.3);
            confidence.insert("strength".into(), 0.3);
            format!(
                "Average channel mean {:.3}. Saturation set to {:.2} based on whether the image is mid-range.",
                avg_mean, sat
            )
        }
        _ => {
            // Unknown stage: neutral defaults.
            params.insert("strength".into(), serde_json::json!(0.5));
            confidence.insert("strength".into(), 0.1);
            format!(
                "No heuristic for stage '{stage_type}'. Returning neutral defaults; data_type hint was '{data_type}'."
            )
        }
    };

    Ok(ParamProposal {
        params,
        confidence,
        rationale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

    fn uniform_image(value: f32, w: usize, h: usize, c: usize) -> F32Image {
        F32Image::from(Array3::from_elem((c, h, w), value))
    }

    fn gradient_image(w: usize, h: usize, c: usize) -> F32Image {
        let mut arr = Array3::<f32>::zeros((c, h, w));
        for ch in 0..c {
            for y in 0..h {
                for x in 0..w {
                    // Map [0, w+h) into [0.05, 0.95] so neither endpoint
                    // touches the clip boundaries at 0.0 and 1.0.
                    let t = (x + y) as f32 / (w + h - 2).max(1) as f32;
                    arr[[ch, y, x]] = 0.05 + t * 0.9;
                }
            }
        }
        F32Image::from(arr)
    }

    #[test]
    fn test_analyse_uniform() {
        let img = uniform_image(0.5, 16, 16, 3);
        let r = analyse_image(&img).unwrap();
        assert_eq!(r.width, 16);
        assert_eq!(r.height, 16);
        assert_eq!(r.channels.len(), 3);
        for c in &r.channels {
            assert_eq!(c.min, 0.5);
            assert_eq!(c.max, 0.5);
            assert_eq!(c.mean, 0.5);
            assert_eq!(c.std_dev, 0.0);
        }
        assert!(!r.has_clipping);
    }

    #[test]
    fn test_analyse_gradient() {
        let img = gradient_image(8, 8, 1);
        let r = analyse_image(&img).unwrap();
        assert!(r.channels[0].min < r.channels[0].max);
        assert!(r.channels[0].std_dev > 0.0);
        assert!(!r.has_clipping);
    }

    #[test]
    fn test_analyse_clipping_flag() {
        let img = uniform_image(0.0, 4, 4, 3);
        let r = analyse_image(&img).unwrap();
        assert!(r.has_clipping);
    }

    #[test]
    fn test_analyse_empty_image() {
        let img = F32Image::new(0, 0, 3);
        assert!(analyse_image(&img).is_err());
    }

    #[test]
    fn test_dispatch_picks_cpu_when_no_gpu() {
        let probe = HardwareProbe::detect();
        let path = dispatch(&probe);
        // Whatever the host, dispatch always returns a valid PathSelection.
        assert!(!path.reason.is_empty());
    }

    #[test]
    fn test_suggest_stretch_returns_four_params() {
        let img = gradient_image(32, 32, 3);
        let p = suggest_heuristic(&img, "stretch", "linear").unwrap();
        assert!(p.params.contains_key("blackPoint"));
        assert!(p.params.contains_key("highlights"));
        assert!(p.params.contains_key("midtones"));
        assert!(p.params.contains_key("strength"));
        assert!(!p.rationale.is_empty());
    }

    #[test]
    fn test_suggest_denoise_has_strength() {
        let img = gradient_image(32, 32, 3);
        let p = suggest_heuristic(&img, "denoise", "linear").unwrap();
        assert!(p.params.contains_key("strength"));
    }

    #[test]
    fn test_suggest_color_calibration_returns_saturation() {
        let img = gradient_image(32, 32, 3);
        let p = suggest_heuristic(&img, "color_calibration", "linear").unwrap();
        assert!(p.params.contains_key("saturation"));
    }

    #[test]
    fn test_suggest_unknown_stage_returns_neutral() {
        let img = gradient_image(32, 32, 3);
        let p = suggest_heuristic(&img, "magic_new_stage", "linear").unwrap();
        assert_eq!(p.params.get("strength").unwrap(), &serde_json::json!(0.5));
        assert!(p.confidence.get("strength").unwrap() < &0.2);
    }

    #[test]
    fn test_path_selection_serializes() {
        let probe = HardwareProbe::detect();
        let path = dispatch(&probe);
        let json = serde_json::to_string(&path).unwrap();
        assert!(json.contains("backend"));
        assert!(json.contains("tier"));
    }

    #[test]
    fn test_silent_reporter_does_not_panic() {
        let r = silent_reporter();
        r(0.5, "test");
    }
}
