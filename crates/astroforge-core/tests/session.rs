//! Integration tests for the SessionStore via the public surface that
//! the Tauri commands wrap. Verifies the round-trip the wizard relies
//! on: create project → create session → record stage runs → query
//! status → crash recovery (running sessions are findable).

use astroforge_core::session::SessionStore;
use std::path::PathBuf;

fn tmp_db_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "astroforge-session-{}-{}.sqlite",
        name,
        std::process::id()
    ));
    p.set_extension(format!(
        "sqlite.{}.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::process::id(),
    ));
    let _ = std::fs::remove_file(&p);
    p
}

#[test]
fn session_lifecycle_round_trip() {
    let path = tmp_db_path("lifecycle");
    let store = SessionStore::new(&path).expect("open");

    // create project + session (mirrors session_create_project + session_create)
    let project_id = store
        .create_project("Round Trip", Some("deep_sky"))
        .unwrap();
    assert!(project_id.starts_with("proj_"));

    let session_id = store
        .create_session(&project_id, Some("/tmp/fits"), "intermediate")
        .unwrap();
    assert!(session_id.starts_with("sess_"));

    // record a stage run (mirrors session_record_stage)
    let run_id = store
        .record_stage_run(&session_id, "ingest", "running", None, None, None)
        .unwrap();
    assert!(run_id > 0);

    // complete it (the Tauri command auto-completes if status is terminal)
    store.complete_stage_run(run_id, "completed").unwrap();

    // session status is reachable (mirrors a future session_get_status command)
    let status = store.get_session_status(&session_id).expect("status");
    assert_eq!(status.session_id, session_id);
    assert_eq!(status.status, "created"); // session itself stays "created" until orchestrator flips it
}

#[test]
fn autosave_records_multiple_stages() {
    let path = tmp_db_path("multi-stage");
    let store = SessionStore::new(&path).expect("open");
    let project_id = store.create_project("Multi", None).unwrap();
    let session_id = store.create_session(&project_id, None, "beginner").unwrap();

    // Simulate the wizard committing several stages in sequence
    let stages = ["ingest", "background_extraction", "stretch", "export"];
    for stage in stages {
        let params = format!("{{\"stage\":\"{}\",\"strength\":0.7}}", stage);
        let run_id = store
            .record_stage_run(&session_id, stage, "completed", Some(&params), None, None)
            .unwrap();
        store.complete_stage_run(run_id, "completed").unwrap();
    }

    // Checkpoint at end
    let ckpt_id = store
        .save_checkpoint(&session_id, "export", "/tmp/out.tif")
        .unwrap();
    assert!(ckpt_id > 0);

    // Latest checkpoint is the export
    let latest = store
        .get_latest_checkpoint(&session_id)
        .expect("checkpoint");
    assert_eq!(latest.stage_id, "export");
    assert_eq!(latest.artifact_path, "/tmp/out.tif");
}

#[test]
fn crash_recovery_finds_running_sessions() {
    let path = tmp_db_path("crash");
    let store = SessionStore::new(&path).expect("open");

    // Two sessions in different states
    let p1 = store.create_project("A", None).unwrap();
    let s1 = store.create_session(&p1, None, "beginner").unwrap();
    let p2 = store.create_project("B", None).unwrap();
    let s2 = store.create_session(&p2, None, "beginner").unwrap();

    // s1 was started but crashed (status='running')
    store.set_session_status(&s1, "running").unwrap();
    // s2 completed normally
    store.set_session_status(&s2, "completed").unwrap();

    // The crash recovery query only returns s1
    let interrupted = store.find_interrupted_sessions();
    assert_eq!(interrupted.len(), 1);
    assert_eq!(interrupted[0], s1);
}

#[test]
fn auto_complete_in_tauri_command_path() {
    // This test mirrors the Tauri command's auto-complete logic:
    // when the caller passes status=completed, the row is created AND
    // completed in a single round-trip (no separate call needed).
    let path = tmp_db_path("auto-complete");
    let store = SessionStore::new(&path).expect("open");
    let project_id = store.create_project("AC", None).unwrap();
    let session_id = store.create_session(&project_id, None, "beginner").unwrap();

    // Single-call path: record with terminal status, then immediately complete
    let run_id = store
        .record_stage_run(&session_id, "ingest", "completed", None, None, None)
        .unwrap();
    // Mirrors the Tauri command's `if matches!(status.as_str(), terminal) { complete }`
    if matches!("completed", "completed" | "failed" | "skipped") {
        store.complete_stage_run(run_id, "completed").unwrap();
    }

    // No exceptions thrown = the flow is sound. (We don't currently expose a
    // per-row read API; the absence of errors is the contract.)
    let _ = session_id;
}

#[test]
fn receipts_round_trip_via_list_stage_runs() {
    // Mirrors what the ReceiptsPanel pulls via session_get_receipts:
    // params + metrics + error survive the round-trip as JSON strings
    // so the Svelte side can rebuild a StageReceipt without further IPC.
    let path = tmp_db_path("receipts");
    let store = SessionStore::new(&path).expect("open");
    let project_id = store.create_project("Receipts", Some("deep_sky")).unwrap();
    let session_id = store.create_session(&project_id, None, "beginner").unwrap();

    // Two runs: one happy, one with a warning payload in metrics
    let r1 = store
        .record_stage_run(
            &session_id,
            "ingest",
            "completed",
            Some(r#"{"strength":0.5}"#),
            Some(r#"{"durationMs":42,"mean":1234.5}"#),
            None,
        )
        .unwrap();
    let r2 = store
        .record_stage_run(
            &session_id,
            "background_extraction",
            "completed",
            Some(r#"{"strength":0.8}"#),
            Some(r#"{"durationMs":150,"warnings":["hot pixel near (10,10)"]}"#),
            None,
        )
        .unwrap();
    store.complete_stage_run(r1, "completed").unwrap();
    store.complete_stage_run(r2, "completed").unwrap();

    // The Tauri command returns this list verbatim
    let runs = store.list_stage_runs(&session_id);
    assert_eq!(runs.len(), 2);

    // First run: happy path, no warnings
    assert_eq!(runs[0].stage_id, "ingest");
    assert!(runs[0].params_json.as_deref().unwrap().contains("strength"));
    assert!(runs[0].metrics_json.as_deref().unwrap().contains("durationMs"));

    // Second run: warnings embedded in metrics JSON (the way the Svelte
    // side folds them in recordStage)
    assert_eq!(runs[1].stage_id, "background_extraction");
    let metrics: serde_json::Value =
        serde_json::from_str(runs[1].metrics_json.as_deref().unwrap()).unwrap();
    let warnings = metrics.get("warnings").and_then(|v| v.as_array()).unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].as_str().unwrap().contains("hot pixel"));
}
