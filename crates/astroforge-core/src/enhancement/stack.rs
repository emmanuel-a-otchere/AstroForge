//! CR-06 P4 — Enhancement Stack engine.
//!
//! The enhancement stack is an ordered list of AI
//! operations (denoise, deconv, star refinement, SR,
//! inpaint, etc.) that AstroForge applies on top of an
//! Image Version to produce a new one (CR-06 §22).
//!
//! The stack engine in this module is pure logic —
//! every operation is a small data record (an
//! `OperationId` + parameters + enabled flag + preview
//! status) and the engine itself enforces the §22
//! manipulations:
//!
//! - `reorder` — move an operation to a new index.
//! - `disable` — set `enabled = false` without
//!   removing the row; the engine skips disabled
//!   operations when applying the stack.
//! - `remove` — drop the row.
//! - `re_preview` — mark the row as needing a fresh
//!   preview; the next apply round regenerates it.
//! - `branch` — derive a new stack from an existing
//!   one at a given cutoff index. Both stacks remain
//!   available to Compare (CR-06 §24).
//!
//! Every manipulation is a pure function that returns
//! a new `EnhancementStackRecord`; the original is
//! never mutated. Persisting the updated record is the
//! Tauri command's job (the store's
//! `upsert_enhancement_stack` already exists from P1).

use serde::{Deserialize, Serialize};

/// One operation in the enhancement stack.
///
/// The `operation_id` is a stable string matching the
/// canonical registry (`astroforge-ai::hub::ModelInfo` +
/// the operation layer that wraps it). The
/// `parameters_json` is the operation-specific parameter
/// blob (strength, tile size, mask id, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StackOperation {
    /// Stable operation identifier (`denoise_luminance`,
    /// `detail_enhance`, `super_resolution`, etc.).
    pub operation_id: String,
    /// Optional reference to the `AiRecommendation`
    /// row that produced this operation. The link lets
    /// the UI show "From recommendation X (confidence
    /// 0.91)" alongside the card.
    pub recommendation_id: Option<String>,
    /// Operation parameters. The shape is operation-specific;
    /// `denoise_luminance` carries `strength`,
    /// `detail_preservation`, `tile_size`; `inpaint`
    /// carries a `mask_id`; etc. The JSON blob is the
    /// contract.
    pub parameters_json: String,
    /// `false` operations are skipped when the stack is
    /// applied (CR-06 §22 `disable`). The row stays in
    /// the stack so the user can re-enable it.
    pub enabled: bool,
    /// Set to `true` when the user clicks "re-preview".
    /// The apply round regenerates the preview before
    /// committing. The field is a hint, not a guarantee
    /// — the apply pipeline decides whether to
    /// regenerate based on parameters + preview age.
    #[serde(default)]
    pub needs_preview: bool,
}

/// The full enhancement stack record.
///
/// The DB row (`EnhancementStack`) carries the same
/// fields via `operation_ids_json`, but the JSON blob is
/// just a serialised `Vec<StackOperation>`. This struct
/// gives the engine a typed view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnhancementStackRecord {
    pub stack_id: String,
    pub project_id: String,
    pub source_image_version_id: String,
    pub operations: Vec<StackOperation>,
    pub branched_from_version_id: Option<String>,
    pub created_at: String,
}

impl EnhancementStackRecord {
    /// Parse the DB row's `operation_ids_json` blob into
    /// the typed view. The blob shape is a JSON array of
    /// `StackOperation`s; malformed JSON falls back to an
    /// empty list (the engine surfaces the gap rather
    /// than crashing).
    pub fn operations_from_json(raw: &str) -> Vec<StackOperation> {
        serde_json::from_str(raw).unwrap_or_default()
    }

    /// Serialise the typed operations back into the JSON
    /// blob shape for `upsert_enhancement_stack`.
    pub fn operations_to_json(ops: &[StackOperation]) -> String {
        serde_json::to_string(ops).unwrap_or_else(|_| "[]".to_string())
    }
}

