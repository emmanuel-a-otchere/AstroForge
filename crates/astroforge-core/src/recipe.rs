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
    /// CR-08 §3.1: marks a Recipe as a system Recipe
    /// that cannot be modified by the user. System Recipes
    /// are seeded by the app (e.g. "M42-Natural-v1") and
    /// are protected from `recipe_save` mutations +
    /// `recipe_delete`. The column is mirrored in the
    /// on-disk `recipes` table for an O(1) guard check.
    #[serde(default)]
    pub is_system: bool,
    /// ISO-8601 timestamp the version was created.
    #[serde(default = "default_empty_string")]
    pub created_at: String,
    /// Profile-level metadata flags (not session-level). Empty by default.
    /// Reserved for future profile metadata; currently unused.
    #[serde(default)]
    pub flags: Vec<String>,
    /// CR-07 §22: Quality Profile axis. Defaults to `Natural`
    /// for legacy Recipes that did not carry this field.
    #[serde(default)]
    pub quality_profile: QualityProfile,
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

/// CR-07 §22: Quality Profile axis.
///
/// Optional profile selection on a Recipe or an
/// applied ImageVersion. Independent of
/// `target_type` and of the per-stage params:
/// the profile is the user's "what kind of
/// output do I want" answer, and the stages are
/// the operational realization.
///
/// Serialized as the variant name string so
/// legacy Recipes (without the field) decode
/// to `Natural` via `serde(default)`.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QualityProfile {
    /// Preserve the original aesthetic; minimal
    /// intervention. Default for legacy data.
    #[default]
    Natural,
    /// Preserve fine structure; resist
    /// sharpening; favour detail over smoothness.
    Detail,
    /// Smooth noise aggressively; favour
    /// smoothness over detail.
    Clean,
    /// Balanced for publication output:
    /// mild smoothing, mild sharpening, strong
    /// integrity gating.
    Publication,
}

impl QualityProfile {
    /// All four variants in display order. Used
    /// by the frontend picker.
    pub const ALL: [QualityProfile; 4] = [
        QualityProfile::Natural,
        QualityProfile::Detail,
        QualityProfile::Clean,
        QualityProfile::Publication,
    ];

