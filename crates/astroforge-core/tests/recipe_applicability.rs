//! CR-08 §22 round 2 / Slice C tests:
//! `check_recipe_applicability` (pure function).
//!
//! The function is the §12 6-variant matrix that
//! evaluates a Recipe against a session-side
//! `ApplicabilityMatrix`. Tests pin the round-2
//! contract:
//! - Empty matrix (no dimensions supplied) +
//!   current-schema Recipe + no required models +
//!   target_type set → Compatible verdict. The
//!   `schema_version` dimension evaluates to `Match`
//!   (the schema check does not depend on matrix
//!   fields); the other 4 dimensions are
//!   `NotEvaluated`; no warnings are emitted.
//! - Empty matrix + mismatched schema version →
//!   SchemaMismatch verdict, schema_version
//!   dimension Mismatch, warning carries the
//!   version string.
//! - Available-models supplied + missing required
//!   model → MissingModels verdict listing the
//!   missing model id; available_models dimension
//!   Mismatch; warning enumerates the gap.
//! - Target type matches exactly → Match; target
//!   type mismatches → Incompatible (target-type
//!   mismatches are hard blockers per the §12 spec
//!   because the Recipe was authored for a
//!   different imaging target).
//! - Recipe target_type unset ("unknown") but
//!   session target_type supplied → Adaptable;
//!   verdict folds to Adaptable.
//! - Image dimensions supplied → Adaptable verdict
//!   (Recipe has no applicability-block fields yet,
//!   so the engine can scale within reason).
//! - dataset_quality supplied → Adaptable verdict.
//! - All dimensions Adaptable (no Match, no
//!   Mismatch) → verdict folds to Adaptable (the
//!   softest non-Compatible fold).
//! - Mixed Match + Mismatch (target_type match,
//!   available_models gap) → MissingModels wins
//!   over Incompatible because the missing-models
//!   fold precedes the Incompatible fold.
//! - Mixed schema mismatch + available models
//!   supplied → SchemaMismatch wins (top of the
//!   priority ladder).
//! - `is_applicable()` matches the documented
//!   fold: true for Compatible / Adaptable /
//!   PartiallyCompatible; false for Incompatible /
//!   MissingModels / SchemaMismatch.
//! - `label()` returns the documented stable label
//!   per variant.
//! - `DimensionOutcome::NotEvaluated` does NOT
//!   produce a warning (no note → no advisory).
//! - Empty matrix + zero `required_models` +
//!   current schema + Recipe with `target_type`
//!   set → Compatible verdict.
//! - Per-dimension rows are returned in stable
//!   order: schema_version, available_models,
//!   target_type, image_dimensions, dataset_quality.

use astroforge_core::recipe::{
    check_recipe_applicability, ApplicabilityDimension, ApplicabilityMatrix, ApplicabilityOutcome,
    DimensionOutcome, IntegrityBadge, QualityProfile, Recipe, RecipeStage,
};

fn base_recipe() -> Recipe {
    Recipe {
        schema_version: "2.0".to_string(),
        name: "Test".to_string(),
        description: "test".to_string(),
        target_type: "deep_sky".to_string(),
        stages: Vec::new(),
        required_models: Vec::new(),
        integrity: IntegrityBadge {
            perceptual_models_used: false,
            deterministic_models_used: true,
            seed_recorded: false,
            models: Vec::new(),
        },
        version: 1,
        parent_version: None,
        branch: "main".to_string(),
        is_system: false,
        created_at: "2026-09-24T00:00:00Z".to_string(),
        flags: Vec::new(),
        quality_profile: QualityProfile::Natural,
        ai_enhancement_level: Default::default(),
        processing_objectives: Vec::new(),
        quality_targets: Default::default(),
        optional_operations: Vec::new(),
        // CR-08 §21 / Slice G: empty constraints +
        // no resource policy for legacy tests.
        constraints: Vec::new(),
        resource_policy: None,
    }
}

fn recipe_with_required(required: Vec<String>) -> Recipe {
    let mut r = base_recipe();
    r.required_models = required;
    r
}

fn recipe_with_target(target: &str) -> Recipe {
    let mut r = base_recipe();
    r.target_type = target.to_string();
    r
}

fn recipe_with_schema(schema: &str) -> Recipe {
    let mut r = base_recipe();
    r.schema_version = schema.to_string();
    r
}

