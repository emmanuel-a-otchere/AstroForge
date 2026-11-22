//! AstroForge render engine — display pipeline for the egui desktop
//! app (`astroforge-ui`).
//!
//! Slice P1 (CPU path): artifact bytes -> decode -> tone map ->
//! `egui::ColorImage`, with pan/zoom interaction handled by the
//! `image_canvas` component in the UI crate. The math is NOT
//! re-implemented here: decoding delegates to `astroforge_core::
//! decoders`, tone mapping to `astroforge_core::stretching` (the
//! canonical sources behind the web app's GLSL/WGSL layers).
//!
//! Later slices:
//! - wgpu path: rgba16float textures + the WGSL compute shaders from
//!   `src/lib/webgpu-*.ts` for interactive diff/compare, painted into
//!   egui via `CallbackTrait` hooks.
//! - Histogram rendering (Solar Flare Red curve on darkest neutral,
//!   per DESIGN.md).

pub mod color;
pub mod demo;
pub mod source;
pub mod tone;

pub use color::{prepare_display, to_color_image};
pub use source::decode_image;
pub use tone::ToneMap;
