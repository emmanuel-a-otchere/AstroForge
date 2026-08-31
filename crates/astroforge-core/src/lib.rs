pub mod artifact;
pub mod background;
pub mod bayer_detection;
pub mod calibration;
pub mod color_calibration;
pub mod cosmetic;
pub mod crop;
pub mod db;
pub mod debayer;
pub mod dialog_modes;
pub mod dip;
pub mod dng_parser;
pub mod export;
pub mod fits;
pub mod image;
pub mod ingest;
pub mod mvp_pipeline;
pub mod narrowband;
pub mod orchestrator;
pub mod planetary_drizzle;
pub mod planetary_features;
pub mod planetary_lucky;
pub mod planetary_pipeline;
pub mod planetary_routing;
pub mod pipeline;
pub mod plate_solve;
pub mod quality;
pub mod recipe;
pub mod registration;
pub mod session;
pub mod stacking;
pub mod star_segmentation;
pub mod stretching;
pub mod telemetry;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
