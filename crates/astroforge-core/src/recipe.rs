use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-stage parameter map produced by [`apply_recipe`].
pub type StageParams = Vec<(String, HashMap<String, serde_json::Value>)>;

// ─── Schema versioning ───────────────────────────────────────────────────────
//
// Recipe's on-disk schema has been bumped from "1.0" to "2.0" to carry
// per-version metadata (version, parent_version, branch, created_at).
// `migrate_v1_to_v2()` upgrades old JSON transparently; `Recipe::new`
// and all builders produce v2 going forward.

pub const SCHEMA_VERSION_V1: &str = "1.0";
pub const SCHEMA_VERSION_CURRENT: &str = "2.0";

/// Outcome of [`migrate_recipe`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationResult {
    /// Recipe was already at the current schema; no changes made.
    AlreadyCurrent,
    /// Recipe was successfully upgraded; carries the new (current) version.
    Migrated,
    /// Recipe is at a newer schema than this binary knows about.
    UnknownFuture(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub schema_version: String,
    pub name: String,
    pub description: String,
    pub target_type: String,
    pub stages: Vec<RecipeStage>,
    pub required_models: Vec<String>,
    pub integrity: IntegrityBadge,
    /// Linear version counter for this profile (1-based). v1 is the first
    /// save of a given profile name+target_type; every save increments.
    #[serde(default = "default_version")]
    pub version: u32,
    /// The version this one was saved from. `None` for the first version.
    #[serde(default)]
    pub parent_version: Option<u32>,
    /// Branch name. D-1 = 2 (linear) means this is always `"main"`; the
    /// field is kept for future flexibility and to keep schema migration
    /// straightforward.
    #[serde(default = "default_branch")]
    pub branch: String,
    /// ISO-8601 timestamp the version was created.
    #[serde(default = "default_empty_string")]
    pub created_at: String,
    /// Profile-level metadata flags (not session-level). Empty by default.
    /// Reserved for future profile metadata; currently unused.
    #[serde(default)]
    pub flags: Vec<String>,
}

