//! RT-3 (#421) / DP#4 close-out: the bundled-model license + digest
//! manifest is verified against the actual bytes on disk.
//!
//! Every ONNX file shipped in `crates/astroforge-ai/models/builtin/`
//! must have a MANIFEST.json entry with a pinned SHA-256 digest and an
//! explicitly declared permissive license. This test fails CI when:
//!
//! 1. A model file exists on disk without a manifest entry (untracked
//!    binary shipped into the bundle).
//! 2. A manifest entry points at a missing file (stale manifest).
//! 3. A file's bytes do not hash to the pinned digest (model tampered
//!    with, or regenerated without re-pinning the manifest).
//! 4. A declared license is not in the permissive allow-list.

use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ManifestModel {
    file: String,
    sha256: String,
    #[allow(dead_code)]
    size_bytes: u64,
    license: String,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    manifest_version: u64,
    #[allow(dead_code)]
    default_license: String,
    models: Vec<ManifestModel>,
}

const PERMISSIVE_LICENSES: [&str; 5] = [
    "MIT",
    "Apache-2.0",
    "MIT OR Apache-2.0",
    "BSD-3-Clause",
    "ISC",
];

fn builtin_dir() -> std::path::PathBuf {
    let crate_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set during cargo test");
    Path::new(&crate_dir).join("models/builtin")
}

fn sha256_hex(data: &[u8]) -> String {
    // sha2 is already a dependency of astroforge-ai; reuse it here.
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn builtin_models_manifest_pins_every_file() {
    let manifest_path = builtin_dir().join("MANIFEST.json");
    let raw = std::fs::read_to_string(&manifest_path)
        .expect("MANIFEST.json exists next to the bundled models");
    let manifest: Manifest =
        serde_json::from_str(&raw).expect("MANIFEST.json parses as the expected schema");

    assert!(
        manifest.manifest_version >= 1,
        "manifest version must be present"
    );

    // 1. No untracked model binaries.
    let on_disk: BTreeSet<String> = std::fs::read_dir(builtin_dir())
        .expect("builtin models directory exists")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "onnx").unwrap_or(false))
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .collect();
    let pinned: BTreeSet<&str> = manifest.models.iter().map(|m| m.file.as_str()).collect();

    for f in &on_disk {
        assert!(
            pinned.contains(f.as_str()),
            "untracked model binary on disk: {f} — add it to MANIFEST.json with its SHA-256"
        );
    }
    for f in &pinned {
        assert!(
            on_disk.contains(*f),
            "manifest entry points at a missing file: {f} — re-pin the manifest"
        );
    }

    // 2. Digests + licenses verified per entry.
    for m in &manifest.models {
        let bytes = std::fs::read(builtin_dir().join(&m.file))
            .unwrap_or_else(|e| panic!("read {}: {e}", m.file));
        let actual = sha256_hex(&bytes);
        assert_eq!(
            actual, m.sha256,
            "digest drift for {}: regenerate and re-pin MANIFEST.json (run scripts/generate_builtin_models.py, then sha256sum)",
            m.file
        );
        assert_eq!(
            bytes.len() as u64,
            m.size_bytes,
            "size_bytes drift for {}",
            m.file
        );
        assert!(
            PERMISSIVE_LICENSES.contains(&m.license.as_str()),
            "non-permissive license '{}' declared for {} — DP#4 requires permissive licenses for bundled models",
            m.license,
            m.file
        );
    }

    // 3. Sanity: the bundle actually contains the five CR-06 P5.1 kernels.
    assert!(
        on_disk.len() >= 5,
        "expected the five builtin classical kernels, found {on_disk:?}"
    );
}
