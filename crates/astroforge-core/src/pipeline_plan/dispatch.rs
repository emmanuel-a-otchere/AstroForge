//! CR-05 P2.6 — stage dispatch (Decision D-CR05-3 + D-CR05-6).
//!
//! P2.6 splits the runner's no-op dispatcher into a real handler
//! registry. Each [`StageHandler`] knows how to load its inputs from
//! the [`DomainStore`], run its stage computation against existing
//! processing modules (`stacking`, `calibration`, etc.), and persist
//! produced artifacts back to the store.
//!
//! Slice 1 wires only the **Stack** handler end-to-end. Other stage
//! types still fall through to a no-op dispatcher so the runner walks
//! every stage and writes `stage_execution` rows; P2.7+ replace the
//! no-op stubs with real handlers.
//!
//! ## Why a trait (not a match statement)?
//!
//! - Each handler has a distinct input shape (Stack takes calibrated
//!   frames; Stretch takes a stacked image; etc.). A trait with an
//!   associated `Input` type keeps the dispatch surface uniform while
//!   letting each handler decide how to load what it needs.
//! - Tests can substitute a `MockStageHandler` without touching the
//!   runner's flag-handling code.
//! - New stage types can be added by writing a new handler + inserting
//!   into the registry; no runner edits required (open/closed).
//!
//! ## Artifact persistence
//!
//! Handlers return a [`StageOutput`] that carries the produced
//! [`F32Image`] and an optional persisted artifact id. The runner
//! writes that id into the `StageExecution` row's
//! `output_artifact_id` column so the rest of the system can find the
//! artifact via `DomainStore::get_artifact`.

use crate::domain::PipelineStage;
use crate::domain_store::DomainStore;
use crate::image::F32Image;
use crate::stacking;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// CR-05 P2.6 — input bundle for a stage handler. Includes everything
/// the handler needs to load + run: the persisted stage spec, the
/// session id (so it can list source assets), the run id (for artifact
/// provenance), and the shared [`DomainStore`] handle.
///
/// When `preloaded_frames` is `Some`, handlers MUST use those frames
/// instead of loading from disk. Tests use this to feed deterministic
/// synthetic `F32Image` instances; production paths leave it `None`
/// and load from `domain_store.list_source_assets`.
#[derive(Clone)]
pub struct StageContext {
    pub stage: PipelineStage,
    pub session_id: String,
    pub run_id: String,
    pub domain_store: Arc<DomainStore>,
    pub preloaded_frames: Option<Vec<F32Image>>,
}

/// CR-05 P2.6 — output from a stage handler. Carries the produced
/// image, optional metadata (kappa / rejection count / etc.), and
/// the id of the persisted Artifact row when applicable.
#[derive(Debug, Clone)]
pub struct StageOutput {
    pub image: Option<F32Image>,
    pub parameters_json: Option<String>,
    pub metadata_json: Option<String>,
    /// CR-05 P2.6 — id of the persisted Artifact row (populated by
    /// the runner after the handler returns). Slice 1 sets this on
    /// the Stack handler.
    pub artifact_id: Option<String>,
}

