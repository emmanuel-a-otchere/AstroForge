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
