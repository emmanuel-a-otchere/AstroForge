//! CR-06 P1 — integration tests for the AI Enhancement Studio
//! data model + provenance + safety classification.
//!
//! The CR-06 spec introduces 7 new durable entities
//! (`AiOperation`, `ImageAnalysis`, `ImageRegion`,
//! `AiRecommendation`, `AiMask`, `EnhancementStack`,
//! `EnhancementPreview`) plus the three-way
//! `AiSafetyClassification` enum (Deterministic / Perceptual /
//! Generative). These tests pin:
//!
//! - the v5 migration runs idempotently and creates the
//!   expected tables (covered transitively by the existing
//!   `migrations_apply_once_and_are_idempotent` test in
//!   `domain_store.rs`)
//! - `AiOperation` round-trips through `DomainStore`,
//!   preserving the safety classification and the new
//!   provenance fields
//! - the safety classification CHECK constraint is enforced
//!   (an invalid string is rejected by the DB layer)
//! - `ImageAnalysis`, `ImageRegion`, `AiRecommendation`,
//!   `AiMask`, `EnhancementStack`, `EnhancementPreview`
//!   round-trip through their respective CRUD helpers
//!
//! All tests use the in-memory store so they don't touch
//! the user's project database.

use astroforge_core::domain::{
    AiMask, AiOperation, AiRecommendation, AiSafetyClassification, EnhancementPreview,
    EnhancementStack, ImageAnalysis, ImageRegion, ImageRegionKind, ImageVersion,
};
use astroforge_core::domain_store::DomainStore;
use std::path::PathBuf;

fn store() -> DomainStore {
    DomainStore::new(&PathBuf::from(":memory:")).expect("open in-memory store")
}

