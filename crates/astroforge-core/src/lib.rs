pub mod artifact;
pub mod calibration;
pub mod db;
pub mod export;
pub mod fits;
pub mod image;
pub mod ingest;
pub mod mvp_pipeline;
pub mod orchestrator;
pub mod pipeline;
pub mod registration;
pub mod stacking;
pub mod stretching;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