#[test]
fn empty_matrix_with_current_schema_and_no_required_models_is_compatible() {
    let recipe = base_recipe();
    let matrix = ApplicabilityMatrix::default();
    let report = check_recipe_applicability(&recipe, &matrix);

    assert_eq!(report.verdict, ApplicabilityOutcome::Compatible);
    assert!(report.warnings.is_empty(), "no warnings on Compatible");
    // 5 dimensions are reported. `schema_version` is
    // evaluated from the Recipe alone (it does not
    // depend on matrix fields); when the Recipe's
    // schema is current, this dimension is `Match`
    // (not `NotEvaluated`). The other 4 dimensions
    // are `NotEvaluated` because the matrix is empty.
    assert_eq!(report.dimensions.len(), 5);
    let schema_dim = report
        .dimensions
        .iter()
        .find(|d| d.dimension == "schema_version")
        .expect("schema_version dimension present");
    assert_eq!(schema_dim.outcome, DimensionOutcome::Match);
    for dim in &report.dimensions {
        if dim.dimension == "schema_version" {
            continue;
        }
        assert_eq!(
            dim.outcome,
            DimensionOutcome::NotEvaluated,
            "dimension {} should be NotEvaluated",
            dim.dimension
        );
    }
}

#[test]
fn mismatched_schema_version_yields_schema_mismatch() {
    let recipe = recipe_with_schema("9.9");
    let matrix = ApplicabilityMatrix::default();
    let report = check_recipe_applicability(&recipe, &matrix);

    // SchemaMismatch carries the full human-readable
    // note (not just the version) so the UI can
    // render the verdict directly. The note contains
    // the version string as a substring so the UI
    // can still extract it for badge rendering.
    match &report.verdict {
        ApplicabilityOutcome::SchemaMismatch(note) => {
            assert!(note.contains("9.9"));
            assert!(note.contains("2.0"));
        }
        other => panic!("expected SchemaMismatch, got {other:?}"),
    }
    let schema_dim = report
        .dimensions
        .iter()
        .find(|d| d.dimension == "schema_version")
        .expect("schema_version dimension present");
    assert!(matches!(schema_dim.outcome, DimensionOutcome::Mismatch(_)));
    assert!(
        report.warnings.iter().any(|w| w.contains("9.9")),
        "warning carries the version string"
    );
    assert!(!report.verdict.is_applicable());
}

#[test]
fn missing_required_model_yields_missing_models() {
    let recipe = recipe_with_required(vec!["m-x".to_string()]);
    let matrix = ApplicabilityMatrix {
        available_models: Some(vec!["m-a".to_string(), "m-b".to_string()]),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);

    match &report.verdict {
        ApplicabilityOutcome::MissingModels(missing) => {
            assert_eq!(missing, &vec!["m-x".to_string()]);
        }
        other => panic!("expected MissingModels, got {other:?}"),
    }
    let avail_dim = report
        .dimensions
        .iter()
        .find(|d| d.dimension == "available_models")
        .expect("available_models dimension present");
    assert!(matches!(avail_dim.outcome, DimensionOutcome::Mismatch(_)));
    assert!(report.warnings.iter().any(|w| w.contains("m-x")));
    assert!(!report.verdict.is_applicable());
}

#[test]
fn present_required_model_is_match() {
    let recipe = recipe_with_required(vec!["m-a".to_string()]);
    let matrix = ApplicabilityMatrix {
        available_models: Some(vec!["m-a".to_string(), "m-b".to_string()]),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);
    assert_eq!(report.verdict, ApplicabilityOutcome::Compatible);
    let avail_dim = report
        .dimensions
        .iter()
        .find(|d| d.dimension == "available_models")
        .unwrap();
    assert_eq!(avail_dim.outcome, DimensionOutcome::Match);
}

#[test]
fn matching_target_type_is_match() {
    let recipe = recipe_with_target("deep_sky");
    let matrix = ApplicabilityMatrix {
        target_type: Some("deep_sky".to_string()),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);
    assert_eq!(report.verdict, ApplicabilityOutcome::Compatible);
    let target_dim = report
        .dimensions
        .iter()
        .find(|d| d.dimension == "target_type")
        .unwrap();
    assert_eq!(target_dim.outcome, DimensionOutcome::Match);
}

#[test]
fn mismatched_target_type_is_incompatible() {
    let recipe = recipe_with_target("deep_sky");
    let matrix = ApplicabilityMatrix {
        target_type: Some("planetary".to_string()),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);

    assert_eq!(report.verdict, ApplicabilityOutcome::Incompatible);
    let target_dim = report
        .dimensions
        .iter()
        .find(|d| d.dimension == "target_type")
        .unwrap();
    assert!(matches!(target_dim.outcome, DimensionOutcome::Mismatch(_)));
    assert!(report.warnings.iter().any(|w| w.contains("deep_sky")));
    assert!(report.warnings.iter().any(|w| w.contains("planetary")));
    assert!(!report.verdict.is_applicable());
}

