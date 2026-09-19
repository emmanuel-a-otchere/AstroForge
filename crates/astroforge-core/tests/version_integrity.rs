//! CR-07 §32.3: version integrity test.
//!
//! The audit's "Version integrity" line requires that
//! "Version A + Version B + Comparison doesn't modify either
//! artifact". This integration test pins the contract:
//! running the comparison-side operations against an
//! in-memory `DomainStore` MUST NOT change any row
//! referencing `version_a` or `version_b` in the
//! `image_versions`, `image_decisions`, or
//! `decision_history` tables.
//!
//! Comparison-side operations covered:
//!
//! - `astroforge_core::comparison_metrics::compare_version_images`
//!   (pure Rust delta-table function, no DB access)
//! - `decision_store::save_comparison_set`
//! - `decision_store::load_comparison_set`
//! - `decision_store::list_comparison_sets_for_project`
//! - `decision_store::delete_comparison_set`
//!
//! Operations explicitly OUT OF SCOPE:
//!
//! - `apply_and_save_decision` / `apply_and_save_decision_with_profile`:
//!   these are *decision* operations, not comparison operations.
//!   They DO mutate `image_decisions` and `decision_history` rows
//!   for the affected version. That's their job. They will be
//!   covered by a §33 ADR-side test (decision state transitions).
//!
//! The audit's CR-07 ADR-07.4 ("Comparison Is Non-Destructive")
//! is the source of truth for the contract this slice pins.

use astroforge_core::comparison::ComparisonSet;
use astroforge_core::comparison_metrics::compare_version_images;
use astroforge_core::decision_store::{
    delete_comparison_set, list_comparison_sets_for_project, load_comparison_set,
    save_comparison_set,
};
use astroforge_core::domain_store::DomainStore;
use rusqlite::Connection;
use std::path::PathBuf;

// ─── helpers ────────────────────────────────────────────────

fn fresh_store() -> DomainStore {
    DomainStore::new(&PathBuf::from(":memory:")).unwrap()
}

/// Insert a stub `image_versions` row so the test fixture has
/// rows to verify stayed-unchanged.
fn stub_image_version(conn: &Connection, version_id: &str, project_id: &str, label: &str) {
    conn.execute(
        "INSERT OR REPLACE INTO image_versions
            (version_id, project_id, label, sequence, primary_artifact_id,
             source_version_id, created_at, hidden)
         VALUES (?1, ?2, ?3, 0, 'artifact-stub', NULL, '2026-01-01T00:00:00Z', 0)",
        rusqlite::params![version_id, project_id, label],
    )
    .unwrap();
}

/// Insert a stub `image_decisions` row. This is the row
/// comparison ops MUST NOT touch (they don't have business
/// reason to).
fn stub_image_decision(conn: &Connection, version_id: &str, state: &str) {
    conn.execute(
        "INSERT OR REPLACE INTO image_decisions
            (version_id, state, decided_at, reason, quality_profile)
         VALUES (?1, ?2, '2026-01-01T00:00:00Z', 'stub', NULL)",
        rusqlite::params![version_id, state],
    )
    .unwrap();
}

/// Insert a stub `decision_history` row. Same as above:
/// comparison ops MUST NOT touch this.
fn stub_decision_history(conn: &Connection, version_id: &str) {
    conn.execute(
        "INSERT INTO decision_history
            (version_id, from_state, to_state, at, reason)
         VALUES (?1, NULL, ?2, '2026-01-01T00:00:00Z', 'stub history')",
        rusqlite::params![version_id, "candidate"],
    )
    .unwrap();
}

/// Hash of every column of every row in `image_versions`
/// whose `version_id` is in `version_ids`. Stable for the
/// lifetime of the store. Used as a fingerprint to compare
/// pre-/post-operation state.
fn image_versions_fingerprint(conn: &Connection, version_ids: &[&str]) -> String {
    let placeholders: String = version_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT version_id, project_id, label, sequence, primary_artifact_id,
                source_version_id, created_at, hidden
         FROM image_versions
         WHERE version_id IN ({})
         ORDER BY version_id",
        placeholders
    );
    let mut stmt = conn.prepare(&sql).unwrap();
    let rows = stmt
        .query_map(rusqlite::params_from_iter(version_ids.iter()), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, i64>(7)?,
            ))
        })
        .unwrap();
    let mut out = String::new();
    for row in rows {
        let row = row.unwrap();
        out.push_str(&format!("{:?}|", row));
    }
    out
}