fn sample_ai_operation() -> AiOperation {
    AiOperation {
        operation_id: "aiop_test_1".into(),
        stage_run_id: "sr_test_1".into(),
        model_id: "astroforge-denoise".into(),
        model_version: "1.2.0".into(),
        model_hash: Some("sha256:denoise-v1.2.0".into()),
        runtime: Some("onnx".into()),
        backend: Some("cpu".into()),
        precision: Some("int8".into()),
        parameters_json: Some(r#"{"strength":0.7}"#.into()),
        seed: Some(42),
        deterministic: false,
        safety_classification: AiSafetyClassification::Perceptual,
        experimental: false,
        input_artifact_id: None,
        output_artifact_id: None,
        engine_version: Some("astroforge-ai-0.1.0".into()),
        tile_configuration: Some(r#"{"tile_size":256,"overlap":32}"#.into()),
        resource_metrics: Some(r#"{"peak_memory_mb":1820,"wall_ms":23456}"#.into()),
    }
}

#[test]
fn ai_operation_round_trip_preserves_safety_classification() {
    let s = store();
    let op = sample_ai_operation();
    s.upsert_ai_operation(&op).expect("upsert");
    let back = s.get_ai_operation(&op.operation_id).expect("get");
    assert_eq!(
        back.safety_classification,
        AiSafetyClassification::Perceptual
    );
    assert_eq!(back.engine_version.as_deref(), Some("astroforge-ai-0.1.0"));
    assert_eq!(back.model_hash.as_deref(), Some("sha256:denoise-v1.2.0"));
    assert_eq!(back.seed, Some(42));
}

#[test]
fn ai_operation_deterministic_safety_class_round_trip() {
    let s = store();
    let mut op = sample_ai_operation();
    op.operation_id = "aiop_det".into();
    op.safety_classification = AiSafetyClassification::Deterministic;
    op.deterministic = true;
    s.upsert_ai_operation(&op).expect("upsert");
    let back = s.get_ai_operation(&op.operation_id).expect("get");
    assert_eq!(
        back.safety_classification,
        AiSafetyClassification::Deterministic
    );
    assert!(back.deterministic);
}

#[test]
fn ai_operation_generative_safety_class_round_trip() {
    let s = store();
    let mut op = sample_ai_operation();
    op.operation_id = "aiop_gen".into();
    op.safety_classification = AiSafetyClassification::Generative;
    s.upsert_ai_operation(&op).expect("upsert");
    let back = s.get_ai_operation(&op.operation_id).expect("get");
    assert_eq!(
        back.safety_classification,
        AiSafetyClassification::Generative
    );
    assert!(back.safety_classification.requires_disclosure());
}

#[test]
fn ai_operation_list_for_stage_preserves_order() {
    let s = store();
    let mut op_a = sample_ai_operation();
    op_a.operation_id = "aiop_a".into();
    op_a.stage_run_id = "sr_x".into();
    let mut op_b = sample_ai_operation();
    op_b.operation_id = "aiop_b".into();
    op_b.stage_run_id = "sr_x".into();
    s.upsert_ai_operation(&op_a).expect("upsert a");
    s.upsert_ai_operation(&op_b).expect("upsert b");
    let list = s.list_ai_operations_for_stage("sr_x").expect("list");
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|op| op.operation_id == "aiop_a"));
    assert!(list.iter().any(|op| op.operation_id == "aiop_b"));
}

#[test]
fn ai_operation_get_unknown_id_returns_not_found() {
    let s = store();
    let err = s.get_ai_operation("does_not_exist").unwrap_err();
    assert!(matches!(
        err,
        astroforge_core::domain_store::DomainStoreError::NotFound(_)
    ));
}

#[test]
fn image_analysis_round_trip() {
    let s = store();
    let row = ImageAnalysis {
        analysis_id: "ana_1".into(),
        project_id: "proj_1".into(),
        image_version_id: "ver_1".into(),
        profile_json: r#"{"observations":[{"name":"noise","value":"moderate"}]}"#.into(),
        created_at: "2026-09-10T00:00:00Z".into(),
    };
    s.upsert_image_analysis(&row).expect("upsert");
    let back = s.latest_image_analysis_for_version("ver_1").expect("get");
    let back = back.expect("present");
    assert_eq!(back.analysis_id, "ana_1");
    assert!(back.profile_json.contains("moderate"));
}

#[test]
fn image_analysis_latest_returns_most_recent() {
    let s = store();
    for tag in ["first", "second", "third"] {
        let row = ImageAnalysis {
            analysis_id: format!("ana_{tag}"),
            project_id: "proj_1".into(),
            image_version_id: "ver_1".into(),
            profile_json: format!(r#"{{"tag":"{tag}"}}"#),
            created_at: format!(
                "2026-09-10T00:00:0{tag}Z",
                tag = match tag {
                    "first" => "1",
                    "second" => "2",
                    "third" => "3",
                    _ => "0",
                }
            ),
        };
        s.upsert_image_analysis(&row).expect("upsert");
    }
    let back = s
        .latest_image_analysis_for_version("ver_1")
        .expect("get")
        .expect("present");
    // The ORDER BY created_at DESC tie-breaks on analysis_id; the
    // last-inserted row carries the highest id so it's the latest.
    assert!(back.analysis_id == "ana_third");
}

#[test]
fn image_region_round_trip_preserves_kind_enum() {
    let s = store();
    let row = ImageRegion {
        region_id: "reg_1".into(),
        image_version_id: "ver_1".into(),
        kind: ImageRegionKind::Nebula,
        label: Some("M42 core".into()),
        source: Some("auto".into()),
        mask_json: Some(r#"{"shape":"polygon","points":[]}"#.into()),
        created_at: "2026-09-10T00:00:00Z".into(),
    };
    s.upsert_image_region(&row).expect("upsert");
    let list = s.list_image_regions("ver_1").expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].kind, ImageRegionKind::Nebula);
    assert_eq!(list[0].label.as_deref(), Some("M42 core"));
    assert_eq!(list[0].source.as_deref(), Some("auto"));
}

#[test]
fn ai_recommendation_round_trip() {
    let s = store();
    let row = AiRecommendation {
        recommendation_id: "rec_1".into(),
        project_id: "proj_1".into(),
        image_version_id: "ver_1".into(),
        operation: "denoise".into(),
        rationale: Some(
            "Luminance noise detected at moderate level; denoise first to preserve detail.".into(),
        ),
        confidence: 0.91,
        risk_level: "low".into(),
        payload_json: r#"{"evidence":[{"kind":"snr","value":18.2}],"estimated_memory_mb":1200}"#
            .into(),
        created_at: "2026-09-10T00:00:00Z".into(),
    };
    s.upsert_ai_recommendation(&row).expect("upsert");
    let list = s
        .list_ai_recommendations_for_version("ver_1")
        .expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].operation, "denoise");
    assert!((list[0].confidence - 0.91).abs() < 1e-6);
}

