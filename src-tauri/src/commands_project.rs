//! CR-02.6 (P6 scaffold) — Tauri command surface for the project/pipeline
//! service layer.
//!
//! **This module is additive only.** Existing widgets continue to read
//! from `sessionStore`, `gallery.ts`, etc. The commands below are dormant
//! until a future PR wires the Svelte side to them. The PR's single rule:
//! *"add the IPC surface; do not replace existing readers."*
//!
//! The §36 architecture (UI → Tauri command → Application service →
//! Domain → SQLite/Artifact store) becomes executable end-to-end with
//! this PR. Each future UI migration PR is then small and reversible on
//! its own (see Option 2 rationale in the PR body).

use std::path::PathBuf;
use std::sync::Mutex;

use astroforge_core::domain::{PipelineRun, PipelineRunStatus, Project, ProjectStatus, StageRunRecord};
use astroforge_core::domain_store::{DomainStore, DomainStoreError};
use astroforge_core::project::{OpenedProject, ProjectError, ProjectManager};
use serde::Serialize;
use tauri::State;

/// Tauri-managed state for the durable Project layer (CR-02 §18–§36).
///
/// Holds both:
///   - `DomainStore`: SQLite persistence for project/session/run lineage
///   - `ProjectManager`: the §36 application-service layer over the
///     self-contained per-project directory layout
pub struct ProjectState {
    pub manager: Mutex<ProjectManager>,
    /// Global store for cross-project queries (list, find_interrupted).
    /// Per-project CRUD routes through `ProjectManager::open_project` so
    /// the per-project `project.db` is the source of truth for that
    /// project's data (CR-02 §18: "self-contained per-project").
    pub store: Mutex<DomainStore>,
    pub projects_root: PathBuf,
}

#[derive(Serialize)]
pub struct ProjectSummary {
    pub project_id: String,
    pub name: String,
    pub status: ProjectStatus,
    pub created_at: String,
    pub updated_at: String,
    pub active_session_id: Option<String>,
    pub application_version: String,
}

