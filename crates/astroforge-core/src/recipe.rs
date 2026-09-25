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
    /// CR-08 §10: AI Enhancement Level axis (Beginner
    /// editor tier). Defaults to `Recommended` via
    /// `#[serde(default)]` so legacy Recipes deserialize
    /// cleanly. The Beginner tier picker writes this
    /// field; the Guided + Expert tiers may override it
    /// per-stage without touching the recipe-level
    /// value (the recipe-level value is the default
    /// when no per-stage override is set).
    #[serde(default)]
    pub ai_enhancement_level: AiEnhancementLevel,
    /// CR-08 §10.2 Guided tier: processing objectives
    /// (high-level goals the user wants the pipeline
    /// to honour). Empty by default; the apply round
    /// reads the list to influence stage selection +
    /// ordering. The list is finite (see
    /// [`ProcessingObjective`] for the canonical
    /// vocabulary).
    #[serde(default)]
    pub processing_objectives: Vec<ProcessingObjective>,
    /// CR-08 §10.2 Guided tier: measurable quality
    /// criteria the Recipe targets. Each field is
    /// optional; absent fields don't constrain the
    /// Recipe. Defaults to empty targets via
    /// `#[serde(default)]`.
    #[serde(default)]
    pub quality_targets: QualityTargets,
    /// CR-08 §10.2 Guided tier: optional stage IDs
    /// the user has elected to include. The apply
    /// round uses this list to enable the listed
    /// stages if they were otherwise disabled by
    /// the §10.1 Beginner defaults. Empty by default.
    #[serde(default)]
    pub optional_operations: Vec<String>,
    /// CR-08 §21 follow-on / Slice E + §28 / Slice H:
    /// the Recipe's pipeline-plan content hash
    /// (`Recipe::pipeline_plan_hash()`), persisted
    /// at save time so the §6 reproducibility
    /// aggregator can compare the stored hash
    /// against the current Recipe hash without
    /// recomputing. `String::new()` for legacy
    /// Recipes (the field is `#[serde(default)]` so
    /// deserialization stays backward-compatible);
    /// `recipe_save` fills this field on every save.
    #[serde(default = "default_empty_string")]
    pub content_hash: String,
    /// CR-08 §21 / Slice G: Recipe constraints
    /// surfaced as a first-class type. Each entry
    /// carries a `kind` discriminant + the typed
    /// payload (`ParamRange` / `StageDependency` /
    /// `OrderConstraint`). The validation report
    /// from `recipe_security_validate` produces the
    /// same shape; this field persists them on the
    /// Recipe so the §21 `recipe_constraint` row
    /// has both a type + a home. Defaults to empty
    /// via `#[serde(default)]` so legacy Recipes
    /// deserialize cleanly.
    #[serde(default)]
    pub constraints: Vec<RecipeConstraint>,
    /// CR-08 §21 / Slice G: optional Recipe
    /// resource policy. The `apply_recipe` round
    /// reads this field to gate stages whose
    /// `resource_units` exceed the policy's caps.
    /// `None` for legacy Recipes that did not
    /// declare a policy (the apply round falls
    /// back to the `ResourceSnapshot::detect()`
    /// live measurement). Defaults to `None` via
    /// `#[serde(default)]` for backward compat.
    #[serde(default)]
    pub resource_policy: Option<RecipeResourcePolicy>,
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
    /// CR-08 §10.2 Guided tier: per-stage AI
    /// Enhancement Level override. When `Some`,
    /// takes precedence over the recipe-level
    /// `ai_enhancement_level` for this stage.
    /// Defaults to `None` (inherit recipe-level)
    /// via `#[serde(default)]` so legacy Recipes
    /// deserialize unchanged.
    #[serde(default)]
    pub ai_enhancement_override: Option<AiEnhancementLevel>,
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

/// CR-08 §10.2 Guided tier: high-level processing
/// objectives the user can select. Each objective
/// is a stable string tag (serde-snake-case) so
/// downstream code can pattern-match on it without
/// parsing free-form labels. The list is intentionally
/// finite: extending it requires updating
/// `ProcessingObjective::ALL` + adding the tag to
/// every code site that pattern-matches.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingObjective {
    /// Preserve star colors (avoid color shifts).
    PreserveStarColors,
    /// Maximize fine detail (resist smoothing).
    MaximizeDetail,
    /// Maximize smoothness (favour low noise).
    MaximizeSmoothness,
    /// Maximize dynamic range (preserve headroom).
    MaximizeDynamicRange,
    /// Favor reproducibility (deterministic chain).
    MaximizeReproducibility,
}

impl ProcessingObjective {
    /// All variants in display order. Used by the
    /// frontend picker.
    pub const ALL: [ProcessingObjective; 5] = [
        ProcessingObjective::PreserveStarColors,
        ProcessingObjective::MaximizeDetail,
        ProcessingObjective::MaximizeSmoothness,
        ProcessingObjective::MaximizeDynamicRange,
        ProcessingObjective::MaximizeReproducibility,
    ];

    /// Short display label.
    pub fn label(self) -> &'static str {
        match self {
            ProcessingObjective::PreserveStarColors => "Preserve star colors",
            ProcessingObjective::MaximizeDetail => "Maximize detail",
            ProcessingObjective::MaximizeSmoothness => "Maximize smoothness",
            ProcessingObjective::MaximizeDynamicRange => "Maximize dynamic range",
            ProcessingObjective::MaximizeReproducibility => "Maximize reproducibility",
        }
    }

    /// Snake-case tag (matches serde default).
    pub fn tag(self) -> &'static str {
        match self {
            ProcessingObjective::PreserveStarColors => "preserve_star_colors",
            ProcessingObjective::MaximizeDetail => "maximize_detail",
            ProcessingObjective::MaximizeSmoothness => "maximize_smoothness",
            ProcessingObjective::MaximizeDynamicRange => "maximize_dynamic_range",
            ProcessingObjective::MaximizeReproducibility => "maximize_reproducibility",
        }
    }
}

/// CR-08 §10.2 Guided tier: measurable quality
/// criteria the Recipe targets. Each field is
/// optional; absent fields don't constrain the
/// Recipe. Ranges are enforced by the §20
/// validation pipeline (which now also covers
/// these targets as a follow-on).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct QualityTargets {
    /// Target signal-to-noise ratio in decibels.
    /// Typical natural-night pipelines aim for
    /// 30-45 dB; publication-grade pipelines aim
    /// for 40+ dB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_snr_db: Option<f64>,
    /// Target sharpness score (0.0-1.0). The
    /// pipeline measures edge sharpness on a
    /// reference patch; the score is compared
    /// against this target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_sharpness: Option<f64>,
    /// Target background smoothness score
    /// (0.0-1.0). The pipeline measures low-
    /// frequency noise on a background patch;
    /// the score is compared against this
    /// target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_background_smoothness: Option<f64>,
}

/// CR-08 §10: AI enhancement level axis. The Beginner
/// editor tier surfaces this as a 4-radio picker; the
/// Guided tier can override per-stage; the Expert tier
/// can opt out entirely. The level is persisted on the
/// Recipe itself so a "Recommended" Beginner pick
/// survives a future Expert pass (no recomputation
/// needed when the user opens the editor again).
///
/// `serde(default)` keeps legacy Recipes readable: the
/// field is absent on rows that predate §10's slice,
/// deserialization falls back to `Recommended` (the
/// project's default AI posture).
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AiEnhancementLevel {
    /// No AI enhancement at all. The Recipe's
    /// `integrity.perceptual_models_used` must be false.
    Off,
    /// Minimal AI; the apply round skips any
    /// "advanced" AI stages.
    Conservative,
    /// Project default. Honors the Recipe's
    /// `required_models` as written.
    #[default]
    Recommended,
    /// All available AI stages run; the apply round
    /// surfaces warnings for stages that exceed the
    /// session's resource budget.
    Advanced,
}

impl AiEnhancementLevel {
    /// All four variants in display order. Used by the
    /// frontend picker.
    pub const ALL: [AiEnhancementLevel; 4] = [
        AiEnhancementLevel::Off,
        AiEnhancementLevel::Conservative,
        AiEnhancementLevel::Recommended,
        AiEnhancementLevel::Advanced,
    ];