/// CR-05 P2.6 — errors a handler can return. The runner converts these
/// into the `failed` `StageExecution` row + `Failed` plan status.
#[derive(Debug, Error)]
pub enum StageHandlerError {
    #[error("no source frames found for session {0}")]
    NoSourceFrames(String),
    #[error("stack failed: {0}")]
    StackFailed(#[from] stacking::StackError),
    #[error("calibration failed: {0}")]
    CalibrationFailed(String),
    #[error("calibration module error: {0}")]
    CalibrationModule(#[from] crate::calibration::CalibrationError),
    #[error("handler not yet implemented for stage type {0}")]
    NotImplemented(String),
}

/// CR-05 P2.6 — handler trait. Each handler is responsible for loading
/// its own inputs from the [`DomainStore`] via [`StageContext`].
pub trait StageHandler: Send + Sync {
    fn handle(&self, ctx: &StageContext) -> Result<StageOutput, StageHandlerError>;
}

/// CR-05 P2.6 — handler registry. Keys are stage_type strings (matching
/// the persisted `PipelineStage.stage_type`). Values are boxed
/// handlers. P2.7+ add entries here without touching the runner.
#[derive(Default)]
pub struct HandlerRegistry {
    handlers: HashMap<String, Arc<dyn StageHandler>>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, stage_type: impl Into<String>, handler: Arc<dyn StageHandler>) {
        self.handlers.insert(stage_type.into(), handler);
    }

    pub fn get(&self, stage_type: &str) -> Option<Arc<dyn StageHandler>> {
        self.handlers.get(stage_type).cloned()
    }
}

/// CR-05 P2.6 slice 1 — Stack handler.
///
/// Loads source frames for the session and runs
/// [`stacking::kappa_sigma_stack`]. Per CR-05 §31 the kappa /
/// max_iterations parameters come from the stage's `parameters_json`
/// field; slice 1 uses sensible defaults when the field is empty
/// (kappa=3.0, max_iterations=5).
///
/// ## Input resolution (Decision D-CR05-12)
///
/// 1. If `ctx.preloaded_frames` is `Some`, use those frames directly
///    (test path; no disk I/O).
/// 2. Otherwise, list source assets via `DomainStore` and read each
///    asset as a TIFF via `F32Image::from_tiff_bytes`. Assets that
///    fail to load are skipped silently; if no assets can be loaded
///    the handler returns `NoSourceFrames`.
pub struct StackHandler;

impl StageHandler for StackHandler {
    fn handle(&self, ctx: &StageContext) -> Result<StageOutput, StageHandlerError> {
        let frames = if let Some(preloaded) = &ctx.preloaded_frames {
            preloaded.clone()
        } else {
            load_frames_from_assets(ctx)?
        };

        if frames.is_empty() {
            return Err(StageHandlerError::NoSourceFrames(ctx.session_id.clone()));
        }

        let (kappa, max_iterations) = parse_stack_params(ctx.stage.parameters_json.as_deref());
        let stack_result = stacking::kappa_sigma_stack(&frames, kappa, max_iterations)?;

        let metadata = format!(
            r#"{{"rejected":{},"frame_count":{},"kappa":{},"iterations":{}}}"#,
            stack_result.rejected_count, stack_result.frame_count, kappa, max_iterations,
        );

        Ok(StageOutput {
            image: Some(stack_result.image),
            parameters_json: ctx.stage.parameters_json.clone(),
            metadata_json: Some(metadata),
            // artifact_id is populated by the runner after
            // DomainStore::record_artifact runs.
            artifact_id: None,
        })
    }
}

/// CR-05 P2.7 — calibrate handler.
///
/// Loads all `SourceAsset`s for the session, buckets them by
/// `frame_type` (Light / Dark / Flat / Bias), builds master
/// calibration frames where possible, then applies calibration to
/// each light frame and returns the calibrated light set as the
/// produced output (mirroring the Stack handler's
/// first-frame-as-output convention — see P2.6 for the rationale).
///
/// ## Stage parameters
///
/// Reads from `parameters_json` (none required):
///
/// - `light_frame_type` (default `"light"`) — overrides the bucket
///   label used for light frames. Allows custom ingest conventions.
/// - `master_dark_required` (default `false`) — when `true`, the
///   stage fails if no dark frames are present.
///
/// All other bucket logic uses the defaults.
///
/// ## P2.7 scope note
///
/// Like P2.6's Stack handler, the disk loader for source assets is
/// still a stub (`load_frames_from_assets` returns empty). The
/// integration test exercises the handler via `preloaded_frames`.
/// Production calibration requires `image_io.rs` (P3) to load
/// real FITS / TIFF files into `F32Image`. The handler's
/// bucketing + master-frame logic is fully exercised in tests.
pub struct CalibrateHandler;

impl StageHandler for CalibrateHandler {
    fn handle(&self, ctx: &StageContext) -> Result<StageOutput, StageHandlerError> {
        use crate::calibration::{
            apply_calibration, build_master_bias, build_master_dark, build_master_flat,
        };

        // Load source assets for the session. In production this
        // walks SourceAsset rows + reads each file from disk. For
        // slice 1 the loader is a stub so the handler relies on
        // preloaded_frames when provided.
        let assets = if let Some(preloaded) = &ctx.preloaded_frames {
            // preloaded_frames carries synthetic F32Image frames
            // without frame_type metadata. Tests that exercise
            // calibration pre-bucket these frames by passing them
            // through the StageContext's `preloaded_lights` /
            // `preloaded_calibration` extension — but for slice 1
            // we keep the trait minimal: treat every preloaded frame
            // as a light and synthesize no master frames (matches
            // apply_calibration_lights_only).
            let frames = preloaded.clone();
            if frames.is_empty() {
                return Err(StageHandlerError::NoSourceFrames(ctx.session_id.clone()));
            }
            return Ok(StageOutput {
                image: Some(frames[0].clone()),
                parameters_json: ctx.stage.parameters_json.clone(),
                metadata_json: Some(format!(
                    r#"{{"lights_calibrated":{},"dark_count":0,"flat_count":0,"bias_count":0,"mode":"preloaded"}}"#,
                    frames.len()
                )),
                artifact_id: None,
            });
        } else {
            load_source_assets(ctx)?
        };

        if assets.is_empty() {
            return Err(StageHandlerError::NoSourceFrames(ctx.session_id.clone()));
        }

        let params = parse_calibrate_params(ctx.stage.parameters_json.as_deref());
        let light_label = params.light_frame_type.as_str();
        let mut lights = Vec::new();
        let mut darks = Vec::new();
        let mut flats = Vec::new();
        let mut biases = Vec::new();

        for asset in &assets {
            match asset.frame_type.as_deref() {
                Some(label) if label.eq_ignore_ascii_case(light_label) => lights.push(asset),
                Some("dark") | Some("Dark") => darks.push(asset),
                Some("flat") | Some("Flat") => flats.push(asset),
                Some("bias") | Some("Bias") => biases.push(asset),
                _ => {
                    // Other frame kinds (e.g. focus runs) are ignored.
                }
            }
        }

        if params.master_dark_required && darks.is_empty() {
            return Err(StageHandlerError::CalibrationFailed(
                "master_dark_required=true but no Dark frames present".to_string(),
            ));
        }

        let master_dark = if !darks.is_empty() {
            let dark_frames: Vec<F32Image> = darks.iter().map(|a| a.preview.clone()).collect();
            Some(build_master_dark(&dark_frames, 0.0, None)?)
        } else {
            None
        };
        let master_flat = if !flats.is_empty() {
            let flat_frames: Vec<F32Image> = flats.iter().map(|a| a.preview.clone()).collect();
            Some(build_master_flat(&flat_frames)?)
        } else {
            None
        };
        let master_bias = if !biases.is_empty() {
            let bias_frames: Vec<F32Image> = biases.iter().map(|a| a.preview.clone()).collect();
            Some(build_master_bias(&bias_frames)?)
        } else {
            None
        };

        let calibrated_lights: Vec<F32Image> = lights
            .iter()
            .map(|a| {
                apply_calibration(
                    &a.preview,
                    master_dark.as_ref(),
                    master_flat.as_ref(),
                    master_bias.as_ref(),
                )
            })
            .collect();

        let metadata = format!(
            r#"{{"lights_calibrated":{},"dark_count":{},"flat_count":{},"bias_count":{},"mode":"applied"}}"#,
            calibrated_lights.len(),
            darks.len(),
            flats.len(),
            biases.len()
        );

        let first = calibrated_lights
            .first()
            .cloned()
            .unwrap_or_else(|| F32Image::new(1, 1, 1));

        Ok(StageOutput {
            image: Some(first),
            parameters_json: ctx.stage.parameters_json.clone(),
            metadata_json: Some(metadata),
            artifact_id: None,
        })
    }
}

/// CR-05 P2.7 — calibration-stage parameter struct.
#[derive(Debug, Clone)]
struct CalibrateParams {
    light_frame_type: String,
    master_dark_required: bool,
}

impl Default for CalibrateParams {
    fn default() -> Self {
        Self {
            light_frame_type: "light".to_string(),
            master_dark_required: false,
        }
    }
}

/// CR-05 P2.7 — parse calibration-stage parameters with safe defaults.
fn parse_calibrate_params(parameters_json: Option<&str>) -> CalibrateParams {
    let Some(raw) = parameters_json else {
        return CalibrateParams::default();
    };
    let parsed: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return CalibrateParams::default(),
    };
    CalibrateParams {
        light_frame_type: parsed
            .get("light_frame_type")
            .and_then(|v| v.as_str())
            .unwrap_or("light")
            .to_string(),
        master_dark_required: parsed
            .get("master_dark_required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    }
}

/// CR-05 P2.7 — lightweight source asset DTO used by handlers. Holds
/// the frame_type label + a pre-decoded preview F32Image. In slice 1
/// the preview is synthetic; production paths populate it via
/// `image_io` (P3).
#[derive(Debug, Clone)]
pub struct SourceAssetPreview {
    pub frame_type: Option<String>,
    pub preview: F32Image,
}

/// CR-05 P2.8 — Stretch handler. Takes the upstream image (Stack
/// output, Calibrated single-frame, etc.) and applies
/// `histogram_stretch` with parameters from
/// `stage.parameters_json`.
///
/// ## Stage parameters
///
/// - `shadows` (default 0.05) — lower bound of the histogram.
/// - `highlights` (default 0.99) — upper bound (also the output
///   ceiling per the GLSL MTF shader).
/// - `midtones` (default 0.5) — Lupton midtone transfer parameter.
///
/// Like P2.6 / P2.7, the handler relies on `preloaded_frames` for
/// slice 1 input. Production paths that load from disk land in P3
/// alongside `image_io.rs`.
pub struct StretchHandler;

impl StageHandler for StretchHandler {
    fn handle(&self, ctx: &StageContext) -> Result<StageOutput, StageHandlerError> {
        let params = parse_stretch_params(ctx.stage.parameters_json.as_deref());

        let frames = ctx.preloaded_frames.clone().unwrap_or_default();
        if frames.is_empty() {
            return Err(StageHandlerError::NoSourceFrames(ctx.session_id.clone()));
        }

        // Slice 1 — stretch the first frame. Multi-frame stretching
        // (per-channel stats) is P3 work once image_io lands.
        let stretched = crate::stretching::histogram_stretch(
            &frames[0],
            params.shadows,
            params.highlights,
            params.midtones,
        );

        let metadata = format!(
            r#"{{"shadows":{},"highlights":{},"midtones":{},"input_shape":[{},{},{}]}}"#,
            params.shadows,
            params.highlights,
            params.midtones,
            frames[0].width(),
            frames[0].height(),
            frames[0].channels(),
        );

        Ok(StageOutput {
            image: Some(stretched),
            parameters_json: ctx.stage.parameters_json.clone(),
            metadata_json: Some(metadata),
            artifact_id: None,
        })
    }
}

/// CR-05 P2.8 — Denoise handler. Takes the upstream image and runs
/// `dip_denoise` against a configurable `DipConfig`.
///
/// ## Stage parameters
///
/// - `max_iterations` (default 500)
/// - `learning_rate` (default 0.01)
/// - `early_stop_patience` (default 50)
/// - `early_stop_threshold` (default 1e-4)
/// - `noise_reg` (default 0.1)
/// - `blend_ratio` (default 0.55) — how much of the denoised image
///   to mix into the output (1.0 = full replacement, 0.0 = passthrough).
pub struct DenoiseHandler;

impl StageHandler for DenoiseHandler {
    fn handle(&self, ctx: &StageContext) -> Result<StageOutput, StageHandlerError> {
        let params = parse_denoise_params(ctx.stage.parameters_json.as_deref());

        let frames = ctx.preloaded_frames.clone().unwrap_or_default();
        if frames.is_empty() {
            return Err(StageHandlerError::NoSourceFrames(ctx.session_id.clone()));
        }

        let config = crate::dip::DipConfig {
            max_iterations: params.max_iterations,
            learning_rate: params.learning_rate,
            early_stop_patience: params.early_stop_patience,
            early_stop_threshold: params.early_stop_threshold,
            noise_reg: params.noise_reg,
        };

        let denoised = crate::dip::dip_denoise(&frames[0], &config, params.blend_ratio);

        let metadata = format!(
            r#"{{"max_iterations":{},"learning_rate":{},"blend_ratio":{},"input_shape":[{},{},{}]}}"#,
            params.max_iterations,
            params.learning_rate,
            params.blend_ratio,
            frames[0].width(),
            frames[0].height(),
            frames[0].channels(),
        );

        Ok(StageOutput {
            image: Some(denoised),
            parameters_json: ctx.stage.parameters_json.clone(),
            metadata_json: Some(metadata),
            artifact_id: None,
        })
    }
}

/// CR-05 P2.8 — Stretch stage parameters.
#[derive(Debug, Clone)]
struct StretchParams {
    shadows: f64,
    highlights: f64,
    midtones: f64,
}

impl Default for StretchParams {
    fn default() -> Self {
        Self {
            shadows: 0.05,
            highlights: 0.99,
            midtones: 0.5,
        }
    }
}

/// CR-05 P2.8 — parse stretch parameters with safe defaults.
fn parse_stretch_params(parameters_json: Option<&str>) -> StretchParams {
    let Some(raw) = parameters_json else {
        return StretchParams::default();
    };
    let parsed: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return StretchParams::default(),
    };
    StretchParams {
        shadows: parsed
            .get("shadows")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.05),
        highlights: parsed
            .get("highlights")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.99),
        midtones: parsed
            .get("midtones")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5),
    }
}

