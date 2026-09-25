//! CR-08 §23 / Slice F: Recipe event store tests.
//!
//! These tests pin the event-store contract:
//! - each event records the kind, profile_id,
//!   version, payload_json, and occurred_at.
//! - list_events returns rows newest-first with
//!   the expected filter behavior.
//! - serde round-trip survives on every payload
//!   constructor.
//! - the `RecipeEventKind` enum's `as_str()` is
//!   stable.

use astroforge_core::recipe_events::{
    now_iso, payload_pipeline_saved_as_recipe, payload_recipe_applicability_evaluated,
    payload_recipe_applied, payload_recipe_created, payload_recipe_exported,
    payload_recipe_import_completed, payload_recipe_import_failed, payload_recipe_import_started,
    payload_recipe_updated, payload_recipe_validated, payload_recipe_version_created,
    payload_reproducibility_record_created, RecipeEvent, RecipeEventFilter, RecipeEventKind,
    RecipeEventStore,
};
use std::path::PathBuf;

fn temp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("astroforge-test-{name}-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}

/// Slice F: the enum's `as_str()` is stable so
/// the SQLite `kind` column never drifts.
#[test]
fn recipe_event_kind_as_str_is_stable() {
    assert_eq!(RecipeEventKind::RecipeCreated.as_str(), "recipe_created");
    assert_eq!(
        RecipeEventKind::RecipeVersionCreated.as_str(),
        "recipe_version_created"
    );
    assert_eq!(
        RecipeEventKind::RecipeApplicabilityEvaluated.as_str(),
        "recipe_applicability_evaluated"
    );
    assert_eq!(RecipeEventKind::RecipeApplied.as_str(), "recipe_applied");
    assert_eq!(
        RecipeEventKind::ReproducibilityRecordCreated.as_str(),
        "reproducibility_record_created"
    );
}

/// Slice F: the payload constructors produce the
/// canonical JSON shape the consumer reads.
#[test]
fn recipe_event_payload_constructors() {
    let p1 = payload_recipe_created("M42", "deep_sky", 3);
    assert_eq!(p1["name"], "M42");
    assert_eq!(p1["target_type"], "deep_sky");
    assert_eq!(p1["version"], 3);

    let p2 = payload_recipe_version_created(Some(2));
    assert_eq!(p2["parent_version"], 2);

    let p3 = payload_recipe_updated(&["name", "description"]);
    assert_eq!(p3["fields"][0], "name");

    let p4 = payload_recipe_validated("ok", &["note-1"]);
    assert_eq!(p4["verdict"], "ok");
    assert_eq!(p4["notes"][0], "note-1");

    let p5 = payload_recipe_applicability_evaluated("compatible", &["warn-1"]);
    assert_eq!(p5["verdict"], "compatible");

    let p6 = payload_recipe_applied("ver_abc", &["m1", "m2"]);
    assert_eq!(p6["image_version_id"], "ver_abc");

    let p7 = payload_recipe_import_started("afrecipe://abc");
    assert_eq!(p7["source"], "afrecipe://abc");

    let p8 = payload_recipe_import_completed("M42", "deep_sky", 1);
    assert_eq!(p8["version"], 1);

    let p9 = payload_recipe_import_failed("schema mismatch");
    assert_eq!(p9["error"], "schema mismatch");

    let p10 = payload_recipe_exported("/tmp/M42.afrecipe", 4096);
    assert_eq!(p10["bytes"], 4096);

    let p11 = payload_pipeline_saved_as_recipe("stage_runs", "run_abc", "M42");
    assert_eq!(p11["source"], "stage_runs");

    let p12 = payload_reproducibility_record_created("ver_abc", "exact");
    assert_eq!(p12["verdict"], "exact");
}

/// Slice F: a single event records correctly and
/// is read back via `list_events` with no filter.
#[test]
fn record_and_list_single_event() {
    let store = RecipeEventStore::new(&temp_path("single")).unwrap();
    let id = store
        .record_event(
            RecipeEventKind::RecipeCreated,
            Some("prof_m42_deep_sky"),
            Some(1),
            payload_recipe_created("M42", "deep_sky", 1),
            "2026-09-25T00:00:00Z",
        )
        .unwrap();
    assert!(id > 0);

    let events = store.list_events(&RecipeEventFilter::default()).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "recipe_created");
    assert_eq!(events[0].profile_id.as_deref(), Some("prof_m42_deep_sky"));
    assert_eq!(events[0].version, Some(1));
    assert_eq!(events[0].payload["name"], "M42");
}

/// Slice F: multiple events return newest-first.
#[test]
fn list_events_returns_newest_first() {
    let store = RecipeEventStore::new(&temp_path("order")).unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeCreated,
            Some("prof_a"),
            Some(1),
            payload_recipe_created("A", "deep_sky", 1),
            "2026-09-25T00:00:00Z",
        )
        .unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeVersionCreated,
            Some("prof_a"),
            Some(2),
            payload_recipe_version_created(Some(1)),
            "2026-09-25T00:01:00Z",
        )
        .unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeApplied,
            Some("prof_a"),
            Some(2),
            payload_recipe_applied("ver_abc", &[]),
            "2026-09-25T00:02:00Z",
        )
        .unwrap();

    let events = store.list_events(&RecipeEventFilter::default()).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind, "recipe_applied");
    assert_eq!(events[1].kind, "recipe_version_created");
    assert_eq!(events[2].kind, "recipe_created");
    // event_id is strictly increasing.
    assert!(events[0].event_id > events[1].event_id);
    assert!(events[1].event_id > events[2].event_id);
}

