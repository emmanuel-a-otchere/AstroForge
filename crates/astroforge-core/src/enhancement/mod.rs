//! CR-06 P4 — Enhancement engine (stack + preview).
//!
//! Public surface for the stack engine (CR-06 §22) and
//! preview state machine (CR-06 §15). See the
//! sub-modules for details.

pub mod preview;
pub mod stack;

pub use preview::{
    mark_completed, mark_failed, mark_rendering, parse_status, produce_placeholder_preview,
    EnhancementPreviewRecord, PlaceholderPreview, PreviewStatus,
};
pub use stack::{
    apply_mutation, branch, enabled_count, enabled_operations, EnhancementStackRecord, StackError,
    StackMutation, StackOperation,
};