    /// Short display label (e.g. "Off").
    pub fn label(self) -> &'static str {
        match self {
            AiEnhancementLevel::Off => "Off",
            AiEnhancementLevel::Conservative => "Conservative",
            AiEnhancementLevel::Recommended => "Recommended",
            AiEnhancementLevel::Advanced => "Advanced",
        }
    }

    /// One-line description for the picker UI.
    pub fn description(self) -> &'static str {
        match self {
            AiEnhancementLevel::Off => {
                "No AI enhancement. The Recipe runs as a pure deterministic pipeline."
            }
            AiEnhancementLevel::Conservative => {
                "Minimal AI. The apply round skips advanced AI stages."
            }
            AiEnhancementLevel::Recommended => {
                "Project default. Honors the Recipe's required_models as written."
            }
            AiEnhancementLevel::Advanced => {
                "All AI stages run. Apply surfaces warnings when the session budget is tight."
            }
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
            ai_enhancement_level: AiEnhancementLevel::default(),
            processing_objectives: Vec::new(),
            quality_targets: QualityTargets::default(),
            optional_operations: Vec::new(),
            // CR-08 §21 follow-on / Slice E + §28 / Slice H:
            // empty content hash for new Recipes; the
            // `recipe_save` IPC fills this field on every
            // save (the function recomputes the hash from
            // the canonical projection so a save always
            // produces a hash that matches the saved
            // content).
            content_hash: String::new(),
            // CR-08 §21 / Slice G: new fields default to
            // empty constraints + no resource policy so
            // legacy Recipes (and unit-test `Recipe::new`
            // calls) keep building without changes.
            constraints: Vec::new(),
            resource_policy: None,
            is_system: false,
        }
    }

    pub fn add_stage(&mut self, stage_id: &str, params: HashMap<String, serde_json::Value>) {
        self.stages.push(RecipeStage {
            stage_id: stage_id.into(),
            enabled: true,
            params,
            ai_enhancement_override: None,
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

    /// CR-08 §10.2 Guided tier: resolve the
    /// effective [`AiEnhancementLevel`] for a given
    /// stage. Per-stage overrides (set via the
    /// Guided-tier UI) take precedence over the
    /// recipe-level default. When the stage isn't
    /// found in `stages`, the recipe-level default
    /// is returned (the helper never panics on
    /// unknown stage IDs).
    pub fn effective_ai_enhancement_for_stage(&self, stage_id: &str) -> AiEnhancementLevel {
        self.stages
            .iter()
            .find(|s| s.stage_id == stage_id)
            .and_then(|s| s.ai_enhancement_override)
            .unwrap_or(self.ai_enhancement_level)
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationResult {
    Compatible,
    MissingModels(Vec<String>),
    IncompatibleVersion(String),
}

impl ValidationResult {
    /// CR-08 §28 / Slice H: `true` when the Recipe
    /// is applicable (no missing models, no
    /// incompatible-version flag). The §28 import
    /// gate uses this helper to decide whether to
    /// accept or reject an imported Recipe.
    pub fn is_compatible(&self) -> bool {
        matches!(self, ValidationResult::Compatible)
    }
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
            // CR-08 §10.4 apply round: consult
            // `effective_ai_enhancement_for_stage` so
            // the returned params hash carries the
            // resolved AI Enhancement Level for each
            // stage. The helper resolves per-stage
            // override > recipe-level default;
            // unknown stage IDs fall back to the
            // recipe-level default. The underscore
            // prefix marks the key as metadata (not
            // a pipeline parameter); the IPC
            // serializes it alongside the user-set
            // params so the apply round can drive
            // per-stage AI posture without a second
            // round-trip.
            let mut params = stage.params.clone();
            let resolved = recipe.effective_ai_enhancement_for_stage(&stage.stage_id);
            params.insert(
                "_ai_enhancement_level".to_string(),
                serde_json::Value::String(resolved.label().to_string()),
            );
            stage_params.push((stage.stage_id.clone(), params));
        }
    }

    Ok(stage_params)
}

/// CR-08 §14: convert a [`PipelinePlan`] into a new
/// [`Recipe`] for "Save as Recipe" UX. Pure function --
/// no IO, no side effects. The caller is responsible
/// for persisting via `RecipeStore::save()`.
///
/// Mapping rules:
/// - `name` + `target_type` come from the IPC request;
///   when `target_type` is `None`, the plan's
///   `target_type` (serialized via serde_json's
///   `rename_all = "snake_case"`) is used.
/// - `description` is "Saved from pipeline plan
///   <plan_id> (session <session_id>)".
/// - Each `PipelineStage` becomes a `RecipeStage`
///   with `enabled = pipeline.enabled`. The stage
///   `parameters_json` is deserialized into the
///   `RecipeStage.params` HashMap; missing or empty
///   JSON falls back to an empty HashMap.
/// - `version` is 1, `parent_version` is None,
///   `branch` is "main". `required_models` is empty
///   (the plan does not encode AI model
///   requirements); `quality_profile` is `Balanced`;
///   `flags` is empty.
/// - `is_system` is false (a user-saved Recipe is
///   never auto-flagged as a system Recipe; the
///   `mark_as_system` IPC flips it later if needed).
pub fn recipe_from_pipeline_plan(
    plan: &crate::domain::PipelinePlan,
    name: &str,
    target_type: Option<&str>,
) -> Recipe {
    use crate::domain::PipelineStage;
    use std::collections::HashMap;

    // Derive target_type from the plan when the caller
    // didn't supply one. `serde_json::to_value` gives us
    // the canonical snake_case string the plan was
    // serialized with ("deep_sky", "planet", ...).
    let resolved_target_type: String = match target_type {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => serde_json::to_value(plan.target_type)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string()),
    };

    let mut recipe = Recipe::new(name, &resolved_target_type);
    recipe.description = format!(
        "Saved from pipeline plan {} (session {})",
        plan.plan_id, plan.session_id
    );
    recipe.version = 1;
    recipe.parent_version = None;
    recipe.branch = "main".into();
    recipe.quality_profile = QualityProfile::Natural;
    recipe.is_system = false;
    recipe.required_models.clear();
    recipe.flags.clear();

    for PipelineStage {
        stage_id,
        parameters_json,
        enabled,
        ..
    } in &plan.stages
    {
        let params: HashMap<String, serde_json::Value> = parameters_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        recipe.add_stage(stage_id, params);
        // add_stage always pushes enabled=true; mirror the
        // plan's own enabled flag so a disabled plan stage
        // does not sneak back on in the saved Recipe.
        if let Some(last) = recipe.stages.last_mut() {
            last.enabled = *enabled;
        }
    }

    recipe
}

/// CR-08 §22 round 2 / Slice B: convert a terminal
/// processing run's stage records into a new
/// [`Recipe`] for "Save Processing as Recipe" UX.
/// Pure function — no IO, no side effects. The caller
/// is responsible for persisting via `RecipeStore::save`.
///
/// This is the **execution-history sibling** of
/// `recipe_from_pipeline_plan` (§14):
/// - §14 builds a Recipe from the *intended* plan (what
///   the engine was told to do).
/// - Slice B builds a Recipe from the *actual* stage
///   runs that executed (what the engine really did,
///   including any per-stage parameter overrides that
///   diverged from the plan).
///
/// Input: an unordered slice of [`StageRunRecord`]s
/// keyed by `run_id` + `stage_id`. Reruns add multiple
/// rows per stage (per CR-02 §12, `attempt` is 1-based).
/// We pick the **terminal attempt per stage** (highest
/// `attempt`) and ignore earlier attempts entirely.
///
/// Per-stage Recipe behaviour:
/// - `enabled` mirrors the terminal attempt's status:
///   `enabled = (terminal.status == "completed")`.
///   A `failed` stage is recorded with `enabled = false`
///   so the Recipe captures the user's exact history
///   (the user can re-enable it later via RecipeEditor).
///   This mirrors how `preview_recipe` surfaces ALL
///   stages even when disabled.
/// - `params` is parsed from the terminal attempt's
///   `params_json`. Malformed or missing JSON falls
///   back to an empty HashMap (no panic).
/// - Stages are emitted in **first-attempt-first order**
///   (i.e. the order they were originally recorded),
///   not in the order the slice happened to arrive in.
///
/// Recipe defaults match the §14 sibling:
/// - `version = 1`, `parent_version = None`,
///   `branch = "main"`.
/// - `quality_profile = Natural` (the actual runs
///   don't encode a quality profile; the user can
///   promote it in RecipeEditor).
/// - `required_models` is empty (the stage records
///   don't enumerate model requirements; the user can
///   add them in RecipeEditor).
/// - `flags` is empty.
/// - `is_system = false` (the `mark_as_system` IPC
///   flips it later if needed).
/// - `processing_objectives`, `quality_targets`,
///   `optional_operations` are all empty (the user's
///   intent at the time of the run was implicit; the
///   RecipeEditor surfaces them later).
///
/// `description` is auto-populated as
/// `"Saved from processing run {run_id} ({n} stages, {m} completed)"`
/// so the Recipe's provenance line is honest about its
/// origin without the user having to type it.
///
/// `run_id` is taken from the first record's `run_id`
/// field (all records in the slice share the same
/// `run_id` per the schema). The IPC wrapper passes
/// `run_id` explicitly so callers don't have to rely
/// on the slice being non-empty.
pub fn recipe_from_stage_runs(
    stage_runs: &[crate::domain::StageRunRecord],
    run_id: &str,
    name: &str,
    target_type: Option<&str>,
) -> Recipe {
    use std::collections::HashMap;

    // Resolve target_type: explicit override wins, else
    // "unknown". Unlike §14 we don't have a plan-level
    // target_type to fall back on — the stage records
    // don't carry one. Callers should pass an explicit
    // target_type when they know it (the typical case:
    // the ImageVersion row carries it).
    let resolved_target_type: String = match target_type {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => "unknown".to_string(),
    };

    // Pick the terminal attempt per stage_id. Reruns add
    // multiple rows; we keep the highest-`attempt` row
    // for each stage.
    use std::collections::BTreeMap;
    let mut terminal_per_stage: BTreeMap<String, &crate::domain::StageRunRecord> = BTreeMap::new();
    for sr in stage_runs {
        let entry = terminal_per_stage.entry(sr.stage_id.clone()).or_insert(sr);
        if sr.attempt > entry.attempt {
            *entry = sr;
        }
    }

    // Emit stages in first-attempt-first order. The
    // canonical ordering is captured by the earliest
    // `attempt == 1` row's position among the records.
    let mut first_seen_order: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for sr in stage_runs {
        if sr.attempt == 1 && seen.insert(sr.stage_id.clone()) {
            first_seen_order.push(sr.stage_id.clone());
        }
    }

    let total_stages = terminal_per_stage.len();
    let completed_stages = terminal_per_stage
        .values()
        .filter(|sr| sr.status == "completed")
        .count();

    let mut recipe = Recipe::new(name, &resolved_target_type);
    recipe.description = format!(
        "Saved from processing run {} ({} stages, {} completed)",
        run_id, total_stages, completed_stages
    );
    recipe.version = 1;
    recipe.parent_version = None;
    recipe.branch = "main".into();
    recipe.quality_profile = QualityProfile::Natural;
    recipe.is_system = false;
    recipe.required_models.clear();
    recipe.flags.clear();
    recipe.processing_objectives.clear();
    recipe.quality_targets = QualityTargets::default();
    recipe.optional_operations.clear();

    for stage_id in &first_seen_order {
        let Some(terminal) = terminal_per_stage.get(stage_id) else {
            continue;
        };
        let params: HashMap<String, serde_json::Value> = terminal
            .params_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        recipe.add_stage(stage_id, params);
        // add_stage always pushes enabled=true; mirror
        // the terminal attempt's status so a failed
        // stage doesn't sneak back on as enabled.
        if let Some(last) = recipe.stages.last_mut() {
            last.enabled = terminal.status == "completed";
        }
    }

    recipe
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

/// CR-08 §13: one row of a per-stage parameter diff.
/// `key` is the parameter name; `a` is the value in Recipe
/// A (or `None` if absent); `b` is the value in Recipe B
/// (or `None` if absent); `change` classifies the kind of
/// difference (Added / Removed / Changed / Unchanged).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamChange {
    /// Param exists in A but not B.
    Removed,
    /// Param exists in B but not A.
    Added,
    /// Param exists in both with different JSON values.
    Changed,
    /// Param exists in both with the same JSON value (omitted
    /// from the diff response by default; only surfaced when
    /// the caller asks for the full table).
    Unchanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParamDiffEntry {
    pub key: String,
    pub a: Option<serde_json::Value>,
    pub b: Option<serde_json::Value>,
    pub change: ParamChange,
}

/// CR-08 §13: per-stage parameter diff. One entry per
/// `stage_id` that exists in either Recipe. The `enabled`
/// flag captures whether the stage was enabled / disabled
/// in either Recipe (a side-channel that does not fit into
/// the per-param diff).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageDiffEntry {
    pub stage_id: String,
    pub enabled_a: Option<bool>,
    pub enabled_b: Option<bool>,
    pub enabled_differs: bool,
    pub params: Vec<ParamDiffEntry>,
}

/// CR-08 §13: the full mechanical parameter diff between
/// two Recipes. Splits stages into three buckets:
/// - `added`: stages only in Recipe B (params take the
///   "Added" classification against a `None` A side).
/// - `removed`: stages only in Recipe A (params take the
///   "Removed" classification against a `None` B side).
/// - `modified`: stages in both Recipes. `params` carries
///   the union of keys across both stages, each classified
///   Added / Removed / Changed / Unchanged.
///
/// `identical` is `true` iff both Recipes have the same
/// stages in the same order with the same enabled flags
/// and the same params. The flag drives the "no changes"
/// empty state in the §13 RecipeDiffPanel UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeParameterDiff {
    pub added: Vec<StageDiffEntry>,
    pub removed: Vec<StageDiffEntry>,
    pub modified: Vec<StageDiffEntry>,
    pub identical: bool,
}

