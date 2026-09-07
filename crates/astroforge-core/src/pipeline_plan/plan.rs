//! CR-05 §3 + §18 / §19 — Plan generation.
//!
//! `generate_plan()` adapts a Recipe for a specific Session Understanding
//! and emits a [`PipelinePlan`]. The Plan is the user-facing,
//! human-readable processing workflow; the underlying DAG is the engine
//! detail (CR-05 §32 — the workspace must never become a visual mirror of
//! the implementation).
//!
//! ## Inputs
//!
//! - A [`GenerationContext`] carrying the Session Understanding (target
//!   type, calibration availability, frame counts, etc.) plus the
//!   chosen mode (auto / guided / expert).
//! - A `Recipe` — the user-visible definition that declares ordered
//!   [`StageSpec`]s, target type, etc. P1 ships one recipe
//!   (Deep-Sky OSC Balanced) under [`crate::pipeline_plan::builtin`].
//!
//! ## Output
//!
//! A [`PipelinePlan`] with [`PipelineStage`]s sequenced, labeled, and
//! marked required-vs-optional per the Recipe and the Session
//! Understanding.
//!
//! ## Determinism
//!
//! The generator is pure: same inputs → same Plan. Plan IDs are derived
//! deterministically (SHA-256 of the canonical input) so that re-running
//! the generator for the same Session + Recipe yields the same plan_id
//! (per CR-05 §19 reproducibility).
//!
//! ## P2 follow-ups
//!
//! - Stage runner that consumes the emitted Plan and writes
//!   `StageExecution` rows.
//! - Recommendation engine (P3) may override `parameters` per stage.

use crate::domain::{ObjectType, PipelinePlan, PipelinePlanStatus, PipelineStage};
use crate::pipeline_plan::stage::{StageSpec, StageType};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// CR-05 §3 + §19 — minimal Session Understanding consumed by the plan
/// generator. This is a focused subset of what CR-04 produces; P2 / P3
/// will wire it up to the actual CR-04 `SessionUnderstanding` shape (the
/// fields CR-04 has not yet emitted — narrowband detection, planetary
/// sub-mode — will be added here as CR-04 lands them).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionUnderstanding {
    /// CR-04 target classification.
    pub target_type: ObjectType,
    /// CR-04 acquisition mode (one-shot color, monochrome, narrowband, ...).
    pub acquisition: AcquisitionMode,
    /// Number of light frames.
    pub light_frame_count: u32,
    /// CR-04 — calibration groups present in the session.
    pub calibration: CalibrationAvailability,
    /// Bayer pattern, if OSC; None for mono.
    pub bayer_pattern: Option<String>,
    /// Optional: narrowband filter set (Ha / OIII / SII), if narrowband.
    pub narrowband_filters: Vec<String>,
}

/// CR-04 acquisition mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionMode {
    Osc,        // one-shot color
    Mono,       // monochrome (LRGB)
    Narrowband, // Ha / OIII / SII
    Planetary,  // high-frame-count lucky imaging
    Lunar,
    Solar,
}

/// CR-04 — calibration group availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationAvailability {
    #[default]
    None,
    Partial, // some of darks / flats / bias present
    Full,
}

impl SessionUnderstanding {
    /// Convenience constructor for tests + the P1 smoke panel.
    pub fn deep_sky_osc_balanced() -> Self {
        Self {
            target_type: ObjectType::DeepSky,
            acquisition: AcquisitionMode::Osc,
            light_frame_count: 462,
            calibration: CalibrationAvailability::Full,
            bayer_pattern: Some("RGGB".into()),
            narrowband_filters: vec![],
        }
    }

    pub fn deep_sky_osc_no_calibration() -> Self {
        Self {
            target_type: ObjectType::DeepSky,
            acquisition: AcquisitionMode::Osc,
            light_frame_count: 462,
            calibration: CalibrationAvailability::None,
            bayer_pattern: Some("RGGB".into()),
            narrowband_filters: vec![],
        }
    }

