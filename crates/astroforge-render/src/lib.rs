//! AstroForge render engine — wgpu-backed display layer for the egui
//! desktop app (`astroforge-ui`).
//!
//! Scope (migration P1; see docs in the egui-migration plan):
//! - Float texture upload (16-bit TIFF previews -> rgba16float)
//! - MTF stretch render, reusing the canonical math in
//!   `astroforge_core::stretching` (the GLSL `shaders.ts` layer is NOT
//!   ported — the Rust side is the source of truth)
//! - Bilinear resampling + zoom/pan, mirroring `image-canvas-webgl.ts`
//! - Before/after + per-pixel diff, reusing the WGSL from
//!   `src/lib/webgpu-shaders.ts` verbatim (wgpu consumes it unchanged)
//! - CPU fallback path (egui `ColorImage`) for machines without wgpu
//!
//! Intentionally empty for now: this crate exists so the workspace
//! wiring lands early and later slices have a stable home.