/// CR-08 §13: compute the mechanical parameter diff
/// between two Recipes. Pure function; no I/O. Walks both
/// stages lists, classifying each `stage_id`:
/// - In A only → `removed` (B side = None).
/// - In B only → `added` (A side = None).
/// - In both → `modified` (per-param diff against the
///   union of keys; "enabled" flag carries side-channel).
///
/// Stages are ordered: `added` by B's order, `removed` by
/// A's order, `modified` by A's order (matches the §15
/// RecipeCard's "left = A" convention used by
/// CompareWorkspace).
///
/// `identical` flips true iff `added` + `removed` are
/// empty AND every `modified` entry has zero Changed /
/// Added / Removed params AND the enabled flags match.
pub fn recipe_parameter_diff(a: &Recipe, b: &Recipe) -> RecipeParameterDiff {
    let a_stages: std::collections::HashMap<&str, &RecipeStage> =
        a.stages.iter().map(|s| (s.stage_id.as_str(), s)).collect();
    let b_stages: std::collections::HashMap<&str, &RecipeStage> =
        b.stages.iter().map(|s| (s.stage_id.as_str(), s)).collect();

    // Walk A's stages in order. Anything not in B is `removed`.
    let mut removed: Vec<StageDiffEntry> = Vec::new();
    let mut modified: Vec<StageDiffEntry> = Vec::new();
    for stage_a in &a.stages {
        match b_stages.get(stage_a.stage_id.as_str()) {
            None => {
                removed.push(StageDiffEntry {
                    stage_id: stage_a.stage_id.clone(),
                    enabled_a: Some(stage_a.enabled),
                    enabled_b: None,
                    enabled_differs: true,
                    params: stage_a
                        .params
                        .iter()
                        .map(|(k, v)| ParamDiffEntry {
                            key: k.clone(),
                            a: Some(v.clone()),
                            b: None,
                            change: ParamChange::Removed,
                        })
                        .collect(),
                });
            }
            Some(stage_b) => {
                let enabled_differs = stage_a.enabled != stage_b.enabled;
                // Union of param keys, ordered: A's keys first
                // in A's order, then B-only keys in B's order.
                let mut seen = std::collections::HashSet::new();
                let mut params: Vec<ParamDiffEntry> = Vec::new();
                for (k, v_a) in &stage_a.params {
                    seen.insert(k.clone());
                    let entry = match stage_b.params.get(k) {
                        None => ParamDiffEntry {
                            key: k.clone(),
                            a: Some(v_a.clone()),
                            b: None,
                            change: ParamChange::Removed,
                        },
                        Some(v_b) if v_a != v_b => ParamDiffEntry {
                            key: k.clone(),
                            a: Some(v_a.clone()),
                            b: Some(v_b.clone()),
                            change: ParamChange::Changed,
                        },
                        Some(_) => ParamDiffEntry {
                            key: k.clone(),
                            a: Some(v_a.clone()),
                            b: Some(v_a.clone()),
                            change: ParamChange::Unchanged,
                        },
                    };
                    params.push(entry);
                }
                for (k, v_b) in &stage_b.params {
                    if !seen.contains(k) {
                        params.push(ParamDiffEntry {
                            key: k.clone(),
                            a: None,
                            b: Some(v_b.clone()),
                            change: ParamChange::Added,
                        });
                    }
                }
                modified.push(StageDiffEntry {
                    stage_id: stage_a.stage_id.clone(),
                    enabled_a: Some(stage_a.enabled),
                    enabled_b: Some(stage_b.enabled),
                    enabled_differs,
                    params,
                });
            }
        }
    }

    // Anything in B not in A is `added`.
    let mut added: Vec<StageDiffEntry> = Vec::new();
    for stage_b in &b.stages {
        if !a_stages.contains_key(stage_b.stage_id.as_str()) {
            added.push(StageDiffEntry {
                stage_id: stage_b.stage_id.clone(),
                enabled_a: None,
                enabled_b: Some(stage_b.enabled),
                enabled_differs: true,
                params: stage_b
                    .params
                    .iter()
                    .map(|(k, v)| ParamDiffEntry {
                        key: k.clone(),
                        a: None,
                        b: Some(v.clone()),
                        change: ParamChange::Added,
                    })
                    .collect(),
            });
        }
    }

    let identical = added.is_empty()
        && removed.is_empty()
        && modified.iter().all(|m| {
            !m.enabled_differs && m.params.iter().all(|p| p.change == ParamChange::Unchanged)
        });

    RecipeParameterDiff {
        added,
        removed,
        modified,
        identical,
    }
}
// ─── CR-08 §22 round 1: Recipe comparison + provenance ───
//
// The §22 table in docs/CR-08-AUDIT.md flags six ❌ rows;
// round 1 ships the two thin wrappers. The remaining
// four (`save_processing_as_recipe`, `preview_recipe`,
// `check_recipe_applicability`, `get_reproducibility_report`)
// need new core logic and are deferred to §22 round 2.

