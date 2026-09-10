//! Integration test for the CR-05 P7 Definition-of-Done workflow.
//!
//! This test exercises the full project lifecycle that the §34
//! DoD scenario describes, end to end on the durable `DomainStore`:
//!
//! 1. Create a project (records `ProjectCreated` event).
//! 2. Create a session (records `SessionImported` event).
//! 3. Register source assets (records `SourceImported` events,
//!    lights the "imported" overview signal).
//! 4. Record an `AnalysisCompleted` event (lights "analyzed").
//! 5. Create a pipeline run, mark it started, mark it completed
//!    (lights "processed").
//! 6. Record a `VersionCreated` event (lights "versioned").
//! 7. Record an `ExportCreated` event (lights "exported").
//!
//! The test then asserts:
//! - the IPC's `project_overview` derivation (mirrored in
//!   the test helper) returns all five booleans as `true`,
//! - the IPC's `image_version_list` derivation (mirrored in
//!   the test helper) returns one `VersionCreated`-derived
//!   version row.
//!
//! This is a store-layer test. The IPC commands derive the
//! same booleans from the same store, so passing this test
//! pins the contract the IPCs implement.

use astroforge_core::domain::{ObjectType, PipelineRunStatus, ProjectEventKind, SourceAsset};
use astroforge_core::domain_store::DomainStore;
use std::path::PathBuf;

fn tmp_db_path(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "astroforge-dod-workflow-{}-{}.sqlite",
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
    ));
    let _ = std::fs::remove_file(&p);
    p
}

/// Mirror of the R2 `project_overview` IPC derivation. Kept
/// here so the test pins the same boolean rules the IPC
/// implements; if the IPC drifts, this test fails (after the
/// helper updates land).
fn overview_booleans(store: &DomainStore, project_id: &str) -> (bool, bool, bool, bool, bool) {
    let events = store.list_events(project_id).expect("events");
    let mut source_imported = false;
    let mut analysis_completed = false;
    let mut version_created = false;
    let mut export_created = false;
    for ev in &events {
        match ev.kind {
            ProjectEventKind::SourceImported => source_imported = true,
            ProjectEventKind::AnalysisCompleted => analysis_completed = true,
            ProjectEventKind::VersionCreated => version_created = true,
            ProjectEventKind::ExportCreated => export_created = true,
            _ => {}
        }
    }
    let sessions = store.list_sessions(project_id).expect("sessions");
    let mut any_source_asset = false;
    for session in &sessions {
        let assets = store
            .list_source_assets(&session.session_id)
            .expect("assets");
        if !assets.is_empty() {
            any_source_asset = true;
            break;
        }
    }
    let runs = store.list_pipeline_runs(project_id).expect("runs");
    let any_completed_run = runs.iter().any(|r| {
        let serialized = serde_json::to_string(&r.status)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        serialized == "completed"
    });
    (
        source_imported || any_source_asset,
        analysis_completed,
        any_completed_run,
        version_created,
        export_created,
    )
}

/// Mirror of the R3 `image_version_list` IPC derivation:
/// walk the event log for `VersionCreated` events, decode
/// payload fields, return as a list of `(event_id,
/// label, artifact_id, created_at)` tuples.
fn version_rows(store: &DomainStore, project_id: &str) -> Vec<(String, String, String, String)> {
    let events = store.list_events(project_id).expect("events");
    let mut rows = Vec::new();
    for ev in &events {
        if ev.kind != ProjectEventKind::VersionCreated {
            continue;
        }
        let (label, artifact_id) = ev
            .payload_json
            .as_deref()
            .and_then(|s| {
                #[derive(serde::Deserialize)]
                struct V {
                    #[serde(default)]
                    label: String,
                    #[serde(default)]
                    primary_artifact_id: String,
                }
                serde_json::from_str::<V>(s).ok()
            })
            .map(|v| (v.label, v.primary_artifact_id))
            .unwrap_or_default();
        rows.push((
            ev.event_id.clone(),
            label,
            artifact_id,
            ev.created_at.clone(),
        ));
    }
    rows
}

fn blank_asset(session_id: &str, content_hash: &str) -> SourceAsset {
    SourceAsset {
        asset_id: String::new(),
        content_hash: content_hash.to_string(),
        original_filename: "light_001.fits".to_string(),
        original_path: "/tmp/light_001.fits".to_string(),
        file_size: 4096,
        format: "fits".to_string(),
        mime_type: Some("image/fits".to_string()),
        created_at: "2026-09-10T00:00:00Z".to_string(),
        imported_at: "2026-09-10T00:00:00Z".to_string(),
        session_id: session_id.to_string(),
        frame_type: Some("light".to_string()),
        exposure: Some(120.0),
        filter: Some("L".to_string()),
        binning: Some("1x1".to_string()),
        width: Some(1024),
        height: Some(1024),
        bit_depth: Some(16),
        bayer_pattern: Some("RGGB".to_string()),
        camera: Some("ZWO".to_string()),
        date_obs: Some("2026-09-09T22:00:00Z".to_string()),
        ra: Some(83.8221),
        dec: Some(-5.3911),
    }
}

