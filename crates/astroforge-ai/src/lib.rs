pub mod hardware;
pub mod hub;
pub mod inference;
pub mod models;
pub mod operations;
pub mod recommendations;
pub mod registry;
pub mod service;
pub mod target_classify;
pub mod tiling;

pub use hardware::{GpuBackend, HardwareProbe, QualityTier};
pub use hub::{get_model, get_models_for_stage, model_catalog, ModelInfo};
pub use inference::{
    builtin_model, BuiltinInputKind, BuiltinModel, InferenceError, OnnxEngine, BUILTIN_MODELS,
};
pub use models::*;
pub use operations::{
    dispatch_operation, get_operation, model_binding, registry as operations_registry,
    DispatchInputs, DispatchResult, OperationCategory, OperationError, OperationInfo,
    OperationModelBinding, OperationOutcome, SafetyClassification,
};
pub use recommendations::{
    analyze, analyze_at, flatten_for_store, AiRecommendation, ModelCandidate, RecommendationReport,
    RecommendationSet, SequencingNote,
};
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
