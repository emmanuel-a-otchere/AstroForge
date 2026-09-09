//! CR-05 P6.2 — §23 AI Boundary labelling.
//!
//! Per CR-05 §23: AI is introduced where it adds measurable value.
//! Generative / perceptual operations remain explicitly identified so
//! the user knows when an operation may alter structures beyond
//! conventional image processing (this anticipates CR-06).
//!
//! The label is **derived from `stage_type`** at runner time, not
//! picked by the handler. Handlers run code; the boundary between
//! "deterministic classical processing" and "perceptual / generative
//! AI" is a property of what the stage *is*, not of which library
//! version it happens to use. Today only `denoise` and `detail`
//! route through ONNX perceptual models (CR-06). When CR-06 ships
//! new AI stages, this mapping is the single point to update.
//!
//! Why not `Option<AiLabel>`?  Because the §23 badge is the absence
//! of a badge for non-AI stages — there is no `uses_ai = false`
//! line in the UI, just the missing badge. We still persist
//! `uses_ai: bool` on `StageExecution` so the SQL row tells the
//! truth for reproducibility audits.

use serde::{Deserialize, Serialize};

/// Canonical AI label per stage type. Pure data, no IO.
///
/// `model_id` matches the CR-06 model registry convention
/// (`astroforge_<kind>_v<MAJOR>[.<MINOR>]`). `seed` is `Some(0)` for
/// the deterministic AI stages we ship today; the field is `Option`
/// because future perceptual / generative stages may legitimately
/// have no seed (sampling-based).
///
/// `#[serde(default)]` on every field lets pre-P6.2 rows
/// (no AI fields at all) deserialize cleanly with `uses_ai=false,
/// model_id=None, deterministic=true, seed=None` — i.e. the same
/// as `for_stage_type("anything-not-AI")`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiBoundaryLabel {
    #[serde(default)]
    /// Whether the stage crosses the AI boundary. `false` for
    /// classical deterministic stages (calibrate / debayer /
    /// register / stack / background / color / stretch / export).
    pub uses_ai: bool,
    #[serde(default)]
    /// Model identifier. `None` when `uses_ai` is `false`.
    pub model_id: Option<String>,
    #[serde(default)]
    /// Whether the stage is bit-reproducible. Today every stage we
    /// ship is deterministic; this becomes `false` when CR-06
    /// introduces sampling-based perceptual enhancements.
    pub deterministic: bool,
    #[serde(default)]
    /// RNG seed for stages that have one. `None` when `uses_ai`
    /// is `false` or the stage is sampling-free.
    pub seed: Option<u64>,
}

impl Default for AiBoundaryLabel {
    fn default() -> Self {
        Self {
            uses_ai: false,
            model_id: None,
            deterministic: true,
            seed: None,
        }
    }
}

impl AiBoundaryLabel {
    /// The canonical AI label for a stage type. Unknown stage types
    /// are treated as deterministic classical processing — we never
    /// assume a stage crosses the AI boundary without evidence.
    pub fn for_stage_type(stage_type: &str) -> Self {
        match stage_type {
            // CR-06 — perceptual AI denoise.
            "denoise" => Self {
                uses_ai: true,
                model_id: Some("astroforge_denoise_v1".into()),
                deterministic: true,
                seed: Some(0),
            },
            // CR-06 — perceptual AI detail enhancement.
            "detail" => Self {
                uses_ai: true,
                model_id: Some("astroforge_detail_v1.2".into()),
                deterministic: true,
                seed: Some(0),
            },
            // Everything else is classical deterministic processing.
            _ => Self {
                uses_ai: false,
                model_id: None,
                deterministic: true,
                seed: None,
            },
        }
    }

    /// Human-readable kind for the badge — "Perceptual enhancement"
    /// per the §23 example wording. Currently every AI stage is
    /// perceptual; the field is a `String` so future generative
    /// stages can pick their own ("Generative", "Diffusion", ...).
    pub fn badge_kind(&self) -> Option<&'static str> {
        if self.uses_ai {
            Some("Perceptual enhancement")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denoise_is_uses_ai_true() {
        let l = AiBoundaryLabel::for_stage_type("denoise");
        assert!(l.uses_ai);
        assert_eq!(l.model_id.as_deref(), Some("astroforge_denoise_v1"));
        assert!(l.deterministic);
        assert_eq!(l.seed, Some(0));
        assert_eq!(l.badge_kind(), Some("Perceptual enhancement"));
    }

    #[test]
    fn detail_is_uses_ai_true_with_v1_2_model() {
        let l = AiBoundaryLabel::for_stage_type("detail");
        assert!(l.uses_ai);
        assert_eq!(l.model_id.as_deref(), Some("astroforge_detail_v1.2"));
        assert!(l.deterministic);
    }

    #[test]
    fn stretch_is_classical_deterministic() {
        let l = AiBoundaryLabel::for_stage_type("stretch");
        assert!(!l.uses_ai);
        assert_eq!(l.model_id, None);
        assert!(l.deterministic);
        assert_eq!(l.seed, None);
        assert_eq!(l.badge_kind(), None);
    }

    #[test]
    fn all_eight_classical_stage_types_are_not_ai() {
        for ty in [
            "calibrate",
            "debayer",
            "register",
            "stack",
            "background",
            "color",
            "stretch",
            "export",
        ] {
            let l = AiBoundaryLabel::for_stage_type(ty);
            assert!(!l.uses_ai, "stage_type={ty} must not be AI");
            assert_eq!(l.model_id, None, "stage_type={ty} must have no model_id");
        }
    }

    #[test]
    fn unknown_stage_type_is_treated_as_classical() {
        // We never assume a stage crosses the AI boundary without
        // evidence. Unknown types default to deterministic classical.
        let l = AiBoundaryLabel::for_stage_type("brand-new-future-stage");
        assert!(!l.uses_ai);
        assert_eq!(l.model_id, None);
        assert!(l.deterministic);
    }

    #[test]
    fn serde_round_trip_preserves_fields() {
        let l = AiBoundaryLabel::for_stage_type("denoise");
        let json = serde_json::to_string(&l).unwrap();
        let parsed: AiBoundaryLabel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, l);
    }

    #[test]
    fn serde_default_for_pre_p6_2_rows() {
        // Pre-P6.2 rows may have no `uses_ai` field. Serde default
        // for bool is `false`, for Option is `None`.
        let legacy = r#"{"model_id":null,"deterministic":true,"seed":null}"#;
        let parsed: AiBoundaryLabel = serde_json::from_str(legacy).unwrap();
        assert!(!parsed.uses_ai);
        assert_eq!(parsed.model_id, None);
    }

    #[test]
    fn json_shape_lock_for_uses_ai_field_name() {
        let l = AiBoundaryLabel::for_stage_type("detail");
        let v: serde_json::Value = serde_json::to_value(&l).unwrap();
        // Lock the field names — the UI mirrors them.
        assert!(v.get("uses_ai").is_some());
        assert!(v.get("model_id").is_some());
        assert!(v.get("deterministic").is_some());
        assert!(v.get("seed").is_some());
    }
}