/// The set of stack mutations the engine supports.
///
/// Each variant carries the data needed to apply that
/// mutation. The engine never mutates in place — every
/// mutation produces a fresh `EnhancementStackRecord`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StackMutation {
    /// Move `operation_id` to `new_index`. Indices are
    /// 0-based; `new_index == operations.len()` means
    /// "move to the end".
    Reorder {
        operation_id: String,
        new_index: usize,
    },
    /// Toggle `enabled` on `operation_id`.
    SetEnabled { operation_id: String, enabled: bool },
    /// Drop `operation_id` from the stack.
    Remove { operation_id: String },
    /// Mark `operation_id` as needing a fresh preview.
    MarkNeedsPreview { operation_id: String },
    /// Append a new operation at the end of the stack.
    Append(StackOperation),
}

/// Apply a `StackMutation` and return the updated stack.
///
/// Returns `Err(StackError)` if the mutation references
/// an operation that does not exist (reorder/remove
/// against a missing id) or violates invariants
/// (reorder index out of bounds).
pub fn apply_mutation(
    stack: EnhancementStackRecord,
    mutation: StackMutation,
) -> Result<EnhancementStackRecord, StackError> {
    match mutation {
        StackMutation::Reorder {
            operation_id,
            new_index,
        } => reorder(stack, &operation_id, new_index),
        StackMutation::SetEnabled {
            operation_id,
            enabled,
        } => set_enabled(stack, &operation_id, enabled),
        StackMutation::Remove { operation_id } => remove(stack, &operation_id),
        StackMutation::MarkNeedsPreview { operation_id } => {
            mark_needs_preview(stack, &operation_id)
        }
        StackMutation::Append(op) => Ok(append(stack, op)),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StackError {
    UnknownOperation(String),
    IndexOutOfBounds { index: usize, len: usize },
}

impl std::fmt::Display for StackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackError::UnknownOperation(id) => write!(f, "unknown operation: {id}"),
            StackError::IndexOutOfBounds { index, len } => {
                write!(f, "index {index} out of bounds (len {len})")
            }
        }
    }
}

impl std::error::Error for StackError {}

fn reorder(
    mut stack: EnhancementStackRecord,
    operation_id: &str,
    new_index: usize,
) -> Result<EnhancementStackRecord, StackError> {
    let len = stack.operations.len();
    if new_index > len {
        return Err(StackError::IndexOutOfBounds {
            index: new_index,
            len,
        });
    }
    let current = stack
        .operations
        .iter()
        .position(|op| op.operation_id == operation_id)
        .ok_or_else(|| StackError::UnknownOperation(operation_id.into()))?;
    let op = stack.operations.remove(current);
    // After the removal the list has `len - 1` elements.
    // The user-supplied `new_index` is interpreted as the
    // final position in the moved list, so clamp to
    // `len - 1`. (When `current == new_index` we just
    // remove and re-insert at the same spot — idempotent.)
    let target = new_index.min(len - 1);
    stack.operations.insert(target, op);
    Ok(stack)
}

fn set_enabled(
    mut stack: EnhancementStackRecord,
    operation_id: &str,
    enabled: bool,
) -> Result<EnhancementStackRecord, StackError> {
    let op = stack
        .operations
        .iter_mut()
        .find(|op| op.operation_id == operation_id)
        .ok_or_else(|| StackError::UnknownOperation(operation_id.into()))?;
    op.enabled = enabled;
    // Re-enabling a row clears the `needs_preview` flag —
    // the user has consciously turned the operation back
    // on, so the next apply round should re-derive the
    // preview from the current parameters.
    if enabled {
        op.needs_preview = true;
    }
    Ok(stack)
}

fn remove(
    mut stack: EnhancementStackRecord,
    operation_id: &str,
) -> Result<EnhancementStackRecord, StackError> {
    let before = stack.operations.len();
    stack
        .operations
        .retain(|op| op.operation_id != operation_id);
    if stack.operations.len() == before {
        return Err(StackError::UnknownOperation(operation_id.into()));
    }
    Ok(stack)
}