/// Hash of every column of every row in `image_decisions`
/// whose `version_id` is in `version_ids`.
fn image_decisions_fingerprint(conn: &Connection, version_ids: &[&str]) -> String {
    let placeholders: String = version_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT version_id, state, decided_at, reason, quality_profile
         FROM image_decisions
         WHERE version_id IN ({})
         ORDER BY version_id",
        placeholders
    );
    let mut stmt = conn.prepare(&sql).unwrap();
    let rows = stmt
        .query_map(rusqlite::params_from_iter(version_ids.iter()), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })
        .unwrap();
    let mut out = String::new();
    for row in rows {
        let row = row.unwrap();
        out.push_str(&format!("{:?}|", row));
    }
    out
}

/// Hash of every column of every row in `decision_history`
/// whose `version_id` is in `version_ids`.
fn decision_history_fingerprint(conn: &Connection, version_ids: &[&str]) -> String {
    let placeholders: String = version_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT id, version_id, from_state, to_state, at, reason
         FROM decision_history
         WHERE version_id IN ({})
         ORDER BY id",
        placeholders
    );
    let mut stmt = conn.prepare(&sql).unwrap();
    let rows = stmt
        .query_map(rusqlite::params_from_iter(version_ids.iter()), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
            ))
        })
        .unwrap();
    let mut out = String::new();
    for row in rows {
        let row = row.unwrap();
        out.push_str(&format!("{:?}|", row));
    }
    out
}

/// Count rows in `comparison_sets` whose `version_ids_json`
/// references any of the given `version_ids`. Used as the
/// positive control: comparison ops ARE allowed to create /
/// update `comparison_sets` rows.
fn comparison_sets_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM comparison_sets", [], |r| r.get(0))
        .unwrap()
}

/// Snapshot the version-side fingerprints for a list of
/// version IDs. Returns a `(versions_fp, decisions_fp,
/// history_fp)` tuple.
fn snapshot_version_side(store: &DomainStore, version_ids: &[&str]) -> (String, String, String) {
    let conn = store.lock_conn();
    (
        image_versions_fingerprint(&conn, version_ids),
        image_decisions_fingerprint(&conn, version_ids),
        decision_history_fingerprint(&conn, version_ids),
    )
}

// ─── §32.3.1: pure compare_version_images is deterministic ──

#[test]
fn compare_version_images_is_deterministic() {
    // The pure Rust delta-table function must produce
    // identical output for identical input across multiple
    // invocations. No DB access; the function is read-only
    // by construction. This is the §32.3 determinism
    // guarantee for the comparison math itself (the
    // lifecycle tests below cover the DB-side guarantees).
    use astroforge_core::image::F32Image;
    let baseline = F32Image::new(8, 8, 3);
    let compared = F32Image::new(8, 8, 3);
    let r1 = compare_version_images(&baseline, &compared, "v-a", "v-b");
    let r2 = compare_version_images(&baseline, &compared, "v-a", "v-b");
    assert_eq!(r1.rows.len(), r2.rows.len(), "row count must match");
    for (row_1, row_2) in r1.rows.iter().zip(r2.rows.iter()) {
        assert_eq!(row_1, row_2, "row payloads must match");
    }
    assert_eq!(
        r1.summary, r2.summary,
        "summary prose must match across invocations"
    );
}

// ─── §32.3.2: save_comparison_set doesn't touch versions ────

#[test]
fn save_comparison_set_does_not_touch_version_rows() {
    let store = fresh_store();
    {
        let conn = store.lock_conn();
        stub_image_version(&conn, "v-a", "m42-final", "Natural");
        stub_image_version(&conn, "v-b", "m42-final", "AI Enhanced");
        stub_image_decision(&conn, "v-a", "candidate");
        stub_image_decision(&conn, "v-b", "candidate");
        stub_decision_history(&conn, "v-a");
        stub_decision_history(&conn, "v-b");
    }
    let pre = snapshot_version_side(&store, &["v-a", "v-b"]);

    let set = ComparisonSet::new(
        "m42-final",
        "M42 Final Candidates",
        vec!["v-a".into(), "v-b".into()],
    );
    save_comparison_set(&store, &set).unwrap();

    let post = snapshot_version_side(&store, &["v-a", "v-b"]);
    assert_eq!(pre.0, post.0, "image_versions rows must be unchanged");
    assert_eq!(
        pre.1, post.1,
        "image_decisions rows must be unchanged after save_comparison_set"
    );
    assert_eq!(
        pre.2, post.2,
        "decision_history rows must be unchanged after save_comparison_set"
    );
}

