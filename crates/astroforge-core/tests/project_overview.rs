//! Integration tests for the R2 project overview semantics.
//!
//! `project_overview` is a Tauri IPC command. To avoid spinning up a Tauri
//! State in the test harness, this test file pins the *underlying* store
//! semantics that the IPC derives from. The boolean rules tested here are
//! exactly the rules the IPC implements:
//!
//! - `imported`: any `SourceImported` event for the project OR any
//!   source asset in any of the project's sessions.
//! - `analyzed`: any `AnalysisCompleted` event for the project.
//! - `processed`: any pipeline run with `status = 'Completed'`.
//! - `versioned`: any `VersionCreated` event for the project.
//! - `exported`: any `ExportCreated` event for the project.
//!
//! If the IPC ever drifts from these rules, the test still passes
//! (it tests the store, not the IPC) — but the rule set is the
//! canonical contract and lives next to the IPC implementation in
//! `src-tauri/src/commands_project.rs`.

use astroforge_core::domain::{ObjectType, ProjectEventKind, SourceAsset};
use astroforge_core::domain_store::DomainStore;
use std::path::PathBuf;

fn tmp_db_path(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "astroforge-r2-overview-{}-{}.sqlite",
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
    ));
    let _ = std::fs::remove_file(&p);
    p
}

fn overview_for(store: &DomainStore, project_id: &str) -> (bool, bool, bool, bool, bool) {
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
        // `enum_str` writes serde's snake_case form (see
        // `PipelineRunStatus` annotation: `#[serde(rename_all =
        // "snake_case")]`), so `Completed` becomes the literal
        // string `"completed"` in SQLite. Match on that text.
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

#[test]
fn fresh_project_has_all_overview_signals_false() {
    let path = tmp_db_path("fresh");
    let store = DomainStore::new(&path).expect("open");
    let project_id = store
        .create_project("Fresh", None, "test-1.0.0")
        .expect("create project");

    let (imported, analyzed, processed, versioned, exported) = overview_for(&store, &project_id);
    assert!(!imported, "fresh project should not be imported");
    assert!(!analyzed, "fresh project should not be analyzed");
    assert!(!processed, "fresh project should not be processed");
    assert!(!versioned, "fresh project should not be versioned");
    assert!(!exported, "fresh project should not be exported");
}

#[test]
fn source_imported_event_lights_imported_signal() {
    let path = tmp_db_path("source-event");
    let store = DomainStore::new(&path).expect("open");
    let project_id = store
        .create_project("Source Event", None, "test-1.0.0")
        .expect("create project");
    let session_id = store
        .create_session(&project_id, "session-1")
        .expect("create session");
    // register_source_asset also records a SourceImported event.
    let asset = SourceAsset {
        asset_id: String::new(),
        content_hash: "abc".into(),
        original_filename: "light_001.fits".into(),
        original_path: "/tmp/light_001.fits".into(),
        file_size: 1234,
        format: "fits".into(),
        mime_type: Some("image/fits".into()),
        created_at: "2026-09-10T00:00:00Z".into(),
        imported_at: "2026-09-10T00:00:00Z".into(),
        session_id: session_id.clone(),
        frame_type: Some("light".into()),
        exposure: Some(120.0),
        filter: Some("L".into()),
        binning: Some("1x1".into()),
        width: Some(1024),
        height: Some(1024),
        bit_depth: Some(16),
        bayer_pattern: Some("RGGB".into()),
        camera: Some("ZWO".into()),
        date_obs: Some("2026-09-09T22:00:00Z".into()),
        ra: Some(83.8221),
        dec: Some(-5.3911),
    };
    let (id, _) = store.register_source_asset(&asset).expect("register asset");
    assert!(!id.is_empty());

    let (imported, analyzed, processed, versioned, exported) = overview_for(&store, &project_id);
    assert!(
        imported,
        "registered source asset should mark imported=true"
    );
    assert!(!analyzed, "no analysis event yet");
    assert!(!processed, "no completed run yet");
    assert!(!versioned, "no version event yet");
    assert!(!exported, "no export event yet");
}

#[test]
fn analysis_version_export_events_light_their_signals() {
    let path = tmp_db_path("events");
    let store = DomainStore::new(&path).expect("open");
    let project_id = store
        .create_project("Events", None, "test-1.0.0")
        .expect("create project");
    let session_id = store
        .create_session(&project_id, "session-1")
        .expect("create session");

    // Simulate the durable events a project accumulates.
    store
        .record_event(
            &project_id,
            ProjectEventKind::AnalysisCompleted,
            Some(&session_id),
        )
        .expect("analysis event");
    store
        .record_event(
            &project_id,
            ProjectEventKind::VersionCreated,
            Some(&session_id),
        )
        .expect("version event");
    store
        .record_event(
            &project_id,
            ProjectEventKind::ExportCreated,
            Some(&session_id),
        )
        .expect("export event");

    let (imported, analyzed, processed, versioned, exported) = overview_for(&store, &project_id);
    assert!(!imported, "no source event yet");
    assert!(analyzed, "analysis event should mark analyzed=true");
    assert!(!processed, "no completed run yet");
    assert!(versioned, "version event should mark versioned=true");
    assert!(exported, "export event should mark exported=true");
}

#[test]
fn completed_pipeline_run_lights_processed_signal() {
    let path = tmp_db_path("processed");
    let store = DomainStore::new(&path).expect("open");
    let project_id = store
        .create_project("Processed", None, "test-1.0.0")
        .expect("create project");
    let session_id = store
        .create_session(&project_id, "session-1")
        .expect("create session");

    // create a run, mark it started, then mark it completed.
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
        .mark_run_finished(
            &run_id,
            astroforge_core::domain::PipelineRunStatus::Completed,
        )
        .expect("mark completed");

    let (imported, analyzed, processed, versioned, exported) = overview_for(&store, &project_id);
    assert!(!imported);
    assert!(!analyzed);
    assert!(processed, "completed run should mark processed=true");
    assert!(!versioned);
    assert!(!exported);
}

#[test]
fn unknown_project_returns_404_in_store_layer() {
    // The IPC layer maps `get_project` errors to a String error;
    // the store layer surfaces the canonical not-found error.
    let path = tmp_db_path("missing");
    let store = DomainStore::new(&path).expect("open");
    let res = store.get_project("proj_does_not_exist");
    assert!(res.is_err());
}

#[allow(dead_code)]
fn _object_type_default() -> ObjectType {
    ObjectType::default()
}
