//! CR-07 B3 — Tauri command shells for Image Decisions + Comparison Sets.
//!
//! Exposes the B3 persistence layer to the frontend. The B3 store
//! uses a single global `DomainStore` at `~/.astroforge/cr-07.sqlite`.
//! Per-project wiring (matching the main.rs `DomainStore` lifecycle
//! for project DBs) is deferred to B4 alongside the rest of the UX
//! surface.

use crate::commands_ai_enhancement::lock_err;
use astroforge_core::comparison::{ComparisonSet, ImageDecision, ImageDecisionState};
use astroforge_core::decision_store;
use astroforge_core::domain_store::DomainStore;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

static STORE: Mutex<Option<DomainStore>> = Mutex::new(None);

/// Open the global CR-07 store on first use. Mirrors the pattern
/// in `commands_ai_enhancement::with_store`.
fn with_store<R>(f: impl FnOnce(&DomainStore) -> R) -> Result<R, String> {
    let mut guard = STORE.lock().map_err(lock_err)?;
    if guard.is_none() {
        let mut path = dirs_home().ok_or_else(|| "no home directory".to_string())?;
        path.push(".astroforge");
        path.push("cr-07.sqlite");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let s = DomainStore::new(&path).map_err(|e| e.to_string())?;
        *guard = Some(s);
    }
    f(guard.as_ref().unwrap())
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

// ─── Decision commands ─────────────────────────────────────────────────────

/// Persist an `ImageDecision` (B1 type) to the store. The decision's
/// full history is rewritten atomically.
#[tauri::command]
pub fn save_image_decision(decision: ImageDecision) -> Result<(), String> {
    with_store(|s| decision_store::save_decision(s, &decision).map_err(|e| e.to_string()))
}

/// Load the current `ImageDecision` for a version. Returns
/// `Err("not found: ...")` if no decision row exists.
#[tauri::command]
pub fn load_image_decision(version_id: String) -> Result<ImageDecision, String> {
    with_store(|s| decision_store::load_decision(s, &version_id).map_err(|e| e.to_string()))
}

/// List all `ImageDecision` rows for a project, newest first.
#[tauri::command]
pub fn list_image_decisions_for_project(
    project_id: String,
) -> Result<Vec<ImageDecision>, String> {
    with_store(|s| {
        decision_store::list_decisions_for_project(s, &project_id).map_err(|e| e.to_string())
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyDecisionRequest {
    pub version_id: String,
    pub new_state: ImageDecisionState,
    pub reason: Option<String>,
}

/// Apply a state transition and persist atomically. Returns the
/// updated `ImageDecision` (with the new state + appended history).
#[tauri::command]
pub fn apply_image_decision(
    request: ApplyDecisionRequest,
) -> Result<ImageDecision, String> {
    with_store(|s| {
        decision_store::apply_and_save_decision(
            s,
            &request.version_id,
            request.new_state,
            request.reason,
        )
        .map_err(|e| e.to_string())
    })
}

// ─── Comparison set commands ───────────────────────────────────────────────

/// Persist a `ComparisonSet`.
#[tauri::command]
pub fn save_comparison_set(set: ComparisonSet) -> Result<(), String> {
    with_store(|s| decision_store::save_comparison_set(s, &set).map_err(|e| e.to_string()))
}

/// Load a `ComparisonSet` by ID. Returns `Err("not found: ...")` if
/// the set doesn't exist.
#[tauri::command]
pub fn load_comparison_set(set_id: String) -> Result<ComparisonSet, String> {
    with_store(|s| decision_store::load_comparison_set(s, &set_id).map_err(|e| e.to_string()))
}

/// List all `ComparisonSet` rows for a project, newest first.
#[tauri::command]
pub fn list_comparison_sets_for_project(
    project_id: String,
) -> Result<Vec<ComparisonSet>, String> {
    with_store(|s| {
        decision_store::list_comparison_sets_for_project(s, &project_id)
            .map_err(|e| e.to_string())
    })
}

/// Delete a `ComparisonSet` by ID. Returns `true` if a row was
/// deleted, `false` if the set didn't exist. The underlying Image
/// Versions are not affected (per CR-07 ADR-07.4).
#[tauri::command]
pub fn delete_comparison_set(set_id: String) -> Result<bool, String> {
    with_store(|s| decision_store::delete_comparison_set(s, &set_id).map_err(|e| e.to_string()))
}

// `State` import is required by the macro above (Tauri command
// signature inference), but the commands in this module use a
// module-level `STORE` static instead. The import silences unused-
// import warnings.
#[allow(dead_code)]
fn _state_marker(_: State<'_, ()>) {}