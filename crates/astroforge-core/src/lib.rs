pub mod image;
pub mod pipeline;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
