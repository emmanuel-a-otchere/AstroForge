pub mod artifact;
pub mod db;
pub mod image;
pub mod orchestrator;
pub mod pipeline;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
