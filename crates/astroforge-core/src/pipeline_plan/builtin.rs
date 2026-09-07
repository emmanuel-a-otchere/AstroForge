//! CR-05 P1 — built-in recipes (Decision D-CR05-3 + PLAN.md P1).
//!
//! Each recipe returns `(Vec<StageSpec>, ObjectType)` for the plan
//! generator. P1 ships one recipe: Deep-Sky OSC Balanced. P5+ will
//! land Deep-Sky OSC High Quality, Planetary, Narrowband, etc.
//!
//! Each recipe is a small, testable function — no I/O, no side effects.

use crate::domain::ObjectType;
use crate::pipeline_plan::stage::{StageSpec, StageType};

/// CR-05 §18 — Deep-Sky OSC Balanced.
///
/// 10 stages: Calibrate → Debayer → QualityFilter → Register → Stack →
/// Background → Color → Stretch → Denoise → Detail. Calibrate, Debayer,
/// QualityFilter, Register, Stack, Color, Stretch are required; Background,
/// Denoise, Detail are optional (default-disabled per CR-05 §18).
pub fn deep_sky_osc_balanced() -> (Vec<StageSpec>, ObjectType) {
    let stages = vec![
        StageSpec::required(StageType::Calibrate),
        StageSpec::required(StageType::Debayer),
        StageSpec::required(StageType::QualityFilter),
        StageSpec::required(StageType::Register),
        StageSpec::required(StageType::Stack),
        StageSpec::optional(StageType::Background),
        StageSpec::required(StageType::Color),
        StageSpec::required(StageType::Stretch),
        StageSpec::optional(StageType::Denoise),
        StageSpec::optional(StageType::Detail),
    ];
    (stages, ObjectType::DeepSky)
}

/// CR-05 §18 — Deep-Sky OSC High Quality.
///
/// Same shape as Balanced but with optional stages enabled by default
/// (Background, Denoise, Detail). P5 candidate; placeholder for now so
/// the registry compiles. P5 lands the full implementation.
#[allow(dead_code)]
pub fn deep_sky_osc_high_quality() -> (Vec<StageSpec>, ObjectType) {
    let mut stages = deep_sky_osc_balanced().0;
    for spec in stages.iter_mut() {
        if !spec.required {
            spec.enabled = true;
        }
    }
    (stages, ObjectType::DeepSky)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_recipe_has_ten_stages() {
        let (stages, target) = deep_sky_osc_balanced();
        assert_eq!(stages.len(), 10);
        assert_eq!(target, ObjectType::DeepSky);
    }

    #[test]
    fn balanced_recipe_required_vs_optional() {
        let (stages, _) = deep_sky_osc_balanced();
        let required: Vec<&StageSpec> = stages.iter().filter(|s| s.required).collect();
        let optional: Vec<&StageSpec> = stages.iter().filter(|s| !s.required).collect();
        // Calibrate, Debayer, QualityFilter, Register, Stack, Color,
        // Stretch (7 required).
        assert_eq!(required.len(), 7);
        // Background, Denoise, Detail (3 optional).
        assert_eq!(optional.len(), 3);
    }

    #[test]
    fn high_quality_enables_optional() {
        let (stages, _) = deep_sky_osc_high_quality();
        let all_enabled = stages.iter().all(|s| s.enabled);
        assert!(
            all_enabled,
            "high quality should enable all optional stages by default"
        );
    }
}