/// CR-08 §22 round 1: combined view of two Recipes.
///
/// Wraps the existing `recipe_ai_diff_summary` +
/// `recipe_parameter_diff` into a single response shape
/// so the UI can fetch both halves in one IPC round-trip.
/// The `identical` flag folds the two halves together:
/// it is `true` iff `parameter_diff.identical == true`
/// AND `ai_diff.hash_differs == false` AND
/// `ai_diff.ai_classification_differs == false` AND
/// `ai_diff.required_models_differ == false`.
///
/// `lineage_summary` is a small human-readable block the
/// CompareWorkspace header can render (e.g.
/// "Recipe A v3 (Conservative, Detail) vs Recipe B v5
/// (Conservative, Publication)"); format mirrors the
/// `provenance` field on `RecipeAiDiffSummary` but is
/// duplicated here so consumers don't have to reach
/// into the nested `ai` struct just to render a label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeComparison {
    pub ai: RecipeAiDiffSummary,
    pub parameter_diff: RecipeParameterDiff,
    /// True iff `parameter_diff.identical == true` AND no
    /// AI-classification / hash / required-models flag
    /// diverges between the two Recipes.
    pub identical: bool,
    /// Human-readable summary line (same format as
    /// `RecipeAiDiffSummary::provenance`).
    pub lineage_summary: String,
}

/// CR-08 §22 round 1: combine the existing §13 diff
/// primitives into a single response. Pure function;
/// no I/O. Reads only the two Recipes; the IPC wrapper
/// loads them from the `RecipeStore`.
pub fn recipe_compare_versions(a: &Recipe, b: &Recipe) -> RecipeComparison {
    let ai = recipe_ai_diff_summary(a, b);
    let parameter_diff = recipe_parameter_diff(a, b);
    let identical = parameter_diff.identical
        && !ai.hash_differs
        && !ai.ai_classification_differs
        && !ai.required_models_differ;
    RecipeComparison {
        lineage_summary: ai.provenance.clone(),
        ai,
        parameter_diff,
        identical,
    }
}

/// CR-08 §22 round 1: a Recipe's full provenance line.
///
/// Distinct from `RecipeAiDiffSummary::provenance` (which
/// describes a comparison) and from the image-version
/// provenance rendered by `ProvenancePanel.svelte` (which
/// walks the image's `recipe_id` chain). This struct
/// describes the Recipe itself: who built it, what
/// perceptual models it touches, how it classifies on the
/// AI/natural axis, and how it fits in the system /
/// project lineage.
///
/// `lineage_steps` is an ordered list of human-readable
/// steps describing how the Recipe came to be. The first
/// entry is always the Recipe's own creation event;
/// subsequent entries may chain to parent versions
/// (`"Adapted from v{N}"`) or system-recipe provenance
/// (`"System recipe"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeProvenance {
    pub profile_id: String,
    pub version: u32,
    pub parent_version: Option<u32>,
    pub schema_version: String,
    pub name: String,
    pub target_type: String,
    pub quality_profile: QualityProfile,
    pub pipeline_plan_hash: String,
    pub ai_used: bool,
    pub perceptual_models: Vec<String>,
    pub required_models: Vec<String>,
    pub is_system: bool,
    pub lineage_steps: Vec<String>,
}

/// CR-08 §22 round 1: build the provenance surface for a
/// Recipe. Pure function (plus the derived `profile_id`
/// argument, which is computed from `name + target_type`
/// via `RecipeStore::profile_id_for` and supplied by the
/// IPC wrapper so this function stays pure). Walks the
/// Recipe's metadata + the parent-version chain (when
/// present) to assemble `lineage_steps`. Does NOT walk
/// image-version consumers (that's the existing
/// `ProvenancePanel` surface); this surface is Recipe-only.
pub fn recipe_provenance(profile_id: &str, recipe: &Recipe) -> RecipeProvenance {
    // Collect perceptual models in sorted order so the
    // output is deterministic (mirrors the convention in
    // `recipe_ai_diff_summary`).
    let mut perceptual_models: Vec<String> = recipe
        .integrity
        .models
        .iter()
        .filter(|m| m.model_type == ModelType::Perceptual)
        .map(|m| m.model_name.clone())
        .collect();
    perceptual_models.sort();

    let mut required_models = recipe.required_models.clone();
    required_models.sort();

    let mut steps = Vec::new();
    if recipe.is_system {
        steps.push("System recipe".to_string());
    } else {
        steps.push(format!(
            "Created as v{} on {} target_type",
            recipe.version, recipe.target_type
        ));
    }
    if let Some(parent) = recipe.parent_version {
        steps.push(format!("Adapted from v{parent}"));
    }

    RecipeProvenance {
        profile_id: profile_id.to_string(),
        version: recipe.version,
        parent_version: recipe.parent_version,
        schema_version: recipe.schema_version.clone(),
        name: recipe.name.clone(),
        target_type: recipe.target_type.clone(),
        quality_profile: recipe.quality_profile,
        pipeline_plan_hash: recipe.pipeline_plan_hash(),
        ai_used: recipe.integrity.perceptual_models_used,
        perceptual_models,
        required_models,
        is_system: recipe.is_system,
        lineage_steps: steps,
    }
}
// ─── CR-08 §22 round 2 — Slice A: preview_recipe ───
//
// `preview_recipe` is the read-only sibling of
// `recipe_apply`. Where `apply_recipe` runs the
// compatibility check + filters to enabled stages +
// stamps `last_used_at`, `preview_recipe` returns
// the full preview surface even when the Recipe is
// not applicable so the UI can show the user WHY
// (missing models, schema mismatch, disabled
// stages) without forcing them into a fix-or-abort
// loop.
//
// The response surface is intentionally larger than
// `RecipeApplyResponse`:
// - `metadata` lets the UI render a Recipe header
//   without a separate `recipe_get` round-trip.
// - `provenance` is the round-1 surface — the
//   preview is the natural place to surface
//   Recipe-level lineage (system marker, parent
//   chain, perceptual-models list).
// - `applicability` echoes the existing
//   `ValidationResult` 3-variant enum. The full
//   §12 6-variant matrix lands in Slice C.
// - `resolved_stages` walks ALL stages (enabled +
//   disabled) so the UI can show disabled stages
//   in the preview even though `apply_recipe`
//   filters them out.
// - `warnings` is a list of human-readable
//   advisories derived from the applicability
//   result + the per-stage enabled state.

