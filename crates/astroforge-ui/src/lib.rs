//! AstroForge egui desktop shell — the Rust-native UI that eventually
//! replaces the Tauri + Svelte app (`src/` + `src-tauri/`).
//!
//! Migration slices (strangler pattern; the Tauri app stays the
//! shipping app until this crate reaches feature parity):
//! - P0 (this slice): eframe shell, nav rail, dark theme + bundled
//!   fonts, `Services` open path mirroring the Tauri managed state,
//!   and the `api::project` slice (list / get / create) calling
//!   `astroforge-core` directly — no IPC.
//! - P1: `astroforge-render` wgpu display engine (image canvas, MTF
//!   stretch, before/after + diff).
//! - P2: full chrome (keyboard shortcuts, dialogs, drag-drop).
//! - P3+: workspaces in dependency order.
//!
//! The data directory is the same one the Tauri app uses
//! (`<system data dir>/com.astroforge.app`), so both apps read and
//! write the same projects / recipes / gallery until cutover.

pub mod api;
pub mod app;
pub mod fonts;
pub mod image_canvas;
pub mod semantics;
pub mod services;
pub mod theme;
pub mod tokens;

use std::sync::Arc;

/// Resolve the data directory: `ASTROFORGE_DATA_DIR` override, else
/// the Tauri-equivalent `<system data dir>/com.astroforge.app` so both
/// front-ends share one store set during the migration window.
pub fn default_data_dir() -> Result<std::path::PathBuf, String> {
    if let Ok(dir) = std::env::var("ASTROFORGE_DATA_DIR") {
        if !dir.is_empty() {
            return Ok(std::path::PathBuf::from(dir));
        }
    }
    dirs::data_dir()
        .map(|d| d.join("com.astroforge.app"))
        .ok_or_else(|| "could not resolve the user data directory".to_string())
}

/// Open the stores and run the eframe event loop. Errors before the
/// loop starts (store open failures, bad data dir) are returned;
/// errors inside eframe are logged and mapped to exit code 1 by the
/// caller.
pub fn run() -> Result<(), String> {
    let data_dir = default_data_dir()?;
    let services = Arc::new(services::Services::open(&data_dir)?);
    log::info!("stores opened under {}", data_dir.display());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("AstroForge")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AstroForge",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::AstroForgeApp::new(cc, services)))),
    )
    .map_err(|e| format!("eframe failed: {e}"))
}
