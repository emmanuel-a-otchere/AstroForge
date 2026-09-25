//! CR-08 §6 + §22 round 2 / Slice D:
//! Reproducibility aggregation.
//!
//! Walks the [`ImageVersion`] -> [`PipelineRun`] -> [`Recipe`] ->
//! [`AiOperation`] chain and produces a
//! [`ReproducibilityRecord`] that distinguishes exact
//! reproducibility from material reproducibility per
//! the §6 spec.
//!
//! The CR-08 §6 spec calls out two distinct
//! reproducibility grades:
//!
//! - **Exact reproducibility**: possible when the
//!   runner reproduces every condition listed in §6
//!   verbatim: same source assets, application
//!   version, engine version, Recipe, models,
//!   parameters, backend, precision, random seed,
//!   and execution configuration.
//! - **Material reproducibility**: the output may
//!   differ slightly because of hardware, FP
//!   implementation, GPU backend, library, or
//!   nondeterministic acceleration differences.
//!   AstroForge should NOT falsely claim exact
//!   reproducibility; it should record the
//!   conditions and explain deviations.
//!
//! The aggregator returns one of three verdicts:
//!
//! - `Exact`: every §6 condition met (the user
//!   can in principle reproduce the exact pixels).
//! - `Material`: at least one known deviation
//!   (hardware, backend, library, etc.); the user
//!   can reproduce *material* results but pixel
//!   parity is not guaranteed.
//! - `Indeterminate`: insufficient data to
//!   decide (some condition field is unknown).
//!
//! Per-dimension `ReproducibilityCondition` rows
//! carry the verdict per §6 dimension (so the UI
//! can render a checklist without re-running the
//! aggregator) + a list of human-readable
//! `deviations` for any non-`Exact` row.
//!
//! The aggregator is intentionally pure: it does
//! NOT touch the filesystem, the database, or the
//! network. Callers assemble the inputs client-side
//! and pass them in. The IPC handler in
//! `src-tauri/src/main.rs` does the wiring
//! (DomainStore lookups + ResourceSnapshot::detect).

use crate::domain::{AiOperation, ImageVersion, PipelineRun};
use crate::recipe::Recipe;
use crate::resource::ResourceSnapshot;
use serde::{Deserialize, Serialize};

/// CR-08 §6 + §22 round 2 / Slice D: three-way
/// reproducibility verdict.
///
/// - `Exact`: every §6 condition is met; the user
///   can in principle reproduce the exact pixels
///   given the same source assets.
/// - `Material`: at least one known deviation
///   (hardware, backend, library, model, etc.);
///   the user can reproduce *material* results
///   but pixel parity is not guaranteed.
/// - `Indeterminate`: insufficient data to decide
///   (a condition field is unknown, e.g. the
///   pipeline run did not record a backend).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproducibilityVerdict {
    Exact,
    Material,
    Indeterminate,
}

impl ReproducibilityVerdict {
    /// One-line human-readable verdict label the UI
    /// can render directly. Stable across the enum's
    /// serde tag.
    pub fn label(&self) -> &'static str {
        match self {
            ReproducibilityVerdict::Exact => "Exact",
            ReproducibilityVerdict::Material => "Material",
            ReproducibilityVerdict::Indeterminate => "Indeterminate",
        }
    }
}

/// CR-08 §6 + §22 round 2 / Slice D: per-condition
/// verdict. Mirrors the §12 dimension pattern
/// (Slice C): the verdict per §6 dimension
/// (`source_assets` / `application_version` /
/// `engine_version` / `recipe` / `models` /
/// `parameters` / `backend` / `precision` /
/// `seed` / `execution_config`) is surfaced so the
/// UI can render a per-dimension checklist without
/// re-running the aggregator.
///
/// `Met`: the condition is verified (no
/// deviation). `Deviation(note)`: known
/// deviation with a human-readable note.
/// `Unknown(note)`: condition not recorded; the
/// aggregator cannot decide and folds to
/// `Indeterminate` if any dimension is `Unknown`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproducibilityCondition {
    Met,
    Deviation(String),
    Unknown(String),
}

