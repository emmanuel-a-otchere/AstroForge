pub mod hardware;
pub mod hub;
pub mod models;
pub mod registry;
pub mod tiling;

pub use hardware::{GpuBackend, HardwareProbe, QualityTier};
pub use hub::{get_model, get_models_for_stage, model_catalog, ModelInfo};
pub use models::*;
pub use registry::ModelRegistry;
pub use tiling::{run_tiled_inference, TileConfig, Tile};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