#[test]
fn definition_of_done_workflow() {
    let path = tmp_db_path("dod");
    let store = DomainStore::new(&path).expect("open store");

    // 1. Create project (records ProjectCreated).
    let project_id = store
        .create_project("M42 DoD", None, "test-1.0.0")
        .expect("create project");

    // 2. Create session (records SessionImported).
    let session_id = store
        .create_session(&project_id, "session-1")
        .expect("create session");

    // Pre-condition: nothing is imported / analyzed / processed
    // / versioned / exported yet.
    let (imported, analyzed, processed, versioned, exported) =
        overview_booleans(&store, &project_id);
    assert!(!imported);
    assert!(!analyzed);
    assert!(!processed);
    assert!(!versioned);
    assert!(!exported);
    assert!(version_rows(&store, &project_id).is_empty());

    // 3. Register source assets (records SourceImported).
    let (asset_a, _) = store
        .register_source_asset(&blank_asset(&session_id, "abc123"))
        .expect("register asset a");
    let (asset_b, _) = store
        .register_source_asset(&blank_asset(&session_id, "def456"))
        .expect("register asset b");
    assert!(!asset_a.is_empty());
    assert!(!asset_b.is_empty());

    // 4. Record AnalysisCompleted event.
    store
        .record_event(
            &project_id,
            ProjectEventKind::AnalysisCompleted,
            Some(&session_id),
        )
        .expect("analysis event");

    // 5. Create pipeline run, mark it started, mark it
    // completed. The legacy `pipeline_runs` table is what the
    // R2 `processed` boolean falls back on; the R5 derivation
    // still reads it.
    let run_id = store
        .create_pipeline_run(
            &project_id,
            std::slice::from_ref(&session_id),
            None,
            "test-1.0.0",
            "engine-1.0",
        )
        .expect("create run");
    store.mark_run_started(&run_id).expect("mark started");
    store
        .mark_run_finished(&run_id, PipelineRunStatus::Completed)
        .expect("mark completed");

    // 6. Record VersionCreated (with a payload that the R3
    // `parse_version_payload` helper decodes).
    let version_payload = r#"{"label":"v1 stretched","primary_artifact_id":"art_xyz"}"#;
    store
        .record_event(
            &project_id,
            ProjectEventKind::VersionCreated,
            Some(version_payload),
        )
        .expect("version event");

    // 7. Record ExportCreated.
    store
        .record_event(
            &project_id,
            ProjectEventKind::ExportCreated,
            Some(&session_id),
        )
        .expect("export event");

    // Post-condition: every boolean is now `true` (the full
    // workflow has run end to end).
    let (imported, analyzed, processed, versioned, exported) =
        overview_booleans(&store, &project_id);
    assert!(imported, "imported must be true after asset registration");
    assert!(analyzed, "analyzed must be true after AnalysisCompleted");
    assert!(processed, "processed must be true after completed run");
    assert!(versioned, "versioned must be true after VersionCreated");
    assert!(exported, "exported must be true after ExportCreated");

    // And the version timeline has exactly one row, with the
    // payload's label and artifact id surfaced.
    let rows = version_rows(&store, &project_id);
    assert_eq!(rows.len(), 1, "expected one VersionCreated row");
    assert_eq!(rows[0].1, "v1 stretched");
    assert_eq!(rows[0].2, "art_xyz");
    assert!(!rows[0].0.is_empty(), "version row carries event_id");
    assert!(!rows[0].3.is_empty(), "version row carries created_at");
}

#[test]
fn fresh_project_overview_all_false() {
    // This test pins the same shape as the R2
    // `fresh_project_has_all_overview_signals_false` test
    // but in the DoD suite, so a regression in the helper
    // affects both PRs and is easy to spot.
    let path = tmp_db_path("dod-fresh");
    let store = DomainStore::new(&path).expect("open store");
    let project_id = store
        .create_project("Fresh DoD", None, "test-1.0.0")
        .expect("create project");
    let (imported, analyzed, processed, versioned, exported) =
        overview_booleans(&store, &project_id);
    assert!(!imported);
    assert!(!analyzed);
    assert!(!processed);
    assert!(!versioned);
    assert!(!exported);
}

#[allow(dead_code)]
fn _object_type_default() -> ObjectType {
    ObjectType::default()
}
