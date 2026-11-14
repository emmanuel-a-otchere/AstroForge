//! CR-10 P1.6.2.4 / Slice P1.6.2.4 + P1.6.1.3 / Slice P1.6.1.3:
//! Tauri command surface for the Node-Based Editor.
//!
//! Houses the two IPCs the Node-Based Editor's constrained layout
//! + palette insert path needs:
//!
//! - `read_node_catalog` (moved here from `main.rs` in P1.6.2.4
//!   to live next to its sibling node-graph IPCs).
//! - `insert_stage` (new in P1.6.2.4; appends a new stage to a
//!   `PipelinePlan`'s `stages` Vec, looking up default params +
//!   label + image-version / undo flags from the canonical
//!   `NodeCatalog`).
//!
//! Future slices will add the remaining mutation primitives
//! (P1.6.3.1: `update_graph` / `remove_stage` /
//! `reorder_stage` / `set_stage_enabled`) + the free-form layout
//! state query (P1.6.3.3).
//!
//! The IPC commands are thin wrappers around the Rust primitives
//! in `astroforge_core::node_graph::mutation`; the primitives are
//! pure (no Tauri types) and can be exercised in `astroforge-core`'s
//! own test suite.

use astroforge_core::domain::PipelinePlan as DomainPipelinePlan;
use astroforge_core::node_catalog::{
    node_catalog as core_node_catalog, NodeCatalog, CATALOG_VERSION,
};
use astroforge_core::node_graph::mutation::{
    insert_stage as core_insert_stage, remove_stage as core_remove_stage,
    reorder_stage as core_reorder_stage, set_stage_enabled as core_set_stage_enabled,
    update_graph as core_update_graph, GraphNodeState, NodeGraphMutationError,
};
use astroforge_core::node_graph::recipe_io::{
    plan_to_recipe_json, recipe_to_plan,
    reset_session_to_project_default as core_reset_session_to_project_default,
};
use astroforge_core::pipeline_plans_store::PipelinePlanStoreError;
use serde::Deserialize;
use tauri::State;

use super::commands_pipeline_plan::PipelinePlanState;

// ─── read_node_catalog (P1.6.1.3, moved here in P1.6.2.4) ──────────────

/// Frontend-facing wrapper around the canonical NodeCatalog. The
/// manifest is regenerated at build time by the
/// `emit_node_catalog` example. The TS side consumes it via
/// `loadNodeCatalog()` in `src/lib/node-catalog.ts` and caches
/// the result for the session; the catalog is read-only and
/// versioned, so a single load is enough for the Node editor's
/// lifetime.
///
/// Returns the full `NodeCatalog` struct (serialised as JSON
/// via Tauri's automatic `Serialize` impl). The TS-side
/// `NodeCatalog` interface mirrors the Rust struct 1:1.
#[tauri::command]
pub fn read_node_catalog() -> Result<NodeCatalog, String> {
    let catalog = core_node_catalog();
    // Belt-and-braces version check: if the in-Rust version
    // drifts from the manifest version we are about to emit,
    // surface the discrepancy so the TS side can retry after
    // a rebuild. The manifest is checked in; `node_catalog()`
    // is the live source of truth.
    if catalog.version != CATALOG_VERSION {
        return Err(format!(
            "node catalog version mismatch: in-Rust={} manifest={}",
            catalog.version, CATALOG_VERSION
        ));
    }
    Ok(catalog)
}

// ─── insert_stage (P1.6.2.4) ────────────────────────────────────────────

/// Request payload for `insert_stage`. Mirrors the TS-side
/// `InsertStageRequest` interface in `src/lib/astroforge-api.ts`.
#[derive(Debug, Deserialize)]
pub struct InsertStageRequest {
    pub plan_id: String,
    pub stage_type: String,
}

/// Append a new stage to the end of the plan's `stages` Vec.
/// The new stage's `sequence` is one past the prior max, so
/// re-issuing an insert always appends at the tail.
///
/// Returns the updated `DomainPipelinePlan` so the TS side can
/// update its local `pipeline-store` mirror. The IPC error
/// surface follows the existing convention: `String` carrying
/// a human-readable message, with the `InvalidStageType`
/// variants surfaced via a clear `"unknown stage_type '...'"`
/// prefix so the TS side can pattern-match on it (per the
/// `loadNodeCatalog` `not registered` retry convention).
#[tauri::command]
pub fn insert_stage(
    state: State<'_, PipelinePlanState>,
    request: InsertStageRequest,
) -> Result<DomainPipelinePlan, String> {
    let store = &*state.store;
    core_insert_stage(store, &request.plan_id, &request.stage_type)
        .map_err(node_graph_err_to_string)
}

