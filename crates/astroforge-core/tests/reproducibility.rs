//! CR-08 §22 round 2 / Slice D tests:
//! `summarize_reproducibility` (pure function).
//!
//! The function walks the §6 chain (ImageVersion
//! -> PipelineRun -> Recipe -> AiOperation ->
//! hardware summary) and folds the per-dimension
//! outcomes into the 3-way ReproducibilityVerdict
//! (Exact / Material / Indeterminate).
//!
//! Tests pin the round-2 contract:
//! - All-Met inputs (every §6 dimension met) -> Exact
//!   verdict, no deviations.
//! - Any Unknown dimension -> Indeterminate verdict,
//!   deviations carry the per-dimension note.
//! - Any Deviation dimension -> Material verdict,
//!   deviations carry the per-dimension note.
//! - Per-dimension rows are returned in stable order:
//!   source_assets / application_version /
//!   engine_version / recipe / models / parameters /
//!   backend / precision / seed / execution_config /
//!   hardware.
//! - Hardware summary from ResourceSnapshot is a
//!   pure projection (CPU model + GPU names +
//!   available memory + recommended backend tag).
//! - `label()` returns the documented stable label
//!   per variant.
//! - `summarize_reproducibility` is pure (no I/O).
//! - Verdict folding: any Unknown -> Indeterminate;
//!   any Deviation (no Unknown) -> Material;
//!   all Met -> Exact.
//! - Serde round-trip via JSON preserves the
//!   verdict enum tag + per-dimension list.
//! - Missing parameters_json surfaces as Deviation
//!   for the parameters dimension.
//! - Missing model_hash surfaces as Deviation for
//!   the models dimension.
//! - Missing backend surfaces as Unknown for the
//!   backend dimension.
//! - Missing precision surfaces as Deviation for the
//!   precision dimension (precision drift is a known
//!   source of material-but-not-exact per §6).
//! - Stochastic AiOperation without seed is
//!   Deviation for the seed dimension (CR-06 §16).
//! - Deterministic AiOperation without seed is Met
//!   for the seed dimension.
//! - Empty `ai_ops` surfaces as Unknown for every
//!   AI-derived dimension.

use astroforge_core::domain::{
    AiOperation, AiSafetyClassification, ImageVersion, PipelineRun, PipelineRunStatus,
};
use astroforge_core::recipe::{IntegrityBadge, QualityProfile, Recipe, RecipeStage};
use astroforge_core::reproducibility::{
    summarize_reproducibility, HardwareSummary, ReproducibilityCondition, ReproducibilityVerdict,
};

fn base_image_version() -> ImageVersion {
    ImageVersion {
        version_id: "v-001".to_string(),
        project_id: "p-001".to_string(),
        label: "V1".to_string(),
        sequence: 1,
        primary_artifact_id: "art-001".to_string(),
        source_version_id: Some("v-000".to_string()),
        created_at: "2026-09-24T00:00:00Z".to_string(),
        hidden: false,
        recipe_id: Some("prof_test_deep_sky".to_string()),
        recipe_version: None,
        recipe_hash: None,
    }
}

fn root_image_version() -> ImageVersion {
    let mut iv = base_image_version();
    iv.sequence = 0;
    iv.source_version_id = None;
    iv
}

fn base_run() -> PipelineRun {
    PipelineRun {
        run_id: "pr-001".to_string(),
        project_id: "p-001".to_string(),
        session_ids: vec!["s-001".to_string()],
        recipe_id: Some("prof_test_deep_sky".to_string()),
        started_at: Some("2026-09-24T00:00:00Z".to_string()),
        completed_at: Some("2026-09-24T00:00:01Z".to_string()),
        status: PipelineRunStatus::Completed,
        application_version: "0.1.0".to_string(),
        engine_version: "astroforge-ai-0.1.0".to_string(),
        hardware_profile: Some("x86_64-linux".to_string()),
        execution_mode: Some("interactive".to_string()),
        input_artifacts: vec!["art-000".to_string()],
        output_artifacts: vec!["art-001".to_string()],
    }
}