// ─── §32.3.3: load_comparison_set doesn't touch versions ────

#[test]
fn load_comparison_set_does_not_touch_version_rows() {
    let store = fresh_store();
    {
        let conn = store.lock_conn();
        stub_image_version(&conn, "v-a", "m42-final", "Natural");
        stub_image_version(&conn, "v-b", "m42-final", "AI Enhanced");
        stub_image_decision(&conn, "v-a", "candidate");
        stub_image_decision(&conn, "v-b", "candidate");
        stub_decision_history(&conn, "v-a");
        stub_decision_history(&conn, "v-b");
    }
    let set = ComparisonSet::new(
        "m42-final",
        "M42 Final Candidates",
        vec!["v-a".into(), "v-b".into()],
    );
    save_comparison_set(&store, &set).unwrap();
    let pre = snapshot_version_side(&store, &["v-a", "v-b"]);

    let loaded = load_comparison_set(&store, &set.id).unwrap();
    assert_eq!(loaded.version_ids, vec!["v-a", "v-b"]);

    let post = snapshot_version_side(&store, &["v-a", "v-b"]);
    assert_eq!(pre.0, post.0);
    assert_eq!(pre.1, post.1);
    assert_eq!(pre.2, post.2);
}

// ─── §32.3.4: list_comparison_sets_for_project is read-only ─

#[test]
fn list_comparison_sets_for_project_does_not_touch_version_rows() {
    let store = fresh_store();
    {
        let conn = store.lock_conn();
        stub_image_version(&conn, "v-a", "m42-final", "Natural");
        stub_image_version(&conn, "v-b", "m42-final", "AI Enhanced");
        stub_image_decision(&conn, "v-a", "candidate");
        stub_image_decision(&conn, "v-b", "candidate");
        stub_decision_history(&conn, "v-a");
        stub_decision_history(&conn, "v-b");
    }
    let set1 = ComparisonSet::new("m42-final", "Set 1", vec!["v-a".into(), "v-b".into()]);
    save_comparison_set(&store, &set1).unwrap();
    let pre = snapshot_version_side(&store, &["v-a", "v-b"]);

    let listed = list_comparison_sets_for_project(&store, "m42-final").unwrap();
    assert!(!listed.is_empty(), "should list at least one set");

    let post = snapshot_version_side(&store, &["v-a", "v-b"]);
    assert_eq!(pre.0, post.0);
    assert_eq!(pre.1, post.1);
    assert_eq!(pre.2, post.2);
}

// ─── §32.3.5: delete_comparison_set doesn't touch versions ─

#[test]
fn delete_comparison_set_does_not_touch_version_rows() {
    let store = fresh_store();
    {
        let conn = store.lock_conn();
        stub_image_version(&conn, "v-a", "m42-final", "Natural");
        stub_image_version(&conn, "v-b", "m42-final", "AI Enhanced");
        stub_image_decision(&conn, "v-a", "candidate");
        stub_image_decision(&conn, "v-b", "candidate");
        stub_decision_history(&conn, "v-a");
        stub_decision_history(&conn, "v-b");
    }
    let set = ComparisonSet::new("m42-final", "Throwaway", vec!["v-a".into(), "v-b".into()]);
    save_comparison_set(&store, &set).unwrap();
    let pre = snapshot_version_side(&store, &["v-a", "v-b"]);

    let removed = delete_comparison_set(&store, &set.id).unwrap();
    assert!(removed, "delete should report the row was removed");

    let post = snapshot_version_side(&store, &["v-a", "v-b"]);
    assert_eq!(
        pre.0, post.0,
        "image_versions rows must be unchanged after delete_comparison_set"
    );
    assert_eq!(
        pre.1, post.1,
        "image_decisions rows must be unchanged after delete_comparison_set"
    );
    assert_eq!(
        pre.2, post.2,
        "decision_history rows must be unchanged after delete_comparison_set"
    );
}

// ─── §32.3.6: full end-to-end integrity sweep ──────────────

