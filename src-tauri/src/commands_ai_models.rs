//! CR-05 R1 — AI model catalog IPC + RT-7 (#426) install flow.
//!
//! Surfaces `astroforge_ai::hub::model_catalog()` so the application-level
//! "AI Models" navigation target renders the canonical catalog (denoise,
//! super-resolution, de-jpeg, star segmentation, cloud score, color
//! calibration, trail inpainting), and installs pinned catalog models on
//! demand: `ai_model_download` fetches the artifact, verifies it against
//! the pinned SHA-256 fail-closed (`astroforge_ai::downloader`), and
//! writes it to `<home>/AstroForge/models/<id>.onnx` atomically.
//!
//! `installed` reflects on-disk presence; integrity is enforced at
//! download time and again when a session opens the model
//! (`inference::OnnxEngine::open_verified`), so a tampered file can never
//! execute.

use astroforge_ai::downloader::{download_spec, install_model, HttpByteSource};
use astroforge_ai::hub::{model_catalog, ModelInfo};
use std::path::PathBuf;

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
    /// Runtime-download URL for installable models; `None` for
    /// unannounced placeholders.
    pub download_url: Option<String>,
    /// True when a model artifact exists on disk at the standard
    /// `<models_dir>/<name>.onnx` path.
    pub installed: bool,
}

/// The user's models directory (`<home>/AstroForge/models`), matching
/// the path the enhancement pipeline resolves for
/// `ModelRegistry::new`. Returns `None` when no home directory is
/// set (embedded/kiosk environments) — catalog listing still works,
/// installs do not.
fn models_dir() -> Option<PathBuf> {
    crate::commands_ai_enhancement::dirs_home().map(|mut p| {
        p.push("AstroForge");
        p.push("models");
        p
    })
}

impl AiModelInfoDto {
    fn from_info_with_disk(m: ModelInfo) -> Self {
        let installed = models_dir()
            .map(|dir| dir.join(format!("{}.onnx", m.name)).is_file())
            .unwrap_or(false);
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
            download_url: m.download_url,
            installed,
        }
    }
}

impl From<ModelInfo> for AiModelInfoDto {
    fn from(m: ModelInfo) -> Self {
        Self::from_info_with_disk(m)
    }
}

/// List the full AI model catalog. Read-only and infallible — the
/// catalog is a static `Vec<ModelInfo>` in the AI crate. The Tauri
/// command is the single source for the AI Models application screen.
#[tauri::command]
pub fn ai_model_list() -> Vec<AiModelInfoDto> {
    model_catalog()
        .into_iter()
        .map(AiModelInfoDto::from_info_with_disk)
        .collect()
}

/// Install a catalog model by id: fetch from the pinned URL, verify
/// the exact bytes against the pinned SHA-256 (fail-closed — a
/// mismatched artifact is discarded, never loaded), and write it
/// atomically into the models directory. Returns the installed path.
#[tauri::command]
pub fn ai_model_download(name: String) -> Result<String, String> {
    let spec = download_spec(&name)
        .ok_or_else(|| format!("no downloadable artifact for model '{name}'"))?;
    let dir = models_dir()
        .ok_or_else(|| "home directory unavailable; cannot resolve models dir".to_string())?;
    let path = install_model(&dir, spec, &HttpByteSource).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}