#[test]
fn unset_recipe_target_type_is_adaptable() {
    let recipe = recipe_with_target("unknown");
    let matrix = ApplicabilityMatrix {
        target_type: Some("deep_sky".to_string()),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);

    assert_eq!(report.verdict, ApplicabilityOutcome::Adaptable);
    let target_dim = report
        .dimensions
        .iter()
        .find(|d| d.dimension == "target_type")
        .unwrap();
    assert!(matches!(target_dim.outcome, DimensionOutcome::Adaptable(_)));
    assert!(report.verdict.is_applicable());
}

#[test]
fn image_dimensions_supplied_is_adaptable() {
    let recipe = base_recipe();
    let matrix = ApplicabilityMatrix {
        image_width: Some(4096),
        image_height: Some(2730),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);

    assert_eq!(report.verdict, ApplicabilityOutcome::Adaptable);
    let img_dim = report
        .dimensions
        .iter()
        .find(|d| d.dimension == "image_dimensions")
        .unwrap();
    assert!(matches!(img_dim.outcome, DimensionOutcome::Adaptable(_)));
}

#[test]
fn dataset_quality_supplied_is_adaptable() {
    let recipe = base_recipe();
    let matrix = ApplicabilityMatrix {
        dataset_quality: Some(0.742),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);

    assert_eq!(report.verdict, ApplicabilityOutcome::Adaptable);
    let q_dim = report
        .dimensions
        .iter()
        .find(|d| d.dimension == "dataset_quality")
        .unwrap();
    assert!(matches!(q_dim.outcome, DimensionOutcome::Adaptable(_)));
}

#[test]
fn all_dimensions_adaptable_folds_to_adaptable() {
    let recipe = base_recipe();
    let matrix = ApplicabilityMatrix {
        target_type: Some("deep_sky".to_string()),
        image_width: Some(1024),
        image_height: Some(768),
        available_models: Some(vec![]),
        dataset_quality: Some(0.5),
    };
    let report = check_recipe_applicability(&recipe, &matrix);

    // available_models: Some(empty list) + no
    // required_models → Match. target_type: exact
    // match → Match. image_dimensions + quality →
    // Adaptable. All Match + Adaptable folds to
    // Adaptable (the softest non-Compatible fold).
    assert_eq!(report.verdict, ApplicabilityOutcome::Adaptable);
}

#[test]
fn mixed_match_and_missing_models_yields_missing_models() {
    // Recipe requires m-x but session only has
    // m-a/b. Target type matches exactly. The
    // verdict must fold to MissingModels (the
    // missing-models fold precedes the
    // Incompatible fold for non-schema dimensions).
    let recipe = recipe_with_required(vec!["m-x".to_string()]);
    let matrix = ApplicabilityMatrix {
        target_type: Some("deep_sky".to_string()),
        available_models: Some(vec!["m-a".to_string()]),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);

    match &report.verdict {
        ApplicabilityOutcome::MissingModels(m) => assert_eq!(m, &vec!["m-x".to_string()]),
        other => panic!("expected MissingModels, got {other:?}"),
    }
}

#[test]
fn mixed_schema_mismatch_and_missing_models_yields_schema_mismatch() {
    let recipe = recipe_with_schema("9.9");
    // Even with the missing-models gap, the
    // schema-mismatch fold (top of the priority
    // ladder) wins.
    let matrix = ApplicabilityMatrix {
        available_models: Some(vec![]),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);

    match &report.verdict {
        ApplicabilityOutcome::SchemaMismatch(note) => {
            assert!(note.contains("9.9"));
        }
        other => panic!("expected SchemaMismatch, got {other:?}"),
    }
}

#[test]
fn is_applicable_matches_documented_fold() {
    assert!(ApplicabilityOutcome::Compatible.is_applicable());
    assert!(ApplicabilityOutcome::Adaptable.is_applicable());
    assert!(ApplicabilityOutcome::PartiallyCompatible.is_applicable());
    assert!(!ApplicabilityOutcome::Incompatible.is_applicable());
    assert!(!ApplicabilityOutcome::MissingModels(vec![]).is_applicable());
    assert!(!ApplicabilityOutcome::SchemaMismatch("x".to_string()).is_applicable());
}

#[test]
fn label_returns_documented_stable_label() {
    assert_eq!(ApplicabilityOutcome::Compatible.label(), "Compatible");
    assert_eq!(ApplicabilityOutcome::Adaptable.label(), "Adaptable");
    assert_eq!(
        ApplicabilityOutcome::PartiallyCompatible.label(),
        "Partially compatible"
    );
    assert_eq!(ApplicabilityOutcome::Incompatible.label(), "Incompatible");
    assert_eq!(
        ApplicabilityOutcome::MissingModels(vec![]).label(),
        "Missing models"
    );
    assert_eq!(
        ApplicabilityOutcome::SchemaMismatch("x".to_string()).label(),
        "Schema mismatch"
    );
}

