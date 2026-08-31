pub mod artifact;
pub mod calibration;
pub mod db;
pub mod fits;
pub mod image;
pub mod ingest;
pub mod orchestrator;
pub mod pipeline;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
