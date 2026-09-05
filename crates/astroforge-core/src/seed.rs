//! Built-in seed profiles.
//!
//! Profiles seeded into a fresh database so the user has something to
//! load on first run. Adding a new built-in is just a `pub fn` that
//! returns a `Recipe`; `RecipeStore::seed_if_empty()` decides whether to
//! insert it based on the `profile_id`.
//!
//! The DwarfII v1 profile ships core-preservation defaults for the
//! DwarfII Smart Telescope's OSC captures. Param values come from the
//! user's published DwarfII pipeline; see docs/PROFILE_PIPELINES_PLAN.md
//! for the full mapping table.

use crate::recipe::Recipe;

pub const DWARF2_V1_NAME: &str = "DwarfII Smart Telescope \u{00b7} OSC";
pub const DWARF2_V1_TARGET_TYPE: &str = "smart_telescope_osc";

/// Build the canonical DwarfII v1 profile. Pure function — no IO, no
/// side effects. Caller persists via `RecipeStore::save()`.
pub fn dwarf2_v1() -> Recipe {
    let mut r = Recipe::new(DWARF2_V1_NAME, DWARF2_V1_TARGET_TYPE);
    r.description = "Core-preserved detail enhancement for DwarfII Smart Telescope OSC captures. Polynomial background, multiscale wavelet denoise, G2V white balance, midtone 0.40 stretch, PSF deconvolution with core-protection, CLAHE, SCNR green, Lanczos 4K resample, unsharp mask.".into();
    r.version = 1;
    r.parent_version = None;
    r.branch = "main".into();

    // Stage 1: ingest — hot pixel map subtraction (sigma-clipping)
    r.add_stage(
        "ingest",
        hashmap_json(&[("hotPixelSigma", json_number(3.0))]),
    );

    // Stage 1b: background_extraction — polynomial fitting order 3
    r.add_stage(
        "background_extraction",
        hashmap_json(&[
            ("polyOrder", json_number(3.0)),
            ("model", json_string("polynomial")),
        ]),
    );

    // Stage 1c: denoise — multiscale wavelet (layers 1-3, strength 0.25)
    // + adaptive smoothing with edge protection.
    r.add_stage(
        "denoise",
        hashmap_json(&[
            ("method", json_string("wavelet")),
            ("layers", json_number(3.0)),
            ("strength", json_number(0.25)),
            ("edgeProtect", json_bool(true)),
        ]),
    );

    // Stage 2a: color_wb — G2V star reference, background neutralization
    r.add_stage(
        "color_wb",
        hashmap_json(&[
            ("wbReference", json_string("G2V")),
            ("bgNeutralRGB", json_array(&[25.0, 25.0, 25.0])),
        ]),
    );

    // Stage 2b: stretch — midtone 0.40, black 0.02, white 0.98
    r.add_stage(
        "stretch",
        hashmap_json(&[
            ("blackPoint", json_number(0.02)),
            ("midtone", json_number(0.40)),
            ("highlights", json_number(0.98)),
        ]),
    );

    // Stage 3: sharpen_deconvolution — PSF radius 2.0 px, 15 iterations,
    // core-protection mask (Gaussian falloff 0.6) — gated by the
    // session-level `coreProtectMask` flag per D-6.
    r.add_stage(
        "sharpen_deconvolution",
        hashmap_json(&[
            ("psfRadius", json_number(2.0)),
            ("iterations", json_number(15.0)),
            ("coreProtectRequired", json_bool(true)),
            ("coreProtectFalloff", json_number(0.6)),
        ]),
    );

    // Stage 4a: creative_polish — CLAHE (radius 45, clip 0.012),
    // +10% Hα saturation, Lanczos 4K resample, unsharp mask.
    r.add_stage(
        "creative_polish",
        hashmap_json(&[
            ("claheRadius", json_number(45.0)),
            ("claheClip", json_number(0.012)),
            ("saturationBoost", json_number(0.10)),
            ("haOnly", json_bool(true)),
            ("upscaleTarget", json_array(&[4096.0, 3072.0])),
            ("resampleMethod", json_string("lanczos")),
            ("unsharpRadius", json_number(1.2)),
            ("unsharpAmount", json_number(0.25)),
        ]),
    );

    // Stage 4b: color_scnr — SCNR green (strength 0.6). Split out per D-4.
    r.add_stage(
        "color_scnr",
        hashmap_json(&[
            ("strength", json_number(0.6)),
            ("targetChannel", json_string("green")),
        ]),
    );

    r
}

// ─── JSON helpers (kept local so seed.rs has zero deps beyond serde_json) ───

use std::collections::HashMap;

fn hashmap_json(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn json_number(n: f64) -> serde_json::Value {
    serde_json::json!(n)
}
fn json_string(s: &str) -> serde_json::Value {
    serde_json::json!(s)
}
fn json_bool(b: bool) -> serde_json::Value {
    serde_json::json!(b)
}
fn json_array(nums: &[f64]) -> serde_json::Value {
    serde_json::json!(nums)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dwarf2_v1_is_well_formed() {
        let r = dwarf2_v1();
        assert_eq!(r.schema_version, crate::recipe::SCHEMA_VERSION_CURRENT);
        assert_eq!(r.name, DWARF2_V1_NAME);
        assert_eq!(r.target_type, DWARF2_V1_TARGET_TYPE);
        assert_eq!(r.version, 1);
        assert_eq!(r.parent_version, None);
        assert_eq!(r.branch, "main");

        // All 7 stages must be present, in a sensible order.
        let stage_ids: Vec<&str> = r.stages.iter().map(|s| s.stage_id.as_str()).collect();
        assert!(stage_ids.contains(&"ingest"));
        assert!(stage_ids.contains(&"background_extraction"));
        assert!(stage_ids.contains(&"denoise"));
        assert!(stage_ids.contains(&"color_wb"));
        assert!(stage_ids.contains(&"stretch"));
        assert!(stage_ids.contains(&"sharpen_deconvolution"));
        assert!(stage_ids.contains(&"creative_polish"));
        assert!(stage_ids.contains(&"color_scnr"));
    }

    #[test]
    fn test_dwarf2_v1_roundtrips_via_json() {
        let r = dwarf2_v1();
        let json = r.to_json().unwrap();
        let loaded = crate::recipe::Recipe::from_json_migrated(&json).unwrap();
        assert_eq!(loaded.name, r.name);
        assert_eq!(loaded.stages.len(), r.stages.len());
    }

    #[test]
    fn test_dwarf2_v1_no_ai_models() {
        let r = dwarf2_v1();
        assert!(!r.integrity.perceptual_models_used);
        assert!(!r.integrity.deterministic_models_used);
        assert!(r.required_models.is_empty());
    }
}
