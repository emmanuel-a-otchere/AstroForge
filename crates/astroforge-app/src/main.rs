fn main() {
    println!(
        "AstroForge {} — core v{}, ai v{}",
        env!("CARGO_PKG_VERSION"),
        astroforge_core::version(),
        astroforge_ai::version(),
    );
}