/// CR-08 §22 round 2 / Slice A: the preview metadata
/// block surfaced alongside the resolved stages.
/// Lets the UI render a Recipe header (with name,
/// description, target_type, quality_profile,
/// ai_enhancement_level, is_system) without a
/// separate `recipe_get` round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipePreviewMetadata {
    pub name: String,
    pub description: String,
    pub target_type: String,
    pub quality_profile: QualityProfile,
    pub ai_enhancement_level: AiEnhancementLevel,
    pub is_system: bool,
}

/// CR-08 §22 round 2 / Slice A: one row in the
/// `resolved_stages` list. Mirrors `apply_recipe`'s
/// `_ai_enhancement_level` stamping convention but
/// extracts the resolved level to a dedicated field
/// instead of nesting it inside `params` (the preview
/// is human-facing; a dedicated field is easier to
/// render).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedStagePreview {
    pub stage_id: String,
    pub enabled: bool,
    pub params: HashMap<String, serde_json::Value>,
    /// Resolved per-stage AI Enhancement Level label
    /// (`Off` / `Conservative` / `Recommended` /
    /// `Advanced`). Mirrors `apply_recipe`'s
    /// `_ai_enhancement_level` stamp but lifted to a
    /// dedicated field.
    pub resolved_ai_enhancement_level: String,
}

/// CR-08 §22 round 2 / Slice A: full preview response.
/// Distinct from `RecipeApplyResponse` (the existing
/// `recipe_apply` IPC's response) on three axes:
/// - No `last_used_at` side effect (preview is pure).
/// - Surfaces ALL stages, including disabled ones
///   (apply filters to enabled only).
/// - Carries metadata + provenance in the response so
///   the UI does not need a separate `recipe_get`
///   round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipePreviewResponse {
    pub profile_id: String,
    pub version: u32,
    pub branch: String,
    pub metadata: RecipePreviewMetadata,
    pub provenance: RecipeProvenance,
    /// Existing 3-variant `ValidationResult`. The
    /// full §12 6-variant matrix lands in Slice C
    /// (`check_recipe_applicability` round 2).
    pub applicability: ValidationResult,
    pub resolved_stages: Vec<ResolvedStagePreview>,
    /// Human-readable advisories. Always present
    /// (empty Vec when no advisories apply). When
    /// `applicability != Compatible`, the
    /// applicability result is also surfaced as a
    /// warning so the UI does not have to render the
    /// enum to make the cause visible.
    pub warnings: Vec<String>,
}

/// CR-08 §22 round 2 / Slice A: build the preview
/// surface for a Recipe. Pure function (plus the
/// derived `profile_id` argument, mirroring the
/// round-1 `recipe_provenance` convention). The
/// `available_models` slice is optional; when empty
/// (the common preview-against-fleet case), the
/// function still surfaces the Recipe but the
/// `applicability` may flag `MissingModels`.
///
/// `warnings` is populated from:
/// - the `ValidationResult` outcome (missing models
///   list, schema version mismatch),
/// - any disabled stages (informational),
/// - the `_ai_enhancement_level = "Off"` flag
///   (informational, helps the user spot a fully
///   disabled AI posture).
pub fn preview_recipe(
    profile_id: &str,
    recipe: &Recipe,
    available_models: &[String],
) -> RecipePreviewResponse {
    let applicability = validate_compatibility(recipe, available_models);

    let mut resolved_stages: Vec<ResolvedStagePreview> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for stage in &recipe.stages {
        let resolved_level = recipe.effective_ai_enhancement_for_stage(&stage.stage_id);
        let label = resolved_level.label().to_string();
        if !stage.enabled {
            warnings.push(format!(
                "Stage \"{}\" is disabled via §10.2 Guided tier override; apply will skip it",
                stage.stage_id
            ));
        }
        if label == "Off" {
            warnings.push(format!(
                "Stage \"{}\" resolves AI Enhancement Level to Off; AI models will not be used",
                stage.stage_id
            ));
        }
        resolved_stages.push(ResolvedStagePreview {
            stage_id: stage.stage_id.clone(),
            enabled: stage.enabled,
            params: stage.params.clone(),
            resolved_ai_enhancement_level: label,
        });
    }

    // Mirror the applicability result as a human-
    // readable advisory so the UI does not have to
    // render the enum to make the cause visible.
    match &applicability {
        ValidationResult::Compatible => {}
        ValidationResult::MissingModels(missing) => {
            warnings.push(format!(
                "Missing required models: {} (Recipe lists these as required; install them or remove from the Recipe before applying)",
                missing.join(", ")
            ));
        }
        ValidationResult::IncompatibleVersion(v) => {
            warnings.push(format!(
                "Schema version mismatch: this binary expects {v}; the Recipe was authored against a different version. Re-save the Recipe against the current schema or downgrade the binary"
            ));
        }
    }

    RecipePreviewResponse {
        profile_id: profile_id.to_string(),
        version: recipe.version,
        branch: recipe.branch.clone(),
        metadata: RecipePreviewMetadata {
            name: recipe.name.clone(),
            description: recipe.description.clone(),
            target_type: recipe.target_type.clone(),
            quality_profile: recipe.quality_profile,
            ai_enhancement_level: recipe.ai_enhancement_level,
            is_system: recipe.is_system,
        },
        provenance: recipe_provenance(profile_id, recipe),
        applicability,
        resolved_stages,
        warnings,
    }
}

// ─── CR-08 §22 round 2 / Slice C ─────────────────────────────────────────────
//
// Full §12 6-variant applicability matrix. Distinct from the
// existing 3-variant `ValidationResult` (which is the apply-time
// gate: schema-version + missing-models only). The §12 matrix
// surfaces the 4 §12 outcomes (`Compatible` / `Adaptable` /
// `PartiallyCompatible` / `Incompatible`) PLUS the 2 missing-data
// failure modes (`MissingModels` / `SchemaMismatch`), giving a
// 6-variant enum the audit doc tracks as the "6-variant matrix".
//
// Per-dimension outcomes are returned so the UI can show the user
// WHICH dimension caused the verdict (target type mismatch vs
// image dimensions vs available resources vs AI models vs schema
// version) rather than forcing them to guess from a flat enum tag.

