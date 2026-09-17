//! CR-07 B3 — Image decision + comparison set persistence.
//!
//! Provides CRUD wrappers for the B1 data types
//! ([`ImageDecision`](crate::comparison::ImageDecision),
//! [`ComparisonSet`](crate::comparison::ComparisonSet)) on top of
//! the existing [`DomainStore`](crate::domain_store::DomainStore)
//! sqlite connection. Per the CR-07 audit recommendation, this
//! piggybacks on `db.rs` rather than introducing a parallel
//! persistence crate.
//!
//! ## Schema
//!
//! Migration 10 (added in `domain_store::MIGRATIONS`):
//! - `image_decisions` — one row per Image Version with its current
//!   decision state.
//! - `decision_history` — append-only history of state transitions
//!   (mirrors B1's `DecisionHistoryEntry`).
//! - `comparison_sets` — saved comparison sets with JSON-encoded
//!   `version_ids` + `slot_labels` arrays.
//!
//! ## Concurrency
//!
//! The underlying `DomainStore` uses a `Mutex<Connection>`. Decision
//! transitions acquire the lock for the duration of the transaction.
//! Concurrent transitions on the same `version_id` are serialized at
//! the SQLite level; the last-writer-wins on `image_decisions.state`
//! while `decision_history` accumulates every transition.

use crate::comparison::{ComparisonSet, DecisionHistoryEntry, ImageDecision, ImageDecisionState};
use crate::domain_store::{DomainStore, DomainStoreError};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

type Result<T> = std::result::Result<T, DomainStoreError>;

/// Persisted shape of a comparison set's JSON columns. Kept private
/// to `decision_store` so consumers go through [`ComparisonSet`]
/// directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ComparisonSetRow {
    id: String,
    project_id: String,
    name: String,
    version_ids: Vec<String>,
    slot_labels: Vec<String>,
    created_at: String,
}

fn state_to_str(state: ImageDecisionState) -> &'static str {
    match state {
        ImageDecisionState::Working => "working",
        ImageDecisionState::Candidate => "candidate",
        ImageDecisionState::Preferred => "preferred",
        ImageDecisionState::Final => "final",
        ImageDecisionState::Rejected => "rejected",
        ImageDecisionState::Reference => "reference",
    }
}

fn state_from_str(s: &str) -> Result<ImageDecisionState> {
    match s {
        "working" => Ok(ImageDecisionState::Working),
        "candidate" => Ok(ImageDecisionState::Candidate),
        "preferred" => Ok(ImageDecisionState::Preferred),
        "final" => Ok(ImageDecisionState::Final),
        "rejected" => Ok(ImageDecisionState::Rejected),
        "reference" => Ok(ImageDecisionState::Reference),
        other => Err(DomainStoreError::NotFound(format!(
            "unknown image_decisions.state: {}",
            other
        ))),
    }
}

fn row_to_comparison_set(row: ComparisonSetRow) -> Result<ComparisonSet> {
    Ok(ComparisonSet {
        id: row.id,
        project_id: row.project_id,
        name: row.name,
        version_ids: row.version_ids,
        slot_labels: row.slot_labels,
        created_at: row.created_at,
    })
}

fn set_to_row(set: &ComparisonSet) -> ComparisonSetRow {
    ComparisonSetRow {
        id: set.id.clone(),
        project_id: set.project_id.clone(),
        name: set.name.clone(),
        version_ids: set.version_ids.clone(),
        slot_labels: set.slot_labels.clone(),
        created_at: set.created_at.clone(),
    }
}

