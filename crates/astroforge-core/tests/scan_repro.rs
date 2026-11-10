// Quick headless repro of ingest_scan_directory's body
#[test]
fn scan_sample_dataset_repro() {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("../../tests/fixtures/sample-dataset");
    let paths = astroforge_core::import_scan::scan_directory(dir).expect("scan_directory");
    let supported: Vec<_> = paths
        .into_iter()
        .filter(|p| astroforge_core::import_scan::is_supported(p))
        .collect();
    println!("discovered+supported: {}", supported.len());
    for p in &supported {
        let bytes = std::fs::read(p).expect("read");
        let frame = astroforge_core::import_scan::classify_frame(p, &bytes);
        println!(
            "  {} -> {:?}",
            p.file_name().unwrap().to_string_lossy(),
            frame.frame_type
        );
    }
    assert_eq!(supported.len(), 22);
}