fn base_recipe() -> Recipe {
    Recipe {
        schema_version: "2.0".to_string(),
        name: "test".to_string(),
        description: "test recipe".to_string(),
        target_type: "deep_sky".to_string(),
        stages: vec![RecipeStage {
            stage_id: "stacking".to_string(),
            enabled: true,
            params: Default::default(),
            ai_enhancement_override: None,
        }],
        required_models: vec![],
        integrity: IntegrityBadge {
            perceptual_models_used: false,
            deterministic_models_used: true,
            seed_recorded: false,
            models: vec![],
        },
        version: 1,
        parent_version: None,
        branch: "main".to_string(),
        is_system: false,
        created_at: "2026-09-24T00:00:00Z".to_string(),
        flags: vec![],
        quality_profile: QualityProfile::Natural,
        ai_enhancement_level: Default::default(),
        processing_objectives: vec![],
        quality_targets: Default::default(),
        optional_operations: vec![],
        // CR-08 §21 / Slice G: empty constraints +
        // no resource policy for legacy tests.
        constraints: vec![],
        resource_policy: None,
    }
}

fn base_ai_op() -> AiOperation {
    AiOperation {
        operation_id: "op-001".to_string(),
        stage_run_id: "sr-001".to_string(),
        model_id: "m-stacking".to_string(),
        model_version: "1.0.0".to_string(),
        model_hash: Some("sha256:abc123".to_string()),
        runtime: Some("ort".to_string()),
        backend: Some("cpu".to_string()),
        precision: Some("float32".to_string()),
        parameters_json: Some(r#"{"gain": 1.5}"#.to_string()),
        seed: None,
        deterministic: true,
        safety_classification: AiSafetyClassification::Deterministic,
        experimental: false,
        input_artifact_id: Some("art-000".to_string()),
        output_artifact_id: Some("art-001".to_string()),
        engine_version: Some("astroforge-ai-0.1.0".to_string()),
        tile_configuration: None,
        resource_metrics: None,
    }
}

fn base_hardware() -> HardwareSummary {
    HardwareSummary {
        cpu_model: "AMD Ryzen 9 7950X".to_string(),
        gpu_models: vec!["NVIDIA RTX 4090".to_string()],
        available_memory_bytes: 64 * 1024 * 1024 * 1024,
        recommended_backend: "cuda".to_string(),
    }
}

#[test]
fn all_met_inputs_yield_exact_verdict() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let ops = vec![base_ai_op()];
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &ops, Some(&hw));

    assert_eq!(record.verdict, ReproducibilityVerdict::Exact);
    assert!(record.deviations.is_empty());
    assert_eq!(record.image_version_id, "v-001");
    assert_eq!(record.run_id, Some("pr-001".to_string()));
    assert_eq!(record.recipe_version, Some(1));
    assert_eq!(record.application_version, Some("0.1.0".to_string()));
    assert_eq!(
        record.engine_version,
        Some("astroforge-ai-0.1.0".to_string())
    );
    assert!(record.recipe_hash.is_some());
}

#[test]
fn root_image_version_yields_indeterminate_via_unknown_source_assets() {
    let iv = root_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let ops = vec![base_ai_op()];
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &ops, Some(&hw));

    // The source_assets dimension is Unknown (no
    // source_version_id), so the verdict folds to
    // Indeterminate.
    assert_eq!(record.verdict, ReproducibilityVerdict::Indeterminate);
    let source_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "source_assets")
        .unwrap();
    assert!(matches!(
        source_dim.outcome,
        ReproducibilityCondition::Unknown(_)
    ));
    assert!(record
        .deviations
        .iter()
        .any(|d| d.contains("source_version_id")));
}

