#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use astroforge_core::gallery::{GalleryItemUpdate, GalleryStore};
use astroforge_core::session::SessionStore;
use serde::Serialize;
use tauri::{Manager, State};

/// Tauri-managed state: holds the GalleryStore (rusqlite) behind a
/// mutex so the IPC handlers can borrow it immutably across awaits.
struct GalleryState(Mutex<GalleryStore>);

/// Tauri-managed state: holds the SessionStore (rusqlite) for crash-safe
/// autosave. Mirrors the GalleryState pattern.
struct SessionState(Mutex<SessionStore>);

#[derive(Serialize)]
struct CommandError {
    message: String,
}

impl<E: std::fmt::Display> From<E> for CommandError {
    fn from(e: E) -> Self {
        CommandError {
            message: e.to_string(),
        }
    }
}

#[tauri::command]
fn gallery_list(state: State<'_, GalleryState>) -> Result<Vec<astroforge_core::gallery::GalleryItem>, CommandError> {
    let store = state.0.lock().expect("gallery store mutex poisoned");
    store.list().map_err(Into::into)
}

#[tauri::command]
fn gallery_upsert(
    state: State<'_, GalleryState>,
    update: GalleryItemUpdate,
) -> Result<astroforge_core::gallery::GalleryItem, CommandError> {
    let store = state.0.lock().expect("gallery store mutex poisoned");
    store.upsert(update).map_err(Into::into)
}

#[tauri::command]
fn gallery_delete(state: State<'_, GalleryState>, id: String) -> Result<(), CommandError> {
    let store = state.0.lock().expect("gallery store mutex poisoned");
    store.delete(&id).map_err(Into::into)
}

// ─ ─── Session autosave IPC ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
//
// These commands back the crash-safe autosave promised by the spec (§5
// NFR) and tracked by issue #144. Each maps to a single SessionStore
// method; the Svelte side calls them on every stage commit.

#[tauri::command]
fn session_create_project(
    state: State<'_, SessionState>,
    name: String,
    target_type: Option<String>,
) -> Result<String, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    store
        .create_project(&name, target_type.as_deref())
        .map_err(Into::into)
}

#[tauri::command]
fn session_create(
    state: State<'_, SessionState>,
    project_id: String,
    source_dir: Option<String>,
    verbosity: String,
) -> Result<String, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    store
        .create_session(&project_id, source_dir.as_deref(), &verbosity)
        .map_err(Into::into)
}

#[tauri::command]
fn session_record_stage(
    state: State<'_, SessionState>,
    session_id: String,
    stage_id: String,
    status: String,
    params_json: Option<String>,
    metrics_json: Option<String>,
    error: Option<String>,
) -> Result<i64, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    let run_id = store.record_stage_run(
        &session_id,
        &stage_id,
        &status,
        params_json.as_deref(),
        metrics_json.as_deref(),
        error.as_deref(),
    )?;
    // Auto-complete if the caller already knows the outcome — saves an
    // extra round-trip for the common "commit then persist" path.
    if matches!(status.as_str(), "completed" | "failed" | "skipped") {
        store.complete_stage_run(run_id, &status)?;
    }
    Ok(run_id)
}

#[tauri::command]
fn session_find_interrupted(state: State<'_, SessionState>) -> Result<Vec<String>, CommandError> {
    let store = state.0.lock().expect("session store mutex poisoned");
    Ok(store.find_interrupted_sessions())
}

fn gallery_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Resolves to e.g. <app_data_dir>/gallery.sqlite. Falls back to
    // cwd if the app data dir isn't available (shouldn't happen in
    // Tauri runtime but keeps tests from panicking).
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(dir.join("gallery.sqlite"))
}

fn session_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Resolves to e.g. <app_data_dir>/sessions.sqlite. Separate file
    // from the gallery so the two stores can move independently.
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(dir.join("sessions.sqlite"))
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let gallery_path = gallery_db_path(&app.handle())?;
            let gallery = GalleryStore::new(&gallery_path)
                .map_err(|e| format!("failed to open gallery store: {e}"))?;
            app.manage(GalleryState(Mutex::new(gallery)));

            let session_path = session_db_path(&app.handle())?;
            let sessions = SessionStore::new(&session_path)
                .map_err(|e| format!("failed to open session store: {e}"))?;
            app.manage(SessionState(Mutex::new(sessions)));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            gallery_list,
            gallery_upsert,
            gallery_delete,
            session_create_project,
            session_create,
            session_record_stage,
            session_find_interrupted,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AstroForge");
}