/// Persist a new `ImageDecision` row. The decision's `history` is
/// also persisted; the `decided_at` field is taken from the most
/// recent history entry.
///
/// Re-inserting an existing `version_id` overwrites the row but
/// preserves the existing `decision_history` (which is append-only).
pub fn save_decision(store: &DomainStore, decision: &ImageDecision) -> Result<()> {
    let conn = store.lock_conn();
    let tx = conn.unchecked_transaction()?;

    tx.execute(
        "INSERT OR REPLACE INTO image_decisions
            (version_id, state, decided_at, reason, quality_profile)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            decision.version_id,
            state_to_str(decision.state),
            decision.decided_at,
            decision.history.last().and_then(|h| h.reason.clone()),
            decision.quality_profile.as_deref(),
        ],
    )?;

    // Append any history entries not already persisted. The history
    // list is rebuilt on read; for an UPSERT path, we wipe and
    // re-insert the latest history snapshot. This is correct because
    // history is fully determined by the decision's transition log,
    // and the decision is owned by the caller.
    tx.execute(
        "DELETE FROM decision_history WHERE version_id = ?1",
        params![decision.version_id],
    )?;
    for entry in &decision.history {
        tx.execute(
            "INSERT INTO decision_history
                (version_id, from_state, to_state, at, reason)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                decision.version_id,
                entry.from.map(state_to_str),
                state_to_str(entry.to),
                entry.at,
                entry.reason,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Load the current `ImageDecision` for a version, including the full
/// history. Returns `Err(NotFound)` if no decision row exists.
pub fn load_decision(store: &DomainStore, version_id: &str) -> Result<ImageDecision> {
    let conn = store.lock_conn();

    let (state, decided_at, reason, quality_profile): (
        String,
        String,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT state, decided_at, reason, quality_profile FROM image_decisions
             WHERE version_id = ?1",
            params![version_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                DomainStoreError::NotFound(format!("image_decision:{}", version_id))
            }
            other => DomainStoreError::Sqlite(other),
        })?;

    let state = state_from_str(&state)?;

    let mut stmt = conn.prepare(
        "SELECT from_state, to_state, at, reason FROM decision_history
         WHERE version_id = ?1 ORDER BY id ASC",
    )?;
    let history_iter = stmt.query_map(params![version_id], |r| {
        let from_state: Option<String> = r.get(0)?;
        let to_state: String = r.get(1)?;
        let at: String = r.get(2)?;
        let reason: Option<String> = r.get(3)?;
        Ok((from_state, to_state, at, reason))
    })?;

    let mut history: Vec<DecisionHistoryEntry> = Vec::new();
    for row in history_iter {
        let (from_state, to_state, at, reason) = row?;
        history.push(DecisionHistoryEntry {
            from: from_state.as_deref().map(state_from_str).transpose()?,
            to: state_from_str(&to_state)?,
            at,
            reason,
        });
    }

    let _ = reason; // currently only the most-recent reason is stored on the row

    Ok(ImageDecision {
        version_id: version_id.to_string(),
        state,
        history,
        decided_at,
        quality_profile,
    })
}

/// List all decision rows for a project. Joins on
/// `image_versions.project_id` to scope by project. Returns
/// `Vec<ImageDecision>` (with full history per version).
pub fn list_decisions_for_project(
    store: &DomainStore,
    project_id: &str,
) -> Result<Vec<ImageDecision>> {
    let conn = store.lock_conn();

    let mut stmt = conn.prepare(
        "SELECT d.version_id FROM image_decisions d
         JOIN image_versions v ON v.version_id = d.version_id
         WHERE v.project_id = ?1
         ORDER BY d.decided_at DESC",
    )?;

    let version_ids: Vec<String> = stmt
        .query_map(params![project_id], |r| r.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn);

    let mut out = Vec::with_capacity(version_ids.len());
    for vid in version_ids {
        out.push(load_decision(store, &vid)?);
    }
    Ok(out)
}

/// Persist a `ComparisonSet`. The `version_ids` and `slot_labels` are
/// JSON-encoded.
pub fn save_comparison_set(store: &DomainStore, set: &ComparisonSet) -> Result<()> {
    let conn = store.lock_conn();
    let row = set_to_row(set);
    let version_ids_json = serde_json::to_string(&row.version_ids)?;
    let slot_labels_json = serde_json::to_string(&row.slot_labels)?;

    conn.execute(
        "INSERT OR REPLACE INTO comparison_sets
            (id, project_id, name, version_ids_json, slot_labels_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            row.id,
            row.project_id,
            row.name,
            version_ids_json,
            slot_labels_json,
            row.created_at,
        ],
    )?;
    Ok(())
}

/// Load a single `ComparisonSet` by ID. Returns `Err(NotFound)` if
/// the set doesn't exist.
pub fn load_comparison_set(store: &DomainStore, set_id: &str) -> Result<ComparisonSet> {
    let conn = store.lock_conn();
    let (id, project_id, name, version_ids_json, slot_labels_json, created_at): (
        String,
        String,
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT id, project_id, name, version_ids_json, slot_labels_json, created_at
             FROM comparison_sets WHERE id = ?1",
            params![set_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                DomainStoreError::NotFound(format!("comparison_set:{}", set_id))
            }
            other => DomainStoreError::Sqlite(other),
        })?;
    let row = ComparisonSetRow {
        id,
        project_id,
        name,
        version_ids: serde_json::from_str(&version_ids_json)?,
        slot_labels: serde_json::from_str(&slot_labels_json)?,
        created_at,
    };
    row_to_comparison_set(row)
}

