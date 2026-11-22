//! Project lifecycle — port of `src-tauri/src/commands_project.rs`
//! (`project_list` / `project_get` / `project_create` / `project_open`)
//! and the matching TS wrappers in `astroforge-api.ts`.
//!
//! Slice note: only the functions the P0 Projects screen exercises
//! are ported; the remaining commands land with their screens in P3.

use serde::Serialize;

use crate::services::Services;

use astroforge_core::domain::Project;
use astroforge_core::project::ProjectError;

/// Mirror of `commands_project::ProjectSummary` (same shape as the
/// TS `ProjectSummary` interface — snake_case fields).
#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub project_id: String,
    pub name: String,
    pub status: astroforge_core::domain::ProjectStatus,
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

fn project_err_to_string(e: ProjectError) -> String {
    e.to_string()
}

fn store_err_to_string(e: astroforge_core::domain_store::DomainStoreError) -> String {
    e.to_string()
}

fn lock_err<E: std::fmt::Display>(e: E) -> String {
    format!("lock poisoned: {e}")
}

/// `project_list` — global `projects.db` is the source of truth.
pub fn project_list(services: &Services) -> Result<Vec<ProjectSummary>, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let projects = store.list_projects().map_err(store_err_to_string)?;
    Ok(projects.iter().map(ProjectSummary::from).collect())
}

/// `project_get`
pub fn project_get(services: &Services, project_id: &str) -> Result<ProjectSummary, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let p = store.get_project(project_id).map_err(store_err_to_string)?;
    Ok(ProjectSummary::from(&p))
}

/// `project_create` — including the #465 W1.5 write-through to the
/// GLOBAL projects.db (freshly created projects must not vanish from
/// `project_list`).
pub fn project_create(
    services: &Services,
    name: &str,
    target_id: Option<&str>,
    application_version: &str,
) -> Result<ProjectSummary, String> {
    let mgr = services.project_manager.lock().map_err(lock_err)?;
    let opened = mgr
        .create_project(name, target_id, application_version)
        .map_err(project_err_to_string)?;
    let project = opened
        .store
        .get_project(&opened.manifest.project_id)
        .map_err(store_err_to_string)?;
    {
        let global = services.domain_store.lock().map_err(lock_err)?;
        global
            .register_project(&project)
            .map_err(store_err_to_string)?;
    }
    Ok(ProjectSummary::from(&project))
}

/// `project_open` — resolve by slug, return the summary.
pub fn project_open(services: &Services, slug: &str) -> Result<ProjectSummary, String> {
    let mgr = services.project_manager.lock().map_err(lock_err)?;
    let opened = mgr.open_project(slug).map_err(project_err_to_string)?;
    let project = opened
        .store
        .get_project(&opened.manifest.project_id)
        .map_err(store_err_to_string)?;
    Ok(ProjectSummary::from(&project))
}

/// `project_rename` — including the #465 W1-fix write-through of the
/// new name into the GLOBAL projects.db.
pub fn project_rename(
    services: &Services,
    slug: &str,
    new_name: &str,
) -> Result<ProjectSummary, String> {
    let mgr = services.project_manager.lock().map_err(lock_err)?;
    mgr.rename_project(slug, new_name).map_err(project_err_to_string)?;
    let opened = mgr.open_project(slug).map_err(project_err_to_string)?;
    let project = opened
        .store
        .get_project(&opened.manifest.project_id)
        .map_err(store_err_to_string)?;
    {
        let global = services.domain_store.lock().map_err(lock_err)?;
        global
            .rename_project(&opened.manifest.project_id, new_name)
            .map_err(store_err_to_string)?;
    }
    Ok(ProjectSummary::from(&project))
}

/// `project_archive`
pub fn project_archive(services: &Services, slug: &str) -> Result<(), String> {
    let mgr = services.project_manager.lock().map_err(lock_err)?;
    mgr.archive_project(slug).map_err(project_err_to_string)?;
    Ok(())
}

/// `project_delete`
pub fn project_delete(services: &Services, slug: &str, confirm: bool) -> Result<(), String> {
    let mgr = services.project_manager.lock().map_err(lock_err)?;
    mgr.delete_project(slug, confirm).map_err(project_err_to_string)?;
    Ok(())
}

/// Mirror of `commands_project::RecoverSummary`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RecoverSummary {
    pub project_id: String,
    pub status: astroforge_core::domain::ProjectStatus,
    pub repaired_dirs: Vec<String>,
}