    /// Short display label (e.g. "Natural").
    pub fn label(self) -> &'static str {
        match self {
            QualityProfile::Natural => "Natural",
            QualityProfile::Detail => "Detail",
            QualityProfile::Clean => "Clean",
            QualityProfile::Publication => "Publication",
        }
    }

    /// One-line description for the picker UI.
    pub fn description(self) -> &'static str {
        match self {
            QualityProfile::Natural => "Preserve original aesthetic; minimal intervention.",
            QualityProfile::Detail => "Preserve fine structure; resist sharpening.",
            QualityProfile::Clean => "Smooth noise aggressively; favour smoothness.",
            QualityProfile::Publication => "Balanced for publication; mild smoothing + sharpening.",
        }
    }
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
            quality_profile: QualityProfile::default(),
            is_system: false,
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

    /// CR-07 §32.4: content-addressed hash of the Recipe's
    /// pipeline plan. The hash covers every field that
    /// influences the runtime pipeline: schema_version,
    /// name, target_type, stages (with enabled flag + params),
    /// required_models, integrity (perceptual_models_used,
    /// deterministic_models_used, seed_recorded, models),
    /// version, branch, quality_profile.
    ///
    /// The hash deliberately EXCLUDES `description`,
    /// `created_at`, `parent_version`, and `flags` because
    /// those are presentational / lineage / annotation
    /// fields, not pipeline-shape fields.
    ///
    /// Two Recipes produce the same hash iff their pipeline
    /// shape is identical. The hash format is lowercase hex
    /// SHA-256 (64 chars).
    pub fn pipeline_plan_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        // Build a canonical, ordered projection of the
        // pipeline-shape fields. We deliberately use
        // `serde_json::to_value` + a manual reconstruction
        // so the hash stays stable across serde field-order
        // changes (BTreeMap-backed map fields already have
        // deterministic ordering).
        let mut hasher = Sha256::new();
        hasher.update(self.schema_version.as_bytes());
        hasher.update(b"\x00");
        hasher.update(self.name.as_bytes());
        hasher.update(b"\x00");
        hasher.update(self.target_type.as_bytes());
        hasher.update(b"\x00");
        // Quality profile: serialize the lowercase variant
        // name so the hash survives serde rename changes.
        let qp = match self.quality_profile {
            QualityProfile::Natural => "natural",
            QualityProfile::Detail => "detail",
            QualityProfile::Clean => "clean",
            QualityProfile::Publication => "publication",
        };
        hasher.update(qp.as_bytes());
        hasher.update(b"\x00");
        hasher.update(self.version.to_be_bytes());
        hasher.update(b"\x00");
        hasher.update(self.branch.as_bytes());
        hasher.update(b"\x00");
        // Stages: deterministic order by stage_id.
        let mut stage_ids: Vec<&str> = self.stages.iter().map(|s| s.stage_id.as_str()).collect();
        stage_ids.sort();
        for stage_id in stage_ids {
            hasher.update(stage_id.as_bytes());
            hasher.update(b"\x00");
            if let Some(stage) = self.stages.iter().find(|s| s.stage_id == stage_id) {
                hasher.update(if stage.enabled { b"1" } else { b"0" });
                hasher.update(b"\x00");
                // Params: BTreeMap-backed HashMap sorts by
                // key when serialized, but we canonicalize
                // here explicitly so the hash is robust to
                // serde format drift.
                let params: std::collections::BTreeMap<&str, &serde_json::Value> =
                    stage.params.iter().map(|(k, v)| (k.as_str(), v)).collect();
                for (k, v) in params.iter() {
                    hasher.update(k.as_bytes());
                    hasher.update(b"\x00");
                    hasher.update(v.to_string().as_bytes());
                    hasher.update(b"\x00");
                }
            }
        }
        // Required models: sorted for determinism.
        let mut required: Vec<&str> = self.required_models.iter().map(|s| s.as_str()).collect();
        required.sort();
        for m in required {
            hasher.update(m.as_bytes());
            hasher.update(b"\x00");
        }
        // Integrity badge: booleans + sorted model list.
        hasher.update(if self.integrity.perceptual_models_used {
            b"1"
        } else {
            b"0"
        });
        hasher.update(b"\x00");
        hasher.update(if self.integrity.deterministic_models_used {
            b"1"
        } else {
            b"0"
        });
        hasher.update(b"\x00");
        hasher.update(if self.integrity.seed_recorded {
            b"1"
        } else {
            b"0"
        });
        hasher.update(b"\x00");
        let mut model_usages = self.integrity.models.clone();
        model_usages.sort_by(|a, b| a.model_name.cmp(&b.model_name));
        for mu in model_usages {
            hasher.update(mu.model_name.as_bytes());
            hasher.update(b"\x00");
            let mt = match mu.model_type {
                ModelType::Deterministic => "deterministic",
                ModelType::Perceptual => "perceptual",
            };
            hasher.update(mt.as_bytes());
            hasher.update(b"\x00");
        }
        let digest = hasher.finalize();
        // Lowercase hex, 64 chars.
        let mut out = String::with_capacity(64);
        for byte in digest {
            out.push_str(&format!("{:02x}", byte));
        }
        out
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

    // CR-07 §22: QualityProfile coverage.

    #[test]
    fn test_quality_profile_default_is_natural() {
        let r = Recipe::new("X", "y");
        assert_eq!(r.quality_profile, QualityProfile::Natural);
        assert_eq!(QualityProfile::default(), QualityProfile::Natural);
    }

    #[test]
    fn test_quality_profile_all_returns_four_variants() {
        assert_eq!(QualityProfile::ALL.len(), 4);
        // Display order: Natural, Detail, Clean, Publication.
        assert_eq!(QualityProfile::ALL[0], QualityProfile::Natural);
        assert_eq!(QualityProfile::ALL[1], QualityProfile::Detail);
        assert_eq!(QualityProfile::ALL[2], QualityProfile::Clean);
        assert_eq!(QualityProfile::ALL[3], QualityProfile::Publication);
    }

    #[test]
    fn test_quality_profile_label_and_description() {
        assert_eq!(QualityProfile::Natural.label(), "Natural");
        assert_eq!(QualityProfile::Detail.label(), "Detail");
        assert_eq!(QualityProfile::Clean.label(), "Clean");
        assert_eq!(QualityProfile::Publication.label(), "Publication");

        // Every variant must have a non-empty
        // description (used by the frontend picker).
        for v in QualityProfile::ALL {
            assert!(!v.description().is_empty(), "{:?} has empty description", v);
        }
    }

    #[test]
    fn test_quality_profile_legacy_recipe_defaults_to_natural() {
        // A Recipe JSON without the quality_profile
        // field (legacy data) must decode as Natural.
        let legacy = r#"{
            "schema_version": "2.0",
            "name": "Legacy",
            "description": "",
            "target_type": "deep_sky",
            "stages": [],
            "required_models": [],
            "integrity": {
                "perceptual_models_used": false,
                "deterministic_models_used": false,
                "seed_recorded": false,
                "models": []
            }
        }"#;
        let r: Recipe = serde_json::from_str(legacy).unwrap();
        assert_eq!(r.quality_profile, QualityProfile::Natural);
    }

    #[test]
    fn test_quality_profile_serde_round_trip_all_variants() {
        for v in QualityProfile::ALL {
            let mut r = Recipe::new("X", "y");
            r.quality_profile = v;
            let json = serde_json::to_string(&r).unwrap();
            let back: Recipe = serde_json::from_str(&json).unwrap();
            assert_eq!(back.quality_profile, v, "round-trip failed for {:?}", v);
        }
    }

    // CR-07 §32.4: pipeline_plan_hash tests.

    fn minimal_recipe() -> Recipe {
        let mut r = Recipe::new("test-recipe", "stretch");
        r.add_stage("stretch", Default::default());
        r
    }

    #[test]
    fn pipeline_plan_hash_is_64_char_lowercase_hex() {
        let r = minimal_recipe();
        let h = r.pipeline_plan_hash();
        assert_eq!(h.len(), 64, "SHA-256 hex must be 64 chars");
        assert!(
            h.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "hash must be lowercase hex"
        );
    }

    #[test]
    fn pipeline_plan_hash_is_stable_across_calls() {
        let r = minimal_recipe();
        let h1 = r.pipeline_plan_hash();
        let h2 = r.pipeline_plan_hash();
        assert_eq!(h1, h2, "same Recipe must hash to the same value");
    }

    #[test]
    fn pipeline_plan_hash_changes_with_schema_version() {
        let mut a = minimal_recipe();
        a.schema_version = "2.0".into();
        let mut b = a.clone();
        b.schema_version = "2.1".into();
        assert_ne!(
            a.pipeline_plan_hash(),
            b.pipeline_plan_hash(),
            "schema_version must influence the hash"
        );
    }

    #[test]
    fn pipeline_plan_hash_changes_with_stages() {
        let a = minimal_recipe();
        let mut b = minimal_recipe();
        b.add_stage(
            "grain",
            [("amount".to_string(), serde_json::json!(0.5))]
                .into_iter()
                .collect(),
        );
        assert_ne!(
            a.pipeline_plan_hash(),
            b.pipeline_plan_hash(),
            "additional stages must change the hash"
        );
    }

    #[test]
    fn pipeline_plan_hash_changes_with_stage_params() {
        let mut a = minimal_recipe();
        a.add_stage(
            "stretch",
            [("amount".to_string(), serde_json::json!(0.1))]
                .into_iter()
                .collect(),
        );
        let mut b = a.clone();
        // Replace the stage with one that has a different param value.
        b.stages.clear();
        b.add_stage(
            "stretch",
            [("amount".to_string(), serde_json::json!(0.9))]
                .into_iter()
                .collect(),
        );
        assert_ne!(
            a.pipeline_plan_hash(),
            b.pipeline_plan_hash(),
            "different stage params must change the hash"
        );
    }

    #[test]
    fn pipeline_plan_hash_ignores_description_and_created_at() {
        let mut a = minimal_recipe();
        a.description = "alpha".into();
        a.created_at = "2026-01-01T00:00:00Z".into();
        let mut b = a.clone();
        b.description = "beta (totally different)".into();
        b.created_at = "2099-12-31T23:59:59Z".into();
        assert_eq!(
            a.pipeline_plan_hash(),
            b.pipeline_plan_hash(),
            "description + created_at must NOT influence the hash"
        );
    }

    #[test]
    fn pipeline_plan_hash_ignores_parent_version_and_flags() {
        let mut a = minimal_recipe();
        a.parent_version = Some(1);
        a.flags = vec!["x".into(), "y".into()];
        let mut b = a.clone();
        b.parent_version = Some(42);
        b.flags = vec!["z".into(), "w".into(), "v".into()];
        assert_eq!(
            a.pipeline_plan_hash(),
            b.pipeline_plan_hash(),
            "parent_version + flags must NOT influence the hash"
        );
    }

    #[test]
    fn pipeline_plan_hash_changes_with_integrity_models() {
        let a = minimal_recipe();
        let mut b = minimal_recipe();
        b.add_model("bm3d", ModelType::Deterministic);
        assert_ne!(
            a.pipeline_plan_hash(),
            b.pipeline_plan_hash(),
            "adding a deterministic model must change the hash"
        );
    }

    #[test]
    fn pipeline_plan_hash_changes_with_ai_model() {
        let a = minimal_recipe();
        let mut b = minimal_recipe();
        b.add_model("neural-denoiser", ModelType::Perceptual);
        assert_ne!(
            a.pipeline_plan_hash(),
            b.pipeline_plan_hash(),
            "adding a perceptual model must change the hash"
        );
    }

    // CR-07 §32.4: recipe_ai_diff_summary tests.

    fn natural_recipe() -> Recipe {
        minimal_recipe()
    }

    fn ai_recipe() -> Recipe {
        let mut r = Recipe::new("ai-recipe", "stretch");
        r.add_stage("stretch", Default::default());
        r.add_model("neural-denoiser", ModelType::Perceptual);
        r
    }

    #[test]
    fn ai_diff_summary_natural_vs_ai_classification_differs() {
        let a = natural_recipe();
        let b = ai_recipe();
        let s = recipe_ai_diff_summary(&a, &b);
        assert!(!s.ai_used_a);
        assert!(s.ai_used_b);
        assert!(s.ai_classification_differs);
        assert_eq!(s.perceptual_models_b, vec!["neural-denoiser"]);
        assert!(s.perceptual_models_a.is_empty());
    }

    #[test]
    fn ai_diff_summary_both_natural_no_classification_diff() {
        let a = natural_recipe();
        let b = natural_recipe();
        let s = recipe_ai_diff_summary(&a, &b);
        assert!(!s.ai_classification_differs);
        assert!(!s.ai_used_a && !s.ai_used_b);
    }

    #[test]
    fn ai_diff_summary_both_ai_no_classification_diff() {
        let a = ai_recipe();
        let b = ai_recipe();
        let s = recipe_ai_diff_summary(&a, &b);
        assert!(!s.ai_classification_differs);
        assert!(s.ai_used_a && s.ai_used_b);
        assert!(!s.hash_differs);
    }

    #[test]
    fn ai_diff_summary_identical_recipes_no_hash_diff() {
        let a = natural_recipe();
        let b = natural_recipe();
        let s = recipe_ai_diff_summary(&a, &b);
        assert!(!s.hash_differs);
        assert_eq!(s.hash_a, s.hash_b);
    }

    #[test]
    fn ai_diff_summary_different_stages_hash_differs() {
        let a = natural_recipe();
        let b = ai_recipe();
        let s = recipe_ai_diff_summary(&a, &b);
        assert!(
            s.hash_differs,
            "natural vs AI must produce different hashes"
        );
        assert_ne!(s.hash_a, s.hash_b);
    }

    #[test]
    fn ai_diff_summary_provenance_line_format() {
        let mut a = natural_recipe();
        a.version = 3;
        let mut b = ai_recipe();
        b.version = 5;
        let s = recipe_ai_diff_summary(&a, &b);
        assert_eq!(
            s.provenance,
            "Recipe A v3 (natural, natural) vs Recipe B v5 (ai, natural)"
        );
    }

    #[test]
    fn ai_diff_summary_surfaces_required_models_difference() {
        let a = natural_recipe();
        let mut b = ai_recipe();
        b.required_models.push("extra-model".into());
        let s = recipe_ai_diff_summary(&a, &b);
        assert!(s.required_models_differ);
    }

    #[test]
    fn ai_diff_summary_surfaces_quality_profile_difference() {
        let mut a = natural_recipe();
        a.quality_profile = QualityProfile::Natural;
        let mut b = natural_recipe();
        b.quality_profile = QualityProfile::Publication;
        let s = recipe_ai_diff_summary(&a, &b);
        assert_ne!(s.quality_profile_a, s.quality_profile_b);
        assert_eq!(s.quality_profile_a, QualityProfile::Natural);
        assert_eq!(s.quality_profile_b, QualityProfile::Publication);
    }

    #[test]
    fn ai_diff_summary_surfaces_schema_version_difference() {
        let mut a = natural_recipe();
        a.schema_version = "2.0".into();
        let mut b = a.clone();
        b.schema_version = "1.0".into();
        let s = recipe_ai_diff_summary(&a, &b);
        assert_eq!(s.schema_version_a, "2.0");
        assert_eq!(s.schema_version_b, "1.0");
        assert!(s.hash_differs);
    }
}