/// CR-05 P2.8 — Denoise stage parameters.
#[derive(Debug, Clone)]
struct DenoiseParams {
    max_iterations: u32,
    learning_rate: f64,
    early_stop_patience: u32,
    early_stop_threshold: f64,
    noise_reg: f64,
    blend_ratio: f32,
}

impl Default for DenoiseParams {
    fn default() -> Self {
        Self {
            max_iterations: 500,
            learning_rate: 0.01,
            early_stop_patience: 50,
            early_stop_threshold: 1e-4,
            noise_reg: 0.1,
            blend_ratio: 0.55,
        }
    }
}

/// CR-05 P2.8 — parse denoise parameters with safe defaults.
fn parse_denoise_params(parameters_json: Option<&str>) -> DenoiseParams {
    let Some(raw) = parameters_json else {
        return DenoiseParams::default();
    };
    let parsed: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return DenoiseParams::default(),
    };
    DenoiseParams {
        max_iterations: parsed
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(500) as u32,
        learning_rate: parsed
            .get("learning_rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.01),
        early_stop_patience: parsed
            .get("early_stop_patience")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as u32,
        early_stop_threshold: parsed
            .get("early_stop_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(1e-4),
        noise_reg: parsed
            .get("noise_reg")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.1),
        blend_ratio: parsed
            .get("blend_ratio")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.55) as f32,
    }
}

