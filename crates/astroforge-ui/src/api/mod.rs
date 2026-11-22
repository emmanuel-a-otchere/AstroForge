//! Direct-call API layer — the egui replacement for
//! `src/lib/astroforge-api.ts` (124 `invoke()` wrappers).
//!
//! One module per command group, mirroring the TS file's group
//! order. Each function is a thin wrapper over `astroforge-core`
//! calls, with bodies ported from the corresponding Tauri command so
//! behavior stays identical during the migration window. Errors are
//! `String` (same message text the Tauri commands surface) so screens
//! can render one consistent error path; a later slice may introduce
//! a typed error enum.
//!
//! Parity contract (P5 audit): every `export const` in
//! `astroforge-api.ts` maps 1:1 to a function here in the module of
//! the same group; `scripts/parity_audit.py` enforces it.

pub mod ai;
pub mod comparison;
pub mod export;
pub mod gallery;
pub mod import;
pub mod node_graph;
pub mod pipeline_plan;
pub mod preview;
pub mod project;
pub mod quality;
pub mod recipes;
pub mod resource;
pub mod versions;
