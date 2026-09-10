//! Integration tests for the R3 image-version listing.
//!
//! `image_version_list` is a Tauri IPC command. To avoid spinning
//! up a Tauri State in the test harness, this test pins the
//! underlying rule the IPC implements: the version timeline is
//! derived from `VersionCreated` events in the project's event
//! log, sorted by insertion order, with payload fields (`label`,
//! `primary_artifact_id`) decoded when present.

use astroforge_core::domain::{ObjectType, ProjectEventKind};
use astroforge_core::domain_store::DomainStore;
use std::path::PathBuf;

fn tmp_db_path(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "astroforge-r3-image-versions-{}-{}.sqlite",
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
    ));
    let _ = std::fs::remove_file(&p);
    p
}

#[test]
fn fresh_project_has_empty_version_timeline() {
    let path = tmp_db_path("fresh");
    let store = DomainStore::new(&path).expect("open");
    let project_id = store
        .create_project("Fresh", None, "test-1.0.0")
        .expect("create project");
    let events = store.list_events(&project_id).expect("events");
    let versions: Vec<_> = events
        .iter()
        .filter(|e| e.kind == ProjectEventKind::VersionCreated)
        .collect();
    assert!(versions.is_empty());
}

#[test]
fn version_created_event_lights_a_version_row() {
    let path = tmp_db_path("event");
    let store = DomainStore::new(&path).expect("open");
    let project_id = store
        .create_project("Versions", None, "test-1.0.0")
        .expect("create project");
    let session_id = store
        .create_session(&project_id, "session-1")
        .expect("create session");
    store
        .record_event(
            &project_id,
            ProjectEventKind::VersionCreated,
            Some(&session_id),
        )
        .expect("event 1");
    store
        .record_event(
            &project_id,
            ProjectEventKind::VersionCreated,
            Some(&session_id),
        )
        .expect("event 2");

    let events = store.list_events(&project_id).expect("events");
    let versions: Vec<_> = events
        .iter()
        .filter(|e| e.kind == ProjectEventKind::VersionCreated)
        .collect();
    assert_eq!(versions.len(), 2, "two VersionCreated events");
    // The IPC assigns sequence in event-log order.
    assert!(versions[0].created_at <= versions[1].created_at);
}

#[test]
fn version_created_payload_decodes_label_and_artifact() {
    // The IPC's `parse_version_payload` helper decodes a JSON
    // payload into (label, primary_artifact_id). Mirror the
    // shape in a small unit test so the decode contract is
    // pinned.
    let json = r#"{"label":"v1 stretched","primary_artifact_id":"art_abc"}"#;
    #[derive(serde::Deserialize)]
    struct V {
        #[serde(default)]
        label: String,
        #[serde(default)]
        primary_artifact_id: String,
    }
    let v: V = serde_json::from_str(json).expect("parse");
    assert_eq!(v.label, "v1 stretched");
    assert_eq!(v.primary_artifact_id, "art_abc");
}

#[test]
fn version_payload_forgiving_on_missing_fields() {
    // The IPC's `parse_version_payload` helper returns
    // `("", "")` on any decode failure, not just "missing
    // fields". Empty payload and garbage must both surface
    // empty strings without panicking. Mirror that contract.
    for s in [
        "",
        "{}",
        r#"{"label":""}"#,
        r#"{"unknown":"x"}"#,
        "not json",
    ] {
        let (label, artifact_id) = parse_like_ipc(s);
        assert_eq!(label, "", "empty label for input {s:?}");
        assert_eq!(artifact_id, "", "empty artifact for input {s:?}");
    }
}

fn parse_like_ipc(s: &str) -> (String, String) {
    #[derive(serde::Deserialize)]
    struct V {
        #[serde(default)]
        label: String,
        #[serde(default)]
        primary_artifact_id: String,
    }
    serde_json::from_str::<V>(s)
        .map(|v| (v.label, v.primary_artifact_id))
        .unwrap_or_default()
}

#[test]
fn unknown_project_returns_error_at_store_layer() {
    let path = tmp_db_path("missing");
    let store = DomainStore::new(&path).expect("open");
    let res = store.get_project("proj_does_not_exist");
    assert!(res.is_err());
}

#[allow(dead_code)]
fn _object_type_default() -> ObjectType {
    ObjectType::default()
}