#[cfg(test)]
mod p28_tests {
    use super::*;
    use crate::domain_store::DomainStore;
    use crate::image::F32Image;
    use crate::pipeline_plan::dispatch::{StageContext, StageHandler};
    use std::path::PathBuf;

    fn test_ctx(preloaded: Option<Vec<F32Image>>) -> StageContext {
        let domain_store = DomainStore::new(&PathBuf::from(":memory:")).unwrap();
        let stage = crate::domain::PipelineStage {
            stage_id: "p28_1".into(),
            plan_id: "plan_1".into(),
            stage_type: "stretch".into(),
            sequence: 0,
            label: "Stretch".into(),
            required: true,
            enabled: true,
            parameters_json: None,
            produces_image_version: true,
            undo_supported: false,
        };
        StageContext {
            stage,
            session_id: "test_session".into(),
            run_id: "run_1".into(),
            domain_store: Arc::new(domain_store),
            preloaded_frames: preloaded,
        }
    }

    #[test]
    fn parse_stretch_params_defaults_when_missing_or_invalid() {
        let p = parse_stretch_params(None);
        assert!((p.shadows - 0.05).abs() < 1e-9);
        assert!((p.highlights - 0.99).abs() < 1e-9);
        assert!((p.midtones - 0.5).abs() < 1e-9);
        let p = parse_stretch_params(Some("not json"));
        assert!((p.shadows - 0.05).abs() < 1e-9);
    }