// ─── update_graph + mutations (P1.6.3.1) ────────────────────────────────

/// One node's write-back state in `update_graph`. Mirrors the
/// TS-side `UpdateGraphNode` interface.
#[derive(Debug, Deserialize)]
pub struct UpdateGraphNodeDto {
    pub stage_id: String,
    pub sequence: u32,
    pub enabled: bool,
}

/// Request payload for `update_graph`: the full graph state the
/// constrained-mode UI holds (nodes + order + enabled flags).
#[derive(Debug, Deserialize)]
pub struct UpdateGraphRequest {
    pub plan_id: String,
    pub nodes: Vec<UpdateGraphNodeDto>,
}

/// Write the full graph state in one pass. The node set must match
/// the plan's stages exactly (`graph state does not match the plan
/// stages: ...` on mismatch — the TS side re-syncs from the returned
/// error rather than silently dropping or duplicating stages).
/// Returns the updated plan so the caller can refresh its mirror.
#[tauri::command]
pub fn update_graph(
    state: State<'_, PipelinePlanState>,
    request: UpdateGraphRequest,
) -> Result<DomainPipelinePlan, String> {
    let nodes = request
        .nodes
        .into_iter()
        .map(|n| GraphNodeState {
            stage_id: n.stage_id,
            sequence: n.sequence,
            enabled: n.enabled,
        })
        .collect::<Vec<_>>();
    let store = &*state.store;
    core_update_graph(store, &request.plan_id, &nodes).map_err(node_graph_err_to_string)
}

/// Remove an optional stage (refused for `required` stages — the
/// UI disables Remove for those; the error surfaces the reason).
#[tauri::command]
pub fn remove_stage(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
    stage_id: String,
) -> Result<DomainPipelinePlan, String> {
    let store = &*state.store;
    core_remove_stage(store, &plan_id, &stage_id).map_err(node_graph_err_to_string)
}

/// Move a stage to `new_sequence` (0-based, clamped to range).
#[tauri::command]
pub fn reorder_stage(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
    stage_id: String,
    new_sequence: usize,
) -> Result<DomainPipelinePlan, String> {
    let store = &*state.store;
    core_reorder_stage(store, &plan_id, &stage_id, new_sequence).map_err(node_graph_err_to_string)
}

/// Enable or disable a stage. `enabled` and `required` are
/// orthogonal (workflow-A ships required stages disabled).
#[tauri::command]
pub fn set_stage_enabled(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
    stage_id: String,
    enabled: bool,
) -> Result<DomainPipelinePlan, String> {
    let store = &*state.store;
    core_set_stage_enabled(store, &plan_id, &stage_id, enabled).map_err(node_graph_err_to_string)
}

// ─── recipe round-trip (P1.6.4.1 / P1.6.4.2) ────────────────────────────

/// Serialize the plan to `.astroforge-recipe` JSON (§7.7.7). Paths,
/// GPS/timestamps and machine-specific keys are stripped per §11.2.
#[tauri::command]
pub fn graph_to_recipe(
    state: State<'_, PipelinePlanState>,
    plan_id: String,
    app_version: String,
) -> Result<String, String> {
    let store = &*state.store;
    let plan = store.load_plan(&plan_id).map_err(store_err_string)?;
    plan_to_recipe_json(&plan, &app_version).map_err(|e| e.to_string())
}

/// Materialize a plan from recipe JSON. The result is NOT persisted:
/// per §7.7.7 the UI prompts before replacing the active graph or
/// creating a session. Unknown stages fail with the pipeline entry
/// index ("unknown stage 'x' at pipeline entry N").
#[tauri::command]
pub fn recipe_to_graph(
    recipe_json: String,
    project_id: String,
    session_id: String,
) -> Result<DomainPipelinePlan, String> {
    recipe_to_plan(&recipe_json, &project_id, &session_id).map_err(|e| e.to_string())
}