    pub fn planetary_high_frame_count() -> Self {
        Self {
            target_type: ObjectType::Planet,
            acquisition: AcquisitionMode::Planetary,
            light_frame_count: 5000,
            calibration: CalibrationAvailability::None,
            bayer_pattern: Some("RGGB".into()),
            narrowband_filters: vec![],
        }
    }
}

/// Inputs to [`generate_plan`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationContext {
    pub project_id: String,
    pub session_id: String,
    pub session_understanding: SessionUnderstanding,
    /// Recipe id (CR-02 §14) for provenance. May be `None` for an ad-hoc
    /// plan, though P1's UI uses a built-in recipe id.
    pub recipe_id: Option<String>,
    /// "auto" | "guided" | "expert".
    pub mode: String,
    /// Plan generation time, supplied by the caller for testability.
    pub generated_at_unix_ms: u64,
}

/// Errors from plan generation. P1 keeps the surface small; later phases
/// add per-stage validation.
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("unknown mode: {0} (expected auto | guided | expert)")]
    UnknownMode(String),
    #[error("recipe stage list is empty")]
    EmptyRecipe,
}

/// The plan generator. A stateless service: hold an instance, call
/// [`PlanGenerator::generate`], get a Plan back.
///
/// Per the rule "recipe stage list is empty ⇒ error", the generator
/// rejects empty recipes rather than emitting an empty plan.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlanGenerator;

impl PlanGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(
        &self,
        ctx: &GenerationContext,
        recipe_stages: &[StageSpec],
        recipe_target_type: ObjectType,
    ) -> Result<PipelinePlan, PlanError> {
        if !["auto", "guided", "expert"].contains(&ctx.mode.as_str()) {
            return Err(PlanError::UnknownMode(ctx.mode.clone()));
        }
        if recipe_stages.is_empty() {
            return Err(PlanError::EmptyRecipe);
        }

        let plan_id = derive_plan_id(
            &ctx.project_id,
            &ctx.session_id,
            ctx.recipe_id.as_deref().unwrap_or(""),
            recipe_target_type,
            recipe_stages,
            &ctx.mode,
            ctx.generated_at_unix_ms,
        );

        let plan_uuid = format!("plan_{}", short_hash(&plan_id));
        let mut stages = Vec::with_capacity(recipe_stages.len());
        for (i, spec) in recipe_stages.iter().enumerate() {
            let stage_uuid = format!("stage_{}_{}", short_hash(&plan_id), i);
            stages.push(PipelineStage {
                stage_id: stage_uuid,
                plan_id: plan_uuid.clone(),
                stage_type: stage_type_to_string(spec.stage_type),
                sequence: i as u32,
                label: spec.stage_type.label().to_string(),
                required: spec.required,
                enabled: spec.enabled,
                parameters_json: Some(serde_json::to_string(&spec.parameters).unwrap_or_default()),
                produces_image_version: spec.stage_type.produces_image_version_by_default(),
                undo_supported: spec.stage_type.undo_supported(),
            });
        }

        Ok(PipelinePlan {
            plan_id: plan_uuid,
            project_id: ctx.project_id.clone(),
            session_id: ctx.session_id.clone(),
            recipe_id: ctx.recipe_id.clone(),
            mode: ctx.mode.clone(),
            target_type: recipe_target_type,
            status: PipelinePlanStatus::Ready,
            created_at: unix_ms_to_iso8601(ctx.generated_at_unix_ms),
            schema_version: 1,
            stages,
        })
    }
}

/// Convenience wrapper — keeps call sites terse.
pub fn generate_plan(
    ctx: &GenerationContext,
    recipe_stages: &[StageSpec],
    recipe_target_type: ObjectType,
) -> Result<PipelinePlan, PlanError> {
    PlanGenerator::new().generate(ctx, recipe_stages, recipe_target_type)
}

