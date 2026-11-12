//! CR-10 P1.6.2.4 / P1.6.3.1 : graph mutation primitives.
//!
//! `insert_stage` landed first (P1.6.2.4); P1.6.3.1 completes the
//! set (`remove_stage`, `reorder_stage`, `set_stage_enabled`) and
//! adds `update_graph` — the full-graph write path the IPC exposes
//! and the context-menu UI writes through.
//!
//! Design rules (locked in spec §7.7 + CR-10 §2):
//!
//! - Mutation primitives take a `&PipelinePlanStore` so they can
//!   persist changes. They return the updated `PipelinePlan` so the
//!   IPC layer can serialize it back to the frontend.
//! - Mutation primitives are pure Rust: no Tauri types, no Svelte
//!   types. The IPC layer (`commands_node_graph.rs`) wraps them.
//! - `insert_stage` always appends (sequence = current max + 1).
//!   Arbitrary ordering goes through `reorder_stage` or the
//!   `update_graph` full-state write.
//! - Default parameters for the new stage come from the canonical
//!   `NodeCatalog` (P1.6.1.2). The catalog is the single source of
//!   truth for `label` + `default_params` + `produces_image_version`
//!   + `undo_supported`. The primitive does NOT accept those as
//!     arguments, since that would invite drift from the catalog.
//! - `required` defaults to `true` for mandatory pipeline stages
//!   (e.g. `calibrate` / `stack` / `export`) and `false` otherwise.
//!   This mirrors the existing builtin-recipe convention.
//! - `required` stages cannot be REMOVED (the audit trail of the
//!   recipe stays intact — the already-stacked routing relies on
//!   required-but-disabled stages surviving in the plan). Disabling
//!   is allowed for any stage: `enabled` and `required` are
//!   orthogonal (the #461 workflow-A routing ships required stages
//!   with `enabled = false`).
//! - Sequences are kept contiguous 0..n after every mutation, so
//!   the runner's stage walk and the UI's ordering stay in sync.
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
    /// A mutation targeted a `stage_id` the plan does not contain.
    /// Carries the unknown id for the error surface.
    UnknownStage(String),
    /// A removal targeted a `required` stage. The recipe audit
    /// trail stays intact; the UI disables Remove for these.
    StageRequired(String),
    /// `update_graph`'s node set does not match the plan's stages
    /// (missing or unknown ids). Carries the offending ids.
    GraphMismatch(Vec<String>),
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
            Self::UnknownStage(id) => write!(f, "unknown stage_id '{}'", id),
            Self::StageRequired(id) => write!(
                f,
                "stage '{}' is required and cannot be removed (disable it instead)",
                id
            ),
            Self::GraphMismatch(ids) => {
                write!(
                    f,
                    "graph state does not match the plan stages: {}",
                    ids.join(", ")
                )
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

/// Re-sequence the plan's stages to contiguous `0..n` in their
/// current Vec order. Called after every structural mutation so the
/// runner's stage walk, the sequence-derived stage ids' uniqueness
/// assumptions, and the UI ordering stay coherent.
fn resequence(stages: &mut [PipelineStage]) {
    for (i, stage) in stages.iter_mut().enumerate() {
        stage.sequence = i as u32;
    }
}

/// Remove the stage with `stage_id` from the plan and persist.
/// Refuses `required` stages (the recipe's audit trail stays
/// intact — disable instead). Sequences are re-compacted.
pub fn remove_stage(
    store: &PipelinePlanStore,
    plan_id: &str,
    stage_id: &str,
) -> Result<PipelinePlan, NodeGraphMutationError> {
    let mut plan = store.load_plan(plan_id)?;
    if !plan.stages.iter().any(|s| s.stage_id == stage_id) {
        return Err(NodeGraphMutationError::UnknownStage(stage_id.to_string()));
    }
    let stage = plan.stages.iter().find(|s| s.stage_id == stage_id).unwrap();
    if stage.required {
        return Err(NodeGraphMutationError::StageRequired(stage_id.to_string()));
    }
    plan.stages.retain(|s| s.stage_id != stage_id);
    resequence(&mut plan.stages);
    store.update_plan_stages(plan_id, plan.stages.clone())?;
    Ok(plan)
}

/// Move the stage with `stage_id` to `new_sequence` (0-based,
/// clamped to the valid range) and persist. Sequences are
/// re-compacted so the final ordering is exactly the vec order.
pub fn reorder_stage(
    store: &PipelinePlanStore,
    plan_id: &str,
    stage_id: &str,
    new_sequence: usize,
) -> Result<PipelinePlan, NodeGraphMutationError> {
    let mut plan = store.load_plan(plan_id)?;
    let pos = plan
        .stages
        .iter()
        .position(|s| s.stage_id == stage_id)
        .ok_or_else(|| NodeGraphMutationError::UnknownStage(stage_id.to_string()))?;
    let stage = plan.stages.remove(pos);
    let target = new_sequence.min(plan.stages.len());
    plan.stages.insert(target, stage);
    resequence(&mut plan.stages);
    store.update_plan_stages(plan_id, plan.stages.clone())?;
    Ok(plan)
}

/// Enable or disable the stage with `stage_id` and persist.
/// `enabled` and `required` are orthogonal: the workflow-A routing
/// ships required stages with `enabled = false`, so any stage may
/// be toggled here.
pub fn set_stage_enabled(
    store: &PipelinePlanStore,
    plan_id: &str,
    stage_id: &str,
    enabled: bool,
) -> Result<PipelinePlan, NodeGraphMutationError> {
    let mut plan = store.load_plan(plan_id)?;
    let stage = plan
        .stages
        .iter_mut()
        .find(|s| s.stage_id == stage_id)
        .ok_or_else(|| NodeGraphMutationError::UnknownStage(stage_id.to_string()))?;
    stage.enabled = enabled;
    store.update_plan_stages(plan_id, plan.stages.clone())?;
    Ok(plan)
}

/// One node's write-back state in `update_graph` (P1.6.3.1): the
/// full graph state the constrained-mode UI holds, minus fields
/// the user cannot edit there (labels, parameters, required flags
/// — those have their own dedicated paths).
#[derive(Debug, Clone)]
pub struct GraphNodeState {
    pub stage_id: String,
    pub sequence: u32,
    pub enabled: bool,
}

/// Write the full graph state (nodes + order + enabled flags) in
/// one transaction-equivalent pass (the store's
/// `update_plan_stages` is a single DELETE + re-INSERT). The node
/// set must match the plan's stages exactly — unknown or missing
/// ids fail with `GraphMismatch` and nothing is persisted, so a
/// stale frontend cannot silently drop or duplicate stages.
///
/// Ordering: nodes are applied sorted by their submitted
/// `sequence`, then re-compacted to contiguous 0..n — the
/// submitted numbers only define relative order, not the stored
/// values (mirrors `reorder_stage`).
pub fn update_graph(
    store: &PipelinePlanStore,
    plan_id: &str,
    nodes: &[GraphNodeState],
) -> Result<PipelinePlan, NodeGraphMutationError> {
    let mut plan = store.load_plan(plan_id)?;

    // Set equality both ways: no unknown ids, no missing ids.
    let mut mismatches = Vec::new();
    let plan_ids: std::collections::HashSet<&str> =
        plan.stages.iter().map(|s| s.stage_id.as_str()).collect();
    let submitted: std::collections::HashSet<&str> =
        nodes.iter().map(|n| n.stage_id.as_str()).collect();
    for id in submitted.difference(&plan_ids) {
        mismatches.push(format!("unknown stage '{}'", id));
    }
    for id in plan_ids.difference(&submitted) {
        mismatches.push(format!("missing stage '{}'", id));
    }
    if !mismatches.is_empty() {
        return Err(NodeGraphMutationError::GraphMismatch(mismatches));
    }

    // Duplicate submissions are a stale-client bug too.
    let mut seen = std::collections::HashSet::new();
    for n in nodes {
        if !seen.insert(n.stage_id.as_str()) {
            return Err(NodeGraphMutationError::GraphMismatch(vec![format!(
                "duplicate stage '{}'",
                n.stage_id
            )]));
        }
    }

    // Apply ordering + enabled flags in the submitted order.
    let mut ordered = nodes.to_vec();
    ordered.sort_by_key(|n| n.sequence);
    for n in &ordered {
        let stage = plan
            .stages
            .iter_mut()
            .find(|s| s.stage_id == n.stage_id)
            .expect("set equality checked above");
        stage.enabled = n.enabled;
    }
    plan.stages.sort_by_key(|s| {
        ordered
            .iter()
            .position(|n| n.stage_id == s.stage_id)
            .unwrap_or(usize::MAX)
    });
    resequence(&mut plan.stages);
    store.update_plan_stages(plan_id, plan.stages.clone())?;
    Ok(plan)
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

    // ── P1.6.3.1 primitives ────────────────────────────────────────

    use crate::pipeline_plans_store::PipelinePlanStore;

    /// A plan with three optional stages (stretch / color / denoise)
    /// persisted in an in-memory store — the common fixture for the
    /// structural-mutation tests.
    fn plan_with_three_optional_stages(store: &PipelinePlanStore) -> String {
        let plan = PipelinePlan {
            plan_id: "plan_mutation_test".into(),
            project_id: "proj_1".into(),
            session_id: "sess_1".into(),
            recipe_id: None,
            mode: "auto".into(),
            target_type: crate::domain::ObjectType::default(),
            status: crate::domain::PipelinePlanStatus::Ready,
            created_at: "2026-11-13T00:00:00Z".into(),
            schema_version: 1,
            stages: vec![
                PipelineStage {
                    stage_id: "stage_a".into(),
                    plan_id: "plan_mutation_test".into(),
                    stage_type: "stretch".into(),
                    sequence: 0,
                    label: "Stretch".into(),
                    required: false,
                    enabled: true,
                    parameters_json: None,
                    produces_image_version: true,
                    undo_supported: false,
                },
                PipelineStage {
                    stage_id: "stage_b".into(),
                    plan_id: "plan_mutation_test".into(),
                    stage_type: "color".into(),
                    sequence: 1,
                    label: "Color".into(),
                    required: false,
                    enabled: true,
                    parameters_json: None,
                    produces_image_version: true,
                    undo_supported: false,
                },
                PipelineStage {
                    stage_id: "stage_c".into(),
                    plan_id: "plan_mutation_test".into(),
                    stage_type: "denoise".into(),
                    sequence: 2,
                    label: "Denoise".into(),
                    required: false,
                    enabled: false,
                    parameters_json: None,
                    produces_image_version: false,
                    undo_supported: false,
                },
            ],
        };
        store.insert_plan(&plan).expect("insert fixture");
        plan.plan_id
    }

    #[test]
    fn remove_stage_compacts_sequences() {
        let store = PipelinePlanStore::in_memory().expect("store");
        let plan_id = plan_with_three_optional_stages(&store);
        let plan = remove_stage(&store, &plan_id, "stage_b").expect("remove");
        assert_eq!(plan.stages.len(), 2);
        assert_eq!(plan.stages[0].stage_id, "stage_a");
        assert_eq!(plan.stages[0].sequence, 0);
        assert_eq!(plan.stages[1].stage_id, "stage_c");
        assert_eq!(plan.stages[1].sequence, 1, "sequences re-compacted");
        // Durable: a fresh load sees the same state.
        let reloaded = store.load_plan(&plan_id).expect("reload");
        assert_eq!(reloaded.stages.len(), 2);
    }

    #[test]
    fn remove_stage_refuses_required_and_unknown() {
        let store = PipelinePlanStore::in_memory().expect("store");
        let plan_id = plan_with_three_optional_stages(&store);
        let err = remove_stage(&store, &plan_id, "stage_nope").unwrap_err();
        assert!(matches!(err, NodeGraphMutationError::UnknownStage(_)));
        // Make stage_a required, then refuse the removal.
        set_stage_enabled(&store, &plan_id, "stage_a", true).expect("flag flip");
        let mut plan = store.load_plan(&plan_id).expect("load");
        plan.stages[0].required = true;
        store
            .update_plan_stages(&plan_id, plan.stages.clone())
            .expect("persist");
        let err = remove_stage(&store, &plan_id, "stage_a").unwrap_err();
        assert!(matches!(err, NodeGraphMutationError::StageRequired(_)));
        // Nothing changed.
        assert_eq!(store.load_plan(&plan_id).expect("load").stages.len(), 3);
    }

    #[test]
    fn reorder_stage_moves_and_compacts() {
        let store = PipelinePlanStore::in_memory().expect("store");
        let plan_id = plan_with_three_optional_stages(&store);
        // Move stage_c (last) to the front.
        let plan = reorder_stage(&store, &plan_id, "stage_c", 0).expect("reorder");
        let ids: Vec<&str> = plan.stages.iter().map(|s| s.stage_id.as_str()).collect();
        assert_eq!(ids, vec!["stage_c", "stage_a", "stage_b"]);
        let seqs: Vec<u32> = plan.stages.iter().map(|s| s.sequence).collect();
        assert_eq!(seqs, vec![0, 1, 2], "sequences contiguous after move");
        // Clamp: an out-of-range target lands at the tail.
        let plan = reorder_stage(&store, &plan_id, "stage_c", 99).expect("clamp");
        assert_eq!(plan.stages[2].stage_id, "stage_c");
    }

    #[test]
    fn set_stage_enabled_flips_and_persists() {
        let store = PipelinePlanStore::in_memory().expect("store");
        let plan_id = plan_with_three_optional_stages(&store);
        let plan = set_stage_enabled(&store, &plan_id, "stage_c", true).expect("enable");
        let stage = plan
            .stages
            .iter()
            .find(|s| s.stage_id == "stage_c")
            .unwrap();
        assert!(stage.enabled);
        let reloaded = store.load_plan(&plan_id).expect("reload");
        assert!(
            reloaded
                .stages
                .iter()
                .find(|s| s.stage_id == "stage_c")
                .unwrap()
                .enabled
        );
        let err = set_stage_enabled(&store, &plan_id, "stage_x", false).unwrap_err();
        assert!(matches!(err, NodeGraphMutationError::UnknownStage(_)));
    }

    #[test]
    fn update_graph_reorders_and_flips_atomically() {
        let store = PipelinePlanStore::in_memory().expect("store");
        let plan_id = plan_with_three_optional_stages(&store);
        let nodes = vec![
            GraphNodeState {
                stage_id: "stage_c".into(),
                sequence: 0,
                enabled: true,
            },
            GraphNodeState {
                stage_id: "stage_a".into(),
                sequence: 1,
                enabled: false,
            },
            GraphNodeState {
                stage_id: "stage_b".into(),
                sequence: 2,
                enabled: true,
            },
        ];
        let plan = update_graph(&store, &plan_id, &nodes).expect("update");
        let ids: Vec<&str> = plan.stages.iter().map(|s| s.stage_id.as_str()).collect();
        assert_eq!(ids, vec!["stage_c", "stage_a", "stage_b"]);
        assert!(!plan.stages[1].enabled, "stage_a disabled via graph write");
        assert!(plan.stages[0].enabled);
        let seqs: Vec<u32> = plan.stages.iter().map(|s| s.sequence).collect();
        assert_eq!(seqs, vec![0, 1, 2]);
        // Durable.
        let reloaded = store.load_plan(&plan_id).expect("reload");
        assert_eq!(reloaded.stages[1].stage_id, "stage_a");
        assert!(!reloaded.stages[1].enabled);
    }

    #[test]
    fn update_graph_rejects_mismatched_and_duplicate_sets() {
        let store = PipelinePlanStore::in_memory().expect("store");
        let plan_id = plan_with_three_optional_stages(&store);
        // Unknown id: rejected, nothing persisted.
        let nodes = vec![
            GraphNodeState {
                stage_id: "stage_a".into(),
                sequence: 0,
                enabled: true,
            },
            GraphNodeState {
                stage_id: "stage_b".into(),
                sequence: 1,
                enabled: true,
            },
            GraphNodeState {
                stage_id: "stage_ghost".into(),
                sequence: 2,
                enabled: true,
            },
        ];
        let err = update_graph(&store, &plan_id, &nodes).unwrap_err();
        match err {
            NodeGraphMutationError::GraphMismatch(m) => {
                assert!(m[0].contains("stage_ghost"));
            }
            other => panic!("expected GraphMismatch, got {other:?}"),
        }
        // Missing id: rejected too.
        let nodes = vec![
            GraphNodeState {
                stage_id: "stage_a".into(),
                sequence: 0,
                enabled: true,
            },
            GraphNodeState {
                stage_id: "stage_b".into(),
                sequence: 1,
                enabled: true,
            },
        ];
        let err = update_graph(&store, &plan_id, &nodes).unwrap_err();
        assert!(matches!(err, NodeGraphMutationError::GraphMismatch(_)));
        // Duplicate id: rejected.
        let nodes = vec![
            GraphNodeState {
                stage_id: "stage_a".into(),
                sequence: 0,
                enabled: true,
            },
            GraphNodeState {
                stage_id: "stage_a".into(),
                sequence: 1,
                enabled: true,
            },
            GraphNodeState {
                stage_id: "stage_b".into(),
                sequence: 2,
                enabled: true,
            },
        ];
        let err = update_graph(&store, &plan_id, &nodes).unwrap_err();
        assert!(matches!(err, NodeGraphMutationError::GraphMismatch(_)));
        // The plan is untouched through all three failures.
        assert_eq!(store.load_plan(&plan_id).expect("load").stages.len(), 3);
    }
}