#[test]
fn full_comparison_lifecycle_does_not_modify_either_version() {
    // Full end-to-end: stub 2 versions + decisions + history,
    // run save / load / list / delete on a comparison set
    // referencing both versions, verify the version-side
    // tables are byte-equal across the entire lifecycle.
    let store = fresh_store();
    {
        let conn = store.lock_conn();
        stub_image_version(&conn, "v-a", "m42-final", "Natural");
        stub_image_version(&conn, "v-b", "m42-final", "AI Enhanced");
        stub_image_decision(&conn, "v-a", "candidate");
        stub_image_decision(&conn, "v-b", "candidate");
        stub_decision_history(&conn, "v-a");
        stub_decision_history(&conn, "v-b");
    }
    let pre = snapshot_version_side(&store, &["v-a", "v-b"]);

    // Lifecycle: save → load → list → delete.
    let set = ComparisonSet::new(
        "m42-final",
        "M42 Final Candidates",
        vec!["v-a".into(), "v-b".into()],
    );
    save_comparison_set(&store, &set).unwrap();
    let loaded = load_comparison_set(&store, &set.id).unwrap();
    assert_eq!(loaded.version_ids, vec!["v-a", "v-b"]);
    let _listed = list_comparison_sets_for_project(&store, "m42-final").unwrap();
    let removed = delete_comparison_set(&store, &set.id).unwrap();
    assert!(removed);

    let post = snapshot_version_side(&store, &["v-a", "v-b"]);
    assert_eq!(pre.0, post.0, "image_versions rows must be unchanged");
    assert_eq!(
        pre.1, post.1,
        "image_decisions rows must be unchanged across full lifecycle"
    );
    assert_eq!(
        pre.2, post.2,
        "decision_history rows must be unchanged across full lifecycle"
    );
}

// ─── §32.3.7: positive control, comparison_sets DOES change ─

#[test]
fn comparison_sets_table_does_change_across_lifecycle() {
    // Positive control: pin that the comparison_sets table
    // DOES change across the lifecycle operations. This
    // guards against the test suite accidentally being a
    // no-op (e.g. if the in-memory store weren't actually
    // wired up).
    let store = fresh_store();
    {
        let conn = store.lock_conn();
        stub_image_version(&conn, "v-a", "m42-final", "Natural");
        stub_image_version(&conn, "v-b", "m42-final", "AI Enhanced");
    }
    let pre_count = comparison_sets_count(&store.lock_conn());

    let set = ComparisonSet::new("m42-final", "Test", vec!["v-a".into(), "v-b".into()]);
    save_comparison_set(&store, &set).unwrap();
    let after_save = comparison_sets_count(&store.lock_conn());
    assert_eq!(
        after_save,
        pre_count + 1,
        "comparison_sets should grow by 1 after save"
    );

    let _ = load_comparison_set(&store, &set.id).unwrap();
    let _ = list_comparison_sets_for_project(&store, "m42-final").unwrap();
    let after_reads = comparison_sets_count(&store.lock_conn());
    assert_eq!(
        after_reads, after_save,
        "load + list should not change the comparison_sets row count"
    );

    let removed = delete_comparison_set(&store, &set.id).unwrap();
    assert!(removed);
    let after_delete = comparison_sets_count(&store.lock_conn());
    assert_eq!(
        after_delete, pre_count,
        "comparison_sets should be back to baseline after delete"
    );
}

// ─── §32.3.8: unrelated version rows are also untouched ─────

#[test]
fn comparison_ops_do_not_touch_unrelated_version_rows() {
    // Pin that comparison ops on a (v-a, v-b) pair don't
    // touch other versions in the same project. This
    // prevents a regression where comparison ops leak
    // updates to sibling rows.
    let store = fresh_store();
    {
        let conn = store.lock_conn();
        stub_image_version(&conn, "v-a", "m42-final", "Natural");
        stub_image_version(&conn, "v-b", "m42-final", "AI Enhanced");
        stub_image_version(&conn, "v-c", "m42-final", "Unrelated");
        stub_image_decision(&conn, "v-c", "candidate");
        stub_decision_history(&conn, "v-c");
    }
    let pre = snapshot_version_side(&store, &["v-a", "v-b", "v-c"]);

    let set = ComparisonSet::new("m42-final", "M42 Pair", vec!["v-a".into(), "v-b".into()]);
    save_comparison_set(&store, &set).unwrap();
    let _ = load_comparison_set(&store, &set.id).unwrap();
    let _ = list_comparison_sets_for_project(&store, "m42-final").unwrap();
    let _ = delete_comparison_set(&store, &set.id).unwrap();

    let post = snapshot_version_side(&store, &["v-a", "v-b", "v-c"]);
    assert_eq!(
        pre.0, post.0,
        "all 3 image_versions rows must be unchanged (the unrelated one too)"
    );
    assert_eq!(pre.1, post.1);
    assert_eq!(pre.2, post.2);
}