#[test]
fn missing_run_yields_indeterminate() {
    let iv = base_image_version();
    let recipe = base_recipe();
    let ops = vec![base_ai_op()];
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, None, Some(&recipe), &ops, Some(&hw));

    // application_version, engine_version, and
    // execution_config all surface Unknown when the
    // run is missing.
    assert_eq!(record.verdict, ReproducibilityVerdict::Indeterminate);
    let app = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "application_version")
        .unwrap();
    assert!(matches!(app.outcome, ReproducibilityCondition::Unknown(_)));
    let engine = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "engine_version")
        .unwrap();
    assert!(matches!(
        engine.outcome,
        ReproducibilityCondition::Unknown(_)
    ));
}

#[test]
fn missing_recipe_yields_indeterminate() {
    let iv = base_image_version();
    let run = base_run();
    let ops = vec![base_ai_op()];
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), None, &ops, Some(&hw));

    assert_eq!(record.verdict, ReproducibilityVerdict::Indeterminate);
    let recipe_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "recipe")
        .unwrap();
    assert!(matches!(
        recipe_dim.outcome,
        ReproducibilityCondition::Unknown(_)
    ));
}

#[test]
fn missing_model_hash_yields_material_via_deviation() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let mut op = base_ai_op();
    op.model_hash = None;
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &[op], Some(&hw));

    assert_eq!(record.verdict, ReproducibilityVerdict::Material);
    let models_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "models")
        .unwrap();
    assert!(matches!(
        models_dim.outcome,
        ReproducibilityCondition::Deviation(_)
    ));
    assert!(record.deviations.iter().any(|d| d.contains("model_hash")));
}

#[test]
fn missing_parameters_yields_material_via_deviation() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let mut op = base_ai_op();
    op.parameters_json = None;
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &[op], Some(&hw));

    assert_eq!(record.verdict, ReproducibilityVerdict::Material);
    let params_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "parameters")
        .unwrap();
    assert!(matches!(
        params_dim.outcome,
        ReproducibilityCondition::Deviation(_)
    ));
}

#[test]
fn missing_backend_yields_indeterminate_via_unknown() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let mut op = base_ai_op();
    op.backend = None;
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &[op], Some(&hw));

    // backend is Unknown (not Deviation) per the
    // aggregator's semantics: the engine did not
    // record the runtime backend, so the aggregator
    // cannot guarantee runtime backend for exact
    // reproducibility.
    assert_eq!(record.verdict, ReproducibilityVerdict::Indeterminate);
    let backend_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "backend")
        .unwrap();
    assert!(matches!(
        backend_dim.outcome,
        ReproducibilityCondition::Unknown(_)
    ));
}

#[test]
fn missing_precision_yields_material_via_deviation() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let mut op = base_ai_op();
    op.precision = None;
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &[op], Some(&hw));

    // precision is Deviation (not Unknown) per §6:
    // fp16 / fp32 / mixed drift is a known source of
    // material-but-not-exact reproducibility.
    assert_eq!(record.verdict, ReproducibilityVerdict::Material);
    let prec_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "precision")
        .unwrap();
    assert!(matches!(
        prec_dim.outcome,
        ReproducibilityCondition::Deviation(_)
    ));
    assert!(record
        .deviations
        .iter()
        .any(|d| d.contains("fp16") || d.contains("fp32")));
}

#[test]
fn stochastic_op_without_seed_yields_material_via_deviation() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let mut op = base_ai_op();
    op.deterministic = false;
    op.safety_classification = AiSafetyClassification::Generative;
    op.seed = None;
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &[op], Some(&hw));

    assert_eq!(record.verdict, ReproducibilityVerdict::Material);
    let seed_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "seed")
        .unwrap();
    assert!(matches!(
        seed_dim.outcome,
        ReproducibilityCondition::Deviation(_)
    ));
    assert!(record.deviations.iter().any(|d| d.contains("seed")));
}

#[test]
fn deterministic_op_without_seed_is_met() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let op = base_ai_op();
    // op is Deterministic by default; no seed set.
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &[op], Some(&hw));

    assert_eq!(record.verdict, ReproducibilityVerdict::Exact);
    let seed_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "seed")
        .unwrap();
    assert_eq!(seed_dim.outcome, ReproducibilityCondition::Met);
}