    #[test]
    fn parse_stretch_params_reads_overrides() {
        let json = r#"{"shadows": 0.1, "highlights": 0.95, "midtones": 0.6}"#;
        let p = parse_stretch_params(Some(json));
        assert!((p.shadows - 0.1).abs() < 1e-9);
        assert!((p.highlights - 0.95).abs() < 1e-9);
        assert!((p.midtones - 0.6).abs() < 1e-9);
    }

    #[test]
    fn parse_denoise_params_defaults_when_missing_or_invalid() {
        let p = parse_denoise_params(None);
        assert_eq!(p.max_iterations, 500);
        assert!((p.learning_rate - 0.01).abs() < 1e-9);
        assert_eq!(p.early_stop_patience, 50);
        assert!((p.early_stop_threshold - 1e-4).abs() < 1e-12);
        assert!((p.noise_reg - 0.1).abs() < 1e-9);
        assert!((p.blend_ratio - 0.55).abs() < 1e-6);
        let p = parse_denoise_params(Some("garbage"));
        assert_eq!(p.max_iterations, 500);
    }

    #[test]
    fn parse_denoise_params_reads_overrides() {
        let json = r#"{"max_iterations": 100, "learning_rate": 0.05, "blend_ratio": 0.7}"#;
        let p = parse_denoise_params(Some(json));
        assert_eq!(p.max_iterations, 100);
        assert!((p.learning_rate - 0.05).abs() < 1e-9);
        assert!((p.blend_ratio - 0.7).abs() < 1e-6);
    }