impl From<&Project> for ProjectSummary {
    fn from(p: &Project) -> Self {
        Self {
            project_id: p.project_id.clone(),
            name: p.name.clone(),
            status: p.status,
            created_at: p.created_at.clone(),
            updated_at: p.updated_at.clone(),
            active_session_id: p.active_session_id.clone(),
            application_version: p.application_version.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct PipelineRunSummary {
    pub run_id: String,
    pub project_id: String,
    pub status: PipelineRunStatus,
    pub recipe_id: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<&PipelineRun> for PipelineRunSummary {
    fn from(r: &PipelineRun) -> Self {
        Self {
            run_id: r.id.clone(),
            project_id: r.project_id.clone(),
            status: r.status,
            recipe_id: r.recipe_id.clone(),
            started_at: r.started_at.clone(),
            completed_at: r.completed_at.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct StageRunSummary {
    pub stage_run_id: String,
    pub run_id: String,
    pub stage_id: String,
    pub status: String,
    pub attempt: u32,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<&StageRunRecord> for StageRunSummary {
    fn from(s: &StageRunRecord) -> Self {
        Self {
            stage_run_id: s.stage_run_id.clone(),
            run_id: s.run_id.clone(),
            stage_id: s.stage_id.clone(),
            status: s.status.clone(),
            attempt: s.attempt,
            started_at: s.started_at.clone(),
            completed_at: s.completed_at.clone(),
        }
    }
}

fn lock_err<E: std::fmt::Display>(e: E) -> String {
    format!("mutex poisoned: {e}")
}

fn project_err_to_string(e: ProjectError) -> String {
    e.to_string()
}

fn store_err_to_string(e: DomainStoreError) -> String {
    e.to_string()
}

// ─── Project lifecycle commands ────────────────────────────────────────────

#[tauri::command]
pub fn project_list(
    state: State<'_, ProjectState>,
) -> Result<Vec<ProjectSummary>, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let projects = store.list_projects().map_err(store_err_to_string)?;
    Ok(projects.iter().map(ProjectSummary::from).collect())
}

#[tauri::command]
pub fn project_get(
    state: State<'_, ProjectState>,
    project_id: String,
) -> Result<ProjectSummary, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let p = store.get_project(&project_id).map_err(store_err_to_string)?;
    Ok(ProjectSummary::from(&p))
}

#[tauri::command]
pub fn project_create(
    state: State<'_, ProjectState>,
    name: String,
    target_id: Option<String>,
    application_version: String,
) -> Result<ProjectSummary, String> {
    let mgr = state.manager.lock().map_err(lock_err)?;
    let opened: OpenedProject = mgr
        .create_project(&name, target_id.as_deref(), &application_version)
        .map_err(project_err_to_string)?;
    let project = opened
        .store
        .get_project(&opened.manifest.project_id)
        .map_err(store_err_to_string)?;
    Ok(ProjectSummary::from(&project))
}

#[tauri::command]
pub fn project_open(
    state: State<'_, ProjectState>,
    slug: String,
) -> Result<ProjectSummary, String> {
    let mgr = state.manager.lock().map_err(lock_err)?;
    let opened = mgr.open_project(&slug).map_err(project_err_to_string)?;
    let project = opened
        .store
        .get_project(&opened.manifest.project_id)
        .map_err(store_err_to_string)?;
    Ok(ProjectSummary::from(&project))
}

#[tauri::command]
pub fn project_rename(
    state: State<'_, ProjectState>,
    slug: String,
    new_name: String,
) -> Result<ProjectSummary, String> {
    let mgr = state.manager.lock().map_err(lock_err)?;
    mgr.rename_project(&slug, &new_name)
        .map_err(project_err_to_string)?;
    // Re-open to return the updated manifest.
    let opened = mgr.open_project(&slug).map_err(project_err_to_string)?;
    let project = opened
        .store
        .get_project(&opened.manifest.project_id)
        .map_err(store_err_to_string)?;
    Ok(ProjectSummary::from(&project))
}

#[tauri::command]
pub fn project_archive(
    state: State<'_, ProjectState>,
    slug: String,
) -> Result<(), String> {
    let mgr = state.manager.lock().map_err(lock_err)?;
    mgr.archive_project(&slug).map_err(project_err_to_string)?;
    Ok(())
}

#[tauri::command]
pub fn project_delete(
    state: State<'_, ProjectState>,
    slug: String,
    confirm: bool,
) -> Result<(), String> {
    let mgr = state.manager.lock().map_err(lock_err)?;
    mgr.delete_project(&slug, confirm)
        .map_err(project_err_to_string)?;
    Ok(())
}

#[tauri::command]
pub fn project_recover(
    state: State<'_, ProjectState>,
    slug: String,
) -> Result<RecoverSummary, String> {
    let mgr = state.manager.lock().map_err(lock_err)?;
    let report = mgr
        .recover_project(&slug)
        .map_err(project_err_to_string)?;
    Ok(RecoverSummary {
        project_id: report.project_id,
        status: report.status,
        repaired_dirs: report.repaired_dirs,
    })
}

#[derive(Serialize)]
pub struct RecoverSummary {
    pub project_id: String,
    pub status: ProjectStatus,
    pub repaired_dirs: Vec<String>,
}

// ─── Pipeline-run commands (read-only surface in this scaffold) ────────────

#[tauri::command]
pub fn pipeline_run_list(
    state: State<'_, ProjectState>,
    project_id: String,
) -> Result<Vec<PipelineRunSummary>, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let runs = store
        .list_pipeline_runs(&project_id)
        .map_err(store_err_to_string)?;
    Ok(runs.iter().map(PipelineRunSummary::from).collect())
}

#[tauri::command]
pub fn pipeline_run_get(
    state: State<'_, ProjectState>,
    run_id: String,
) -> Result<PipelineRunSummary, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let r = store.get_pipeline_run(&run_id).map_err(store_err_to_string)?;
    Ok(PipelineRunSummary::from(&r))
}

#[tauri::command]
pub fn pipeline_run_list_stages(
    state: State<'_, ProjectState>,
    run_id: String,
) -> Result<Vec<StageRunSummary>, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let rows = store.list_stage_runs(&run_id).map_err(store_err_to_string)?;
    Ok(rows.iter().map(StageRunSummary::from).collect())
}

#[tauri::command]
pub fn pipeline_run_find_interrupted(
    state: State<'_, ProjectState>,
) -> Result<Vec<PipelineRunSummary>, String> {
    let store = state.store.lock().map_err(lock_err)?;
    let runs = store.find_interrupted_runs().map_err(store_err_to_string)?;
    Ok(runs.iter().map(PipelineRunSummary::from).collect())
}

// DomainStoreError → String helper kept available for future commands that
// need it without re-implementing the conversion.
#[allow(dead_code)]
fn _domain_err_to_string(e: DomainStoreError) -> String {
    e.to_string()
}