// CR-07 §32.4: AI comparison surface.
//
// Given two Recipes (one for Version A, one for Version B),
// `recipe_ai_diff_summary` surfaces the audit's
// "model / version / hash / classification / parameters /
// provenance" comparison fields in a single struct that
// the comparison surface can render directly.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeAiDiffSummary {
    /// Pipeline plan hash of Recipe A (lowercase hex SHA-256).
    pub hash_a: String,
    /// Pipeline plan hash of Recipe B.
    pub hash_b: String,
    /// True iff `hash_a != hash_b`.
    pub hash_differs: bool,
    /// AI-classification flag for Recipe A
    /// (`integrity.perceptual_models_used`).
    pub ai_used_a: bool,
    /// AI-classification flag for Recipe B.
    pub ai_used_b: bool,
    /// True iff one Recipe used perceptual (AI) models and
    /// the other did not.
    pub ai_classification_differs: bool,
    /// Linear version counter of Recipe A.
    pub version_a: u32,
    /// Linear version counter of Recipe B.
    pub version_b: u32,
    /// Schema version string of Recipe A.
    pub schema_version_a: String,
    /// Schema version string of Recipe B.
    pub schema_version_b: String,
    /// Quality profile of Recipe A.
    pub quality_profile_a: QualityProfile,
    /// Quality profile of Recipe B.
    pub quality_profile_b: QualityProfile,
    /// True iff the two Recipes name different required models.
    pub required_models_differ: bool,
    /// Names of perceptual models used by Recipe A.
    pub perceptual_models_a: Vec<String>,
    /// Names of perceptual models used by Recipe B.
    pub perceptual_models_b: Vec<String>,
    /// Human-readable provenance line for the comparison row.
    /// Format: "Recipe A v{N} ({ai/natural}, {profile}) vs Recipe B v{M} ...".
    pub provenance: String,
}

