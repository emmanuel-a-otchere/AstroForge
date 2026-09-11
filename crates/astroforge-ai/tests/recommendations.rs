//! CR-06 P3 — integration tests for the AI recommendation
//! engine.
//!
//! These tests exercise the full path that the
//! `generate_ai_recommendations` Tauri command walks:
//!
//! 1. Build an `ImageAnalysisReport` (the P2 output).
//! 2. Run `recommendations::analyze_at` with a fixed
//!    `created_at` and `engine_version` for determinism.
//! 3. Assert the engine produced the expected list of
//!    recommendations (operations, risk levels,
//!    classifications).
//! 4. Round-trip through `flatten_for_store` and verify
//!    each persisted row's `payload_json` deserialises
//!    cleanly into the same `AiRecommendation`.
//!
//! The integration tests run on the `astroforge-ai`
//! crate directly (which `cargo test --workspace` picks
//! up); the Tauri command layer is tested via the same
//! path through the `commands_ai_enhancement` module
//! separately.

use astroforge_ai::recommendations::{
    analyze_at, flatten_for_store, AiRecommendation, RecommendationReport, SequencingNote,
};
use astroforge_core::image_analysis::metrics::Confidence;
use astroforge_core::image_analysis::report::{ImageAnalysisReport, Observation};

fn base_report() -> ImageAnalysisReport {
    ImageAnalysisReport {
        image_version_id: "v42".into(),
        width: 1024,
        height: 1024,
        channels: 3,
        observations: vec![],
        star_count: 0,
        faint_structure_count: 0,
        has_bright_core: false,
        hot_pixel_count: 0,
        trail_artifact_count: 0,
        engine_version: "test".into(),
        created_at: "2026-01-01 00:00:00 UTC".into(),
    }
}

fn push(report: &mut ImageAnalysisReport, name: &str, label: &str, value: f64, conf: Confidence) {
    report.observations.push(Observation {
        name: name.into(),
        label: label.into(),
        value,
        evidence_region: None,
        confidence: conf,
    });
}

#[test]
fn realistic_clean_image_produces_minimal_recommendations() {
    // Mild luminance noise + a few hot pixels. No
    // gradients, no faint structures, no trails.
    let mut r = base_report();
    push(&mut r, "luminance_noise", "low", 0.01, Confidence::High);
    push(&mut r, "hot_pixels", "present", 5.0, Confidence::High);
    r.hot_pixel_count = 5;
    let out = analyze_at(&r, "p1", "2026-01-01 00:00:00 UTC", "0.0.0");
    let ops: Vec<&str> = out
        .recommendations
        .iter()
        .map(|r| r.operation.as_str())
        .collect();
    // Hot pixel cleanup should fire; denoise should NOT
    // fire (noise is "low").
    assert!(ops.contains(&"hot_pixel_clean"), "ops: {:?}", ops);
    assert!(
        !ops.iter().any(|op| op.starts_with("denoise")),
        "ops: {:?}",
        ops
    );
    assert!(!ops.contains(&"detail_enhance"), "ops: {:?}", ops);
}

#[test]
fn noisy_image_with_trails_produces_full_stack() {
    // Moderate noise + a few faint structures + trails +
    // background gradient + many stars. Every rule
    // should fire; ordering should move detail /
    // super-resolution / deconv to their §23 slots.
    let mut r = base_report();
    r.star_count = 60;
    r.faint_structure_count = 8;
    r.trail_artifact_count = 2;
    push(&mut r, "luminance_noise", "high", 0.1, Confidence::High);
    push(
        &mut r,
        "chromatic_noise",
        "moderate",
        0.05,
        Confidence::Medium,
    );
    push(
        &mut r,
        "background_gradient",
        "moderate",
        0.04,
        Confidence::High,
    );
    push(&mut r, "local_contrast", "moderate", 0.5, Confidence::High);
    push(
        &mut r,
        "trail_artifacts",
        "moderate",
        2.0,
        Confidence::Medium,
    );
    let out = analyze_at(&r, "p2", "2026-01-01 00:00:00 UTC", "0.0.0");

    let ops: Vec<&str> = out
        .recommendations
        .iter()
        .map(|r| r.operation.as_str())
        .collect();
    assert!(ops.contains(&"background_cleanup"), "ops: {:?}", ops);
    assert!(ops.contains(&"trail_clean"), "ops: {:?}", ops);
    assert!(ops.contains(&"denoise_luminance"), "ops: {:?}", ops);
    assert!(ops.contains(&"denoise_chrominance"), "ops: {:?}", ops);
    assert!(ops.contains(&"detail_enhance"), "ops: {:?}", ops);
    assert!(ops.contains(&"super_resolution"), "ops: {:?}", ops);
    assert!(ops.contains(&"deconv"), "ops: {:?}", ops);

    // The final order: background_cleanup first,
    // super_resolution last, denoise before detail.
    let positions: std::collections::HashMap<&str, usize> = out
        .recommendations
        .iter()
        .enumerate()
        .map(|(i, r)| (r.operation.as_str(), i))
        .collect();
    assert!(
        positions["background_cleanup"] < positions["denoise_luminance"],
        "background must come before denoise: {:?}",
        positions
    );
    assert!(
        positions["trail_clean"] < positions["denoise_luminance"],
        "trail cleanup must come before denoise: {:?}",
        positions
    );
    assert!(
        positions["denoise_luminance"] < positions["detail_enhance"],
        "denoise must come before detail: {:?}",
        positions
    );
    assert!(
        positions["denoise_luminance"] < positions["deconv"],
        "denoise must come before deconv: {:?}",
        positions
    );
    assert_eq!(
        out.recommendations.last().unwrap().operation,
        "super_resolution",
        "super_resolution must be last"
    );
}

