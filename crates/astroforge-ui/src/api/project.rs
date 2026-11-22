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