#[test]
fn empty_ai_ops_yields_indeterminate() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &[], Some(&hw));

    assert_eq!(record.verdict, ReproducibilityVerdict::Indeterminate);
    // models / parameters / backend / precision /
    // seed all Unknown.
    for dim in &record.dimensions {
        if matches!(
            dim.dimension.as_str(),
            "models" | "parameters" | "backend" | "precision" | "seed"
        ) {
            assert!(matches!(dim.outcome, ReproducibilityCondition::Unknown(_)));
        }
    }
}

#[test]
fn missing_hardware_yields_indeterminate_via_unknown() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let ops = vec![base_ai_op()];

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &ops, None);

    assert_eq!(record.verdict, ReproducibilityVerdict::Indeterminate);
    let hw_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "hardware")
        .unwrap();
    assert!(matches!(
        hw_dim.outcome,
        ReproducibilityCondition::Unknown(_)
    ));
}

#[test]
fn per_dimension_rows_returned_in_stable_order() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let ops = vec![base_ai_op()];
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &ops, Some(&hw));

    let order: Vec<&str> = record
        .dimensions
        .iter()
        .map(|d| d.dimension.as_str())
        .collect();
    assert_eq!(
        order,
        vec![
            "source_assets",
            "application_version",
            "engine_version",
            "recipe",
            "models",
            "parameters",
            "backend",
            "precision",
            "seed",
            "execution_config",
            "hardware",
        ]
    );
}

#[test]
fn label_returns_documented_stable_label() {
    assert_eq!(ReproducibilityVerdict::Exact.label(), "Exact");
    assert_eq!(ReproducibilityVerdict::Material.label(), "Material");
    assert_eq!(
        ReproducibilityVerdict::Indeterminate.label(),
        "Indeterminate"
    );
}

#[test]
fn verdict_folding_unknown_wins_over_deviation() {
    // Mix one Unknown dimension (root IV -> source_assets)
    // with one Deviation dimension (missing model_hash).
    // Unknown wins -> Indeterminate.
    let iv = root_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let mut op = base_ai_op();
    op.model_hash = None;
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &[op], Some(&hw));

    assert_eq!(record.verdict, ReproducibilityVerdict::Indeterminate);
    let source_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "source_assets")
        .unwrap();
    assert!(matches!(
        source_dim.outcome,
        ReproducibilityCondition::Unknown(_)
    ));
    let models_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "models")
        .unwrap();
    assert!(matches!(
        models_dim.outcome,
        ReproducibilityCondition::Deviation(_)
    ));
}

#[test]
fn serde_round_trip_via_json() {
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let ops = vec![base_ai_op()];
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &ops, Some(&hw));
    let json = serde_json::to_string(&record).expect("serialize");
    let back: astroforge_core::reproducibility::ReproducibilityRecord =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(record, back);
}

#[test]
fn pure_function_no_io() {
    // The aggregator is pure: calling it twice with
    // the same inputs returns the same record.
    let iv = base_image_version();
    let run = base_run();
    let recipe = base_recipe();
    let ops = vec![base_ai_op()];
    let hw = base_hardware();

    let a = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &ops, Some(&hw));
    let b = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &ops, Some(&hw));
    assert_eq!(a, b);
}

#[test]
fn empty_execution_mode_in_run_yields_material() {
    let iv = base_image_version();
    let mut run = base_run();
    run.execution_mode = None;
    let recipe = base_recipe();
    let ops = vec![base_ai_op()];
    let hw = base_hardware();

    let record = summarize_reproducibility(&iv, Some(&run), Some(&recipe), &ops, Some(&hw));

    // execution_config surfaces as Deviation when
    // the run is recorded but execution_mode is
    // empty. (No Unknowns in any other dimension,
    // so the verdict folds to Material.)
    assert_eq!(record.verdict, ReproducibilityVerdict::Material);
    let exec_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "execution_config")
        .unwrap();
    assert!(matches!(
        exec_dim.outcome,
        ReproducibilityCondition::Deviation(_)
    ));
}