fn default_version() -> u32 {
    1
}
fn default_branch() -> String {
    "main".to_string()
}
fn default_empty_string() -> String {
    String::new()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStage {
    pub stage_id: String,
    pub enabled: bool,
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityBadge {
    pub perceptual_models_used: bool,
    pub deterministic_models_used: bool,
    pub seed_recorded: bool,
    pub models: Vec<ModelUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model_name: String,
    pub model_type: ModelType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelType {
    Deterministic,
    Perceptual,
}

impl Recipe {
    /// Build a new v2 Recipe. The current schema is always produced.
    pub fn new(name: &str, target_type: &str) -> Self {
        Self {
            schema_version: SCHEMA_VERSION_CURRENT.into(),
            name: name.into(),
            description: String::new(),
            target_type: target_type.into(),
            stages: Vec::new(),
            required_models: Vec::new(),
            integrity: IntegrityBadge {
                perceptual_models_used: false,
                deterministic_models_used: false,
                seed_recorded: false,
                models: Vec::new(),
            },
            version: 1,
            parent_version: None,
            branch: "main".into(),
            created_at: String::new(),
            flags: Vec::new(),
        }
    }

    pub fn add_stage(&mut self, stage_id: &str, params: HashMap<String, serde_json::Value>) {
        self.stages.push(RecipeStage {
            stage_id: stage_id.into(),
            enabled: true,
            params,
        });
    }

    pub fn add_model(&mut self, name: &str, model_type: ModelType) {
        self.required_models.push(name.into());
        self.integrity.models.push(ModelUsage {
            model_name: name.into(),
            model_type: model_type.clone(),
        });
        match model_type {
            ModelType::Deterministic => self.integrity.deterministic_models_used = true,
            ModelType::Perceptual => self.integrity.perceptual_models_used = true,
        }
    }

    pub fn set_seed_recorded(&mut self, recorded: bool) {
        self.integrity.seed_recorded = recorded;
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a Recipe from JSON, running migration if the on-disk
    /// schema is older than the current. Refuses to load schemas newer
    /// than what this binary knows about (caller should treat that as a
    /// hard error).
    pub fn from_json_migrated(json: &str) -> Result<Self, serde_json::Error> {
        let mut recipe: Self = serde_json::from_str(json)?;
        let result = migrate_recipe(&mut recipe);
        match result {
            MigrationResult::AlreadyCurrent | MigrationResult::Migrated => Ok(recipe),
            MigrationResult::UnknownFuture(v) => Err(serde_json::Error::custom(format!(
                "unknown future schema_version: {v}"
            ))),
        }
    }

    /// Raw deserialization without migration. Mostly for tests and for
    /// callers that want to inspect schema versions directly.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// In-place schema migration. Returns the outcome so callers can decide
/// whether to persist the migrated form.
///
/// - `AlreadyCurrent` — `schema_version == "2.0"`, nothing changed.
/// - `Migrated` — was `"1.0"` (or unknown-older); fields added, version bumped.
/// - `UnknownFuture` — `schema_version` is newer than `"2.0"`; we don't
///   try to guess at unknown future fields, the recipe is left as-is.
pub fn migrate_recipe(recipe: &mut Recipe) -> MigrationResult {
    match recipe.schema_version.as_str() {
        SCHEMA_VERSION_CURRENT => MigrationResult::AlreadyCurrent,
        SCHEMA_VERSION_V1 => {
            migrate_v1_to_v2(recipe);
            MigrationResult::Migrated
        }
        other if other > SCHEMA_VERSION_CURRENT => {
            MigrationResult::UnknownFuture(other.to_string())
        }
        // Unknown older version — treat like v1 since the only field we'd
        // need is `schema_version`, which is already present.
        _ => {
            migrate_v1_to_v2(recipe);
            MigrationResult::Migrated
        }
    }
}

fn migrate_v1_to_v2(recipe: &mut Recipe) {
    // v1 had no version tracking. The first migration is always v1 with
    // parent_version=None and a fresh branch.
    recipe.version = 1;
    recipe.parent_version = None;
    recipe.branch = "main".into();
    recipe.created_at = String::new();
    recipe.flags = Vec::new();
    recipe.schema_version = SCHEMA_VERSION_CURRENT.into();
}

pub fn sanitize_recipe(recipe: &mut Recipe) {
    for stage in &mut recipe.stages {
        let keys_to_remove: Vec<String> = stage
            .params
            .iter()
            .filter(|(k, _)| {
                let key = k.to_lowercase();
                key.contains("path")
                    || key.contains("dir")
                    || key.contains("gps")
                    || key.contains("lat")
                    || key.contains("lon")
                    || key.contains("machine")
                    || key.contains("hostname")
                    || key.contains("user")
            })
            .map(|(k, _)| k.clone())
            .collect();

        for key in keys_to_remove {
            stage.params.remove(&key);
        }
    }
}

pub fn validate_compatibility(recipe: &Recipe, available_models: &[String]) -> ValidationResult {
    let missing: Vec<String> = recipe
        .required_models
        .iter()
        .filter(|m| !available_models.contains(m))
        .cloned()
        .collect();

    if !missing.is_empty() {
        return ValidationResult::MissingModels(missing);
    }

    if recipe.schema_version != SCHEMA_VERSION_CURRENT {
        return ValidationResult::IncompatibleVersion(recipe.schema_version.clone());
    }

    ValidationResult::Compatible
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Compatible,
    MissingModels(Vec<String>),
    IncompatibleVersion(String),
}

pub fn apply_recipe(
    recipe: &Recipe,
    available_models: &[String],
) -> Result<StageParams, ApplyError> {
    match validate_compatibility(recipe, available_models) {
        ValidationResult::Compatible => {}
        ValidationResult::MissingModels(missing) => {
            return Err(ApplyError::MissingModels(missing));
        }
        ValidationResult::IncompatibleVersion(v) => {
            return Err(ApplyError::IncompatibleVersion(v));
        }
    }

    let mut stage_params = Vec::new();
    for stage in &recipe.stages {
        if stage.enabled {
            stage_params.push((stage.stage_id.clone(), stage.params.clone()));
        }
    }

    Ok(stage_params)
}

pub fn integrity_label(badge: &IntegrityBadge) -> String {
    if badge.perceptual_models_used {
        "Perceptual AI used".into()
    } else if badge.deterministic_models_used {
        "Deterministic AI used".into()
    } else {
        "No AI models".into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("Missing models: {0:?}")]
    MissingModels(Vec<String>),
    #[error("Incompatible version: {0}")]
    IncompatibleVersion(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_creation() {
        let recipe = Recipe::new("My Recipe", "deep_sky");
        assert_eq!(recipe.schema_version, SCHEMA_VERSION_CURRENT);
        assert_eq!(recipe.name, "My Recipe");
        assert_eq!(recipe.version, 1);
        assert_eq!(recipe.parent_version, None);
        assert_eq!(recipe.branch, "main");
        assert!(recipe.stages.is_empty());
    }

    #[test]
    fn test_v1_json_migrates_to_v2() {
        let v1_json = r#"{
            "schema_version": "1.0",
            "name": "DwarfII",
            "description": "test",
            "target_type": "smart_telescope_osc",
            "stages": [],
            "required_models": [],
            "integrity": {
                "perceptual_models_used": false,
                "deterministic_models_used": false,
                "seed_recorded": false,
                "models": []
            }
        }"#;
        let recipe = Recipe::from_json_migrated(v1_json).unwrap();
        assert_eq!(recipe.schema_version, SCHEMA_VERSION_CURRENT);
        assert_eq!(recipe.version, 1);
        assert_eq!(recipe.parent_version, None);
        assert_eq!(recipe.branch, "main");
    }

    #[test]
    fn test_already_current_is_no_op() {
        let mut recipe = Recipe::new("X", "y");
        recipe.version = 7;
        let result = migrate_recipe(&mut recipe);
        assert_eq!(result, MigrationResult::AlreadyCurrent);
        assert_eq!(recipe.version, 7);
    }

    #[test]
    fn test_v1_migration_marks_as_migrated() {
        let mut recipe = Recipe::new("X", "y");
        recipe.schema_version = SCHEMA_VERSION_V1.into();
        let result = migrate_recipe(&mut recipe);
        assert_eq!(result, MigrationResult::Migrated);
        assert_eq!(recipe.schema_version, SCHEMA_VERSION_CURRENT);
    }

    #[test]
    fn test_unknown_future_version_errors() {
        let future_json = r#"{
            "schema_version": "99.0",
            "name": "X",
            "description": "",
            "target_type": "deep_sky",
            "stages": [],
            "required_models": [],
            "integrity": {
                "perceptual_models_used": false,
                "deterministic_models_used": false,
                "seed_recorded": false,
                "models": []
            },
            "version": 1,
            "parent_version": null,
            "branch": "main",
            "created_at": "",
            "flags": []
        }"#;
        let result = Recipe::from_json_migrated(future_json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown future"));
    }

    #[test]
    fn test_add_stage_and_model() {
        let mut recipe = Recipe::new("Test", "deep_sky");
        let mut params = HashMap::new();
        params.insert("kappa".into(), serde_json::json!(3.0));
        recipe.add_stage("stacking", params);
        recipe.add_model("swinir-denoise-astro", ModelType::Deterministic);

        assert_eq!(recipe.stages.len(), 1);
        assert!(recipe.integrity.deterministic_models_used);
        assert!(!recipe.integrity.perceptual_models_used);
    }

    #[test]
    fn test_recipe_json_roundtrip() {
        let mut recipe = Recipe::new("Test", "deep_sky");
        recipe.add_stage("stretching", HashMap::new());
        let json = recipe.to_json().unwrap();
        let parsed = Recipe::from_json(&json).unwrap();
        assert_eq!(parsed.name, "Test");
        assert_eq!(parsed.stages.len(), 1);
    }

    #[test]
    fn test_sanitize_recipe_strips_paths() {
        let mut recipe = Recipe::new("Test", "deep_sky");
        let mut params = HashMap::new();
        params.insert("source_path".into(), serde_json::json!("/home/user/data"));
        params.insert("gps_lat".into(), serde_json::json!(51.5));
        params.insert("kappa".into(), serde_json::json!(3.0));
        recipe.add_stage("stacking", params);
        sanitize_recipe(&mut recipe);

        assert!(!recipe.stages[0].params.contains_key("source_path"));
        assert!(!recipe.stages[0].params.contains_key("gps_lat"));
        assert!(recipe.stages[0].params.contains_key("kappa"));
    }

    #[test]
    fn test_validate_compatibility_ok() {
        let recipe = Recipe::new("Test", "deep_sky");
        let available = vec!["swinir-denoise-astro".to_string()];
        assert_eq!(
            validate_compatibility(&recipe, &available),
            ValidationResult::Compatible
        );
    }

    #[test]
    fn test_validate_compatibility_missing_models() {
        let mut recipe = Recipe::new("Test", "deep_sky");
        recipe.add_model("swinir-denoise-astro", ModelType::Deterministic);
        let available = vec!["other-model".to_string()];
        match validate_compatibility(&recipe, &available) {
            ValidationResult::MissingModels(m) => {
                assert!(m.contains(&"swinir-denoise-astro".to_string()))
            }
            _ => panic!("Expected MissingModels"),
        }
    }

    #[test]
    fn test_apply_recipe() {
        let mut recipe = Recipe::new("Test", "deep_sky");
        let mut params = HashMap::new();
        params.insert("kappa".into(), serde_json::json!(3.0));
        recipe.add_stage("stacking", params);
        let available = vec![];
        let result = apply_recipe(&recipe, &available).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "stacking");
    }

    #[test]
    fn test_integrity_label() {
        let badge = IntegrityBadge {
            perceptual_models_used: true,
            deterministic_models_used: false,
            seed_recorded: true,
            models: vec![],
        };
        assert_eq!(integrity_label(&badge), "Perceptual AI used");

        let badge = IntegrityBadge {
            perceptual_models_used: false,
            deterministic_models_used: false,
            seed_recorded: false,
            models: vec![],
        };
        assert_eq!(integrity_label(&badge), "No AI models");
    }

    // ─── Phase 1.5 PR-B: apply_recipe + DwarfII seed ────────────────────────
    //
    // Mirrors the TS `applyProfileToPipeline` semantics. These tests
    // guard the round-trip: a recipe saved via the UI should be
    // re-applicable to a fresh graph with the same params intact.

    #[test]
    fn test_apply_dwarf2_v1_yields_eight_enabled_stages() {
        let r = crate::seed::dwarf2_v1();
        let available: Vec<String> = vec![]; // no models needed for v1
        let result = apply_recipe(&r, &available).expect("compatible");
        assert_eq!(result.len(), 8, "all 8 stages are enabled in v1");

        let stage_ids: Vec<&str> = result.iter().map(|(id, _)| id.as_str()).collect();
        for expected in [
            "ingest",
            "background_extraction",
            "denoise",
            "color_wb",
            "stretch",
            "sharpen_deconvolution",
            "creative_polish",
            "color_scnr",
        ] {
            assert!(stage_ids.contains(&expected), "missing stage {expected}");
        }
    }

    #[test]
    fn test_apply_dwarf2_v1_params_match_user_published_values() {
        let r = crate::seed::dwarf2_v1();
        let result = apply_recipe(&r, &[]).unwrap();

        // Spot-check the most distinctive user-published values to make
        // sure the seed didn't drift.
        let by_id: std::collections::HashMap<
            String,
            std::collections::HashMap<String, serde_json::Value>,
        > = result.into_iter().collect();

        let stretch = by_id.get("stretch").expect("stretch stage");
        assert_eq!(
            stretch.get("midtone").and_then(|v| v.as_f64()),
            Some(0.40),
            "midtone must be 0.40 per user"
        );
        assert_eq!(
            stretch.get("blackPoint").and_then(|v| v.as_f64()),
            Some(0.02)
        );
        assert_eq!(
            stretch.get("highlights").and_then(|v| v.as_f64()),
            Some(0.98)
        );

        let deconv = by_id.get("sharpen_deconvolution").expect("deconv stage");
        assert_eq!(
            deconv.get("iterations").and_then(|v| v.as_f64()),
            Some(15.0),
            "iterations 15 (down from 25) per user core-preservation refinement"
        );
        assert_eq!(
            deconv.get("coreProtectRequired"),
            Some(&serde_json::json!(true))
        );

        let scnr = by_id.get("color_scnr").expect("scnr stage");
        assert_eq!(scnr.get("strength").and_then(|v| v.as_f64()), Some(0.6));

        let polish = by_id.get("creative_polish").expect("polish stage");
        assert_eq!(
            polish.get("resampleMethod"),
            Some(&serde_json::json!("lanczos"))
        );
        assert_eq!(
            polish.get("upscaleTarget"),
            Some(&serde_json::json!([4096.0, 3072.0]))
        );
    }

    #[test]
    fn test_apply_recipe_skips_disabled_stages() {
        let mut r = Recipe::new("Custom", "deep_sky");
        r.add_stage("denoise", Default::default());
        // Disable denoise by mutating the existing entry.
        r.stages[0].enabled = false;
        r.add_stage("stretch", Default::default());

        let result = apply_recipe(&r, &[]).unwrap();
        let ids: Vec<&str> = result.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, vec!["stretch"], "disabled denoise must be skipped");
    }
}
