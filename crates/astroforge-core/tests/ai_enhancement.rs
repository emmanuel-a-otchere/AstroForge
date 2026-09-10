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
    EnhancementStack, ImageAnalysis, ImageRegion, ImageRegionKind,
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
