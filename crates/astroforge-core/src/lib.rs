pub mod artifact;
pub mod background;
pub mod calibration;
pub mod color_calibration;
pub mod cosmetic;
pub mod crop;
pub mod db;
pub mod debayer;
pub mod detail_enhancement;
pub mod export;
pub mod fits;
pub mod image;
pub mod ingest;
pub mod mvp_pipeline;
pub mod narrowband;
pub mod orchestrator;
pub mod pipeline;
pub mod quality;
pub mod registration;
pub mod stacking;
pub mod star_segmentation;
pub mod stretching;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
