//! AstroForge egui desktop front-end (bin `astroforge-ui`).
//!
//! Entry point: init logging, open the shared store set, hand services
//! to the shell. See the crate docs for the migration slice map.

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    if let Err(e) = astroforge_ui::run() {
        log::error!("{e}");
        std::process::exit(1);
    }
}