// ─── per-project default + per-session overrides (P1.6.4.3) ────────────

/// All graph (plan) summaries for a project, newest first. The first
/// entry is the project's DEFAULT graph for the reset semantics.
#[tauri::command]
pub fn list_project_graphs(
    state: State<'_, PipelinePlanState>,
    project_id: String,
) -> Result<Vec<astroforge_core::pipeline_plans_store::PipelinePlanSummary>, String> {
    let store = &*state.store;
    let summaries = store
        .list_plans_for_project(&project_id)
        .map_err(store_err_string)?;
    Ok(summaries.into_iter().map(Into::into).collect())
}

/// The plan pinned for the session, if any. `None` = the session
/// follows the project default.
#[tauri::command]
pub fn get_session_graph_override(
    state: State<'_, PipelinePlanState>,
    project_id: String,
    session_id: String,
) -> Result<Option<DomainPipelinePlan>, String> {
    let store = &*state.store;
    match store
        .get_graph_override(&project_id, &session_id)
        .map_err(store_err_string)?
    {
        Some(plan_id) => {
            let plan = store.load_plan(&plan_id).map_err(store_err_string)?;
            Ok(Some(plan))
        }
        None => Ok(None),
    }
}

/// Pin `plan_id` as the session's graph. The plan must belong to the
/// project (`plan not part of project ...` otherwise).
#[tauri::command]
pub fn set_session_graph_override(
    state: State<'_, PipelinePlanState>,
    project_id: String,
    session_id: String,
    plan_id: String,
) -> Result<DomainPipelinePlan, String> {
    let store = &*state.store;
    let plan = store.load_plan(&plan_id).map_err(store_err_string)?;
    if plan.project_id != project_id {
        return Err(format!(
            "plan '{}' does not belong to project '{}'",
            plan_id, project_id
        ));
    }
    store
        .set_graph_override(&project_id, &session_id, &plan_id)
        .map_err(store_err_string)?;
    Ok(plan)
}

/// Materialize the project's default graph for the session (the
/// round-trip machinery re-binds it), persist it, clear any override
/// row, and return the plan. Idempotent on an unchanged default.
#[tauri::command]
pub fn reset_session_to_project_default(
    state: State<'_, PipelinePlanState>,
    project_id: String,
    session_id: String,
) -> Result<DomainPipelinePlan, String> {
    let store = &*state.store;
    core_reset_session_to_project_default(store, &project_id, &session_id)
        .map_err(|e| e.to_string())
}

/// Load-plan store errors surface through the same prefix the
/// pipeline-plan commands use.
fn store_err_string(e: PipelinePlanStoreError) -> String {
    match e {
        PipelinePlanStoreError::NotFound(id) => format!("plan not found: {}", id),
        other => format!("pipeline-plan store error: {}", other),
    }
}

/// Map the slice's error enum to a `String` for the IPC layer.
///
/// The prefix conventions mirror the existing
/// `commands_pipeline_plan::store_err_to_string`:
/// - `InvalidStageType` => `"unknown stage_type '...'"` (TS-side
///   pattern-matchable).
/// - `Store(NotFound)` => `"plan not found: ..."` (same prefix the
///   existing store surfaces).
/// - `Store(other)` => `"pipeline-plan store error: ..."` (catch-all
///   that preserves the underlying message).
fn node_graph_err_to_string(e: NodeGraphMutationError) -> String {
    match e {
        NodeGraphMutationError::InvalidStageType(s) => {
            format!("unknown stage_type '{}' (not in NodeCatalog)", s)
        }
        NodeGraphMutationError::UnknownStage(id) => {
            format!("unknown stage_id '{}'", id)
        }
        NodeGraphMutationError::StageRequired(id) => {
            format!(
                "stage '{}' is required and cannot be removed (disable it instead)",
                id
            )
        }
        NodeGraphMutationError::GraphMismatch(ids) => {
            format!(
                "graph state does not match the plan stages: {}",
                ids.join(", ")
            )
        }
        NodeGraphMutationError::Store(PipelinePlanStoreError::NotFound(plan_id)) => {
            format!("plan not found: {}", plan_id)
        }
        NodeGraphMutationError::Store(other) => {
            format!("pipeline-plan store error: {}", other)
        }
    }
}
