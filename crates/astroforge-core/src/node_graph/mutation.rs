//! CR-10 P1.6.2.4 / Slice P1.6.2.4 : graph mutation primitives.
//!
//! `insert_stage` is the first primitive; the remaining three
//! (`remove_stage`, `reorder_stage`, `set_stage_enabled`) land in
//! P1.6.3.1 per the CR-10 plan. The IPC surface for this slice is
//! `insert_stage` only.
//!
//! Design rules (locked in spec §7.7 + CR-10 §2):
//!
//! - Mutation primitives take a `&PipelinePlanStore` so they can
//!   persist changes. They return the updated `PipelinePlan` so the
//!   IPC layer can serialize it back to the frontend.
//! - Mutation primitives are pure Rust: no Tauri types, no Svelte
//!   types. The IPC layer (`commands_node_graph.rs`) wraps them.
//! - `insert_stage` always appends (sequence = current max + 1).
//!   Insert-before / insert-after land in P1.6.3.2 once the
//!   `update_graph` IPC is in place to coordinate full graph state.
//! - Default parameters for the new stage come from the canonical
//!   `NodeCatalog` (P1.6.1.2). The catalog is the single source of
//!   truth for `label` + `default_params` + `produces_image_version`
//!   + `undo_supported`. The primitive does NOT accept those as
//!     arguments, since that would invite drift from the catalog.
//! - `required` defaults to `true` for mandatory pipeline stages
//!   (e.g. `calibrate` / `stack` / `export`) and `false` otherwise.
//!   This mirrors the existing builtin-recipe convention; the
//!   user can flip it later via the `set_stage_enabled` primitive
//!   in P1.6.3.1 if a future UX surface demands per-stage
//!   toggling of the required flag.
//! - `stage_id` is generated deterministically from `plan_id` +
//!   `stage_type` + the post-insert sequence so a re-derived
//!   catalog can reconstruct it (matches the existing
//!   `pipeline_plan::plan::generate_plan` convention).

use crate::domain::{PipelinePlan, PipelineStage};
use crate::node_catalog::{node_catalog, NodeCatalogEntry};
use crate::pipeline_plans_store::{PipelinePlanStore, PipelinePlanStoreError};
use sha2::{Digest, Sha256};

/// Errors returned by the node-graph mutation primitives.
///
/// `InvalidStageType` covers the case where the user (or a
/// future stale-frontend bug) supplies a `stage_type` that the
/// canonical NodeCatalog no longer recognises. The IPC layer
/// maps this to a `400`/`CommandError::Invalid` response so the
/// frontend can refresh its cached catalog and re-prompt.
///
/// Note: we do not derive `PartialEq` on the enum as a whole
/// because `PipelinePlanStoreError` (which wraps `rusqlite::Error`)
/// does not implement `PartialEq`. Callers that need to assert
/// on a specific error variant should pattern-match.
#[derive(Debug)]
pub enum NodeGraphMutationError {
    /// The supplied `stage_type` is not present in the canonical
    /// NodeCatalog. The frontend should refresh its cached catalog
    /// and re-issue the mutation. The error carries the unknown
    /// stage_type for logging.
    InvalidStageType(String),
    /// The underlying `PipelinePlanStore` returned an error
    /// (plan not found, persistence failure, etc.). The
    /// `PipelinePlanStoreError` is wrapped verbatim so the IPC
    /// layer can choose how to surface it.
    Store(PipelinePlanStoreError),
}

impl std::fmt::Display for NodeGraphMutationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStageType(s) => {
                write!(f, "unknown stage_type '{}' (not in NodeCatalog)", s)
            }
            Self::Store(e) => write!(f, "pipeline plan store error: {}", e),
        }
    }
}