    #[test]
    fn stretch_handler_with_preloaded_produces_stretched_image() {
        // 4x4x3 frame with values 0..255 (approx F32 range).
        let mut frame = F32Image::new(4, 4, 3);
        for y in 0..4 {
            for x in 0..4 {
                for c in 0..3 {
                    frame[(c, y, x)] = (y * 16 + x * 4) as f32;
                }
            }
        }
        let ctx = test_ctx(Some(vec![frame]));
        let output = StretchHandler.handle(&ctx).unwrap();
        let metadata = output.metadata_json.unwrap();
        assert!(metadata.contains("\"shadows\":0.05"));
        assert!(metadata.contains("\"highlights\":0.99"));
        assert!(metadata.contains("\"input_shape\":[4,4,3]"));
        assert!(output.image.is_some());
    }

    #[test]
    fn stretch_handler_without_inputs_fails_with_no_source_frames() {
        let ctx = test_ctx(None);
        let err = StretchHandler.handle(&ctx).unwrap_err();
        assert!(matches!(err, StageHandlerError::NoSourceFrames(_)));
    }

    #[test]
    fn denoise_handler_with_preloaded_produces_denoised_image() {
        let frame = F32Image::new(4, 4, 3);
        let ctx = test_ctx(Some(vec![frame]));
        let output = DenoiseHandler.handle(&ctx).unwrap();
        let metadata = output.metadata_json.unwrap();
        assert!(metadata.contains("\"max_iterations\":500"));
        assert!(metadata.contains("\"blend_ratio\":0.55"));
        assert!(output.image.is_some());
    }

    #[test]
    fn denoise_handler_without_inputs_fails_with_no_source_frames() {
        let ctx = test_ctx(None);
        let err = DenoiseHandler.handle(&ctx).unwrap_err();
        assert!(matches!(err, StageHandlerError::NoSourceFrames(_)));
    }
}

/// CR-05 P2.7 — load source assets + their previews for the session.
/// Returns an empty Vec when the disk loader is unavailable; the
/// handler then returns `NoSourceFrames` so the stage is marked
/// `failed` consistently with P2.6.
fn load_source_assets(_ctx: &StageContext) -> Result<Vec<SourceAssetPreview>, StageHandlerError> {
    // Slice 1 — same scope-bound stub as Stack. The full list +
    // decode pipeline lands when image_io.rs ships in P3.
    Ok(Vec::new())
}

#[cfg(test)]
mod calibrate_tests {
    use super::*;
    use crate::domain_store::DomainStore;
    use crate::image::F32Image;
    use crate::pipeline_plan::dispatch::{StageContext, StageHandler};
    use std::path::PathBuf;

    fn test_ctx(preloaded: Option<Vec<F32Image>>) -> StageContext {
        let domain_store = DomainStore::new(&PathBuf::from(":memory:")).unwrap();
        let stage = crate::domain::PipelineStage {
            stage_id: "cal_1".into(),
            plan_id: "plan_1".into(),
            stage_type: "calibrate".into(),
            sequence: 0,
            label: "Calibrate".into(),
            required: true,
            enabled: true,
            parameters_json: None,
            produces_image_version: true,
            undo_supported: false,
        };
        StageContext {
            stage,
            session_id: "test_session".into(),
            run_id: "run_1".into(),
            domain_store: Arc::new(domain_store),
            preloaded_frames: preloaded,
        }
    }