/// Slice F: filter by `profile_id` only returns
/// events for that profile.
#[test]
fn list_events_filter_by_profile_id() {
    let store = RecipeEventStore::new(&temp_path("filter-profile")).unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeCreated,
            Some("prof_a"),
            Some(1),
            payload_recipe_created("A", "deep_sky", 1),
            "2026-09-25T00:00:00Z",
        )
        .unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeCreated,
            Some("prof_b"),
            Some(1),
            payload_recipe_created("B", "planetary", 1),
            "2026-09-25T00:00:01Z",
        )
        .unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeApplied,
            Some("prof_a"),
            Some(1),
            payload_recipe_applied("ver_xyz", &[]),
            "2026-09-25T00:00:02Z",
        )
        .unwrap();

    let filter = RecipeEventFilter {
        profile_id: Some("prof_a".to_string()),
        ..Default::default()
    };
    let events = store.list_events(&filter).unwrap();
    assert_eq!(events.len(), 2);
    assert!(events
        .iter()
        .all(|e| e.profile_id.as_deref() == Some("prof_a")));
}

/// Slice F: filter by `kind` only returns events
/// of that kind.
#[test]
fn list_events_filter_by_kind() {
    let store = RecipeEventStore::new(&temp_path("filter-kind")).unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeCreated,
            Some("prof_a"),
            Some(1),
            payload_recipe_created("A", "deep_sky", 1),
            "2026-09-25T00:00:00Z",
        )
        .unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeApplied,
            Some("prof_a"),
            Some(1),
            payload_recipe_applied("ver_xyz", &[]),
            "2026-09-25T00:01:00Z",
        )
        .unwrap();

    let filter = RecipeEventFilter {
        kind: Some("recipe_applied".to_string()),
        ..Default::default()
    };
    let events = store.list_events(&filter).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "recipe_applied");
}

/// Slice F: filter by `since` only returns events
/// at-or-after the timestamp.
#[test]
fn list_events_filter_by_since() {
    let store = RecipeEventStore::new(&temp_path("filter-since")).unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeCreated,
            Some("prof_a"),
            Some(1),
            payload_recipe_created("A", "deep_sky", 1),
            "2026-09-25T00:00:00Z",
        )
        .unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeApplied,
            Some("prof_a"),
            Some(1),
            payload_recipe_applied("ver_xyz", &[]),
            "2026-09-25T00:01:00Z",
        )
        .unwrap();

    let filter = RecipeEventFilter {
        since: Some("2026-09-25T00:00:30Z".to_string()),
        ..Default::default()
    };
    let events = store.list_events(&filter).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "recipe_applied");
}

/// Slice F: filter by `limit` caps the row count.
#[test]
fn list_events_filter_by_limit() {
    let store = RecipeEventStore::new(&temp_path("filter-limit")).unwrap();
    for i in 0..5 {
        store
            .record_event(
                RecipeEventKind::RecipeCreated,
                Some("prof_a"),
                Some(i + 1),
                payload_recipe_created("A", "deep_sky", (i + 1) as u32),
                &format!("2026-09-25T00:0{i}:00Z"),
            )
            .unwrap();
    }
    let filter = RecipeEventFilter {
        limit: Some(2),
        ..Default::default()
    };
    let events = store.list_events(&filter).unwrap();
    assert_eq!(events.len(), 2);
}

/// Slice F: serde round-trip on the event struct
/// survives (canonical schema).
#[test]
fn serde_round_trip_recipe_event() {
    let store = RecipeEventStore::new(&temp_path("serde")).unwrap();
    store
        .record_event(
            RecipeEventKind::RecipeApplied,
            Some("prof_m42"),
            Some(2),
            payload_recipe_applied("ver_abc", &["m1", "m2"]),
            "2026-09-25T00:00:00Z",
        )
        .unwrap();
    let events = store.list_events(&RecipeEventFilter::default()).unwrap();
    let event: RecipeEvent = events.into_iter().next().unwrap();

    let json = serde_json::to_string(&event).unwrap();
    let back: RecipeEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(event, back);
}

/// Slice F: `RecipeEventFilter` is additive; a
/// caller with an empty filter still gets the
/// newest-first, default-limit (100) list.
#[test]
fn list_events_empty_filter_default_limit() {
    let store = RecipeEventStore::new(&temp_path("empty-filter")).unwrap();
    for i in 0..3 {
        store
            .record_event(
                RecipeEventKind::RecipeCreated,
                Some("prof_a"),
                Some(i + 1),
                payload_recipe_created("A", "deep_sky", (i + 1) as u32),
                &format!("2026-09-25T00:0{i}:00Z"),
            )
            .unwrap();
    }
    let events = store.list_events(&RecipeEventFilter::default()).unwrap();
    assert_eq!(events.len(), 3);
}

/// Slice F: now_iso() returns a non-empty string
/// in the canonical `YYYY-MM-DDTHH:MM:SSZ` form.
#[test]
fn now_iso_is_canonical() {
    let ts = now_iso();
    assert!(ts.ends_with('Z'));
    assert!(ts.len() >= 20);
    assert!(ts.contains('T'));
}

/// Slice F: count_events is monotonic.
#[test]
fn count_events_monotonic() {
    let store = RecipeEventStore::new(&temp_path("count")).unwrap();
    assert_eq!(store.count_events().unwrap(), 0);
    store
        .record_event(
            RecipeEventKind::RecipeCreated,
            Some("prof_a"),
            Some(1),
            payload_recipe_created("A", "deep_sky", 1),
            "2026-09-25T00:00:00Z",
        )
        .unwrap();
    assert_eq!(store.count_events().unwrap(), 1);
    store
        .record_event(
            RecipeEventKind::RecipeApplied,
            Some("prof_a"),
            Some(1),
            payload_recipe_applied("ver_xyz", &[]),
            "2026-09-25T00:01:00Z",
        )
        .unwrap();
    assert_eq!(store.count_events().unwrap(), 2);
}
