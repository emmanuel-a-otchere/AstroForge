//! CR-10 P1.6.4.1 / P1.6.4.2 — Graph ↔ `.astroforge-recipe` JSON
//! round-trip (spec §7.7.7, format §11.1, sanitize §11.2).
//!
//! **Graph → Recipe:** serialize a `PipelinePlan` into the shared
//! recipe format — `pipeline[]` entries of `{ "stage", "params" }`,
//! with paths, GPS/timestamps, and machine-specific keys scrubbed
//! per §11.2 so the artifact is portable.
//!
//! **Recipe → Graph:** parse recipe JSON, resolve every `stage`
//! through the catalog (canonical types pass through; user-facing
//! types map via `user_facing_to_canonical`; anything else fails
//! with the offending entry's index), and materialize a
//! `PipelinePlan`. The result is NOT persisted — the caller decides
//! whether to replace the active graph (per §7.7.7 the UI prompts).
//!
//! Recipes and graphs are not isomorphic (a recipe can describe
//! intent without binding to a fixed sequence); the round-trip
//! preserves intent where it can and surfaces a diagnostic where it
//! cannot.

use crate::domain::{ObjectType, PipelinePlan, PipelinePlanStatus, PipelineStage};
use crate::node_catalog::user_facing_to_canonical;
use crate::pipeline_plan::stage::StageType;
use sha2::{Digest, Sha256};

/// Errors from the recipe round-trip. `UnknownStage` carries the
/// offending entry's index (per P1.6.4.2's diagnostic contract) so
/// the UI can point at the exact pipeline row.
#[derive(Debug)]
pub enum RecipeIoError {
    /// A `pipeline[]` entry references a stage the catalog does not
    /// know. Carries the 0-based index and the unknown name.
    UnknownStage { index: usize, stage: String },
    /// The recipe JSON is structurally invalid (not an object, no
    /// pipeline array, bad entry shape).
    Malformed(String),
    /// Serialization / deserialization of stage parameters failed.
    Serde(String),
    /// No plan exists for the project — there is no default graph to
    /// reset to (P1.6.4.3).
    NoDefaultGraph(String),
    /// A store operation failed while materializing the reset
    /// (P1.6.4.3).
    Store(crate::pipeline_plans_store::PipelinePlanStoreError),
}

impl std::fmt::Display for RecipeIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownStage { index, stage } => {
                write!(f, "unknown stage '{}' at pipeline entry {}", stage, index)
            }
            Self::Malformed(m) => write!(f, "malformed recipe: {m}"),
            Self::Serde(e) => write!(f, "recipe parameter error: {e}"),
            Self::NoDefaultGraph(id) => {
                write!(f, "no default graph for project '{id}'")
            }
            Self::Store(e) => write!(f, "pipeline plan store error: {e}"),
        }
    }
}

impl From<crate::pipeline_plans_store::PipelinePlanStoreError> for RecipeIoError {
    fn from(e: crate::pipeline_plans_store::PipelinePlanStoreError) -> Self {
        Self::Store(e)
    }
}

impl std::error::Error for RecipeIoError {}

/// The §11.1 recipe shape this module produces and consumes. Only
/// the fields the round-trip owns are materialized; unknown fields
/// in an incoming file are preserved verbatim in the JSON but not
/// interpreted.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecipeFile {
    pub recipe_version: String,
    pub app_version: String,
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    pub target_type: String,
    #[serde(default)]
    pub equipment_hints: serde_json::Value,
    pub pipeline: Vec<RecipeStageEntry>,
    #[serde(default)]
    pub model_versions: std::collections::BTreeMap<String, String>,
    pub integrity: RecipeIntegrity,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecipeStageEntry {
    pub stage: String,
    #[serde(default = "empty_object")]
    pub params: serde_json::Value,
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecipeIntegrity {
    pub perceptual_models_used: bool,
}

/// §11.2 — keys stripped from params before sharing (paths,
/// GPS/timestamps, machine-specific info). Match is case-insensitive
/// substring on the key name.
const SANITIZE_KEY_MARKERS: &[&str] = &[
    "path",
    "dir",
    "file",
    "folder",
    "gps",
    "latitude",
    "longitude",
    "timestamp",
    "datetime",
    "date_obs",
    "gpu",
    "hostname",
    "machine",
    "username",
    "user_name",
    "home",
];

fn sanitize_params(params: &serde_json::Value) -> serde_json::Value {
    match params {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                let lower = k.to_ascii_lowercase();
                if SANITIZE_KEY_MARKERS
                    .iter()
                    .any(|marker| lower.contains(marker))
                {
                    continue;
                }
                out.insert(k.clone(), sanitize_params(v));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(sanitize_params).collect())
        }
        other => other.clone(),
    }
}