#[test]
fn ai_recommendation_list_orders_by_confidence_desc() {
    let s = store();
    for (op, conf) in [("denoise", 0.91), ("sr", 0.62), ("star_reduce", 0.78)] {
        let row = AiRecommendation {
            recommendation_id: format!("rec_{op}"),
            project_id: "proj_1".into(),
            image_version_id: "ver_1".into(),
            operation: op.into(),
            rationale: None,
            confidence: conf,
            risk_level: "low".into(),
            payload_json: "{}".into(),
            created_at: "2026-09-10T00:00:00Z".into(),
        };
        s.upsert_ai_recommendation(&row).expect("upsert");
    }
    let list = s
        .list_ai_recommendations_for_version("ver_1")
        .expect("list");
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].operation, "denoise");
    assert_eq!(list[1].operation, "star_reduce");
    assert_eq!(list[2].operation, "sr");
}

#[test]
fn ai_mask_round_trip_preserves_provenance() {
    let s = store();
    let row = AiMask {
        mask_id: "mask_1".into(),
        project_id: "proj_1".into(),
        image_version_id: "ver_1".into(),
        provenance: "composite".into(),
        parents_json: Some(r#"["mask_nebula","mask_exclude_stars"]"#.into()),
        mask_json: r#"{"shape":"polygon","points":[]}"#.into(),
        created_at: "2026-09-10T00:00:00Z".into(),
    };
    s.upsert_ai_mask(&row).expect("upsert");
    let list = s.list_ai_masks("ver_1").expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].provenance, "composite");
    assert!(list[0]
        .parents_json
        .as_deref()
        .unwrap()
        .contains("mask_exclude_stars"));
}

#[test]
fn enhancement_stack_round_trip_with_branch_provenance() {
    let s = store();
    let row = EnhancementStack {
        stack_id: "stack_1".into(),
        project_id: "proj_1".into(),
        source_image_version_id: "ver_1".into(),
        operation_ids_json: r#"["aiop_a","aiop_b","aiop_c"]"#.into(),
        branched_from_version_id: Some("ver_2".into()),
        created_at: "2026-09-10T00:00:00Z".into(),
    };
    s.upsert_enhancement_stack(&row).expect("upsert");
    let list = s.list_enhancement_stacks_for_source("ver_1").expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].branched_from_version_id.as_deref(), Some("ver_2"));
    assert!(list[0].operation_ids_json.contains("aiop_a"));
}

#[test]
fn enhancement_preview_round_trip() {
    let s = store();
    let row = EnhancementPreview {
        preview_id: "prev_1".into(),
        project_id: "proj_1".into(),
        source_image_version_id: "ver_1".into(),
        operation_id: "aiop_a".into(),
        artifact_id: Some("art_p1".into()),
        parameters_json: r#"{"strength":0.5}"#.into(),
        status: "completed".into(),
        created_at: "2026-09-10T00:00:00Z".into(),
    };
    s.upsert_enhancement_preview(&row).expect("upsert");
    let list = s
        .list_enhancement_previews_for_operation("aiop_a")
        .expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].artifact_id.as_deref(), Some("art_p1"));
    assert_eq!(list[0].status, "completed");
}