/// CR-08 §22 round 2 / Slice C: 6-variant §12
/// applicability outcome. The first 4 variants mirror
/// the §12 spec verbatim (Compatible / Adaptable /
/// PartiallyCompatible / Incompatible). The last 2
/// (`MissingModels` / `SchemaMismatch`) are the
/// failure modes the apply-time gate already checks
/// separately: surfaced here so the UI can render a
/// single matrix verdict without a second round-trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicabilityOutcome {
    /// Recipe matches every §12 dimension exactly.
    /// No adaptation or substitution required.
    Compatible,
    /// Recipe diverges on one or more dimensions that
    /// can be auto-adapted at apply time (e.g. target
    /// type suffix, image-dimension scaling). User
    /// should be shown the adaptation in the preview.
    Adaptable,
    /// Recipe matches the overall intent but some
    /// stages must be skipped or substituted (e.g. an
    /// AI model is unavailable but the stage has a
    /// non-AI fallback). Apply will still succeed with
    /// a documented caveat.
    PartiallyCompatible,
    /// Recipe cannot safely be applied (e.g. an
    /// enabled stage requires a resource that does
    /// not exist in this build). Apply must refuse.
    Incompatible,
    /// One or more `required_models` are not in the
    /// available-models slice. Distinct from
    /// `PartiallyCompatible` because the gap is a
    /// model-install problem, not a recipe-design
    /// problem.
    MissingModels(Vec<String>),
    /// Recipe's `schema_version` differs from this
    /// binary's `SCHEMA_VERSION_CURRENT`. Distinct
    /// from `Incompatible` because the fix is a
    /// re-save or schema migration, not a recipe edit.
    SchemaMismatch(String),
}

impl ApplicabilityOutcome {
    /// One-line human-readable verdict label the UI
    /// can render directly. Stable across the enum's
    /// serde tag so the panel can switch on it.
    pub fn label(&self) -> &'static str {
        match self {
            ApplicabilityOutcome::Compatible => "Compatible",
            ApplicabilityOutcome::Adaptable => "Adaptable",
            ApplicabilityOutcome::PartiallyCompatible => "Partially compatible",
            ApplicabilityOutcome::Incompatible => "Incompatible",
            ApplicabilityOutcome::MissingModels(_) => "Missing models",
            ApplicabilityOutcome::SchemaMismatch(_) => "Schema mismatch",
        }
    }

    /// Returns `true` when the outcome allows the
    /// Recipe to be applied (Compatible / Adaptable /
    /// PartiallyCompatible). False for the two
    /// hard-blocker variants (Incompatible /
    /// MissingModels / SchemaMismatch).
    pub fn is_applicable(&self) -> bool {
        matches!(
            self,
            ApplicabilityOutcome::Compatible
                | ApplicabilityOutcome::Adaptable
                | ApplicabilityOutcome::PartiallyCompatible
        )
    }
}

/// CR-08 §22 round 2 / Slice C: per-dimension outcome.
/// Every §12 dimension the matrix evaluates gets one
/// row in the report so the UI can render a checklist
/// ("✓ Target type / ✓ Image dimensions / ✗ Available
/// models / ✗ Schema version") without re-running the
/// evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DimensionOutcome {
    /// Dimension matches the Recipe's expectation
    /// exactly. No note required.
    Match,
    /// Dimension matches within an auto-adaptable
    /// margin (e.g. target type suffix difference,
    /// image-dimension ratio within scaling range).
    /// `note` explains the adaptation the apply round
    /// will perform.
    Adaptable(String),
    /// Dimension requires a stage skip or substitution
    /// at apply time. `note` explains which stage(s)
    /// are affected.
    PartialSkip(String),
    /// Dimension cannot be satisfied. `note`
    /// describes the gap.
    Mismatch(String),
    /// Dimension was not supplied in the matrix
    /// (caller did not pass a value for this
    /// dimension). The matrix treats the missing
    /// dimension as `NotEvaluated` rather than
    /// failing the verdict: the verdict still
    /// reflects the dimensions the caller supplied.
    NotEvaluated,
}

/// CR-08 §22 round 2 / Slice C: per-dimension verdict
/// tuple (id + outcome) carried in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicabilityDimension {
    /// Stable dimension id the UI can switch on
    /// (`"target_type"` / `"image_dimensions"` /
    /// `"available_models"` / `"schema_version"` /
    /// `"dataset_quality"`). New dimensions are
    /// additive: old UIs render the new id as
    /// "Unknown dimension" and the verdict still
    /// works.
    pub dimension: String,
    pub outcome: DimensionOutcome,
}

/// CR-08 §22 round 2 / Slice C: session-side criteria
/// the matrix evaluates the Recipe against. Every
/// field is optional so callers can supply only the
/// dimensions they know. Unknown / unset dimensions
/// surface as `DimensionOutcome::NotEvaluated` in the
/// report (the verdict does not fail on missing
/// dimensions: it just reports a less-complete
/// picture).
///
/// The matrix is intentionally pure: it does not call
/// out to the AI model registry, filesystem, or
/// hardware probe. Callers assemble the matrix
/// client-side and pass it in.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ApplicabilityMatrix {
    /// The session's target type (e.g. `"deep_sky"`,
    /// `"planetary"`). When `None`, the target-type
    /// dimension is `NotEvaluated`.
    pub target_type: Option<String>,
    /// Session image dimensions in pixels (width,
    /// height). When `None`, the image-dimensions
    /// dimension is `NotEvaluated`.
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    /// Sorted list of model identifiers the session
    /// can reach (typically populated from the AI
    /// model registry). When `None`, the available-
    /// models dimension is `NotEvaluated`.
    pub available_models: Option<Vec<String>>,
    /// Numeric dataset quality proxy (higher is
    /// better; the engine reports this from the
    /// SNR / FWHM / noise triplet). When `None`,
    /// the dataset-quality dimension is
    /// `NotEvaluated`.
    pub dataset_quality: Option<f32>,
}

/// CR-08 §22 round 2 / Slice C: full applicability
/// report returned to the UI. The verdict is the
/// single 6-variant enum the audit doc tracks; the
/// per-dimension list lets the UI render a
/// per-dimension checklist; the warnings list
/// surfaces human-readable advisories for each
/// non-`Match` outcome so the UI does not have to
/// render the enum tags to make the cause visible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicabilityReport {
    /// Top-level 6-variant verdict.
    pub verdict: ApplicabilityOutcome,
    /// Per-dimension outcome list (one row per
    /// evaluated dimension; unset dimensions are
    /// absent).
    pub dimensions: Vec<ApplicabilityDimension>,
    /// Human-readable advisories. Always present
    /// (empty Vec when every dimension is `Match`).
    pub warnings: Vec<String>,
}

