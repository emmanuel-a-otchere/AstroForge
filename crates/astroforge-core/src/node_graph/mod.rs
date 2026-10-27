//! CR-10 P1.6.2.4 / Slice P1.6.2.4 : Node graph mutation module.
//!
//! Houses the in-process primitives consumed by the Tauri IPC surface
//! (`insert_stage`, `remove_stage`, `reorder_stage`, `set_stage_enabled`)
//! and the tests. This slice ships `insert_stage` only; the remaining
//! primitives land in P1.6.3.1 per
//! `docs/plans/2026-09-28-cr10-node-based-editor/PLAN.md`.
//!
//! The primitives are pure functions over `PipelinePlan` + a small
//! `PipelinePlanStore` handle for persistence; they do not touch the
//! TS state, the Svelte stores, or the IPC layer. Tests exercise the
//! primitives directly via the in-memory store constructor.

pub mod mutation;
