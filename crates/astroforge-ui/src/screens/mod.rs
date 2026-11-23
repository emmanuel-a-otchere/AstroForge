//! Project workspaces (migration P3). Each module draws one
//! workspace over the shared project context; screens consume the
//! api layer only — no store access in UI code.

pub mod compare;
pub mod enhance;
pub mod export_w;
pub mod gallery;
pub mod import_w;
pub mod overview;
pub mod process;
pub mod recipes;