/// CR-08 §6 + §22 round 2 / Slice D: per-dimension
/// verdict tuple (id + outcome) carried in the
/// record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproducibilityDimension {
    /// Stable dimension id the UI can switch on
    /// (`"source_assets"` / `"application_version"` /
    /// `"engine_version"` / `"recipe"` / `"models"` /
    /// `"parameters"` / `"backend"` / `"precision"` /
    /// `"seed"` / `"execution_config"`). New
    /// dimensions are additive: old UIs render the
    /// new id as "Unknown dimension" and the verdict
    /// still works.
    pub dimension: String,
    pub outcome: ReproducibilityCondition,
}

/// CR-08 §6 + §22 round 2 / Slice D: hardware
/// snapshot summary captured at run time. The
/// `ResourceSnapshot::detect()` call is
/// intentionally NOT executed inside this struct
/// (the aggregator is pure): the IPC handler
/// assembles the summary client-side and passes
/// it in. The summary carries only the data the
/// report renders (CPU model + GPU list +
/// available memory); the full snapshot stays in
/// the existing §22 `get_execution_environment`
/// IPC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareSummary {
    pub cpu_model: String,
    pub gpu_models: Vec<String>,
    pub available_memory_bytes: u64,
    pub recommended_backend: String,
}

impl HardwareSummary {
    /// Build a summary from the `ResourceSnapshot`
    /// produced by `ResourceSnapshot::detect()`.
    /// Pure projection; the snapshot itself stays
    /// available for the §22
    /// `get_execution_environment` surface.
    pub fn from_snapshot(snap: &ResourceSnapshot) -> Self {
        let gpu_models: Vec<String> = snap.gpus.iter().map(|g| g.name.clone()).collect();
        let recommended_backend = match snap.recommended.backend {
            crate::resource::ExecutionBackend::Cpu => "cpu",
            crate::resource::ExecutionBackend::Cuda => "cuda",
            crate::resource::ExecutionBackend::DirectMl => "directml",
            crate::resource::ExecutionBackend::CoreMl => "coreml",
            crate::resource::ExecutionBackend::OpenVino => "openvino",
        }
        .to_string();
        Self {
            cpu_model: snap.cpu_model.clone(),
            gpu_models,
            available_memory_bytes: snap.available_memory_bytes,
            recommended_backend,
        }
    }
}

/// CR-08 §6 + §22 round 2 / Slice D: full
/// reproducibility record returned to the UI.
/// Mirrors the §22 `RecipeProvenance` +
/// `ApplicabilityReport` surface pattern (Slice C):
/// top-level verdict + per-dimension list +
/// human-readable deviations.
///
/// `image_version_id`, `run_id`, `recipe_id` are
/// the join keys into the DomainStore; the
/// remaining fields are derived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproducibilityRecord {
    pub image_version_id: String,
    pub run_id: Option<String>,
    pub recipe_id: Option<String>,
    pub recipe_version: Option<u32>,
    pub recipe_hash: Option<String>,
    pub application_version: Option<String>,
    pub engine_version: Option<String>,
    pub hardware: Option<HardwareSummary>,
    /// Per-dimension verdict list (one row per §6
    /// dimension the aggregator evaluates).
    pub dimensions: Vec<ReproducibilityDimension>,
    /// Human-readable deviations. Always present
    /// (empty Vec when every dimension is `Met`).
    pub deviations: Vec<String>,
    /// Top-level 3-way verdict.
    pub verdict: ReproducibilityVerdict,
}