fn mark_needs_preview(
    mut stack: EnhancementStackRecord,
    operation_id: &str,
) -> Result<EnhancementStackRecord, StackError> {
    let op = stack
        .operations
        .iter_mut()
        .find(|op| op.operation_id == operation_id)
        .ok_or_else(|| StackError::UnknownOperation(operation_id.into()))?;
    op.needs_preview = true;
    Ok(stack)
}

fn append(mut stack: EnhancementStackRecord, op: StackOperation) -> EnhancementStackRecord {
    stack.operations.push(op);
    stack
}

/// Build a new stack by copying operations `0..cutoff`
/// from an existing stack and tagging the new stack as
/// branched. The new stack gets a fresh `stack_id`; the
/// `branched_from_version_id` carries the source stack's
/// `source_image_version_id` so Compare can render the
/// lineage.
///
/// `cutoff` is 0-based and exclusive — `cutoff = 3`
/// keeps operations `[0, 1, 2]`. `cutoff >=
/// source.operations.len()` returns the source stack
/// unchanged with a fresh `stack_id` (effectively a
/// fork that re-applies the whole stack).
pub fn branch(
    source: EnhancementStackRecord,
    cutoff: usize,
    new_stack_id: String,
    new_image_version_id: String,
    now_iso: String,
) -> EnhancementStackRecord {
    let prefix: Vec<StackOperation> = source.operations.iter().take(cutoff).cloned().collect();
    EnhancementStackRecord {
        stack_id: new_stack_id,
        project_id: source.project_id.clone(),
        source_image_version_id: new_image_version_id,
        operations: prefix,
        branched_from_version_id: Some(source.source_image_version_id.clone()),
        created_at: now_iso,
    }
}

/// The subset of operations that actually fire during an
/// apply round.
///
/// Disabled rows are skipped; the rest keep their
/// ordering. The function is the source of truth for
/// "which operations will run" — the apply pipeline
/// reads it.
pub fn enabled_operations(stack: &EnhancementStackRecord) -> Vec<&StackOperation> {
    stack.operations.iter().filter(|op| op.enabled).collect()
}