impl std::error::Error for NodeGraphMutationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Store(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PipelinePlanStoreError> for NodeGraphMutationError {
    fn from(e: PipelinePlanStoreError) -> Self {
        Self::Store(e)
    }
}

/// Append a new stage to the end of the plan's `stages` Vec, looking
/// up default parameters + label + image-version / undo flags from
/// the canonical `NodeCatalog`. The new stage's `sequence` is set
/// to one past the current maximum (so re-issuing an insert always
/// appends at the tail, never re-uses a sequence number).
///
/// Persistence: the updated plan is written back via the store
/// before the function returns, so a successful result is durable
/// from the caller's perspective.
///
/// Atomicity: this is not transactional : if the persistence step
/// fails after the in-memory `plan` was mutated, the in-memory
/// copy is dropped on return and the on-disk plan remains in its
/// pre-mutation state. That is the desired behaviour for this
/// slice: a failed insert should NOT corrupt the persisted plan.
pub fn insert_stage(
    store: &PipelinePlanStore,
    plan_id: &str,
    stage_type: &str,
) -> Result<PipelinePlan, NodeGraphMutationError> {
    // 1. Validate stage_type against the canonical catalog.
    let catalog = node_catalog();
    let entry: &NodeCatalogEntry = catalog
        .entries
        .iter()
        .find(|e| e.stage_type == stage_type)
        .ok_or_else(|| NodeGraphMutationError::InvalidStageType(stage_type.to_string()))?;

    // 2. Load the plan (mut copy). This will fail with NotFound if
    //    the plan_id is unknown; we propagate as
    //    NodeGraphMutationError::Store.
    let mut plan = store.load_plan(plan_id)?;

    // 3. Generate the new stage_id deterministically from
    //    plan_id + stage_type + next sequence.
    let next_sequence = plan
        .stages
        .iter()
        .map(|s| s.sequence)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);
    let stage_id = generate_stage_id(plan_id, stage_type, next_sequence);

    // 4. Build the new PipelineStage. parameters_json is the
    //    catalog default_params serialised as JSON. enabled=true,
    //    required = mandatory_for(stage_type), see the module-level
    //    doc comment for the rationale.
    let parameters_json =
        serde_json::to_string(&entry.default_params).unwrap_or_else(|_| "{}".to_string());
    let new_stage = PipelineStage {
        stage_id,
        plan_id: plan.plan_id.clone(),
        stage_type: entry.stage_type.clone(),
        sequence: next_sequence,
        label: entry.label.clone(),
        required: mandatory_for(stage_type),
        enabled: true,
        parameters_json: Some(parameters_json),
        produces_image_version: entry.produces_image_version,
        undo_supported: entry.undo_supported,
    };

    // 5. Append + persist. INSERT OR REPLACE inside `insert_plan`
    //    means existing stages are preserved; the new stage row is
    //    added or updated. The schema_version on the plan is NOT
    //    bumped : the PipelineStage schema is unchanged by this
    //    slice.
    plan.stages.push(new_stage);
    store.insert_plan(&plan)?;

    Ok(plan)
}

/// Returns `true` for stage_types that MUST be present in a
/// well-formed deep-sky pipeline (calibrate / stack / export),
/// `false` for optional refinements (color / stretch / denoise /
/// etc.). Mirrors the convention used by
/// `pipeline_plan::builtin::deep_sky_osc_balanced`.
fn mandatory_for(stage_type: &str) -> bool {
    matches!(stage_type, "calibrate" | "debayer" | "stack" | "export")
}

/// Generate a deterministic `stage_id` for a newly-inserted stage.
///
/// Mirrors the existing convention in
/// `crate::pipeline_plan::plan::generate_plan`:
/// `stage_<short-hash(plan_id)>_<short-hash(stage_type + sequence)>`.
/// The sequence suffix makes the id unique within a plan even if
/// the same `stage_type` is inserted multiple times (the user can
/// re-insert a stage they later removed; the new stage gets a
/// fresh sequence and a fresh id).
fn generate_stage_id(plan_id: &str, stage_type: &str, sequence: u32) -> String {
    let plan_part = short_hash(plan_id);
    let stage_input = format!("{}::{}", stage_type, sequence);
    let stage_part = short_hash(&stage_input);
    format!("stage_{}_{}", plan_part, stage_part)
}

/// First-16-chars-of-SHA256 helper. Mirrors the convention in
/// `crate::pipeline_plan::plan::short_hash`. Local copy so this
/// module has no dependency on `crate::pipeline_plan::plan` (which
/// would force the node-graph module to pull in the full
/// dispatch + handler-registry surface just to mint an id).
fn short_hash(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let hex = format!("{:x}", digest);
    hex.chars().take(16).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_hash_is_16_hex_chars() {
        let h = short_hash("plan_abc");
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn short_hash_is_deterministic() {
        assert_eq!(short_hash("plan_x"), short_hash("plan_x"));
        assert_ne!(short_hash("plan_x"), short_hash("plan_y"));
    }

    #[test]
    fn generate_stage_id_distinct_per_sequence() {
        let a = generate_stage_id("plan_x", "color", 0);
        let b = generate_stage_id("plan_x", "color", 1);
        assert_ne!(a, b, "distinct sequences produce distinct ids");
    }

    #[test]
    fn generate_stage_id_stable_across_calls() {
        // Deterministic: same inputs produce same id. Lets
        // future re-derivation paths reconstruct ids.
        let a = generate_stage_id("plan_x", "stretch", 3);
        let b = generate_stage_id("plan_x", "stretch", 3);
        assert_eq!(a, b);
    }

    #[test]
    fn display_includes_stage_type_for_invalid() {
        let e = NodeGraphMutationError::InvalidStageType("no_such_stage".into());
        let s = e.to_string();
        assert!(s.contains("no_such_stage"));
    }

    #[test]
    fn mandatory_for_locked_set() {
        assert!(mandatory_for("calibrate"));
        assert!(mandatory_for("debayer"));
        assert!(mandatory_for("stack"));
        assert!(mandatory_for("export"));
        assert!(!mandatory_for("color"));
        assert!(!mandatory_for("stretch"));
        assert!(!mandatory_for("denoise"));
        assert!(!mandatory_for("ingest"));
    }
}
