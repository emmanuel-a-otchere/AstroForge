//! CR-06 P1 — Tauri command shells for the AI Enhancement Studio.
//!
//! P1 only persists the data model + provenance fields. The
//! substantive behavior (analysis, recommendations, enhancement
//! operations, masks, stacks, previews) lands in P2–P6; the
//! commands in this module are stubs that return the empty
//! state so the IPC surface and the TypeScript wrappers are
//! in place from day one.
//!
//! Each command follows the same shape:
//! - a `*_list` command returning rows for a given parent
//!   (image_version_id or project_id)
//! - an empty-list / `None` response when the parent has no
//!   rows yet
//! - no error path beyond the store-error → string conversion
//!   (mirrors `commands_project::project_overview`)

use crate::commands_project::lock_err;
use crate::domain_store::DomainStore;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

static STORE: Mutex<Option<DomainStore>> = Mutex::new(None);

/// Open the in-memory CR-02 store under a fixed path so the
/// P1 commands can read & write without taking on the
/// production-grade store lifecycle that ships in P2+. The
/// path is resolved relative to the user's home directory and
/// the file is created on first use. Idempotent on re-open.
fn with_store<R>(f: impl FnOnce(&DomainStore) -> R) -> Result<R, String> {
    let mut guard = STORE.lock().map_err(lock_err)?;
    if guard.is_none() {
        let mut path = dirs_home().ok_or_else(|| "no home directory".to_string())?;
        path.push(".astroforge");
        path.push("cr-06-p1.sqlite");
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

/// Response envelope for list commands so the frontend can
/// distinguish "no rows" from "error". P2+ will move real
/// content here; P1 returns empty vectors.
#[derive(Debug, Serialize)]
pub struct AiEnhancementListResponse<T> {
    pub items: Vec<T>,
}

#[tauri::command]
pub fn ai_operation_get(operation_id: String) -> Result<serde_json::Value, String> {
    with_store(|s| match s.get_ai_operation(&operation_id) {
        Ok(op) => Ok(serde_json::to_value(&op).map_err(|e| e.to_string())?),
        Err(_) => Ok(serde_json::Value::Null),
    })
}

#[tauri::command]
pub fn ai_operation_list_for_stage(
    stage_run_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_ai_operations_for_stage(&stage_run_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|op| serde_json::to_value(op).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

#[tauri::command]
pub fn image_analysis_latest(
    image_version_id: String,
) -> Result<serde_json::Value, String> {
    with_store(|s| {
        match s.latest_image_analysis_for_version(&image_version_id) {
            Ok(Some(row)) => serde_json::to_value(&row).map_err(|e| e.to_string()),
            Ok(None) => Ok(serde_json::Value::Null),
            Err(e) => Err(e.to_string()),
        }
    })
}

#[tauri::command]
pub fn image_region_list(
    image_version_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_image_regions(&image_version_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

#[tauri::command]
pub fn ai_recommendation_list_for_version(
    image_version_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_ai_recommendations_for_version(&image_version_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

#[tauri::command]
pub fn ai_mask_list(
    image_version_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_ai_masks(&image_version_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

#[tauri::command]
pub fn enhancement_stack_list_for_source(
    image_version_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_enhancement_stacks_for_source(&image_version_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

#[tauri::command]
pub fn enhancement_preview_list_for_operation(
    operation_id: String,
) -> Result<AiEnhancementListResponse<serde_json::Value>, String> {
    with_store(|s| {
        let rows = s.list_enhancement_previews_for_operation(&operation_id).unwrap_or_default();
        let items: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|row| serde_json::to_value(row).ok())
            .collect();
        Ok(AiEnhancementListResponse { items })
    })
}

/// Marker so the `State` import isn't flagged unused on a
/// future Rust version that warns on bare imports.
#[allow(dead_code)]
fn _state_marker(_s: State<'_, ()>) {}
