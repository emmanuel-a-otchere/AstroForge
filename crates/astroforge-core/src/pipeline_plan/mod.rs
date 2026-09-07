//! CR-05 P1 — pipeline_plan module (Decision D-CR05-3, renumbered from
//! `pipeline/` to avoid colliding with the legacy `pipeline.rs`).
//!
//! This module sits alongside the existing `pipeline.rs` (legacy Stage
//! trait + PipelineDag, used by `mvp_pipeline` and the wizard). It owns:
//!
//! - [`plan`]: `PlanGenerator` + the plan-shape types
//! - [`stage`]: `StageSpec` (the recipe-side stage definition) + helpers
//! - [`builtin`]: built-in recipes (Deep-Sky OSC Balanced ships in P1;
//!   High Quality, Planetary, etc. land in later phases)
//!
//! The legacy `pipeline::Stage` trait + `mvp_pipeline` runner remain the
//! execution path through P2; the P2 runner will dispatch through
//! `StageType` so the two surfaces stay aligned without a translation
//! table.
//!
//! Per PLAN.md §Drift risks: the wizard path coexists until P2 lands the
//! deprecation wrapper. P1 introduces this module but does not yet run
//! any stages.

pub mod builtin;
pub mod dispatch;
pub mod plan;
pub mod runner;
pub mod stage;

pub use plan::{
    generate_plan, now_unix_ms, AcquisitionMode, CalibrationAvailability, GenerationContext,
    PlanError, PlanGenerator, SessionUnderstanding,
};
pub use runner::{CancelHandle, PauseHandle, PipelineRunner, RunOutcome, RunnerError};
pub use stage::{StageSpec, StageType};