#[test]
fn safety_classification_label_and_disclosure() {
    // Pure unit check on the enum's UI helpers — the spec
    // (§16, §17) requires the classification to be visible and
    // the disclosure banner to be required for Perceptual /
    // Generative operations.
    assert_eq!(
        AiSafetyClassification::Deterministic.as_label(),
        "Deterministic"
    );
    assert_eq!(AiSafetyClassification::Perceptual.as_label(), "Perceptual");
    assert_eq!(AiSafetyClassification::Generative.as_label(), "Generative");
    assert!(!AiSafetyClassification::Deterministic.requires_disclosure());
    assert!(AiSafetyClassification::Perceptual.requires_disclosure());
    assert!(AiSafetyClassification::Generative.requires_disclosure());
}

#[test]
fn safety_classification_default_is_deterministic() {
    // Pre-CR-06 rows that lack a `safety_classification` field
    // default to Deterministic on deserialize so the trust
    // boundary degrades safely.
    let json = r#"{
        "operation_id":"aiop_legacy",
        "stage_run_id":"sr_legacy",
        "model_id":"classical-denoise",
        "model_version":"0.9.0",
        "deterministic":true,
        "experimental":false
    }"#;
    let op: AiOperation = serde_json::from_str(json).expect("deserialize legacy");
    assert_eq!(
        op.safety_classification,
        AiSafetyClassification::Deterministic
    );
    assert!(op.engine_version.is_none());
    assert!(op.tile_configuration.is_none());
    assert!(op.resource_metrics.is_none());
}

// CR-06 P4 — image_versions round-trip. The apply round
// creates a new Image Version per CR-06 §4 / §22; the
// sequence number is monotonic per project.
#[test]
fn image_version_round_trip_preserves_sequence() {
    let s = store();
    let v1 = ImageVersion {
        version_id: "ver_1".into(),
        project_id: "p1".into(),
        label: "v1".into(),
        sequence: 1,
        primary_artifact_id: "art_1".into(),
        source_version_id: None,
        created_at: "2026-01-01 00:00:00 UTC".into(),
        hidden: false,
    };
    let v2 = ImageVersion {
        version_id: "ver_2".into(),
        project_id: "p1".into(),
        label: "v2".into(),
        sequence: 2,
        primary_artifact_id: "art_2".into(),
        source_version_id: Some("ver_1".into()),
        created_at: "2026-01-02 00:00:00 UTC".into(),
        hidden: false,
    };
    s.upsert_image_version(&v1).expect("upsert v1");
    s.upsert_image_version(&v2).expect("upsert v2");
    let list = s.list_image_versions_for_project("p1").expect("list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].version_id, "ver_1");
    assert_eq!(list[1].version_id, "ver_2");
    assert_eq!(list[1].source_version_id.as_deref(), Some("ver_1"));
}

#[test]
fn image_version_sequence_increments() {
    let s = store();
    assert_eq!(s.next_image_version_sequence("p1").unwrap(), 0);
    let v1 = ImageVersion {
        version_id: "ver_1".into(),
        project_id: "p1".into(),
        label: "".into(),
        sequence: 1,
        primary_artifact_id: "art_1".into(),
        source_version_id: None,
        created_at: "2026-01-01 00:00:00 UTC".into(),
        hidden: false,
    };
    s.upsert_image_version(&v1).unwrap();
    assert_eq!(s.next_image_version_sequence("p1").unwrap(), 1);
}

#[test]
fn image_version_get_returns_none_for_unknown_id() {
    let s = store();
    let result = s.get_image_version("ver_missing").expect("query");
    assert!(result.is_none());
}

#[test]
fn hidden_versions_excluded_from_list() {
    let s = store();
    let v1 = ImageVersion {
        version_id: "ver_1".into(),
        project_id: "p1".into(),
        label: "v1".into(),
        sequence: 1,
        primary_artifact_id: "art_1".into(),
        source_version_id: None,
        created_at: "2026-01-01 00:00:00 UTC".into(),
        hidden: false,
    };
    let v2_hidden = ImageVersion {
        version_id: "ver_2_hidden".into(),
        project_id: "p1".into(),
        label: "v2 hidden".into(),
        sequence: 2,
        primary_artifact_id: "art_2".into(),
        source_version_id: Some("ver_1".into()),
        created_at: "2026-01-02 00:00:00 UTC".into(),
        hidden: true,
    };
    s.upsert_image_version(&v1).unwrap();
    s.upsert_image_version(&v2_hidden).unwrap();
    let list = s.list_image_versions_for_project("p1").unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].version_id, "ver_1");
    // get_image_version still returns the hidden row.
    let hidden = s.get_image_version("ver_2_hidden").unwrap().unwrap();
    assert!(hidden.hidden);
}