#[test]
fn hardware_summary_from_snapshot() {
    // Construct a minimal ResourceSnapshot and
    // verify the projection works without I/O.
    use astroforge_core::resource::{
        ExecutionBackend, GpuInfo, Precision, RecommendedExecution, ResourceSnapshot,
    };
    let snap = ResourceSnapshot {
        cpu_model: "AMD Ryzen 9 7950X".to_string(),
        logical_cores: 16,
        physical_cores: Some(8),
        total_memory_bytes: 128 * 1024 * 1024 * 1024,
        available_memory_bytes: 64 * 1024 * 1024 * 1024,
        gpus: vec![GpuInfo {
            name: "NVIDIA RTX 4090".to_string(),
            backend: ExecutionBackend::Cuda,
            vram_bytes: Some(24 * 1024 * 1024 * 1024),
        }],
        recommended: RecommendedExecution {
            backend: ExecutionBackend::Cuda,
            tile_size: 512,
            thread_count: 8,
            precision: Precision::F32,
            memory_budget_bytes: 4 * 1024 * 1024 * 1024,
        },
    };
    let hw = HardwareSummary::from_snapshot(&snap);
    assert_eq!(hw.cpu_model, "AMD Ryzen 9 7950X");
    assert_eq!(hw.gpu_models, vec!["NVIDIA RTX 4090".to_string()]);
    assert_eq!(hw.available_memory_bytes, 64 * 1024 * 1024 * 1024);
    assert_eq!(hw.recommended_backend, "cuda");
}

// ─── CR-08 §21 follow-on / Slice E tests ────────────────────────────────
//
// These tests pin the Slice E behavior of the
// `recipe` dimension in `summarize_reproducibility`.
// Pre-Slice E, the dimension was `Met` whenever
// the caller passed a non-empty Recipe (legacy
// behavior). Slice E extends the dimension so it
// also reads the persisted `recipe_id` +
// `recipe_version` + `recipe_hash` from the
// ImageVersion and compares the persisted hash
// against the current Recipe hash.

/// Build an ImageVersion with the Slice E
/// Recipe identity fields populated.
fn iv_with_recipe_identity() -> ImageVersion {
    let mut iv = base_image_version();
    iv.recipe_version = Some(1);
    iv.recipe_hash = Some("slice_e_test_hash".to_string());
    iv
}

/// Slice E: when the persisted `recipe_hash`
/// matches the current Recipe hash, the `recipe`
/// dimension is `Met`.
#[test]
fn slice_e_matching_recipe_hash_yields_met() {
    let mut iv = iv_with_recipe_identity();
    let recipe = base_recipe();
    // Mirror what the apply round would do: write
    // the Recipe's current hash to the ImageVersion.
    iv.recipe_hash = Some(recipe.pipeline_plan_hash());

    let record = summarize_reproducibility(
        &iv,
        Some(&base_run()),
        Some(&recipe),
        &[base_ai_op()],
        Some(&base_hardware()),
    );

    let recipe_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "recipe")
        .unwrap();
    assert!(
        matches!(recipe_dim.outcome, ReproducibilityCondition::Met),
        "expected Met, got {:?}",
        recipe_dim.outcome
    );
    // No deviation for the recipe dimension when Met.
    assert!(
        !record
            .deviations
            .iter()
            .any(|d| d.contains("Recipe was edited")),
        "no Recipe-edited deviation should appear: {:?}",
        record.deviations
    );
}