/// Serialize a plan to `.astroforge-recipe` JSON (P1.6.4.1). Stages
/// are ordered by `sequence`; each entry's params come from the
/// stage's `parameters_json` (parsed, sanitized per §11.2, and
/// written as an object — a stage with no parameters emits `{}`).
///
/// `perceptual_models_used` is true when any stage's canonical type
/// carries an AI boundary label, so gallery consumers can gate
/// integrity checks without re-deriving the taxonomy.
pub fn plan_to_recipe_json(
    plan: &PipelinePlan,
    app_version: &str,
) -> Result<String, RecipeIoError> {
    let mut stages: Vec<&PipelineStage> = plan.stages.iter().collect();
    stages.sort_by_key(|s| s.sequence);

    let mut pipeline = Vec::with_capacity(stages.len());
    let mut perceptual = false;
    for stage in stages {
        let uses_ai =
            crate::ai_boundary::AiBoundaryLabel::for_stage_type(&stage.stage_type).uses_ai;
        perceptual |= uses_ai;
        let params: serde_json::Value = match stage.parameters_json.as_deref() {
            Some(json) if !json.trim().is_empty() => {
                serde_json::from_str(json).map_err(|e| RecipeIoError::Serde(e.to_string()))?
            }
            _ => serde_json::json!({}),
        };
        pipeline.push(RecipeStageEntry {
            stage: stage.stage_type.clone(),
            params: sanitize_params(&params),
        });
    }

    let recipe = RecipeFile {
        recipe_version: "1.0".into(),
        app_version: app_version.to_string(),
        // The plan has no user-visible name; the plan id is the only
        // identity and carries no machine-specific info (§11.2).
        name: format!("Graph {}", plan.plan_id),
        author: None,
        target_type: object_type_name(plan.target_type).to_string(),
        equipment_hints: serde_json::json!({}),
        pipeline,
        model_versions: std::collections::BTreeMap::new(),
        integrity: RecipeIntegrity {
            perceptual_models_used: perceptual,
        },
    };
    serde_json::to_string_pretty(&recipe).map_err(|e| RecipeIoError::Serde(e.to_string()))
}

/// Parse recipe JSON and materialize a `PipelinePlan` (P1.6.4.2).
/// Every `stage` resolves through the catalog: canonical types pass
/// through, user-facing types map via `user_facing_to_canonical`,
/// and unknown names fail with `UnknownStage { index }`. The plan is
/// returned unpersisted with `status: Ready` and a fresh
/// deterministic id derived from project + session + the recipe
/// content hash — the same recipe materialized twice yields the same
/// plan id, so a re-import cannot duplicate graphs silently.
///
/// Per §7.7.7 the caller prompts before replacing the active graph;
/// this function never touches the store.
pub fn recipe_to_plan(
    recipe_json: &str,
    project_id: &str,
    session_id: &str,
) -> Result<PipelinePlan, RecipeIoError> {
    let recipe: RecipeFile =
        serde_json::from_str(recipe_json).map_err(|e| RecipeIoError::Malformed(e.to_string()))?;
    if !recipe.recipe_version.starts_with('1') {
        return Err(RecipeIoError::Malformed(format!(
            "unsupported recipe_version '{}'",
            recipe.recipe_version
        )));
    }

    let mut stages = Vec::with_capacity(recipe.pipeline.len());
    for (index, entry) in recipe.pipeline.iter().enumerate() {
        let canonical =
            resolve_stage_name(&entry.stage).ok_or_else(|| RecipeIoError::UnknownStage {
                index,
                stage: entry.stage.clone(),
            })?;
        let stage_type: StageType = serde_json::from_value(serde_json::json!(canonical))
            .map_err(|e| RecipeIoError::Serde(e.to_string()))?;
        let params = sanitize_params(&entry.params);
        stages.push(PipelineStage {
            stage_id: format!(
                "stage_{}",
                short_hash(&format!("{project_id}|{session_id}|{canonical}|{index}"))
            ),
            plan_id: String::new(), // filled below with the plan id
            stage_type: canonical.to_string(),
            sequence: index as u32,
            label: stage_type.label().to_string(),
            required: stage_type.required_by_default(),
            enabled: true,
            parameters_json: Some(params.to_string()),
            produces_image_version: stage_type.produces_image_version_by_default(),
            undo_supported: stage_type.undo_supported(),
        });
    }

    let content_hash = short_hash(recipe_json);
    let plan_id = format!(
        "plan_{}",
        short_hash(&format!("recipe|{project_id}|{session_id}|{content_hash}"))
    );
    for stage in &mut stages {
        stage.plan_id = plan_id.clone();
    }

    Ok(PipelinePlan {
        plan_id,
        project_id: project_id.to_string(),
        session_id: session_id.to_string(),
        recipe_id: None,
        mode: "auto".into(),
        target_type: object_type_from_name(&recipe.target_type),
        status: PipelinePlanStatus::Ready,
        created_at: String::new(),
        schema_version: 1,
        stages,
    })
}