/// CR-08 §22 round 2 / Slice C: evaluate the full
/// §12 applicability matrix. Pure function. Walks
/// each dimension, classifies the outcome, and folds
/// the per-dimension verdicts into the 6-variant
/// top-level outcome.
///
/// Verdict folding rules (in priority order):
/// 1. Schema version mismatch (any non-`Match` for
///    `schema_version`) → `SchemaMismatch`.
/// 2. Required models not in the available-models
///    slice (any non-`Match` for `available_models`)
///    → `MissingModels(list)`.
/// 3. Any `DimensionOutcome::Mismatch` →
///    `Incompatible`.
/// 4. Any `DimensionOutcome::PartialSkip` →
///    `PartiallyCompatible`.
/// 5. Any `DimensionOutcome::Adaptable` →
///    `Adaptable`.
/// 6. All dimensions `Match` or `NotEvaluated` →
///    `Compatible`.
///
/// `warnings` is populated from every non-`Match`
/// dimension's note so the UI does not have to render
/// the enum tags to make the cause visible. Empty
/// when every dimension is `Match` (and the verdict
/// is therefore `Compatible`).
pub fn check_recipe_applicability(
    recipe: &Recipe,
    matrix: &ApplicabilityMatrix,
) -> ApplicabilityReport {
    let mut dimensions: Vec<ApplicabilityDimension> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Dimension: schema_version. This dimension is
    // evaluated from the Recipe alone (no matrix
    // input) because the binary's
    // `SCHEMA_VERSION_CURRENT` is a constant. The
    // matrix does not need a separate field for it.
    {
        let outcome = if recipe.schema_version == SCHEMA_VERSION_CURRENT {
            DimensionOutcome::Match
        } else {
            DimensionOutcome::Mismatch(format!(
                "Recipe schema_version '{}' does not match binary current '{}'; re-save the Recipe or run schema migration",
                recipe.schema_version, SCHEMA_VERSION_CURRENT
            ))
        };
        if let DimensionOutcome::Mismatch(note) = &outcome {
            warnings.push(note.clone());
        }
        dimensions.push(ApplicabilityDimension {
            dimension: "schema_version".to_string(),
            outcome,
        });
    }

    // Dimension: available_models. When the caller
    // dimension is `NotEvaluated` and the verdict
    // does not block: the UI is presumed to be
    // rendering the matrix against an unknown fleet
    // (the preview path).
    let mut missing: Vec<String> = Vec::new();
    {
        let outcome = match &matrix.available_models {
            None => DimensionOutcome::NotEvaluated,
            Some(available) => {
                let missing_local: Vec<String> = recipe
                    .required_models
                    .iter()
                    .filter(|m| !available.contains(m))
                    .cloned()
                    .collect();
                if missing_local.is_empty() {
                    DimensionOutcome::Match
                } else {
                    let note = format!(
                        "Recipe requires {} model(s) not in available fleet: {}",
                        missing_local.len(),
                        missing_local.join(", ")
                    );
                    warnings.push(note.clone());
                    missing = missing_local;
                    DimensionOutcome::Mismatch(note)
                }
            }
        };
        dimensions.push(ApplicabilityDimension {
            dimension: "available_models".to_string(),
            outcome,
        });
    }

    // Dimension: target_type. When the caller
    // supplies `target_type`, compare against the
    // Recipe's `target_type`. Empty Recipe
    // `target_type` (the default for Recipes that
    // did not bother to set one) is treated as
    // `Adaptable` with a "Re-save to set target type"
    // note; an exact match is `Match`; any other
    // difference is `Mismatch` (a target-type
    // mismatch is a hard blocker: the Recipe was
    // authored for a different imaging target).
    {
        let outcome = match &matrix.target_type {
            None => DimensionOutcome::NotEvaluated,
            Some(session_target) => {
                if recipe.target_type.is_empty() || recipe.target_type == "unknown" {
                    DimensionOutcome::Adaptable(format!(
                        "Recipe target_type is '{}' (unset / unknown); the session is '{}'. Re-save the Recipe against '{}' to lock the target type",
                        recipe.target_type, session_target, session_target
                    ))
                } else if recipe.target_type == *session_target {
                    DimensionOutcome::Match
                } else {
                    DimensionOutcome::Mismatch(format!(
                        "Recipe target_type '{}' does not match session target_type '{}'",
                        recipe.target_type, session_target
                    ))
                }
            }
        };
        if let DimensionOutcome::Adaptable(note) | DimensionOutcome::Mismatch(note) = &outcome {
            warnings.push(note.clone());
        }
        dimensions.push(ApplicabilityDimension {
            dimension: "target_type".to_string(),
            outcome,
        });
    }

    // Dimension: image_dimensions. The Recipe does
    // not currently carry an explicit pixel-dimension
    // preference (per the §4 audit, the
    // applicability-block fields are not implemented
    // yet). When the caller supplies
    // `image_width` / `image_height`, the dimension
    // surfaces as `Adaptable` (the engine can scale
    // per-stage params within reason) and the
    // verdict reflects that with an Adaptable fold.
    // When the caller does NOT supply the
    // dimensions, the dimension is `NotEvaluated`.
    {
        let outcome = match (matrix.image_width, matrix.image_height) {
            (None, None) => DimensionOutcome::NotEvaluated,
            (Some(w), Some(h)) => {
                // The Recipe does not yet encode a
                // dimension preference; surface as
                // Adaptable so the verdict reflects
                // "the engine can scale" rather than
                // hard-failing. A future R1 slice
                // (applicability-block fields) will
                // narrow this to a true Match /
                // Mismatch.
                DimensionOutcome::Adaptable(format!(
                    "Session image is {w}x{h}; the Recipe does not declare a dimension preference, so the engine will scale per-stage params within reason"
                ))
            }
            _ => DimensionOutcome::NotEvaluated,
        };
        if let DimensionOutcome::Adaptable(note) = &outcome {
            warnings.push(note.clone());
        }
        dimensions.push(ApplicabilityDimension {
            dimension: "image_dimensions".to_string(),
            outcome,
        });
    }

    // Dimension: dataset_quality. Same shape as
    // image_dimensions: the Recipe does not yet
    // encode a quality preference, so any supplied
    // value surfaces as `Adaptable` (the engine can
    // adapt noise / sharpening within reason).
    {
        let outcome = match matrix.dataset_quality {
            None => DimensionOutcome::NotEvaluated,
            Some(q) => DimensionOutcome::Adaptable(format!(
                "Session dataset quality proxy is {q:.3}; the Recipe does not declare a quality preference, so adaptive engine will tune noise / sharpening accordingly"
            )),
        };
        if let DimensionOutcome::Adaptable(note) = &outcome {
            warnings.push(note.clone());
        }
        dimensions.push(ApplicabilityDimension {
            dimension: "dataset_quality".to_string(),
            outcome,
        });
    }

    // Fold per-dimension outcomes into the 6-variant
    // top-level verdict. Priority order matters:
    // schema mismatch and missing-models are
    // hard-blockers and take precedence over the
    // softer target-type / dimension / quality
    // dimensions.
    let verdict = if let Some(ApplicabilityDimension {
        outcome: DimensionOutcome::Mismatch(note),
        ..
    }) = dimensions.iter().find(|d| d.dimension == "schema_version")
    {
        ApplicabilityOutcome::SchemaMismatch(note.clone())
    } else if !missing.is_empty() {
        ApplicabilityOutcome::MissingModels(missing)
    } else if dimensions
        .iter()
        .any(|d| matches!(d.outcome, DimensionOutcome::Mismatch(_)))
    {
        ApplicabilityOutcome::Incompatible
    } else if dimensions
        .iter()
        .any(|d| matches!(d.outcome, DimensionOutcome::PartialSkip(_)))
    {
        ApplicabilityOutcome::PartiallyCompatible
    } else if dimensions
        .iter()
        .any(|d| matches!(d.outcome, DimensionOutcome::Adaptable(_)))
    {
        ApplicabilityOutcome::Adaptable
    } else {
        ApplicabilityOutcome::Compatible
    };

    ApplicabilityReport {
        verdict,
        dimensions,
        warnings,
    }
}

// ─── CR-08 §21 / Slice G ──────────────────────────────────────────────────
//
// Three new types land here to close the last three
// §21 ❌ rows:
//
// 1. `RecipeConstraint` + `RecipeConstraintKind`
//    (closes the `recipe_constraint` row).
// 2. `RecipeResourcePolicy` (closes the
//    `recipe_resource_policy` row).
// 3. `ProvenanceRecord` + `provenance_record()` pure
//    constructor (closes the `provenance_record` row
//    on the ImageVersion side; the existing
//    `RecipeProvenance` is the Recipe-side surface).
//
// See `docs/CR-08-ADR-002-implementation-map-route.md`
// for the §25 routing table that formalizes the
// single-crate `astroforge-core/` home.

/// CR-08 §21 / Slice G: one typed Recipe constraint.
/// The `kind` discriminant carries the constraint
/// variant; the typed payload rides alongside so
/// consumers can deserialize without re-parsing
/// strings. Three variants:
/// - `ParamRange`: the Recipe pins a stage parameter
///   to a numeric range (matches `validation::ParamRange`
///   so the §20 stage-spec table can re-use the same
///   shape).
/// - `StageDependency`: the Recipe requires a stage
///   to run before the annotated stage (mirrors the
///   `StageSpec::required_stages` list).
/// - `OrderConstraint`: the Recipe enforces a stage
///   ordering (e.g. "stretch before denoise") that
///   the apply round must preserve.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecipeConstraintKind {
    ParamRange {
        stage_id: String,
        param: String,
        min: f64,
        max: f64,
    },
    StageDependency {
        stage_id: String,
        depends_on: String,
    },
    OrderConstraint {
        before: String,
        after: String,
    },
}

/// CR-08 §21 / Slice G: typed Recipe constraint.
/// The `description` field carries a human-readable
/// note so the §20 UI panel + the validation report
/// can render the constraint verbatim; the
/// `source` discriminant records whether the
/// constraint was user-declared (`"user"`) or
/// derived by the validation report (`"validator"`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeConstraint {
    pub constraint: RecipeConstraintKind,
    pub description: String,
    pub source: String,
}