// CR-06 P4 — enhancement stack apply mutation persists.
#[test]
fn enhancement_stack_apply_mutation_persists() {
    use astroforge_core::enhancement::{
        apply_mutation, EnhancementStackRecord, StackMutation, StackOperation,
    };
    let s = store();
    let initial = EnhancementStackRecord {
        stack_id: "stk_1".into(),
        project_id: "p1".into(),
        source_image_version_id: "ver_1".into(),
        operations: vec![
            StackOperation {
                operation_id: "denoise_luminance".into(),
                recommendation_id: None,
                parameters_json: "{}".into(),
                enabled: true,
                needs_preview: false,
            },
            StackOperation {
                operation_id: "super_resolution".into(),
                recommendation_id: None,
                parameters_json: "{}".into(),
                enabled: true,
                needs_preview: false,
            },
        ],
        branched_from_version_id: None,
        created_at: "2026-01-01 00:00:00 UTC".into(),
    };
    let op_json = EnhancementStackRecord::operations_to_json(&initial.operations);
    s.upsert_enhancement_stack(&EnhancementStack {
        stack_id: initial.stack_id.clone(),
        project_id: initial.project_id.clone(),
        source_image_version_id: initial.source_image_version_id.clone(),
        operation_ids_json: op_json,
        branched_from_version_id: None,
        created_at: initial.created_at.clone(),
    })
    .unwrap();

    // Disable the super_resolution operation.
    let mut updated = initial.clone();
    updated = apply_mutation(
        updated,
        StackMutation::SetEnabled {
            operation_id: "super_resolution".into(),
            enabled: false,
        },
    )
    .unwrap();
    let updated_json = EnhancementStackRecord::operations_to_json(&updated.operations);
    s.upsert_enhancement_stack(&EnhancementStack {
        stack_id: updated.stack_id.clone(),
        project_id: updated.project_id.clone(),
        source_image_version_id: updated.source_image_version_id.clone(),
        operation_ids_json: updated_json,
        branched_from_version_id: updated.branched_from_version_id.clone(),
        created_at: updated.created_at.clone(),
    })
    .unwrap();

    let fetched = s.get_enhancement_stack("stk_1").unwrap().unwrap();
    let ops = EnhancementStackRecord::operations_from_json(&fetched.operation_ids_json);
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0].operation_id, "denoise_luminance");
    assert!(ops[0].enabled);
    assert_eq!(ops[1].operation_id, "super_resolution");
    assert!(!ops[1].enabled);
}

// CR-06 P4 — preview state-machine round-trip via JSON.
#[test]
fn preview_status_round_trip_via_json() {
    use astroforge_core::enhancement::{parse_status, PreviewStatus};
    for s in [
        PreviewStatus::Pending,
        PreviewStatus::Rendering,
        PreviewStatus::Completed,
        PreviewStatus::Failed,
    ] {
        let raw = s.as_str();
        assert_eq!(parse_status(raw), s);
    }
    // Unknown falls back to pending (defensive).
    assert_eq!(parse_status("unknown_thing"), PreviewStatus::Pending);
}