/// Total number of enabled operations. Convenience for
/// the UI ("3 of 5 operations will run").
pub fn enabled_count(stack: &EnhancementStackRecord) -> usize {
    enabled_operations(stack).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(id: &str) -> StackOperation {
        StackOperation {
            operation_id: id.into(),
            recommendation_id: None,
            parameters_json: "{}".into(),
            enabled: true,
            needs_preview: false,
        }
    }

    fn stack_with(ops: Vec<StackOperation>) -> EnhancementStackRecord {
        EnhancementStackRecord {
            stack_id: "stk_1".into(),
            project_id: "p".into(),
            source_image_version_id: "v1".into(),
            operations: ops,
            branched_from_version_id: None,
            created_at: "2026-01-01 00:00:00 UTC".into(),
        }
    }

    #[test]
    fn reorder_moves_operation() {
        let s = stack_with(vec![op("a"), op("b"), op("c")]);
        let out = apply_mutation(
            s,
            StackMutation::Reorder {
                operation_id: "a".into(),
                new_index: 2,
            },
        )
        .unwrap();
        let ids: Vec<_> = out
            .operations
            .iter()
            .map(|o| o.operation_id.as_str())
            .collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn reorder_unknown_id_errors() {
        let s = stack_with(vec![op("a")]);
        let err = apply_mutation(
            s,
            StackMutation::Reorder {
                operation_id: "missing".into(),
                new_index: 0,
            },
        )
        .unwrap_err();
        assert_eq!(err, StackError::UnknownOperation("missing".into()));
    }

    #[test]
    fn reorder_out_of_bounds_errors() {
        let s = stack_with(vec![op("a"), op("b")]);
        let err = apply_mutation(
            s,
            StackMutation::Reorder {
                operation_id: "a".into(),
                new_index: 99,
            },
        )
        .unwrap_err();
        assert!(matches!(err, StackError::IndexOutOfBounds { .. }));
    }

    #[test]
    fn set_enabled_toggles_and_marks_needs_preview_on_re_enable() {
        let s = stack_with(vec![op("a")]);
        // Disable.
        let out = apply_mutation(
            s.clone(),
            StackMutation::SetEnabled {
                operation_id: "a".into(),
                enabled: false,
            },
        )
        .unwrap();
        assert!(!out.operations[0].enabled);
        assert!(!out.operations[0].needs_preview);
        // Re-enable.
        let out2 = apply_mutation(
            out,
            StackMutation::SetEnabled {
                operation_id: "a".into(),
                enabled: true,
            },
        )
        .unwrap();
        assert!(out2.operations[0].enabled);
        assert!(out2.operations[0].needs_preview);
    }

    #[test]
    fn remove_drops_op() {
        let s = stack_with(vec![op("a"), op("b"), op("c")]);
        let out = apply_mutation(
            s,
            StackMutation::Remove {
                operation_id: "b".into(),
            },
        )
        .unwrap();
        let ids: Vec<_> = out
            .operations
            .iter()
            .map(|o| o.operation_id.as_str())
            .collect();
        assert_eq!(ids, vec!["a", "c"]);
    }

    #[test]
    fn remove_unknown_errors() {
        let s = stack_with(vec![op("a")]);
        let err = apply_mutation(
            s,
            StackMutation::Remove {
                operation_id: "missing".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, StackError::UnknownOperation("missing".into()));
    }

    #[test]
    fn append_adds_to_end() {
        let s = stack_with(vec![op("a")]);
        let out = apply_mutation(s, StackMutation::Append(op("b"))).unwrap();
        assert_eq!(out.operations.len(), 2);
        assert_eq!(out.operations[1].operation_id, "b");
    }

    #[test]
    fn branch_copies_prefix_and_records_lineage() {
        let s = stack_with(vec![op("a"), op("b"), op("c")]);
        let b = branch(
            s,
            2,
            "stk_2".into(),
            "v2_branch".into(),
            "2026-01-01 00:00:00 UTC".into(),
        );
        let ids: Vec<_> = b
            .operations
            .iter()
            .map(|o| o.operation_id.as_str())
            .collect();
        assert_eq!(ids, vec!["a", "b"]);
        assert_eq!(b.branched_from_version_id.as_deref(), Some("v1"));
        assert_eq!(b.source_image_version_id, "v2_branch");
    }

    #[test]
    fn branch_full_cutoff_copies_everything() {
        let s = stack_with(vec![op("a"), op("b")]);
        let b = branch(
            s.clone(),
            99,
            "stk_2".into(),
            "v2".into(),
            "2026-01-01 00:00:00 UTC".into(),
        );
        assert_eq!(b.operations.len(), s.operations.len());
    }

    #[test]
    fn enabled_operations_skips_disabled() {
        let s = stack_with(vec![op("a"), op("b"), op("c")]);
        let s = apply_mutation(
            s,
            StackMutation::SetEnabled {
                operation_id: "b".into(),
                enabled: false,
            },
        )
        .unwrap();
        let enabled: Vec<_> = enabled_operations(&s)
            .into_iter()
            .map(|o| o.operation_id.as_str())
            .collect();
        assert_eq!(enabled, vec!["a", "c"]);
        assert_eq!(enabled_count(&s), 2);
    }

    #[test]
    fn mark_needs_preview_only_sets_flag() {
        let s = stack_with(vec![op("a"), op("b")]);
        let out = apply_mutation(
            s,
            StackMutation::MarkNeedsPreview {
                operation_id: "b".into(),
            },
        )
        .unwrap();
        assert!(!out.operations[0].needs_preview);
        assert!(out.operations[1].needs_preview);
    }

    #[test]
    fn operations_round_trip_via_json() {
        let s = stack_with(vec![op("a"), op("b")]);
        let json = EnhancementStackRecord::operations_to_json(&s.operations);
        let back = EnhancementStackRecord::operations_from_json(&json);
        assert_eq!(back, s.operations);
    }

    #[test]
    fn malformed_json_falls_back_to_empty_list() {
        let back = EnhancementStackRecord::operations_from_json("not json {");
        assert!(back.is_empty());
    }
}