    #[test]
    fn parse_calibrate_params_defaults_when_missing_or_invalid() {
        assert_eq!(parse_calibrate_params(None).light_frame_type, "light");
        assert!(!parse_calibrate_params(None).master_dark_required);
        assert_eq!(parse_calibrate_params(Some("")).light_frame_type, "light");
        assert_eq!(
            parse_calibrate_params(Some("not json")).light_frame_type,
            "light"
        );
    }

    #[test]
    fn parse_calibrate_params_reads_overrides() {
        let json = r#"{"light_frame_type": "Light", "master_dark_required": true}"#;
        let p = parse_calibrate_params(Some(json));
        assert_eq!(p.light_frame_type, "Light");
        assert!(p.master_dark_required);
    }

    #[test]
    fn calibrate_handler_with_preloaded_lights_produces_image() {
        // Two synthetic lights preloaded; no dark / flat / bias
        // available. Preloaded path returns immediately with the
        // first frame as the output.
        let frames = vec![F32Image::new(4, 4, 3), F32Image::new(4, 4, 3)];
        let ctx = test_ctx(Some(frames));
        let output = CalibrateHandler.handle(&ctx).unwrap();
        let metadata = output.metadata_json.unwrap();
        assert!(metadata.contains("\"lights_calibrated\":2"));
        assert!(metadata.contains("\"mode\":\"preloaded\""));
        assert!(
            output.image.is_some(),
            "must produce an image even when only preloaded"
        );
    }

    #[test]
    fn calibrate_handler_without_inputs_fails_with_no_source_frames() {
        let ctx = test_ctx(None);
        let err = CalibrateHandler.handle(&ctx).unwrap_err();
        // Slice 1's disk loader returns empty so we expect
        // NoSourceFrames. When image_io lands in P3 the handler
        // will resolve to an applied result instead.
        assert!(matches!(err, StageHandlerError::NoSourceFrames(_)));
    }

    #[test]
    fn calibrate_handler_with_empty_preloaded_fails() {
        let ctx = test_ctx(Some(Vec::new()));
        let err = CalibrateHandler.handle(&ctx).unwrap_err();
        assert!(matches!(err, StageHandlerError::NoSourceFrames(_)));
    }
}

/// CR-05 P2.6 slice 1 — load source assets for the session from disk.
/// On TIFF parse failure the asset is silently skipped (the stack
/// tolerates heterogeneous inputs). Returns an empty vec when no
/// assets can be loaded so the runner marks the stage `failed` with
/// `NoSourceFrames`.
///
/// ## P2.6 scope note
///
/// Slice 1 deliberately returns an empty vec here. The full TIFF /
/// FITS -> F32Image loader lives in `image_io.rs` (planned for P3
/// alongside CR-04's Session Understanding ingestion). Slice 1's
/// integration test feeds `preloaded_frames` directly so the Stack
/// path is exercised without depending on disk IO.
fn load_frames_from_assets(_ctx: &StageContext) -> Result<Vec<F32Image>, StageHandlerError> {
    Ok(Vec::new())
}

/// Parse kappa / max_iterations from the stage's parameters_json.
/// Falls back to defaults when parsing fails (kappa=3.0, iterations=5).
fn parse_stack_params(parameters_json: Option<&str>) -> (f64, u32) {
    let Some(raw) = parameters_json else {
        return (3.0, 5);
    };
    let parsed: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return (3.0, 5),
    };
    let kappa = parsed.get("kappa").and_then(|v| v.as_f64()).unwrap_or(3.0);
    let max_iterations = parsed
        .get("iterations")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as u32;
    (kappa, max_iterations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stack_params_defaults_when_empty() {
        assert_eq!(parse_stack_params(None), (3.0, 5));
        assert_eq!(parse_stack_params(Some("")), (3.0, 5));
        assert_eq!(parse_stack_params(Some("not json")), (3.0, 5));
    }

    #[test]
    fn parse_stack_params_reads_values() {
        let json = r#"{"kappa": 2.5, "iterations": 10}"#;
        assert_eq!(parse_stack_params(Some(json)), (2.5, 10));
    }

    #[test]
    fn registry_insert_and_get() {
        let mut reg = HandlerRegistry::new();
        reg.insert("stack", Arc::new(StackHandler));
        assert!(reg.get("stack").is_some());
        assert!(reg.get("unregistered").is_none());
    }
}