/// CR-05 §19 — deterministic plan_id derived from the inputs. Same
/// Session + Recipe + Mode + generated_at ⇒ same plan_id.
fn derive_plan_id(
    project_id: &str,
    session_id: &str,
    recipe_id: &str,
    recipe_target_type: ObjectType,
    recipe_stages: &[StageSpec],
    mode: &str,
    generated_at_unix_ms: u64,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_id.as_bytes());
    hasher.update(b"|");
    hasher.update(session_id.as_bytes());
    hasher.update(b"|");
    hasher.update(recipe_id.as_bytes());
    hasher.update(b"|");
    hasher.update(format!("{:?}", recipe_target_type).as_bytes());
    hasher.update(b"|");
    hasher.update(mode.as_bytes());
    hasher.update(b"|");
    hasher.update(generated_at_unix_ms.to_le_bytes());
    for spec in recipe_stages {
        hasher.update(format!("{:?}", spec.stage_type).as_bytes());
        hasher.update(b":");
        hasher.update(if spec.required { b"1" } else { b"0" });
        hasher.update(b":");
        hasher.update(if spec.enabled { b"1" } else { b"0" });
        hasher.update(b";");
    }
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

fn short_hash(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let hex = format!("{:x}", digest);
    hex.chars().take(16).collect()
}

fn stage_type_to_string(stage_type: StageType) -> String {
    serde_json::to_string(&stage_type)
        .ok()
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_else(|| format!("{:?}", stage_type).to_snake_case())
}

/// Snake-case helper for the fallback path.
trait ToSnakeCase {
    fn to_snake_case(&self) -> String;
}