/// Canonical types pass through; user-facing types map via the
/// §7.7.9 table; anything else is `None` (the caller raises
/// `UnknownStage` with the index).
fn resolve_stage_name(stage: &str) -> Option<&'static str> {
    let lower = stage.to_ascii_lowercase();
    if crate::node_catalog::CANONICAL_STAGE_TYPES.contains(&lower.as_str()) {
        // Leak-free: CANONICAL_STAGE_TYPES entries are 'static.
        return Some(
            crate::node_catalog::CANONICAL_STAGE_TYPES
                .iter()
                .find(|s| **s == lower.as_str())
                .expect("checked contains"),
        );
    }
    if crate::node_catalog::USER_FACING_STAGE_TYPES.contains(&lower.as_str()) {
        return Some(user_facing_to_canonical(&lower));
    }
    None
}

fn object_type_name(t: ObjectType) -> &'static str {
    match t {
        ObjectType::DeepSky => "deep_sky",
        ObjectType::Planet => "planetary",
        ObjectType::Lunar => "lunar",
        ObjectType::Solar => "solar",
        ObjectType::Unknown => "unknown",
    }
}

fn object_type_from_name(name: &str) -> ObjectType {
    match name {
        "deep_sky" => ObjectType::DeepSky,
        "planetary" => ObjectType::Planet,
        "lunar" => ObjectType::Lunar,
        "solar" => ObjectType::Solar,
        _ => ObjectType::Unknown,
    }
}

/// First-16-chars-of-SHA256 — the same id convention the mutation
/// primitives use (`node_graph::mutation::short_hash` is private to
/// its module; this copy keeps recipe_io self-contained the same way
/// plan.rs does).
fn short_hash(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let hex = format!("{:x}", digest);
    hex.chars().take(16).collect()
}

/// P1.6.4.3 — materialize the project's DEFAULT graph for a session
/// ("Reset to project default"). The default graph is the project's
/// latest plan; the round-trip machinery re-materializes it bound to
/// `session_id` (deterministic id from project + session + content),
/// persists it, and clears any session override row so the session
/// follows the project default again. Re-running on an unchanged
/// default rewrites the same rows — idempotent by construction.
pub fn reset_session_to_project_default(
    store: &crate::pipeline_plans_store::PipelinePlanStore,
    project_id: &str,
    session_id: &str,
) -> Result<PipelinePlan, RecipeIoError> {
    let summaries = store.list_plans_for_project(project_id)?;
    let latest = summaries
        .first()
        .ok_or_else(|| RecipeIoError::NoDefaultGraph(project_id.to_string()))?;
    let default = store.load_plan(&latest.plan_id)?;
    let recipe = plan_to_recipe_json(&default, "0.1.0")?;
    let plan = recipe_to_plan(&recipe, project_id, session_id)?;
    store.insert_plan(&plan)?;
    store.delete_graph_override(project_id, session_id)?;
    Ok(plan)
}