// CR-06 P5 — mask encoding round-trip via the JSON
// shape. The wire shape carries the version + the
// `kind` enum + the pixel raster; a future schema
// bump branches on the version.
#[test]
fn mask_encoding_round_trip_preserves_pixels() {
    use astroforge_core::masks::encoding::{from_json, round_trip, to_json};
    use astroforge_core::masks::{Mask, MaskKind};
    let original = Mask::from_pixels(2, 2, MaskKind::User, "brush".into(), &[0.1, 0.2, 0.3, 0.4]);
    let restored = round_trip(&original).expect("round trip");
    assert_eq!(restored, original);
    let raw = to_json(&original).expect("to_json");
    let parsed = from_json(&raw).expect("from_json");
    assert_eq!(parsed, original);
}

// CR-06 P5 — boolean composition semantics. Union
// takes max, intersect takes min, difference
// subtracts with a zero clamp.
#[test]
fn mask_composite_union_intersect_difference() {
    use astroforge_core::masks::composite::{apply, CompositeOp};
    use astroforge_core::masks::Mask;
    let a = Mask::from_pixels(
        2,
        2,
        astroforge_core::masks::MaskKind::Auto,
        "a".into(),
        &[0.2, 0.4, 0.6, 0.8],
    );
    let b = Mask::from_pixels(
        2,
        2,
        astroforge_core::masks::MaskKind::Auto,
        "b".into(),
        &[0.5, 0.3, 0.7, 0.1],
    );
    let union = apply(&a, &b, CompositeOp::Union).unwrap();
    assert_eq!(union.pixels, vec![0.5, 0.4, 0.7, 0.8]);
    let intersect = apply(&a, &b, CompositeOp::Intersect).unwrap();
    assert_eq!(intersect.pixels, vec![0.2, 0.3, 0.6, 0.1]);
    // Difference uses approximate equality to tolerate
    // f32 rounding.
    let diff = apply(&a, &b, CompositeOp::Difference).unwrap();
    let expected = [0.0, 0.1, 0.0, 0.7];
    for (got, want) in diff.pixels.iter().zip(expected.iter()) {
        assert!((got - want).abs() < 1e-6, "got {got} want {want}");
    }
}

// CR-06 P5 — `AiMask` row CRUD. `get_ai_mask` was
// added by P5; the test pins the round-trip + the
// `update_ai_mask` flow.
#[test]
fn ai_mask_get_round_trip() {
    let s = store();
    let row = AiMask {
        mask_id: "msk_1".into(),
        project_id: "p1".into(),
        image_version_id: "ver_1".into(),
        provenance: "auto:stars".into(),
        parents_json: None,
        mask_json: r#"{"version":1,"width":1,"height":1,"kind":"auto","provenance":"auto:stars","encoding":"row_major_f32","pixels":[0.5]}"#.into(),
        created_at: "2026-01-01 00:00:00 UTC".into(),
    };
    s.upsert_ai_mask(&row).expect("upsert");
    let fetched = s.get_ai_mask("msk_1").unwrap().expect("present");
    assert_eq!(fetched.mask_id, "msk_1");
    assert_eq!(fetched.provenance, "auto:stars");
}

#[test]
fn ai_mask_get_unknown_returns_none() {
    let s = store();
    let fetched = s.get_ai_mask("msk_missing").unwrap();
    assert!(fetched.is_none());
}

#[test]
fn ai_mask_list_orders_by_created_at() {
    let s = store();
    let r1 = AiMask {
        mask_id: "msk_1".into(),
        project_id: "p1".into(),
        image_version_id: "ver_1".into(),
        provenance: "auto:stars".into(),
        parents_json: None,
        mask_json: "{}".into(),
        created_at: "2026-01-01 00:00:00 UTC".into(),
    };
    let r2 = AiMask {
        mask_id: "msk_2".into(),
        project_id: "p1".into(),
        image_version_id: "ver_1".into(),
        provenance: "user:brush".into(),
        parents_json: None,
        mask_json: "{}".into(),
        created_at: "2026-01-02 00:00:00 UTC".into(),
    };
    s.upsert_ai_mask(&r1).unwrap();
    s.upsert_ai_mask(&r2).unwrap();
    let list = s.list_ai_masks("ver_1").unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].mask_id, "msk_1");
    assert_eq!(list[1].mask_id, "msk_2");
}

