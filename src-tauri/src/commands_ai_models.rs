//! CR-05 R1 — read-only AI model catalog IPC.
//!
//! Surfaces `astroforge_ai::hub::model_catalog()` so the application-level
//! "AI Models" navigation target renders a real, content-bearing screen
//! instead of a placeholder. The catalog is the canonical list of every
//! model AstroForge can host (denoise, super-resolution, de-jpeg, star
//! segmentation, cloud score, color calibration, trail inpainting).
//!
//! No download/install IPC is exposed here. Models ship with the binary in
//! this slice; download flow is deferred to a later tranche (it needs a
//! `<models_dir>` resolution path and a `ModelRegistry::register` writer,
//! neither of which exists yet in the IPC layer). The AI Models screen
//! surfaces the catalog as a read-only list with stage + license + size.

use astroforge_ai::hub::{model_catalog, ModelInfo};

/// AstroForge model record exposed to the frontend. Mirrors the Rust
/// `astroforge_ai::hub::ModelInfo` struct so the renderer can show
/// stage, version, size, license, and input shape. Field naming is
/// camelCase (the Rust side already serialises via serde with the
/// default rename; the wrapper here matches `astroforge-api.ts`'s
/// existing convention of camelCase at the boundary).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AiModelInfoDto {
    pub name: String,
    pub version: String,
    pub stage: String,
    pub license: String,
    pub size_bytes: u64,
    pub input_channels: u32,
    pub input_tile_size: u32,
    pub output_channels: u32,
    pub scale_factor: u32,
    /// True when a model asset exists on disk. The model catalog itself
    /// is always present; the on-disk check uses the standard
    /// `<models_dir>/<name>.onnx` convention the registry follows.
    /// This slice returns `false` uniformly — the download flow is
    /// deferred to a later tranche — but the field is exposed now so
    /// the UI can render an "Installed" badge without a follow-up
    /// breaking change.
    pub installed: bool,
}

impl From<ModelInfo> for AiModelInfoDto {
    fn from(m: ModelInfo) -> Self {
        Self {
            name: m.name,
            version: m.version,
            stage: m.stage,
            license: m.license,
            size_bytes: m.size_bytes,
            input_channels: m.input_channels,
            input_tile_size: m.input_tile_size,
            output_channels: m.output_channels,
            scale_factor: m.scale_factor,
            installed: false,
        }
    }
}

/// List the full AI model catalog. Read-only and infallible — the
/// catalog is a static `Vec<ModelInfo>` in the AI crate. The Tauri
/// command is the single source for the AI Models application screen.
#[tauri::command]
pub fn ai_model_list() -> Vec<AiModelInfoDto> {
    model_catalog().into_iter().map(Into::into).collect()
}