/// Build the AI comparison summary for two Recipes.
///
/// Pure function. Reads only the two Recipes; produces a
/// summary struct. The comparison surface (UI) renders this
/// struct directly without re-deriving any of the fields.
pub fn recipe_ai_diff_summary(a: &Recipe, b: &Recipe) -> RecipeAiDiffSummary {
    let hash_a = a.pipeline_plan_hash();
    let hash_b = b.pipeline_plan_hash();
    let hash_differs = hash_a != hash_b;
    let ai_used_a = a.integrity.perceptual_models_used;
    let ai_used_b = b.integrity.perceptual_models_used;
    // Perceptual-model name lists (sorted by name for stable
    // output).
    let mut perceptual_models_a: Vec<String> = a
        .integrity
        .models
        .iter()
        .filter(|m| m.model_type == ModelType::Perceptual)
        .map(|m| m.model_name.clone())
        .collect();
    perceptual_models_a.sort();
    let mut perceptual_models_b: Vec<String> = b
        .integrity
        .models
        .iter()
        .filter(|m| m.model_type == ModelType::Perceptual)
        .map(|m| m.model_name.clone())
        .collect();
    perceptual_models_b.sort();
    let required_models_differ = a.required_models != b.required_models;
    let ai_label = |used: bool| if used { "ai" } else { "natural" };
    let qp_label = |qp: QualityProfile| match qp {
        QualityProfile::Natural => "natural",
        QualityProfile::Detail => "detail",
        QualityProfile::Clean => "clean",
        QualityProfile::Publication => "publication",
    };
    let provenance = format!(
        "Recipe A v{} ({}, {}) vs Recipe B v{} ({}, {})",
        a.version,
        ai_label(ai_used_a),
        qp_label(a.quality_profile),
        b.version,
        ai_label(ai_used_b),
        qp_label(b.quality_profile),
    );
    RecipeAiDiffSummary {
        hash_a,
        hash_b,
        hash_differs,
        ai_used_a,
        ai_used_b,
        ai_classification_differs: ai_used_a != ai_used_b,
        version_a: a.version,
        version_b: b.version,
        schema_version_a: a.schema_version.clone(),
        schema_version_b: b.schema_version.clone(),
        quality_profile_a: a.quality_profile,
        quality_profile_b: b.quality_profile,
        required_models_differ,
        perceptual_models_a,
        perceptual_models_b,
        provenance,
    }
}