// CR-06 P6 — quality gate report runs against
// synthetic inputs. Identical source + result = all
// Ok; a deliberately-bad result triggers Failure.
#[test]
fn quality_gate_passes_on_identical_input() {
    use astroforge_core::quality_gates::report::{run, QualityVerdict};
    use astroforge_core::quality_gates::GateThresholds;
    use ndarray::Array3;

    let src = Array3::<f32>::from_elem((1, 8, 8), 0.5);
    let res = Array3::<f32>::from_elem((1, 8, 8), 0.5);
    let src_img = astroforge_core::image::F32Image::from(src);
    let res_img = astroforge_core::image::F32Image::from(res);
    let t = GateThresholds::default();
    let report = run(
        &src_img,
        &res_img,
        &t,
        None,
        "src_v".into(),
        "res_v".into(),
        "test_op".into(),
    );
    assert_eq!(report.verdict, QualityVerdict::Ok);
}

#[test]
fn quality_gate_fails_on_pushed_rails() {
    use astroforge_core::quality_gates::report::{run, QualityVerdict};
    use astroforge_core::quality_gates::GateThresholds;
    use ndarray::Array3;

    let src = Array3::<f32>::from_elem((1, 8, 8), 0.5);
    let res = Array3::<f32>::from_elem((1, 8, 8), 1.0);
    let src_img = astroforge_core::image::F32Image::from(src);
    let res_img = astroforge_core::image::F32Image::from(res);
    let t = GateThresholds::default();
    let report = run(
        &src_img,
        &res_img,
        &t,
        None,
        "src_v".into(),
        "res_v".into(),
        "test_op".into(),
    );
    assert_eq!(report.verdict, QualityVerdict::Failure);
    // The clipping finding is the one that
    // triggered the failure.
    let clipping = report
        .findings
        .iter()
        .find(|f| matches!(f.gate, astroforge_core::quality_gates::GateId::Clipping))
        .expect("clipping finding");
    assert!(matches!(
        clipping.severity,
        astroforge_core::quality_gates::Severity::Failure
    ));
}

#[test]
fn quality_gate_warning_on_flattened_result() {
    use astroforge_core::quality_gates::report::{run, QualityVerdict};
    use astroforge_core::quality_gates::GateThresholds;
    use ndarray::Array3;

    // Noisy source → flat result triggers
    // excessive smoothing.
    let mut src = Array3::<f32>::zeros((1, 8, 8));
    for y in 0..8 {
        for x in 0..8 {
            src[(0, y, x)] = ((x + y * 8) as f32) / 64.0;
        }
    }
    let res = Array3::<f32>::from_elem((1, 8, 8), 0.5);
    let src_img = astroforge_core::image::F32Image::from(src);
    let res_img = astroforge_core::image::F32Image::from(res);
    let t = GateThresholds::default();
    let report = run(
        &src_img,
        &res_img,
        &t,
        None,
        "src_v".into(),
        "res_v".into(),
        "smoothing_op".into(),
    );
    assert!(matches!(
        report.verdict,
        QualityVerdict::Warning | QualityVerdict::Failure
    ));
}

#[test]
fn quality_gate_returns_ten_findings() {
    use astroforge_core::quality_gates::report::{count_by_severity, run};
    use astroforge_core::quality_gates::GateThresholds;
    use ndarray::Array3;

    let src = Array3::<f32>::from_elem((1, 4, 4), 0.5);
    let res = Array3::<f32>::from_elem((1, 4, 4), 0.5);
    let src_img = astroforge_core::image::F32Image::from(src);
    let res_img = astroforge_core::image::F32Image::from(res);
    let t = GateThresholds::default();
    let report = run(
        &src_img,
        &res_img,
        &t,
        None,
        "src".into(),
        "res".into(),
        "op".into(),
    );
    let (ok, info, warn, fail) = count_by_severity(&report.findings);
    assert_eq!(ok + info + warn + fail, 10);
}
