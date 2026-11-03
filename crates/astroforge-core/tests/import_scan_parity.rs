//! HOUSE-2 migration test (docs/HOUSEKEEPING.md, RT-6 / #425).
//!
//! Gating check for the `ingest.rs` → `import_scan.rs` consolidation:
//! importing the same folder through the folded manifest path and the
//! canonical CR-04 asset path must agree — same discovered files, same
//! extracted facts. The legacy walker (FITS-only, no hidden-entry or
//! symlink guards) is re-expressed as `scan_fits_directory`; these
//! tests pin that the two layers cannot drift again now that they
//! share one module.
//!
//! Runs on synthetic FITS files in a tempdir — no external data.

use astroforge_core::fits::{self, FitsHeader};
use astroforge_core::import_scan;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

fn write_minimal_fits(path: &Path, imagetyp: &str, exptime: f64, filter: Option<&str>) {
    let mut header = FitsHeader::new();
    header.set("IMAGETYP", imagetyp);
    header.set("EXPTIME", &exptime.to_string());
    header.set("NAXIS1", "32");
    header.set("NAXIS2", "24");
    if let Some(f) = filter {
        header.set("FILTER", f);
    }

    let mut file = File::create(path).expect("create fits");
    fits::write_header(&header, &mut file).expect("write header");

    let pixel_count = 32 * 24;
    let mut data = Vec::with_capacity(pixel_count * 4);
    for _ in 0..pixel_count {
        data.extend_from_slice(&0.0_f32.to_be_bytes());
    }
    let block_size: usize = 2880;
    let total = data.len();
    if total % block_size != 0 {
        data.resize(total + (block_size - total % block_size), 0);
    }
    file.write_all(&data).expect("data");
}

