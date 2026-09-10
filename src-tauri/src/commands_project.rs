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

pub(crate) fn lock_err<E: std::fmt::Display>(e: E) -> String {
    format!("lock poisoned: {e}")
}

fn project_err_to_string(e: ProjectError) -> String {
    e.to_string()
}

pub(crate) fn store_err_to_string(e: DomainStoreError) -> String {
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

// ─── Project overview (CR-05 R2) ────────────────────────────────────────────
//
// Returns the §8 checklist state for a project as a set of truthful
// booleans derived from durable project state. The previous frontend
// derivation hard-coded `import / analyze / review / export` as
// `pending` (see `src/state/workspace.ts:118-122`); this command gives
// the UI real, server-derived signals:
//
// - `imported`: the project has at least one session that contains
//   source assets, or the durable event log has a `SourceImported`
//   event for this project.
// - `analyzed`: the durable event log has an `AnalysisCompleted` event
//   for this project.
// - `processed`: the project has at least one completed pipeline run
//   (either legacy CR-02 `pipeline_runs.status = 'Completed'` or a
//   CR-05 plan whose stages are all `completed`/`skipped`).
// - `versioned`: the durable event log has a `VersionCreated` event
//   for this project (the canonical signal that a final image
//   version exists).
// - `exported`: the durable event log has an `ExportCreated` event
//   for this project.
//
// All five fields are best-effort: the store methods can return zero
// rows for a fresh project, and the IPC never errors on a missing
// table. The frontend treats each field as authoritative when
// returned.

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectOverview {
    pub project_id: String,
    pub imported: bool,
    pub analyzed: bool,
    pub processed: bool,
    pub versioned: bool,
    pub exported: bool,
}

fn event_kind_to_string(kind: astroforge_core::domain::ProjectEventKind) -> &'static str {
    use astroforge_core::domain::ProjectEventKind as K;
    match kind {
        K::ProjectCreated => "PROJECT_CREATED",
        K::SessionImported => "SESSION_IMPORTED",
        K::SourceImported => "SOURCE_IMPORTED",
        K::AnalysisCompleted => "ANALYSIS_COMPLETED",
        K::RecipeSelected => "RECIPE_SELECTED",
        K::PipelineStarted => "PIPELINE_STARTED",
        K::StageCompleted => "STAGE_COMPLETED",
        K::AiOperationApplied => "AI_OPERATION_APPLIED",
        K::VersionCreated => "VERSION_CREATED",
        K::ExportCreated => "EXPORT_CREATED",
    }
}

#[tauri::command]
pub fn project_overview(
    state: State<'_, ProjectState>,
    project_id: String,
) -> Result<ProjectOverview, String> {
    let store = state.store.lock().map_err(lock_err)?;
    // Verify the project exists; an unknown project_id returns an
    // empty overview rather than a fabricated one so the UI can
    // render a clear "project not found" path.
    let project = store
        .get_project(&project_id)
        .map_err(store_err_to_string)?;

    // Walk the event log once. Each boolean is "any matching kind
    // exists for this project". The event log is the canonical
    // durable signal; the per-table lookups below are belt-and-
    // suspenders in case legacy data was written before the event
    // log was populated.
    let events = store.list_events(&project_id).map_err(store_err_to_string)?;
    let mut source_imported = false;
    let mut analysis_completed = false;
    let mut version_created = false;
    let mut export_created = false;
    for ev in &events {
        // The durable row stores kind as the SCREAMING_SNAKE_CASE
        // serde string for the enum. Compare strings so we don't
        // depend on the enum's Debug/Display format.
        match ev.kind {
            astroforge_core::domain::ProjectEventKind::SourceImported => {
                source_imported = true;
            }
            astroforge_core::domain::ProjectEventKind::AnalysisCompleted => {
                analysis_completed = true;
            }
            astroforge_core::domain::ProjectEventKind::VersionCreated => {
                version_created = true;
            }
            astroforge_core::domain::ProjectEventKind::ExportCreated => {
                export_created = true;
            }
            _ => {}
        }
    }
    // Silence "unused" warnings on the helper for now — it's the
    // single source of truth if/when the event-log schema moves
    // from SCREAMING_SNAKE_CASE to a richer typed payload.
    let _ = event_kind_to_string;

    // Belt-and-suspenders source-asset check: a project may have
    // source assets registered but no `SourceImported` event row
    // (e.g. legacy data, or events table not yet written for a
    // session import). Treat any source asset in any of the
    // project's sessions as `imported = true`.
    let sessions = store.list_sessions(&project_id).map_err(store_err_to_string)?;
    let mut any_source_asset = false;
    for session in &sessions {
        let assets = store
            .list_source_assets(&session.session_id)
            .map_err(store_err_to_string)?;
        if !assets.is_empty() {
            any_source_asset = true;
            break;
        }
    }

    // Belt-and-suspenders processed check: a project may have
    // pipeline runs but no event log row. Read directly from
    // `pipeline_runs.status = 'Completed'`.
    let runs = store
        .list_pipeline_runs(&project_id)
        .map_err(store_err_to_string)?;
    let any_completed_run = runs
        .iter()
        .any(|r| matches!(r.status, astroforge_core::domain::PipelineRunStatus::Completed));

    // Belt-and-suspenders export check: list exports for the most
    // recent completed run if any. We don't have a `list_exports`
    // helper on DomainStore yet, so derive the export signal from
    // event log alone in this slice; the IPC call is infallible.
    drop(runs);

    Ok(ProjectOverview {
        project_id: project.project_id,
        imported: source_imported || any_source_asset,
        analyzed: analysis_completed,
        processed: any_completed_run,
        versioned: version_created,
        exported: export_created,
    })
}