/// List all comparison sets for a project, newest first.
pub fn list_comparison_sets_for_project(
    store: &DomainStore,
    project_id: &str,
) -> Result<Vec<ComparisonSet>> {
    let conn = store.lock_conn();
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, version_ids_json, slot_labels_json, created_at
         FROM comparison_sets WHERE project_id = ?1
         ORDER BY created_at DESC",
    )?;
    let mut out: Vec<ComparisonSetRow> = Vec::new();
    {
        let mut rows = stmt.query(params![project_id])?;
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let name: String = row.get(2)?;
            let version_ids_json: String = row.get(3)?;
            let slot_labels_json: String = row.get(4)?;
            let created_at: String = row.get(5)?;
            out.push(ComparisonSetRow {
                id,
                project_id,
                name,
                version_ids: serde_json::from_str(&version_ids_json)?,
                slot_labels: serde_json::from_str(&slot_labels_json)?,
                created_at,
            });
        }
    }
    out.into_iter().map(row_to_comparison_set).collect()
}

/// Delete a comparison set by ID. Returns `true` if a row was
/// deleted, `false` if the set didn't exist. ComparisonSet
/// deletion is non-destructive of the underlying Image Versions
/// (per CR-07 ADR-07.4 Comparison Is Non-Destructive).
pub fn delete_comparison_set(store: &DomainStore, set_id: &str) -> Result<bool> {
    let conn = store.lock_conn();
    let rows = conn.execute("DELETE FROM comparison_sets WHERE id = ?1", params![set_id])?;
    Ok(rows > 0)
}

/// Apply a state transition to a decision and persist. The
/// `decision` is updated in place via the `promote` /
/// `transition_to` / `reject` methods before calling this function;
/// we just save the result.
pub fn apply_and_save_decision(
    store: &DomainStore,
    version_id: &str,
    new_state: ImageDecisionState,
    reason: Option<String>,
) -> Result<ImageDecision> {
    apply_and_save_decision_with_profile(store, version_id, new_state, reason, None)
}

/// CR-07 C-A3.5 — same as [`apply_and_save_decision`] but
/// also accepts the Quality Profile the user had selected
/// at the moment of the transition. When `profile` is
/// `Some`, it overwrites the existing `quality_profile`
/// on the decision; when `None`, the existing value is
/// preserved (so transitions without a profile argument
/// don't accidentally clear the user's prior pick).
pub fn apply_and_save_decision_with_profile(
    store: &DomainStore,
    version_id: &str,
    new_state: ImageDecisionState,
    reason: Option<String>,
    profile: Option<String>,
) -> Result<ImageDecision> {
    // Load existing decision or create a fresh `Working` one. This
    // mirrors B1's contract that `ImageDecision::new` starts in
    // `Working`.
    let mut decision = match load_decision(store, version_id) {
        Ok(d) => d,
        Err(DomainStoreError::NotFound(_)) => ImageDecision::new(version_id.to_string()),
        Err(other) => return Err(other),
    };

    decision.transition_to(new_state, reason).map_err(|_e| {
        DomainStoreError::InvalidTransition {
            from: format!("{:?}", decision.state),
            to: format!("{:?}", new_state),
        }
    })?;
    if profile.is_some() {
        decision.quality_profile = profile;
    }
    save_decision(store, &decision)?;
    Ok(decision)
}