/// Slice E: when the persisted `recipe_hash`
/// differs from the current Recipe hash, the
/// dimension flips to `Deviation` with the
/// "Recipe was edited after the ImageVersion was
/// produced" note.
#[test]
fn slice_e_modified_recipe_hash_yields_deviation() {
    let iv = iv_with_recipe_identity();
    let recipe = base_recipe();
    // ImageVersion carries a stale hash.
    assert_ne!(
        iv.recipe_hash.as_deref(),
        Some(recipe.pipeline_plan_hash().as_str())
    );

    let record = summarize_reproducibility(
        &iv,
        Some(&base_run()),
        Some(&recipe),
        &[base_ai_op()],
        Some(&base_hardware()),
    );

    let recipe_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "recipe")
        .unwrap();
    assert!(
        matches!(recipe_dim.outcome, ReproducibilityCondition::Deviation(_)),
        "expected Deviation, got {:?}",
        recipe_dim.outcome
    );
    assert!(
        record
            .deviations
            .iter()
            .any(|d| d.contains("Recipe was edited after")),
        "expected Recipe-edited deviation, got {:?}",
        record.deviations
    );
}

/// Slice E: when the ImageVersion carries all
/// three identity fields but the RecipeStore can
/// no longer resolve the Recipe (recipe: None),
/// the dimension is `Unknown` with the sharper
/// "Recipe was deleted from the store" note.
#[test]
fn slice_e_deleted_recipe_yields_unknown_with_sharper_note() {
    let iv = iv_with_recipe_identity();
    // Recipe: None simulates "RecipeStore can no
    // longer resolve this profile".
    let record = summarize_reproducibility(
        &iv,
        Some(&base_run()),
        None,
        &[base_ai_op()],
        Some(&base_hardware()),
    );

    let recipe_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "recipe")
        .unwrap();
    assert!(
        matches!(recipe_dim.outcome, ReproducibilityCondition::Unknown(_)),
        "expected Unknown, got {:?}",
        recipe_dim.outcome
    );
    assert!(
        record
            .deviations
            .iter()
            .any(|d| d.contains("Recipe was deleted from the store")),
        "expected deleted-from-store deviation, got {:?}",
        record.deviations
    );
}

/// Slice E: the `recipe` dimension still surfaces
/// `Unknown` for legacy ImageVersions (no
/// `recipe_id` + `recipe_version` + `recipe_hash`)
/// - the honest pre-Slice E surface.
#[test]
fn slice_e_legacy_image_version_yields_unknown_with_legacy_note() {
    let iv = base_image_version();
    assert!(iv.recipe_id.is_some());
    assert!(iv.recipe_version.is_none());
    assert!(iv.recipe_hash.is_none());

    let record = summarize_reproducibility(
        &iv,
        Some(&base_run()),
        None,
        &[base_ai_op()],
        Some(&base_hardware()),
    );

    let recipe_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "recipe")
        .unwrap();
    assert!(
        matches!(recipe_dim.outcome, ReproducibilityCondition::Unknown(_)),
        "expected Unknown, got {:?}",
        recipe_dim.outcome
    );
    assert!(
        record
            .deviations
            .iter()
            .any(|d| d.contains("Recipe is not recorded")),
        "expected legacy-note deviation, got {:?}",
        record.deviations
    );
}

/// Slice E: when the Recipe was already passed
/// in but the ImageVersion has no Recipe identity
/// fields (legacy apply round + Recipe exists),
/// the dimension is still `Met` because the
/// aggregator falls back to the legacy behavior.
#[test]
fn slice_e_legacy_image_version_with_resolved_recipe_yields_met() {
    let iv = base_image_version();
    assert!(iv.recipe_version.is_none());

    let record = summarize_reproducibility(
        &iv,
        Some(&base_run()),
        Some(&base_recipe()),
        &[base_ai_op()],
        Some(&base_hardware()),
    );

    let recipe_dim = record
        .dimensions
        .iter()
        .find(|d| d.dimension == "recipe")
        .unwrap();
    assert!(
        matches!(recipe_dim.outcome, ReproducibilityCondition::Met),
        "expected Met (legacy fallback), got {:?}",
        recipe_dim.outcome
    );
}
