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
    insert_stage as core_insert_stage, NodeGraphMutationError,
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
    let store = state.store.lock().map_err(lock_err)?;
    core_insert_stage(&store, &request.plan_id, &request.stage_type)
        .map_err(node_graph_err_to_string)
}

fn lock_err<T>(_: std::sync::PoisonError<T>) -> String {
    "pipeline-plan store mutex poisoned".into()
}

/// Map the slice's error enum to a `String` for the IPC layer.
///
/// The prefix conventions mirror the existing
/// `commands_pipeline_plan::store_err_to_string`:
/// - `InvalidStageType` => `"unknown stage_type '...'"` (so the
///     TS side can pattern-match and surface a clear error).
/// - `Store(NotFound)` => `"plan not found: ..."` (same prefix
///     the existing store surfaces).
/// - `Store(other)`     => `"pipeline-plan store error: ..."`
///     (catch-all that preserves the underlying message).
fn node_graph_err_to_string(e: NodeGraphMutationError) -> String {
    match e {
        NodeGraphMutationError::InvalidStageType(s) => {
            format!("unknown stage_type '{}' (not in NodeCatalog)", s)
        }
        NodeGraphMutationError::Store(PipelinePlanStoreError::NotFound(plan_id)) => {
            format!("plan not found: {}", plan_id)
        }
        NodeGraphMutationError::Store(other) => {
            format!("pipeline-plan store error: {}", other)
        }
    }
}