#[test]
fn trail_recommendation_is_classified_as_generative() {
    let mut r = base_report();
    r.trail_artifact_count = 2;
    push(
        &mut r,
        "trail_artifacts",
        "moderate",
        2.0,
        Confidence::Medium,
    );
    let out = analyze_at(&r, "p", "2026-01-01 00:00:00 UTC", "0.0.0");
    let trail = out
        .recommendations
        .iter()
        .find(|r| r.operation == "trail_clean")
        .expect("trail_clean should be present");
    assert_eq!(trail.classification, "generative");
    assert_eq!(trail.risk_level, "high");
}

#[test]
fn flatten_for_store_round_trips_payload_json() {
    let mut r = base_report();
    push(
        &mut r,
        "luminance_noise",
        "moderate",
        0.05,
        Confidence::High,
    );
    let out = analyze_at(&r, "p", "2026-01-01 00:00:00 UTC", "0.0.0");
    let rows = flatten_for_store(&out);
    assert_eq!(rows.len(), out.recommendations.len());
    for (row, rec) in rows.iter().zip(out.recommendations.iter()) {
        assert_eq!(row.project_id, "p");
        assert_eq!(row.image_version_id, "v42");
        assert_eq!(row.operation, rec.operation);
        let parsed: AiRecommendation =
            serde_json::from_str(&row.payload_json).expect("payload_json must round-trip");
        assert_eq!(parsed.operation, rec.operation);
        assert_eq!(parsed.recommended_model, rec.recommended_model);
    }
}

#[test]
fn sequencing_notes_record_reordering_with_rationale() {
    let mut r = base_report();
    // Just enough signal to trigger denoise + detail +
    // deconv + star_refine, which will get reordered.
    r.faint_structure_count = 5;
    r.star_count = 30;
    push(&mut r, "luminance_noise", "high", 0.1, Confidence::High);
    push(&mut r, "local_contrast", "moderate", 0.5, Confidence::High);
    let out = analyze_at(&r, "p", "2026-01-01 00:00:00 UTC", "0.0.0");
    // We expect at least one note explaining why
    // detail_enhance moved (and possibly deconv, star_refine).
    let moved: Vec<&str> = out
        .sequencing_notes
        .iter()
        .map(|n| n.moved.as_str())
        .collect();
    assert!(
        moved.contains(&"detail_enhance") || moved.contains(&"deconv"),
        "expected reordering notes for detail/deconv, got: {:?}",
        out.sequencing_notes
    );
    for note in &out.sequencing_notes {
        assert!(
            !note.note.is_empty(),
            "note must have rationale: {:?}",
            note
        );
        assert!(
            note.before_after.contains("denoise") || note.before_after.contains("end of stack"),
            "note should mention what it moved past: {:?}",
            note
        );
    }
}

#[test]
fn resource_estimates_are_present_for_model_backed_ops() {
    let mut r = base_report();
    r.star_count = 30;
    push(
        &mut r,
        "luminance_noise",
        "moderate",
        0.05,
        Confidence::High,
    );
    push(&mut r, "local_contrast", "moderate", 0.5, Confidence::High);
    let out = analyze_at(&r, "p", "2026-01-01 00:00:00 UTC", "0.0.0");
    // denoise_luminance + super_resolution are both
    // model-backed, so both should carry estimates.
    for op in ["denoise_luminance", "super_resolution"] {
        let rec = out
            .recommendations
            .iter()
            .find(|r| r.operation == op)
            .unwrap_or_else(|| panic!("{op} should be present"));
        assert!(rec.estimated_runtime.is_some(), "{op} missing runtime");
        assert!(rec.estimated_memory.is_some(), "{op} missing memory");
        assert!(rec.recommended_model.is_some(), "{op} missing model");
    }
}

#[test]
fn deterministic_recommendation_ids_are_stable() {
    let mut r = base_report();
    r.star_count = 30;
    push(
        &mut r,
        "luminance_noise",
        "moderate",
        0.05,
        Confidence::High,
    );
    push(&mut r, "local_contrast", "moderate", 0.5, Confidence::High);
    let out = analyze_at(&r, "p", "2026-01-01 00:00:00 UTC", "0.0.0");
    let rows1 = flatten_for_store(&out);
    let rows2 = flatten_for_store(&out);
    let ids1: Vec<&str> = rows1.iter().map(|r| r.recommendation_id.as_str()).collect();
    let ids2: Vec<&str> = rows2.iter().map(|r| r.recommendation_id.as_str()).collect();
    assert_eq!(ids1, ids2, "recommendation_id should be deterministic");
    // The id encodes image_version_id + operation.
    for r in &rows1 {
        assert!(
            r.recommendation_id.starts_with("rec_v42_"),
            "id should encode image_version_id: {}",
            r.recommendation_id
        );
    }
}

#[test]
fn sequencing_note_serializes_to_json() {
    let note = SequencingNote {
        moved: "detail_enhance".into(),
        before_after: "after denoise_luminance".into(),
        note: "Detail enhancement would amplify noise.".into(),
    };
    let json = serde_json::to_string(&note).expect("serialize");
    let parsed: SequencingNote = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed, note);
}

#[test]
fn recommendation_report_round_trips_via_json() {
    let mut r = base_report();
    push(
        &mut r,
        "luminance_noise",
        "moderate",
        0.05,
        Confidence::High,
    );
    let original: RecommendationReport = analyze_at(&r, "p", "2026-01-01 00:00:00 UTC", "0.0.0");
    let json = original.to_json().expect("serialize");
    let parsed = RecommendationReport::from_json(&json).expect("deserialize");
    assert_eq!(parsed, original);
}