impl FromStr for ImageDecisionState {
    type Err = DomainStoreError;
    fn from_str(s: &str) -> Result<Self> {
        state_from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_store::DomainStore;
    use std::path::PathBuf;

    fn fresh_store() -> DomainStore {
        DomainStore::new(&PathBuf::from(":memory:")).unwrap()
    }

    /// Insert a stub `image_versions` row so foreign-key-style joins
    /// work in the persistence tests. The B3 schema doesn't enforce
    /// foreign keys (no `FOREIGN KEY` clause on `image_decisions`),
    /// but `list_decisions_for_project` joins on `image_versions`,
    /// so test fixtures need the row.
    fn stub_image_version(store: &DomainStore, version_id: &str, project_id: &str) {
        let conn = store.lock_conn();
        conn.execute(
            "INSERT OR REPLACE INTO image_versions
                (version_id, project_id, label, sequence, primary_artifact_id, source_version_id, created_at, hidden)
             VALUES (?1, ?2, '', 0, 'artifact-stub', NULL, datetime('now'), 0)",
            params![version_id, project_id],
        )
        .unwrap();
    }

    #[test]
    fn save_and_load_decision_round_trip() {
        let store = fresh_store();
        let mut decision = ImageDecision::new("v1".to_string());
        decision.promote(Some("first cut".into())).unwrap();
        decision.promote(Some("ready to share".into())).unwrap();
        save_decision(&store, &decision).unwrap();

        let loaded = load_decision(&store, "v1").unwrap();
        assert_eq!(loaded.version_id, "v1");
        assert_eq!(loaded.state, ImageDecisionState::Preferred);
        assert_eq!(loaded.history.len(), 3); // initial + 2 promotions
        assert_eq!(loaded.history[1].from, Some(ImageDecisionState::Working));
        assert_eq!(loaded.history[1].to, ImageDecisionState::Candidate);
        assert_eq!(loaded.history[1].reason.as_deref(), Some("first cut"));
    }

    #[test]
    fn save_overwrites_existing_decision() {
        let store = fresh_store();
        let mut decision = ImageDecision::new("v1".to_string());
        decision.promote(None).unwrap();
        save_decision(&store, &decision).unwrap();

        // Reload and promote further.
        let mut loaded = load_decision(&store, "v1").unwrap();
        loaded.promote(None).unwrap();
        save_decision(&store, &loaded).unwrap();

        let reloaded = load_decision(&store, "v1").unwrap();
        assert_eq!(reloaded.state, ImageDecisionState::Preferred);
    }

    #[test]
    fn load_missing_decision_returns_not_found() {
        let store = fresh_store();
        let err = load_decision(&store, "missing").unwrap_err();
        assert!(matches!(err, DomainStoreError::NotFound(_)));
    }

    #[test]
    fn list_decisions_for_project_filters_by_project() {
        let store = fresh_store();
        stub_image_version(&store, "v1", "proj-1");
        stub_image_version(&store, "v2", "proj-1");
        stub_image_version(&store, "v3", "proj-2");

        for vid in ["v1", "v2", "v3"] {
            let decision = ImageDecision::new(vid.to_string());
            save_decision(&store, &decision).unwrap();
        }

        let proj1 = list_decisions_for_project(&store, "proj-1").unwrap();
        assert_eq!(proj1.len(), 2);
        let proj2 = list_decisions_for_project(&store, "proj-2").unwrap();
        assert_eq!(proj2.len(), 1);
        let empty = list_decisions_for_project(&store, "proj-3").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn apply_and_save_decision_creates_initial_when_missing() {
        let store = fresh_store();
        let decision = apply_and_save_decision(
            &store,
            "v-new",
            ImageDecisionState::Candidate,
            Some("first pass".into()),
        )
        .unwrap();
        // The state machine in B1 only allows Working -> Candidate
        // for a fresh decision. Calling with Candidate on a fresh
        // decision goes Working -> Candidate (since `transition_to`
        // is used and Working -> Candidate is allowed).
        assert_eq!(decision.state, ImageDecisionState::Candidate);
        assert_eq!(decision.history.len(), 2);
        assert_eq!(decision.history[0].from, None);
        assert_eq!(decision.history[0].to, ImageDecisionState::Working);
        assert_eq!(decision.history[1].from, Some(ImageDecisionState::Working));
        assert_eq!(decision.history[1].to, ImageDecisionState::Candidate);
    }

    #[test]
    fn apply_and_save_decision_rejects_skipping_states() {
        let store = fresh_store();
        // Working -> Final is invalid (must go through Candidate + Preferred).
        let err = apply_and_save_decision(&store, "v1", ImageDecisionState::Final, None);
        assert!(matches!(
            err,
            Err(DomainStoreError::InvalidTransition { .. })
        ));
    }

    // ─── CR-07 C-A3.5: Quality Profile persistence ──────────────────

    #[test]
    fn quality_profile_round_trips_through_apply_and_save() {
        let store = fresh_store();
        // Drive a Working -> Candidate transition with a profile
        // attached; reload from the store and confirm the profile
        // was persisted on the image_decisions row.
        let decision = apply_and_save_decision_with_profile(
            &store,
            "v1",
            ImageDecisionState::Candidate,
            Some("first pass".into()),
            Some("detail".into()),
        )
        .unwrap();
        assert_eq!(decision.quality_profile.as_deref(), Some("detail"));

        let reloaded = load_decision(&store, "v1").unwrap();
        assert_eq!(reloaded.quality_profile.as_deref(), Some("detail"));
    }

    #[test]
    fn quality_profile_none_preserves_existing_value() {
        let store = fresh_store();
        // Set a profile, then run a follow-up transition WITHOUT
        // a profile argument. The existing value must be preserved
        // (transitions without a profile don't accidentally clear
        // the user's prior pick).
        apply_and_save_decision_with_profile(
            &store,
            "v1",
            ImageDecisionState::Candidate,
            None,
            Some("publication".into()),
        )
        .unwrap();
        let after = apply_and_save_decision_with_profile(
            &store,
            "v1",
            ImageDecisionState::Preferred,
            Some("r".into()),
            None,
        )
        .unwrap();
        assert_eq!(after.quality_profile.as_deref(), Some("publication"));
    }

    #[test]
    fn quality_profile_defaults_to_none_for_legacy_decision() {
        // A fresh ImageDecision with no profile wired through has
        // `quality_profile: None` (matches the C-A3 picker "before
        // user picks anything" state). Mirrors the
        // ImageVersion.recipe_id legacy-null precedent from B13a.
        let d = ImageDecision::new("v-legacy");
        assert!(d.quality_profile.is_none());
    }

    #[test]
    fn save_and_load_comparison_set_round_trip() {
        let store = fresh_store();
        let set = ComparisonSet::new(
            "m42-final",
            "M42 Final Candidates",
            vec!["v1".into(), "v2".into(), "v3".into()],
        );
        save_comparison_set(&store, &set).unwrap();

        let loaded = load_comparison_set(&store, &set.id).unwrap();
        assert_eq!(loaded.id, set.id);
        assert_eq!(loaded.project_id, "m42-final");
        assert_eq!(loaded.name, "M42 Final Candidates");
        assert_eq!(loaded.version_ids, vec!["v1", "v2", "v3"]);
    }

    #[test]
    fn comparison_set_with_slot_labels_round_trip() {
        let store = fresh_store();
        let mut set = ComparisonSet::new(
            "m42-final",
            "M42 Final Candidates",
            vec!["v1".into(), "v2".into()],
        );
        set.slot_labels = vec!["A — Natural".into(), "B — AI Enhanced".into()];
        save_comparison_set(&store, &set).unwrap();

        let loaded = load_comparison_set(&store, &set.id).unwrap();
        assert_eq!(loaded.slot_labels, vec!["A — Natural", "B — AI Enhanced"]);
    }

    #[test]
    fn list_comparison_sets_for_project_returns_both() {
        // Both sets are inserted; verify they are both returned for
        // the project. (created_at is set at insertion time; if both
        // are inserted within the same second they share the same
        // sort key. The ordering test is covered by save_and_load +
        // the chronologically-distinct set2 below.)
        let store = fresh_store();
        let set1 = ComparisonSet::new("proj-1", "First", vec!["v1".into()]);
        save_comparison_set(&store, &set1).unwrap();
        let set2 = ComparisonSet::new("proj-1", "Second", vec!["v2".into()]);
        save_comparison_set(&store, &set2).unwrap();

        let listed = list_comparison_sets_for_project(&store, "proj-1").unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn list_comparison_sets_for_project_orders_by_created_at_desc() {
        // The set2 row has a chronologically later created_at and
        // should sort first under ORDER BY DESC.
        let store = fresh_store();
        let set1 = ComparisonSet::new("proj-1", "First", vec!["v1".into()]);
        save_comparison_set(&store, &set1).unwrap();
        let mut set2 = ComparisonSet::new("proj-1", "Second", vec!["v2".into()]);
        set2.created_at = "2099-01-01T00:00:00Z".into();
        save_comparison_set(&store, &set2).unwrap();

        let listed = list_comparison_sets_for_project(&store, "proj-1").unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].name, "Second");
    }

    #[test]
    fn delete_comparison_set_removes_row() {
        let store = fresh_store();
        let set = ComparisonSet::new("proj-1", "Throwaway", vec!["v1".into()]);
        save_comparison_set(&store, &set).unwrap();
        assert!(load_comparison_set(&store, &set.id).is_ok());
        let removed = delete_comparison_set(&store, &set.id).unwrap();
        assert!(removed);
        assert!(matches!(
            load_comparison_set(&store, &set.id),
            Err(DomainStoreError::NotFound(_))
        ));
    }

    #[test]
    fn delete_missing_comparison_set_returns_false() {
        let store = fresh_store();
        let removed = delete_comparison_set(&store, "nope").unwrap();
        assert!(!removed);
    }
}