/// `project_recover`
pub fn project_recover(services: &Services, slug: &str) -> Result<RecoverSummary, String> {
    let mgr = services.project_manager.lock().map_err(lock_err)?;
    let report = mgr.recover_project(slug).map_err(project_err_to_string)?;
    Ok(RecoverSummary {
        project_id: report.project_id,
        status: report.status,
        repaired_dirs: report.repaired_dirs,
    })
}

// ─── Pipeline-run read surface (global store) ──────────────────────────────

/// Mirror of `commands_project::PipelineRunSummary`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PipelineRunSummary {
    pub run_id: String,
    pub project_id: String,
    pub status: astroforge_core::domain::PipelineRunStatus,
    pub recipe_id: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<&astroforge_core::domain::PipelineRun> for PipelineRunSummary {
    fn from(r: &astroforge_core::domain::PipelineRun) -> Self {
        Self {
            run_id: r.run_id.clone(),
            project_id: r.project_id.clone(),
            status: r.status,
            recipe_id: r.recipe_id.clone(),
            started_at: r.started_at.clone(),
            completed_at: r.completed_at.clone(),
        }
    }
}

/// Mirror of `commands_project::StageRunSummary`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StageRunSummary {
    pub stage_run_id: String,
    pub run_id: String,
    pub stage_id: String,
    pub status: String,
    pub attempt: u32,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<&astroforge_core::domain::StageRunRecord> for StageRunSummary {
    fn from(s: &astroforge_core::domain::StageRunRecord) -> Self {
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

/// `pipeline_run_list`
pub fn pipeline_run_list(
    services: &Services,
    project_id: &str,
) -> Result<Vec<PipelineRunSummary>, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let runs = store
        .list_pipeline_runs(project_id)
        .map_err(store_err_to_string)?;
    Ok(runs.iter().map(PipelineRunSummary::from).collect())
}

/// `pipeline_run_get`
pub fn pipeline_run_get(services: &Services, run_id: &str) -> Result<PipelineRunSummary, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let r = store.get_pipeline_run(run_id).map_err(store_err_to_string)?;
    Ok(PipelineRunSummary::from(&r))
}

/// `pipeline_run_list_stages`
pub fn pipeline_run_list_stages(
    services: &Services,
    run_id: &str,
) -> Result<Vec<StageRunSummary>, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let rows = store.list_stage_runs(run_id).map_err(store_err_to_string)?;
    Ok(rows.iter().map(StageRunSummary::from).collect())
}

/// `pipeline_run_find_interrupted`
pub fn pipeline_run_find_interrupted(services: &Services) -> Result<Vec<PipelineRunSummary>, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let runs = store.find_interrupted_runs().map_err(store_err_to_string)?;
    Ok(runs.iter().map(PipelineRunSummary::from).collect())
}

// ─── Project overview (CR-05 R2) ───────────────────────────────────────────

/// Mirror of `commands_project::ProjectOverview` — §8 checklist state
/// as truthful booleans derived from durable project state.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectOverview {
    pub project_id: String,
    pub imported: bool,
    pub analyzed: bool,
    pub processed: bool,
    pub versioned: bool,
    pub exported: bool,
}

/// `project_overview` — walks the event log once, with the same
/// belt-and-suspenders fallbacks as the Tauri command.
pub fn project_overview(services: &Services, project_id: &str) -> Result<ProjectOverview, String> {
    use astroforge_core::domain::ProjectEventKind as K;

    let store = services.domain_store.lock().map_err(lock_err)?;
    let project = store.get_project(project_id).map_err(store_err_to_string)?;

    let events = store.list_events(project_id).map_err(store_err_to_string)?;
    let mut source_imported = false;
    let mut analysis_completed = false;
    let mut version_created = false;
    let mut export_created = false;
    for ev in &events {
        match ev.kind {
            K::SourceImported => source_imported = true,
            K::AnalysisCompleted => analysis_completed = true,
            K::VersionCreated => version_created = true,
            K::ExportCreated => export_created = true,
            _ => {}
        }
    }

    // Belt-and-suspenders: any source asset in any session counts as
    // imported (legacy data may predate the event rows).
    let sessions = store.list_sessions(project_id).map_err(store_err_to_string)?;
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

    // Belt-and-suspenders processed: a completed legacy pipeline run.
    let runs = store
        .list_pipeline_runs(project_id)
        .map_err(store_err_to_string)?;
    let any_completed_run = runs.iter().any(|r| {
        matches!(
            r.status,
            astroforge_core::domain::PipelineRunStatus::Completed
        )
    });

    Ok(ProjectOverview {
        project_id: project.project_id,
        imported: source_imported || any_source_asset,
        analyzed: analysis_completed,
        processed: any_completed_run,
        versioned: version_created,
        exported: export_created,
    })
}