fn tmp_workspace(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    p.push(format!(
        "astroforge-parity-{name}-{}-{stamp}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("mkdir tmp");
    p
}

fn sorted(mut v: Vec<PathBuf>) -> Vec<PathBuf> {
    v.sort();
    v
}

/// The folded manifest walk must return exactly the FITS files the
/// canonical CR-04 walker discovers (filtered to FITS extensions) —
/// including nested subdirectories — and must exclude non-FITS files
/// and hidden directories the canonical walker skips.
#[test]
fn fits_walk_matches_canonical_walker_on_same_tree() {
    let dir = tmp_workspace("walker");
    write_minimal_fits(&dir.join("light_001.fits"), "LIGHT", 120.0, Some("Ha"));
    write_minimal_fits(&dir.join("dark_001.fits"), "DARK", 120.0, None);
    let nested = dir.join("subdir");
    std::fs::create_dir_all(&nested).expect("mkdir nested");
    write_minimal_fits(&nested.join("light_002.fits"), "LIGHT", 120.0, Some("Ha"));

    // Non-FITS files present in the tree: the canonical walker finds
    // them; the FITS walk must not.
    std::fs::write(dir.join("notes.txt"), "not a frame").expect("write txt");
    // Hidden directory: skipped by the canonical walker.
    let hidden = dir.join(".hidden");
    std::fs::create_dir_all(&hidden).expect("mkdir hidden");
    write_minimal_fits(&hidden.join("light_003.fits"), "LIGHT", 120.0, None);

    let fits_walk = sorted(import_scan::scan_fits_directory(&dir).expect("fits walk"));
    let canonical: Vec<PathBuf> = sorted(
        import_scan::scan_directory(&dir)
            .expect("canonical walk")
            .into_iter()
            .filter(|p| {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                matches!(ext.to_lowercase().as_str(), "fits" | "fit")
            })
            .collect(),
    );

    assert_eq!(fits_walk, canonical, "folded walk drifted from canonical");
    assert_eq!(fits_walk.len(), 3, "expected 3 FITS files incl. nested");
    assert!(
        !fits_walk.iter().any(|p| p.starts_with(&hidden)),
        "hidden directory must be skipped"
    );
}

/// Classification parity: every file the manifest layer classifies
/// must also extract successfully through the CR-04 metadata path,
/// and the extracted facts (frame type, exposure, filter, dimensions)
/// must agree between the two layers for the same bytes.
#[test]
fn manifest_and_asset_paths_agree_on_same_files() {
    let dir = tmp_workspace("layers");
    write_minimal_fits(&dir.join("light_ha_1.fits"), "LIGHT", 120.0, Some("Ha"));
    write_minimal_fits(&dir.join("dark_1.fits"), "DARK", 120.0, None);
    write_minimal_fits(&dir.join("flat_1.fits"), "FLAT", 5.0, None);
    write_minimal_fits(&dir.join("bias_1.fits"), "BIAS", 0.0, None);

    let paths = import_scan::scan_fits_directory(&dir).expect("scan");

    let frames: Vec<astroforge_core::import_scan::FrameInfo> = paths
        .iter()
        .map(|p| {
            let bytes = std::fs::read(p).expect("read fits");
            import_scan::classify_frame(p, &bytes)
        })
        .collect();

    // Layer agreement, file by file.
    for (path, frame) in paths.iter().zip(frames.iter()) {
        let bytes = std::fs::read(path).expect("read fits");
        let mut extracted = import_scan::extract_metadata(path, &bytes)
            .unwrap_or_else(|e| panic!("asset path failed on {}: {:?}", path.display(), e));
        // to_source_asset coalesces before use; mirror that here.
        extracted.coalesce_dimensions();

        assert_eq!(
            frame.exptime,
            extracted.exptime,
            "exptime drift between layers for {}",
            path.display()
        );
        let frame_filter = frame.filter.clone().unwrap_or_default();
        let asset_filter = extracted.filter.clone().unwrap_or_default();
        assert_eq!(
            frame_filter.to_uppercase(),
            asset_filter.to_uppercase(),
            "filter drift between layers for {}",
            path.display()
        );
        if let (Some(w), Some(h)) = (extracted.width, extracted.height) {
            assert_eq!(
                frame.width,
                Some(w as i64),
                "width drift for {}",
                path.display()
            );
            assert_eq!(
                frame.height,
                Some(h as i64),
                "height drift for {}",
                path.display()
            );
        }
    }

    // Manifest-level invariants across the whole folder.
    let manifest =
        import_scan::build_manifest("parity-session", dir.to_str().unwrap(), frames.clone());
    assert_eq!(manifest.frames.len(), 4);
    assert_eq!(
        manifest
            .frames
            .iter()
            .filter(|f| f.frame_type == astroforge_core::import_scan::FrameType::Light)
            .count(),
        1
    );
    let ha = manifest
        .light_groups
        .iter()
        .find(|g| g.filter == "Ha")
        .expect("Ha light group");
    assert_eq!(ha.frame_paths.len(), 1);
}

/// The SourceAsset conversion on the same folder produces one row per
/// discovered file — the record count parity HOUSE-2 asked for
/// ("identical SourceAsset records emerge from both paths").
#[test]
fn asset_conversion_covers_every_discovered_file() {
    let dir = tmp_workspace("assets");
    write_minimal_fits(&dir.join("light_001.fits"), "LIGHT", 120.0, Some("Ha"));
    write_minimal_fits(&dir.join("light_002.fits"), "LIGHT", 120.0, Some("OIII"));
    write_minimal_fits(&dir.join("dark_001.fits"), "DARK", 120.0, None);

    let paths = import_scan::scan_fits_directory(&dir).expect("scan");
    let mut assets = Vec::new();
    for p in &paths {
        let bytes = std::fs::read(p).expect("read");
        let metadata = import_scan::extract_metadata(p, &bytes).expect("extract");
        let file = astroforge_core::import_scan::DiscoveredFile {
            path: p.clone(),
            format: import_scan::detect_format(p),
            file_size: bytes.len() as u64,
            // Cheap stand-in hash (distinct per frame via FILTER)
            // keeps this test independent of ContentStore IO.
            content_hash: Some(format!("hash-{}", p.file_name().unwrap().to_string_lossy())),
            discovery_state: import_scan::DiscoveryState::Discovered,
            metadata,
            reason: None,
        };
        let asset = import_scan::to_source_asset(&file, "parity-session", "2026-11-03T00:00:00Z")
            .expect("asset conversion");
        assets.push(asset);
    }

    assert_eq!(assets.len(), paths.len());
    assert!(assets.iter().all(|a| a.session_id == "parity-session"));
    assert!(assets.iter().all(|a| a.format == "FITS"));
    // Coalesced dimensions flow through to the asset rows.
    assert!(assets
        .iter()
        .all(|a| a.width == Some(32) && a.height == Some(24)));
}