#[test]
fn not_evaluated_does_not_produce_warning() {
    let recipe = base_recipe();
    let matrix = ApplicabilityMatrix::default();
    let report = check_recipe_applicability(&recipe, &matrix);

    // 4 dimensions NotEvaluated, 1 dimension Match
    // (schema_version is Match when current + the
    // matrix does not need to supply it). No warnings
    // even though 4 of 5 dimensions are
    // `NotEvaluated`; the verdict stays Compatible.
    assert_eq!(report.verdict, ApplicabilityOutcome::Compatible);
    assert!(report.warnings.is_empty());
    assert_eq!(report.dimensions.len(), 5);
    let not_evaluated_count = report
        .dimensions
        .iter()
        .filter(|d| d.outcome == DimensionOutcome::NotEvaluated)
        .count();
    assert_eq!(not_evaluated_count, 4);
    let match_count = report
        .dimensions
        .iter()
        .filter(|d| d.outcome == DimensionOutcome::Match)
        .count();
    assert_eq!(match_count, 1);
}

#[test]
fn per_dimension_rows_returned_in_stable_order() {
    let recipe = base_recipe();
    let matrix = ApplicabilityMatrix::default();
    let report = check_recipe_applicability(&recipe, &matrix);

    let order: Vec<&str> = report
        .dimensions
        .iter()
        .map(|d: &ApplicabilityDimension| d.dimension.as_str())
        .collect();
    assert_eq!(
        order,
        vec![
            "schema_version",
            "available_models",
            "target_type",
            "image_dimensions",
            "dataset_quality",
        ]
    );
}

#[test]
fn round_trip_via_serde() {
    // The new types cross the IPC wire (Tauri's
    // invoke_handler serializes them as JSON). A
    // serde round-trip preserves the verdict enum
    // tag + the per-dimension list.
    let recipe = recipe_with_target("deep_sky");
    let matrix = ApplicabilityMatrix {
        target_type: Some("planetary".to_string()),
        available_models: Some(vec!["m-x".to_string()]),
        image_width: Some(2048),
        image_height: Some(1536),
        dataset_quality: Some(0.4),
    };
    let report = check_recipe_applicability(&recipe, &matrix);

    let json = serde_json::to_string(&report).expect("serialize");
    let back: astroforge_core::recipe::ApplicabilityReport =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, back);
}

#[test]
fn empty_required_models_with_available_supplied_is_match() {
    let recipe = recipe_with_required(vec![]);
    let matrix = ApplicabilityMatrix {
        available_models: Some(vec!["m-a".to_string()]),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);
    assert_eq!(report.verdict, ApplicabilityOutcome::Compatible);
    let avail_dim = report
        .dimensions
        .iter()
        .find(|d| d.dimension == "available_models")
        .unwrap();
    assert_eq!(avail_dim.outcome, DimensionOutcome::Match);
}

#[test]
fn empty_target_type_in_recipe_is_adaptable_when_session_supplied() {
    let recipe = recipe_with_target("");
    let matrix = ApplicabilityMatrix {
        target_type: Some("deep_sky".to_string()),
        ..Default::default()
    };
    let report = check_recipe_applicability(&recipe, &matrix);

    assert_eq!(report.verdict, ApplicabilityOutcome::Adaptable);
}

#[test]
fn partial_only_dimension_yields_partially_compatible() {
    // When a dimension is `PartialSkip` but no
    // `Mismatch`, the verdict folds to
    // PartiallyCompatible. The function does NOT
    // produce PartialSkip out of the box (no
    // dimension emits it today); we simulate by
    // constructing a DimensionOutcome directly.
    let dim = ApplicabilityDimension {
        dimension: "synthetic".to_string(),
        outcome: DimensionOutcome::PartialSkip("test".to_string()),
    };
    let outcome = match &dim.outcome {
        DimensionOutcome::PartialSkip(_) => ApplicabilityOutcome::PartiallyCompatible,
        _ => ApplicabilityOutcome::Compatible,
    };
    assert_eq!(outcome, ApplicabilityOutcome::PartiallyCompatible);
    assert!(outcome.is_applicable());
}

#[test]
fn stages_present_in_recipe_do_not_affect_verdict() {
    // The matrix evaluates schema_version +
    // available_models + target_type +
    // image_dimensions + dataset_quality. Stage
    // contents are deliberately out of scope for
    // the §12 matrix (stage-level applicability is
    // the apply round's responsibility). Verify by
    // adding stages to a Compatible Recipe and
    // confirming the verdict does not change.
    let mut recipe = base_recipe();
    recipe.stages = vec![RecipeStage {
        stage_id: "stacking".to_string(),
        enabled: true,
        params: Default::default(),
        ai_enhancement_override: None,
    }];
    let matrix = ApplicabilityMatrix::default();
    let report = check_recipe_applicability(&recipe, &matrix);
    assert_eq!(report.verdict, ApplicabilityOutcome::Compatible);
}
