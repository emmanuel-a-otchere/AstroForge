pub mod hardware;
pub mod hub;
pub mod models;
pub mod registry;
pub mod service;
pub mod tiling;

pub use hardware::{GpuBackend, HardwareProbe, QualityTier};
pub use hub::{get_model, get_models_for_stage, model_catalog, ModelInfo};
pub use models::*;
pub use registry::ModelRegistry;
pub use service::{
    analyse_image, default_service, dispatch, suggest_heuristic, AIError, AIService,
    AnalysisResult, ChannelStats, DefaultAIService, ParamProposal, ParamRecommendation,
    PathSelection, ProcessedImage, ProgressReporter,
};
pub use tiling::{run_tiled_inference, Tile, TileConfig};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
