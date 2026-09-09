//! CR-05 P4 slice 5 — preview driver (CR-05 §11).
//!
//! Runs a stage handler against a **downsampled** copy of the stage's
//! input frames and returns the resulting [`F32Image`] without
//! touching the real `ImageVersion` chain. The driver reuses the same
//! [`HandlerRegistry`] + [`StageContext`] the full runner uses, so a
//! preview exercises exactly the code path a full run would — only
//! the input resolution differs.
//!
//! ## Input loading
//!
//! Frames are loaded through the same helper the production handlers
//! use ([`dispatch::load_frames_for_session`]). Slice 5.1 replaced
//! the stub with real TIFF/FITS decoding; files that fail to decode
//! are logged and skipped so one corrupt asset doesn't kill the
//! whole run. **No synthetic frames are substituted in production**
//! — a failed preview row with a real error message is the honest
//! outcome when no source assets exist.
//! Tests feed frames through [`StageContext::preloaded_frames`] so
//! the downsample → dispatch → output path is exercised end-to-end.

use crate::domain::PipelineStage;
use crate::domain_store::DomainStore;
use crate::image::F32Image;
use crate::pipeline_plan::dispatch::{
    load_frames_for_session, HandlerRegistry, StageContext, StageHandlerError, StageOutput,
};
use crate::resource::{
    derive_stage_budget, parse_dataset_size_bytes_opt, ResourceSnapshot, UNKNOWN_DATASET_SIZE_BYTES,
};
use std::sync::Arc;
use thiserror::Error;

/// CR-05 P4 slice 5 — inputs for a single preview run.
pub struct PreviewRequest {
    /// The stage being previewed (type + parameters).
    pub stage: PipelineStage,
    /// Session whose assets feed the stage.
    pub session_id: String,
    /// Stable id used for artifact provenance (the preview_run id).
    pub run_id: String,
    /// Downscale factor, e.g. 0.25 for a quarter-size preview.
    /// Clamped to (0, 1].
    pub scale: f64,
    /// Shared stores / registries.
    pub domain_store: Arc<DomainStore>,
    pub handler_registry: Arc<HandlerRegistry>,
    /// Test-only frame injection. Production leaves this `None` and
    /// the handler loads from the store; when that path returns no
    /// frames the preview fails with [`PreviewError::NoSourceFrames`].
    pub preloaded_frames: Option<Vec<F32Image>>,
}

/// CR-05 P4 slice 5 — errors the preview driver can surface. All of
/// these are written verbatim into the `PreviewRun.error_json` column
/// so the UI can show the user what actually happened.
#[derive(Debug, Error)]
pub enum PreviewError {
    /// The stage type has no registered handler.
    #[error("no handler registered for stage type '{0}'")]
    NoHandler(String),

    /// No source frames could be loaded for the session (asset
    /// loading is stubbed as of P2.6 slice 1).
    #[error("no source frames available for session '{0}'")]
    NoSourceFrames(String),