impl RecipeConstraint {
    /// Build a `ParamRange` constraint (the most
    /// common shape: a stage parameter bounded to
    /// a numeric range).
    pub fn param_range(stage_id: &str, param: &str, min: f64, max: f64) -> Self {
        Self {
            constraint: RecipeConstraintKind::ParamRange {
                stage_id: stage_id.to_string(),
                param: param.to_string(),
                min,
                max,
            },
            description: format!("{stage_id}.{param} in [{min}, {max}]"),
            source: "user".to_string(),
        }
    }

    /// Build a `StageDependency` constraint.
    pub fn stage_dependency(stage_id: &str, depends_on: &str) -> Self {
        Self {
            constraint: RecipeConstraintKind::StageDependency {
                stage_id: stage_id.to_string(),
                depends_on: depends_on.to_string(),
            },
            description: format!("{stage_id} requires {depends_on}"),
            source: "user".to_string(),
        }
    }

    /// Build an `OrderConstraint` (e.g.
    /// `order("stretch", "denoise")` means stretch
    /// must run before denoise).
    pub fn order_constraint(before: &str, after: &str) -> Self {
        Self {
            constraint: RecipeConstraintKind::OrderConstraint {
                before: before.to_string(),
                after: after.to_string(),
            },
            description: format!("{before} before {after}"),
            source: "user".to_string(),
        }
    }
}

/// CR-08 §21 / Slice G: Recipe resource policy.
/// The apply round reads the policy's caps to gate
/// stages whose `resource_units` exceed any cap.
/// `None` for any cap means "no cap; let the live
/// `ResourceSnapshot::detect()` measurement
/// decide". All numeric fields are `f64` so the
/// policy can express both absolute caps (MB, ms)
/// and relative caps (fraction of `MAX_RESOURCE_UNITS`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeResourcePolicy {
    pub max_cpu_units: Option<f64>,
    pub max_memory_mb: Option<f64>,
    pub max_disk_mb: Option<f64>,
    pub max_wall_clock_secs: Option<f64>,
    pub notes: String,
}

impl RecipeResourcePolicy {
    /// Build the default `RecipeResourcePolicy`
    /// (no caps + an empty notes string). Used by
    /// the `Recipe::default()` and by callers that
    /// want an "allow everything" policy.
    pub fn unbounded() -> Self {
        Self {
            max_cpu_units: None,
            max_memory_mb: None,
            max_disk_mb: None,
            max_wall_clock_secs: None,
            notes: String::new(),
        }
    }

    /// `true` when every cap is `None` (the policy
    /// places no constraints). Used by the apply
    /// round to short-circuit the cap-check.
    pub fn is_unbounded(&self) -> bool {
        self.max_cpu_units.is_none()
            && self.max_memory_mb.is_none()
            && self.max_disk_mb.is_none()
            && self.max_wall_clock_secs.is_none()
    }
}

/// CR-08 §21 / Slice G: ImageVersion-side provenance
/// record.
///
/// Distinct from `RecipeProvenance` (which is the
/// Recipe-only surface: identity + lineage_steps +
/// perceptual_models + ...). This struct records
/// the actual record of how this ImageVersion came
/// from this Recipe via these stage runs.
///
/// The constructor `provenance_record()` is a pure
/// function that walks the chain `ImageVersion`
/// then `Artifact` then `PipelineRun` then
/// `StageRunRecord` then `AiOperation` and assembles
/// the record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceRecord {
    /// The ImageVersion the record describes.
    pub version_id: String,
    /// The Recipe identity (profile_id + version)
    /// when the ImageVersion was produced by a
    /// Recipe; `None` for AI-applied versions whose
    /// `recipe_id` is null.
    pub recipe_id: Option<String>,
    pub recipe_version: Option<u32>,
    pub recipe_hash: Option<String>,
    /// Counts the §8 processing provenance chain:
    /// how many stage runs produced this ImageVersion
    /// (1 per enabled stage, regardless of attempt
    /// count).
    pub stage_runs: usize,
    /// Counts the §9 AI provenance chain: how many
    /// `AiOperation` rows this ImageVersion
    /// references. `0` for Recipes that did not
    /// invoke any AI enhancement.
    pub ai_operations: usize,
    /// Human-readable summary lines (one per stage
    /// run + one per ai operation, in execution
    /// order). The ProvenancePanel renders this
    /// list verbatim; consumers can also parse it
    /// line-by-line if they want the canonical text.
    pub summary_lines: Vec<String>,
    /// `true` when the record is reproducible to
    /// the bit (mirrors the §6 `ReproducibilityVerdict::Exact`
    /// verdict; `false` for Material / Indeterminate).
    /// Computed lazily from the same chain.
    pub is_exact: bool,
}

/// CR-08 §21 / Slice G: build the ImageVersion-side
/// `ProvenanceRecord`. Pure function: takes the
/// already-loaded `ImageVersion` (currently surfaced
/// only via the `version_id` argument; the full
/// struct is wired through so future slice can fold
/// its `recipe_id` + `recipe_version` + `recipe_hash`
/// fields into the record), optional Recipe
/// identity (the `Recipe` itself is optional because
/// the RecipeStore may not be able to resolve the
/// `recipe_id` at record-build time), the ordered
/// list of `StageRunRecord`s (from the project
/// store), and the ordered list of `AiOperation`s
/// (also from the project store). Returns the
/// assembled `ProvenanceRecord`.
///
/// The `is_exact` field is computed by the same
/// `summarize_reproducibility` aggregator that the
/// §22 round 2 `recipe_get_reproducibility_report`
/// IPC uses; this slice routes through the
/// aggregator so the §21 + §22 surfaces agree.
pub fn provenance_record(
    version_id: &str,
    _image_version: &crate::domain::ImageVersion,
    recipe: Option<&Recipe>,
    stage_runs: &[crate::domain::StageRunRecord],
    ai_ops: &[crate::domain::AiOperation],
    reproducibility_verdict: ReproducibilityVerdictLite,
) -> ProvenanceRecord {
    // Slice G narrows the §22 round 2
    // `ReproducibilityVerdict` enum down to the
    // 3-valued "exact?" flag the §21 record needs.
    // We avoid pulling the full enum + aggregator
    // into a method on `ProvenanceRecord` to keep
    // the §21 type decoupled from §22's
    // reproducibility module.
    let is_exact = matches!(reproducibility_verdict, ReproducibilityVerdictLite::Exact);

    let mut summary_lines: Vec<String> = Vec::with_capacity(stage_runs.len() + ai_ops.len());
    for sr in stage_runs {
        summary_lines.push(format!(
            "stage_run {stage_id} attempt {attempt} -> {outcome}",
            stage_id = sr.stage_id,
            attempt = sr.attempt,
            outcome = sr.status,
        ));
    }
    for op in ai_ops {
        summary_lines.push(format!(
            "ai_op {op_id} model {model} backend {backend}",
            op_id = op.operation_id,
            model = op.model_id,
            backend = op.backend.as_deref().unwrap_or("none"),
        ));
    }

    ProvenanceRecord {
        version_id: version_id.to_string(),
        recipe_id: recipe.map(|r| RecipeStore::profile_id_for(&r.name, &r.target_type)),
        recipe_version: recipe.map(|r| r.version),
        recipe_hash: recipe.map(|r| r.pipeline_plan_hash()),
        stage_runs: stage_runs.len(),
        ai_operations: ai_ops.len(),
        summary_lines,
        is_exact,
    }
}

/// CR-08 §21 / Slice G: minimal verdict shape the
/// `provenance_record()` constructor needs. The §22
/// round 2 `ReproducibilityVerdict` enum is richer
/// (3 variants); this lite enum is what the §21
/// record exposes publicly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReproducibilityVerdictLite {
    Exact,
    NotExact,
}

// `RecipeStore::profile_id_for` is in
// `recipe_store.rs`; re-export under a local alias
// so the constructor compiles without an extra
// `use` at the call site.
use crate::recipe_store::RecipeStore;