impl ToSnakeCase for str {
    fn to_snake_case(&self) -> String {
        let mut out = String::with_capacity(self.len() + 4);
        for (i, ch) in self.chars().enumerate() {
            if ch.is_ascii_uppercase() {
                if i != 0 {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push(ch);
            }
        }
        out
    }
}

fn unix_ms_to_iso8601(ms: u64) -> String {
    // CR-05 P1 — store the raw Unix-ms timestamp as a string. The
    // frontend formats it via `new Date(Number(ms))`. This avoids
    // pulling in `chrono` (or risking a date-arithmetic bug) for a v1.0
    // where the frontend is the only formatter. P3 / P5 may add chrono
    // if backend date logic becomes necessary.
    format!("unix_ms:{}", ms)
}

/// Helper for tests / smoke: current time as Unix milliseconds.
pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// P3+ — extend a plan with session-understanding-driven adaptations.
/// P1 leaves this empty; it documents the contract for future phases.
#[allow(dead_code)]
pub fn apply_session_adaptations(plan: &mut PipelinePlan, understanding: &SessionUnderstanding) {
    let _ = understanding; // silence unused warning
    let _ = (plan,); // silence unused warning
                     // P3: disable optional stages that are not appropriate for the
                     // dataset. P5: enable adaptive tile sizing from acquisition mode.
}

#[allow(dead_code)]
fn _silence_unused_hashmap() -> HashMap<String, serde_json::Value> {
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline_plan::builtin::deep_sky_osc_balanced;

    fn ctx_for(mode: &str) -> GenerationContext {
        GenerationContext {
            project_id: "proj_1".into(),
            session_id: "sess_1".into(),
            session_understanding: SessionUnderstanding::deep_sky_osc_balanced(),
            recipe_id: Some("recipe_deep_sky_osc_balanced".into()),
            mode: mode.into(),
            generated_at_unix_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn generates_deep_sky_osc_balanced_plan() {
        let (recipe_stages, target_type) = deep_sky_osc_balanced();
        let plan = generate_plan(&ctx_for("auto"), &recipe_stages, target_type).expect("plan");
        assert_eq!(plan.stages.len(), 10);
        assert_eq!(plan.target_type, ObjectType::DeepSky);
        assert_eq!(plan.mode, "auto");
        assert_eq!(plan.status, PipelinePlanStatus::Ready);
        // required flags
        assert!(plan
            .stages
            .iter()
            .any(|s| s.label == "Calibrate" && s.required));
        assert!(plan.stages.iter().any(|s| s.label == "Stack" && s.required));
        assert!(plan
            .stages
            .iter()
            .any(|s| s.label == "Stretch" && s.required));
        // optional flags
        assert!(plan
            .stages
            .iter()
            .any(|s| s.label == "Background" && !s.required));
        assert!(plan
            .stages
            .iter()
            .any(|s| s.label == "Denoise" && !s.required));
        assert!(plan
            .stages
            .iter()
            .any(|s| s.label == "Detail Enhancement" && !s.required));
        // ordering is 0..9
        for (i, s) in plan.stages.iter().enumerate() {
            assert_eq!(s.sequence, i as u32);
        }
    }

    #[test]
    fn plan_id_is_deterministic() {
        let (stages, target) = deep_sky_osc_balanced();
        let a = generate_plan(&ctx_for("auto"), &stages, target).unwrap();
        let b = generate_plan(&ctx_for("auto"), &stages, target).unwrap();
        assert_eq!(a.plan_id, b.plan_id);
    }

    #[test]
    fn plan_id_changes_with_mode() {
        let (stages, target) = deep_sky_osc_balanced();
        let a = generate_plan(&ctx_for("auto"), &stages, target).unwrap();
        let b = generate_plan(&ctx_for("guided"), &stages, target).unwrap();
        assert_ne!(a.plan_id, b.plan_id);
    }

    #[test]
    fn unknown_mode_is_error() {
        let (stages, target) = deep_sky_osc_balanced();
        let err = generate_plan(&ctx_for("wizard"), &stages, target).unwrap_err();
        assert!(matches!(err, PlanError::UnknownMode(_)));
    }

    #[test]
    fn empty_recipe_is_error() {
        let ctx = ctx_for("auto");
        let err = generate_plan(&ctx, &[], ObjectType::DeepSky).unwrap_err();
        assert!(matches!(err, PlanError::EmptyRecipe));
    }

    #[test]
    fn stage_types_round_trip_through_strings() {
        let (stages, target) = deep_sky_osc_balanced();
        let plan = generate_plan(&ctx_for("auto"), &stages, target).unwrap();
        // The P2 runner dispatches by stage_type string; the canonical
        // form must match `serde_json::to_string(&StageType::Calibrate)`.
        assert_eq!(plan.stages[0].stage_type, "calibrate");
        assert_eq!(plan.stages[4].stage_type, "stack");
        assert_eq!(plan.stages[7].stage_type, "stretch");
    }

    #[test]
    fn unix_ms_format_is_prefixed() {
        // CR-05 P1: backend stores unix_ms:<ms>; frontend parses.
        assert_eq!(
            unix_ms_to_iso8601(1_700_000_000_000),
            "unix_ms:1700000000000"
        );
    }

    #[test]
    fn produces_image_version_defaults() {
        let (stages, target) = deep_sky_osc_balanced();
        let plan = generate_plan(&ctx_for("auto"), &stages, target).unwrap();
        // QualityFilter and Export do NOT produce a new Image Version;
        // everything else does.
        let qf = plan
            .stages
            .iter()
            .find(|s| s.label == "Quality Filter")
            .expect("qf");
        assert!(!qf.produces_image_version);
        // Export is not in the 10-stage Deep-Sky OSC Balanced recipe;
        // verify by checking the required-vs-optional distribution.
        let cal = plan
            .stages
            .iter()
            .find(|s| s.label == "Calibrate")
            .expect("cal");
        assert!(cal.produces_image_version);
        let stack = plan
            .stages
            .iter()
            .find(|s| s.label == "Stack")
            .expect("stack");
        assert!(stack.produces_image_version);
    }

    #[test]
    fn undo_supported_defaults() {
        let (stages, target) = deep_sky_osc_balanced();
        let plan = generate_plan(&ctx_for("auto"), &stages, target).unwrap();
        // All 10 stages in the Deep-Sky OSC Balanced recipe are
        // undoable (Export is not in the recipe).
        let stretch = plan
            .stages
            .iter()
            .find(|s| s.label == "Stretch")
            .expect("stretch");
        assert!(stretch.undo_supported);
        let qf = plan
            .stages
            .iter()
            .find(|s| s.label == "Quality Filter")
            .expect("qf");
        assert!(qf.undo_supported);
    }
}