    /// The handler itself failed (bad parameters, downstream error).
    #[error("stage handler failed: {0}")]
    Handler(#[from] StageHandlerError),

    /// The handler returned no image (e.g. a metadata-only stage).
    #[error("stage produced no previewable image")]
    NoImage,
}

/// CR-05 P4 slice 5 — output of a successful preview run: the
/// downsampled, stage-processed image plus the metadata the handler
/// emitted.
#[derive(Debug)]
pub struct PreviewOutput {
    pub image: F32Image,
    pub metadata_json: Option<String>,
}

/// CR-05 P4 slice 5 — run a stage handler against downsampled input
/// frames. The caller (Tauri command) is responsible for persisting
/// the result as an artifact + `PreviewRun` row.
pub fn run_preview(request: &PreviewRequest) -> Result<PreviewOutput, PreviewError> {
    let handler = request
        .handler_registry
        .get(&request.stage.stage_type)
        .ok_or_else(|| PreviewError::NoHandler(request.stage.stage_type.clone()))?;

    // Load or inject the full-resolution frames, then downsample
    // each by the requested scale. Downsampling happens per-frame so
    // multi-frame stages (stack, calibrate) see consistent input
    // sizes.
    let frames: Vec<F32Image> = match &request.preloaded_frames {
        Some(preloaded) => preloaded
            .iter()
            .map(|f| f.downsample_box(request.scale))
            .collect(),
        None => {
            // Production path: load via the dispatch helper. Stubbed
            // as of P2.6 slice 1 — surfaces as NoSourceFrames.
            let loaded = load_frames_for_session(&request.domain_store, &request.session_id);
            if loaded.is_empty() {
                return Err(PreviewError::NoSourceFrames(request.session_id.clone()));
            }
            loaded
                .iter()
                .map(|f| f.downsample_box(request.scale))
                .collect()
        }
    };

    let ctx = StageContext {
        stage: request.stage.clone(),
        session_id: request.session_id.clone(),
        run_id: request.run_id.clone(),
        domain_store: request.domain_store.clone(),
        preloaded_frames: Some(frames),
        // CR-05 P5 slice 2 — preview uses the same budget derivation
        // as the runner so a preview tiles identically to the full run
        // (P5 closed the preview / full-resolution parameter-parity loop).
        execution_budget: derive_stage_budget(
            &ResourceSnapshot::detect(),
            parse_dataset_size_bytes_opt(request.stage.parameters_json.as_ref())
                .unwrap_or(UNKNOWN_DATASET_SIZE_BYTES),
        ),
    };

    let StageOutput {
        image,
        metadata_json,
        ..
    } = handler.handle(&ctx)?;

    let image = image.ok_or(PreviewError::NoImage)?;
    Ok(PreviewOutput {
        image,
        metadata_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PipelineStage;
    use crate::domain_store::DomainStore;
    use crate::pipeline_plan::dispatch::StackHandler;

    fn stage(stage_type: &str) -> PipelineStage {
        PipelineStage {
            stage_id: "s_preview".into(),
            plan_id: "plan_test".into(),
            stage_type: stage_type.into(),
            sequence: 0,
            label: stage_type.into(),
            required: true,
            enabled: true,
            parameters_json: Some(r#"{"kappa":3.0,"iterations":2}"#.into()),
            produces_image_version: true,
            undo_supported: true,
        }
    }

    fn gradient_frame(width: usize, height: usize, channels: usize) -> F32Image {
        let mut img = F32Image::new(width, height, channels);
        for c in 0..channels {
            for y in 0..height {
                for x in 0..width {
                    img[(c, y, x)] = (x + y + c) as f32 / (width + height + channels) as f32;
                }
            }
        }
        img
    }

    fn in_memory_store() -> Arc<DomainStore> {
        Arc::new(DomainStore::new(std::path::Path::new(":memory:")).unwrap())
    }

    fn registry_with_stack() -> Arc<HandlerRegistry> {
        let mut reg = HandlerRegistry::new();
        reg.insert("stack", Arc::new(StackHandler));
        Arc::new(reg)
    }

    #[test]
    fn preview_unknown_stage_type_returns_no_handler() {
        let req = PreviewRequest {
            stage: stage("no_such_stage"),
            session_id: "sess".into(),
            run_id: "prev_test".into(),
            scale: 0.5,
            domain_store: in_memory_store(),
            handler_registry: Arc::new(HandlerRegistry::new()),
            preloaded_frames: Some(vec![gradient_frame(4, 4, 1)]),
        };
        let err = run_preview(&req).unwrap_err();
        assert!(matches!(err, PreviewError::NoHandler(t) if t == "no_such_stage"));
    }

    #[test]
    fn preview_no_frames_returns_no_source_frames() {
        let req = PreviewRequest {
            stage: stage("stack"),
            session_id: "sess_empty".into(),
            run_id: "prev_test".into(),
            scale: 0.5,
            domain_store: in_memory_store(),
            handler_registry: registry_with_stack(),
            preloaded_frames: None,
        };
        let err = run_preview(&req).unwrap_err();
        assert!(matches!(err, PreviewError::NoSourceFrames(s) if s == "sess_empty"));
    }

    #[test]
    fn preview_stack_downsamples_and_stacks() {
        // Three 8x8 mono frames with slightly different gradients.
        // After 0.5 downsample the stack handler should see 4x4
        // frames and produce a 4x4 output.
        let frames = vec![
            gradient_frame(8, 8, 1),
            gradient_frame(8, 8, 1),
            gradient_frame(8, 8, 1),
        ];
        let req = PreviewRequest {
            stage: stage("stack"),
            session_id: "sess".into(),
            run_id: "prev_test".into(),
            scale: 0.5,
            domain_store: in_memory_store(),
            handler_registry: registry_with_stack(),
            preloaded_frames: Some(frames),
        };
        let out = run_preview(&req).expect("preview should succeed");
        assert_eq!(out.image.width(), 4);
        assert_eq!(out.image.height(), 4);
        assert_eq!(out.image.channels(), 1);
        assert!(out.metadata_json.is_some());
    }

    #[test]
    fn preview_scale_one_is_identity_sized() {
        let frames = vec![gradient_frame(6, 4, 1), gradient_frame(6, 4, 1)];
        let req = PreviewRequest {
            stage: stage("stack"),
            session_id: "sess".into(),
            run_id: "prev_test".into(),
            scale: 1.0,
            domain_store: in_memory_store(),
            handler_registry: registry_with_stack(),
            preloaded_frames: Some(frames),
        };
        let out = run_preview(&req).expect("preview should succeed");
        assert_eq!(out.image.width(), 6);
        assert_eq!(out.image.height(), 4);
    }
}