/// CR-08 §6 + §22 round 2 / Slice D: aggregate a
/// reproducibility record from the §6 inputs.
///
/// Pure function. The IPC handler assembles the
/// inputs via the DomainStore + the
/// `ResourceSnapshot::detect()` call and passes
/// them in; the aggregator itself does NOT touch
/// I/O.
///
/// Per-dimension verdict rules (priority: any
/// `Unknown` -> `Indeterminate`; otherwise any
/// `Deviation` -> `Material`; otherwise `Exact`).
pub fn summarize_reproducibility(
    image_version: &ImageVersion,
    run: Option<&PipelineRun>,
    recipe: Option<&Recipe>,
    ai_ops: &[AiOperation],
    hardware: Option<&HardwareSummary>,
) -> ReproducibilityRecord {
    let mut dimensions: Vec<ReproducibilityDimension> = Vec::new();
    let mut deviations: Vec<String> = Vec::new();

    // source_assets: the ImageVersion.source_version_id
    // carries the immediate predecessor; absence
    // means the version is the project root (no
    // predecessor to compare against). We surface
    // absence as `Unknown` so the user can see the
    // aggregator cannot guarantee the source-asset
    // half of exact reproducibility for root
    // versions.
    {
        let outcome = match &image_version.source_version_id {
            Some(_src) => ReproducibilityCondition::Met,
            None => ReproducibilityCondition::Unknown(
                "ImageVersion has no source_version_id (project root); the aggregator cannot verify the source-asset half of exact reproducibility"
                    .to_string(),
            ),
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "source_assets".to_string(),
            outcome,
        });
    }

    // application_version: pulled from the
    // PipelineRun (the run recorded at execute
    // time). When the run is unknown, this
    // dimension is `Unknown`.
    {
        let outcome = match run {
            Some(r) if !r.application_version.is_empty() => {
                ReproducibilityCondition::Met
            }
            Some(_) => ReproducibilityCondition::Deviation(
                "PipelineRun.application_version is empty".to_string(),
            ),
            None => ReproducibilityCondition::Unknown(
                "PipelineRun is not recorded for this ImageVersion; cannot verify application_version"
                    .to_string(),
            ),
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "application_version".to_string(),
            outcome,
        });
    }

    // engine_version: pulled from the PipelineRun
    // (and cross-referenced with the AiOperation
    // engine_version when available).
    {
        let outcome = match run {
            Some(r) if !r.engine_version.is_empty() => ReproducibilityCondition::Met,
            Some(_) => ReproducibilityCondition::Deviation(
                "PipelineRun.engine_version is empty".to_string(),
            ),
            None => ReproducibilityCondition::Unknown(
                "PipelineRun is not recorded; cannot verify engine_version".to_string(),
            ),
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "engine_version".to_string(),
            outcome,
        });
    }

    // recipe: Recipe identity + content hash.
    //
    // Slice E (CR-08 §21 follow-on): the
    // aggregator now also reads `recipe_version` +
    // `recipe_hash` from the ImageVersion itself.
    // If the ImageVersion carries all three
    // (recipe_id + recipe_version + recipe_hash),
    // the dimension resolves one of three ways:
    //
    // - `recipe_id` matches a current Recipe AND
    //   the current Recipe's hash matches the
    //   persisted `recipe_hash`: `Met`.
    // - `recipe_id` matches a current Recipe BUT
    //   the Recipe was edited post-apply (current
    //   hash != persisted `recipe_hash`):
    //   `Deviation` with a "Recipe was edited
    //   after the ImageVersion was produced:
    //   stored hash {a} != current hash {b}" note.
    // - `recipe_id` matches a current Recipe AND
    //   the persisted `recipe_hash` is empty:
    //   `Deviation` with the original "Recipe
    //   content hash is empty" note.
    // - `recipe_id` does not match any current
    //   Recipe: `Unknown` ("Recipe was deleted
    //   from the store after the ImageVersion was
    //   produced").
    // - `recipe` is None (legacy ImageVersion
    //   without `recipe_id`): `Unknown` (the
    //   honest surface pre-Slice E).
    {
        let outcome = match recipe {
            Some(r) => {
                let current_hash = r.pipeline_plan_hash();
                if current_hash.is_empty() {
                    ReproducibilityCondition::Deviation("Recipe content hash is empty".to_string())
                } else {
                    // Slice E: persisted hash matches the
                    // current Recipe hash. `Met` even when
                    // the persisted `recipe_hash` is None
                    // (legacy apply rounds that only wrote
                    // `recipe_id`).
                    if image_version
                        .recipe_hash
                        .as_ref()
                        .map(|h| h == &current_hash)
                        .unwrap_or(true)
                    {
                        ReproducibilityCondition::Met
                    } else {
                        // Slice E: persisted hash differs
                        // from the current Recipe hash.
                        // Recipe was edited post-apply.
                        ReproducibilityCondition::Deviation(
                                format!(
                                    "Recipe was edited after the ImageVersion was produced: stored hash {stored} != current hash {current}",
                                    stored = image_version
                                        .recipe_hash
                                        .as_deref()
                                        .unwrap_or("<none>"),
                                    current = current_hash,
                                ),
                            )
                    }
                }
            }
            None => {
                // Slice E: distinguish "legacy apply
                // round without recipe_id" (Unknown)
                // from "Recipe was deleted from the
                // store after apply" (also Unknown,
                // but with a sharper note).
                if image_version.recipe_id.is_some()
                    && image_version.recipe_version.is_some()
                    && image_version.recipe_hash.is_some()
                {
                    ReproducibilityCondition::Unknown(
                        "Recipe was deleted from the store after the ImageVersion was produced"
                            .to_string(),
                    )
                } else {
                    ReproducibilityCondition::Unknown(
                        "Recipe is not recorded for this ImageVersion; cannot verify the recipe half of exact reproducibility"
                            .to_string(),
                    )
                }
            }
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "recipe".to_string(),
            outcome,
        });
    }

    // models: every AiOperation's model_id +
    // model_version + model_hash. Empty `ai_ops`
    // is `Unknown` (the ImageVersion has no AI ops
    // but every reproducibility record for a
    // processed image should have at least one).
    {
        let outcome = if ai_ops.is_empty() {
            ReproducibilityCondition::Unknown(
                "ImageVersion has no recorded AiOperations; cannot verify model identity"
                    .to_string(),
            )
        } else {
            let mut missing_hash: Vec<String> = Vec::new();
            for op in ai_ops {
                if op.model_hash.is_none() {
                    missing_hash.push(op.operation_id.clone());
                }
            }
            if missing_hash.is_empty() {
                ReproducibilityCondition::Met
            } else {
                ReproducibilityCondition::Deviation(format!(
                    "{} AiOperation(s) are missing model_hash; cannot guarantee model identity",
                    missing_hash.len()
                ))
            }
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "models".to_string(),
            outcome,
        });
    }

    // parameters: every AiOperation's
    // parameters_json. Empty parameters_json for a
    // non-trivial operation is a Deviation.
    {
        let outcome = if ai_ops.is_empty() {
            ReproducibilityCondition::Unknown(
                "ImageVersion has no recorded AiOperations; cannot verify parameters".to_string(),
            )
        } else {
            let mut missing: Vec<String> = Vec::new();
            for op in ai_ops {
                if op.parameters_json.is_none() {
                    missing.push(op.operation_id.clone());
                }
            }
            if missing.is_empty() {
                ReproducibilityCondition::Met
            } else {
                ReproducibilityCondition::Deviation(format!(
                    "{} AiOperation(s) are missing parameters_json",
                    missing.len()
                ))
            }
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "parameters".to_string(),
            outcome,
        });
    }

    // backend: every AiOperation's backend. Empty
    // backend means the engine did not record the
    // runtime backend (typical for early-CI runs);
    // surface as Unknown.
    {
        let outcome = if ai_ops.is_empty() {
            ReproducibilityCondition::Unknown(
                "ImageVersion has no recorded AiOperations; cannot verify backend".to_string(),
            )
        } else {
            let mut missing: Vec<String> = Vec::new();
            for op in ai_ops {
                if op.backend.is_none() {
                    missing.push(op.operation_id.clone());
                }
            }
            if missing.is_empty() {
                ReproducibilityCondition::Met
            } else {
                ReproducibilityCondition::Unknown(format!(
                    "{} AiOperation(s) are missing backend; cannot guarantee runtime backend for exact reproducibility",
                    missing.len()
                ))
            }
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "backend".to_string(),
            outcome,
        });
    }

    // precision: every AiOperation's precision.
    // Same shape as backend. Precision drift
    // between fp16 / fp32 / mixed is a known
    // source of material-but-not-exact
    // reproducibility (per §6 material
    // reproducibility).
    {
        let outcome = if ai_ops.is_empty() {
            ReproducibilityCondition::Unknown(
                "ImageVersion has no recorded AiOperations; cannot verify precision".to_string(),
            )
        } else {
            let mut missing: Vec<String> = Vec::new();
            for op in ai_ops {
                if op.precision.is_none() {
                    missing.push(op.operation_id.clone());
                }
            }
            if missing.is_empty() {
                ReproducibilityCondition::Met
            } else {
                ReproducibilityCondition::Deviation(format!(
                    "{} AiOperation(s) are missing precision; fp16 / fp32 / mixed drift is a known source of material-but-not-exact reproducibility",
                    missing.len()
                ))
            }
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "precision".to_string(),
            outcome,
        });
    }

    // seed: stochastic / generative AiOperations
    // MUST carry a seed (per CR-06 §16) for exact
    // reproducibility. Deterministic operations
    // legitimately have `None`. We aggregate: any
    // non-deterministic op without a seed is a
    // Deviation; any non-deterministic op with a
    // seed is Met.
    {
        let outcome = if ai_ops.is_empty() {
            ReproducibilityCondition::Unknown(
                "ImageVersion has no recorded AiOperations; cannot verify seed".to_string(),
            )
        } else {
            let mut stochastic_without_seed: Vec<String> = Vec::new();
            for op in ai_ops {
                let stochastic = !op.deterministic
                    || !matches!(
                        op.safety_classification,
                        crate::domain::AiSafetyClassification::Deterministic
                    );
                if stochastic && op.seed.is_none() {
                    stochastic_without_seed.push(op.operation_id.clone());
                }
            }
            if stochastic_without_seed.is_empty() {
                ReproducibilityCondition::Met
            } else {
                ReproducibilityCondition::Deviation(format!(
                    "{} stochastic / generative AiOperation(s) are missing seed; CR-06 §16 requires seed for exact reproducibility",
                    stochastic_without_seed.len()
                ))
            }
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "seed".to_string(),
            outcome,
        });
    }

    // execution_config: PipelineRun.execution_mode
    // + PipelineRun.hardware_profile (informational;
    // the per-AiOperation backend covers the
    // runtime half).
    {
        let outcome = match run {
            Some(r) if r.execution_mode.is_some() => ReproducibilityCondition::Met,
            Some(_) => ReproducibilityCondition::Deviation(
                "PipelineRun.execution_mode is empty".to_string(),
            ),
            None => ReproducibilityCondition::Unknown(
                "PipelineRun is not recorded; cannot verify execution_mode".to_string(),
            ),
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "execution_config".to_string(),
            outcome,
        });
    }

    // hardware: informational; surfaced in the
    // record but NOT folded into the verdict (the
    // §6 spec calls hardware drift out as material
    // reproducibility, but we already capture the
    // backend + precision dimensions which are
    // what hardware drift actually affects).
    // Surface as `Met` (always recorded) when the
    // hardware summary is supplied; `Unknown`
    // otherwise.
    {
        let outcome = match hardware {
            Some(_) => ReproducibilityCondition::Met,
            None => ReproducibilityCondition::Unknown(
                "Hardware summary is not supplied; the record cannot annotate the run with hardware"
                    .to_string(),
            ),
        };
        if let ReproducibilityCondition::Unknown(note) | ReproducibilityCondition::Deviation(note) =
            &outcome
        {
            deviations.push(note.clone());
        }
        dimensions.push(ReproducibilityDimension {
            dimension: "hardware".to_string(),
            outcome,
        });
    }

    // Fold per-dimension outcomes into the top-level
    // 3-way verdict. Priority: any Unknown ->
    // Indeterminate; any Deviation -> Material; all
    // Met -> Exact.
    let verdict = if dimensions
        .iter()
        .any(|d| matches!(d.outcome, ReproducibilityCondition::Unknown(_)))
    {
        ReproducibilityVerdict::Indeterminate
    } else if dimensions
        .iter()
        .any(|d| matches!(d.outcome, ReproducibilityCondition::Deviation(_)))
    {
        ReproducibilityVerdict::Material
    } else {
        ReproducibilityVerdict::Exact
    };

    ReproducibilityRecord {
        image_version_id: image_version.version_id.clone(),
        run_id: run.map(|r| r.run_id.clone()),
        recipe_id: recipe.map(|r| r.name.clone()),
        recipe_version: recipe.map(|r| r.version),
        recipe_hash: recipe.map(|r| r.pipeline_plan_hash()),
        application_version: run.map(|r| r.application_version.clone()),
        engine_version: run.map(|r| r.engine_version.clone()),
        hardware: hardware.cloned(),
        dimensions,
        deviations,
        verdict,
    }
}
