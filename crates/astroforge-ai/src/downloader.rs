//! RT-7 (#426) — runtime download of pinned catalog models.
//!
//! Catalog models (SwinIR-class denoise, Swin2SR super-resolution) do
//! not ship with the binary or the installers; they are fetched on
//! demand into the user's models directory (`~/AstroForge/models`)
//! and verified **fail-closed** against the SHA-256 pinned in
//! [`DOWNLOAD_SPECS`] before anything is installed: a byte that does
//! not match the pinned digest is discarded and surfaced as an error,
//! never loaded into a session.
//!
//! The pinned digests double as the DP#4 license close-out: every
//! downloadable spec carries an explicit SPDX license that must match
//! the corresponding `CATALOG_MODELS` entry (pinned by test — the two
//! tables cannot drift).
//!
//! Network access is isolated behind [`ByteSource`] so the offline
//! test suite can exercise the full install path (fetch → verify →
//! atomic write → re-verify) without a socket. The production
//! [`HttpByteSource`] uses `ureq` (rustls); the network-gated e2e
//! (`tests/model_download_e2e.rs`, env `ASTROFORGE_MODEL_E2E=1`)
//! exercises the real URLs.

use crate::inference::LicenseSpdx;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// One downloadable catalog model: where to fetch it, the pinned
/// digest of the exact bytes, and the SPDX license of those bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadSpec {
    pub id: &'static str,
    pub url: &'static str,
    /// SHA-256 of the artifact. Fail-closed: the installer refuses
    /// any byte stream whose digest differs.
    pub sha256: &'static str,
    /// Exact byte length, checked before the digest (fast fail on
    /// truncated transfers and HTML error pages).
    pub size_bytes: u64,
    /// SPDX license of the fetched weights. Must match the license
    /// on the `CATALOG_MODELS` entry with the same id.
    pub license: LicenseSpdx,
    /// Tensor metadata for the inference adapter (documented by the
    /// publisher's model card; asserted by the e2e).
    pub input_name: &'static str,
    pub output_name: &'static str,
    /// Fixed spatial tile the graph accepts (0 = dynamic).
    pub tile_size: u32,
    /// Output spatial multiplier relative to the (padded) input.
    /// 1 for same-size graphs (denoise-class: the adapter crops the
    /// output back to the input dims); >1 for super-resolution
    /// graphs (the adapter returns the full upscaled output — see
    /// `OnnxEngine::run_nchw_tile`). Must be paired with
    /// `tile_size == 0`: a tiled SR stitch needs 4x-output blending,
    /// which the tiler does not do (#458 keeps SR whole-image).
    pub output_scale: u32,
}

/// The downloadable catalog. Sourced and licensed:
///
/// - `swinir-denoise-astro` — SwinIR-M color denoise (σ=25), ONNX
///   re-export of the upstream `JingyunLiang/SwinIR` v0.0 checkpoint,
///   Apache-2.0. Input `image` `[b,3,128,128]` float32 RGB [0,1] →
///   output `denoised`, opset 17. Larger images tile at 128×128 with
///   overlap (the card's recipe; matches `tiling::TileConfig`).
/// - `swin2sr-realworld-x4` — Swin2SR lightweight real-world ×4 SR,
///   int8-quantized ONNX from the official `onnx-community` export of
///   `caidas/swin2SR-realworld-sr-x4-64-bsrgan-psnr`, Apache-2.0.
pub const DOWNLOAD_SPECS: &[ModelDownloadSpec] = &[
    ModelDownloadSpec {
        id: "swinir-denoise-astro",
        url: "https://huggingface.co/Heliosoph/swinir-onnx/resolve/main/swinir_denoising_color_25.onnx",
        sha256: "5fb427215c6a893b4c4aa27b6ab0aad66742fb9cb7ab24f65eff27b09cc2ea74",
        size_bytes: 54_860_607,
        license: LicenseSpdx::Apache20,
        input_name: "image",
        output_name: "denoised",
        tile_size: 128,
        output_scale: 1,
    },
    ModelDownloadSpec {
        id: "swin2sr-realworld-x4",
        url: "https://huggingface.co/onnx-community/swin2SR-realworld-sr-x4-64-bsrgan-psnr-ONNX/resolve/main/onnx/model_int8.onnx",
        sha256: "13474507a83b3dc254df0fbf32f4a34ffa62070d62b33cc99eacef43df3a3692",
        size_bytes: 18_935_979,
        license: LicenseSpdx::Apache20,
        input_name: "pixel_values",
        output_name: "restoration",
        tile_size: 0,
        output_scale: 4,
    },
];

pub fn download_spec(id: &str) -> Option<&'static ModelDownloadSpec> {
    DOWNLOAD_SPECS.iter().find(|s| s.id == id)
}

/// Errors from the install pipeline. All fail-closed: nothing is
/// written to the models directory unless every check passes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DownloadError {
    UnknownModel(String),
    Network(String),
    SizeMismatch {
        id: String,
        expected: u64,
        actual: u64,
    },
    ChecksumMismatch {
        id: String,
        expected: String,
        actual: String,
    },
    Io(String),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::UnknownModel(id) => {
                write!(f, "no download spec for model '{id}'")
            }
            DownloadError::Network(e) => write!(f, "download failed: {e}"),
            DownloadError::SizeMismatch {
                id,
                expected,
                actual,
            } => write!(
                f,
                "model '{id}' size mismatch: expected {expected} bytes, got {actual}"
            ),
            DownloadError::ChecksumMismatch {
                id,
                expected,
                actual,
            } => write!(
                f,
                "model '{id}' checksum mismatch: expected sha256 {expected}, got {actual}"
            ),
            DownloadError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for DownloadError {}

/// Where install bytes come from. Production uses [`HttpByteSource`];
/// tests inject fakes so the whole pipeline runs offline.
pub trait ByteSource {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, DownloadError>;
}

/// `ureq`-based production source (rustls; no native TLS deps).
pub struct HttpByteSource;

impl ByteSource for HttpByteSource {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, DownloadError> {
        let response = ureq::get(url)
            .call()
            .map_err(|e| DownloadError::Network(e.to_string()))?;
        let mut bytes = Vec::new();
        use std::io::Read;
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| DownloadError::Network(e.to_string()))?;
        Ok(bytes)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Fetch, verify, and atomically install a catalog model.
///
/// Returns the installed path (`<models_dir>/<id>.onnx`). The bytes
/// are written to a `.part` sibling first and renamed into place only
/// after the size and digest checks pass, so a killed download never
/// leaves a half-written artifact that a later session could mistake
/// for a valid model.
pub fn install_model(
    models_dir: &Path,
    spec: &ModelDownloadSpec,
    source: &dyn ByteSource,
) -> Result<PathBuf, DownloadError> {
    let bytes = source.fetch(spec.url)?;

    if bytes.len() as u64 != spec.size_bytes {
        return Err(DownloadError::SizeMismatch {
            id: spec.id.to_string(),
            expected: spec.size_bytes,
            actual: bytes.len() as u64,
        });
    }
    let actual = sha256_hex(&bytes);
    if !constant_time_eq(&actual, spec.sha256) {
        return Err(DownloadError::ChecksumMismatch {
            id: spec.id.to_string(),
            expected: spec.sha256.to_string(),
            actual,
        });
    }

    std::fs::create_dir_all(models_dir).map_err(|e| DownloadError::Io(e.to_string()))?;
    let final_path = models_dir.join(format!("{}.onnx", spec.id));
    let part_path = models_dir.join(format!("{}.onnx.part", spec.id));
    std::fs::write(&part_path, &bytes).map_err(|e| DownloadError::Io(e.to_string()))?;
    std::fs::rename(&part_path, &final_path).map_err(|e| DownloadError::Io(e.to_string()))?;
    Ok(final_path)
}

/// Digest comparison that does not short-circuit on the first
/// differing character (models are security-relevant artifacts).
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::UNVERIFIED_SHA256;

    /// Serves fixed bytes; records the URL it was asked for.
    struct StubSource {
        bytes: Vec<u8>,
        asked: std::cell::RefCell<Option<String>>,
    }

    impl ByteSource for StubSource {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, DownloadError> {
            *self.asked.borrow_mut() = Some(url.to_string());
            Ok(self.bytes.clone())
        }
    }

    fn spec_for_bytes(bytes: &[u8]) -> ModelDownloadSpec {
        ModelDownloadSpec {
            id: "test-model",
            url: "test://fixture",
            sha256: Box::leak(sha256_hex(bytes).into_boxed_str()),
            size_bytes: bytes.len() as u64,
            license: LicenseSpdx::Apache20,
            input_name: "image",
            output_name: "denoised",
            tile_size: 128,
            output_scale: 1,
        }
    }

    fn tmp_dir() -> PathBuf {
        let mut p = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        p.push(format!("astroforge-dl-{}-{stamp}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn installs_bytes_that_match_the_pin() {
        // Use a real builtin graph as stand-in content.
        let content = crate::inference::builtin_model("builtin-hotpixel")
            .expect("builtin exists")
            .bytes
            .to_vec();
        let spec = spec_for_bytes(&content);
        let source = StubSource {
            bytes: content.clone(),
            asked: std::cell::RefCell::new(None),
        };
        let dir = tmp_dir();
        let path = install_model(&dir, &spec, &source).expect("install");
        assert_eq!(path, dir.join("test-model.onnx"));
        assert_eq!(std::fs::read(&path).expect("read back"), content);
        // No .part residue.
        assert!(!dir.join("test-model.onnx.part").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_tampered_bytes() {
        let mut content = vec![7u8; 1024];
        let spec = spec_for_bytes(&content);
        content[0] ^= 0xFF; // flip one byte after pinning
        let source = StubSource {
            bytes: content,
            asked: std::cell::RefCell::new(None),
        };
        let dir = tmp_dir();
        let err = install_model(&dir, &spec, &source).expect_err("must refuse");
        assert!(matches!(err, DownloadError::ChecksumMismatch { .. }));
        assert!(!dir.join("test-model.onnx").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_truncated_transfer() {
        let content = vec![1u8; 512];
        let spec = spec_for_bytes(&content);
        let source = StubSource {
            bytes: content[..256].to_vec(),
            asked: std::cell::RefCell::new(None),
        };
        let dir = tmp_dir();
        let err = install_model(&dir, &spec, &source).expect_err("must refuse");
        assert!(matches!(err, DownloadError::SizeMismatch { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The download table cannot drift from the pinned catalog: every
    /// spec must pin the same digest + license as the CATALOG_MODELS
    /// entry of the same id, and unpinned catalog entries must have
    /// no download spec.
    #[test]
    fn download_specs_match_catalog_pins() {
        for spec in DOWNLOAD_SPECS {
            let entry = crate::inference::catalog_model(spec.id)
                .unwrap_or_else(|| panic!("{} missing from CATALOG_MODELS", spec.id));
            assert_ne!(
                entry.sha256, UNVERIFIED_SHA256,
                "{} catalog entry is still unpinned but has a download spec",
                spec.id
            );
            assert_eq!(entry.sha256, spec.sha256, "{} digest drift", spec.id);
            // #458 — SR-class specs must be dynamic-dim: the tiler
            // cannot stitch scaled outputs, so a fixed-tile SR spec
            // would corrupt every multi-tile dispatch.
            assert!(
                spec.output_scale == 1 || spec.tile_size == 0,
                "{}: output_scale {} requires tile_size 0",
                spec.id,
                spec.output_scale
            );
            assert_eq!(
                entry.license,
                Some(spec.license),
                "{} license drift",
                spec.id
            );
        }
        // Every catalog entry pinned with a real digest must offer a
        // download spec (otherwise it can never be installed) —
        // except the builtin- shadow entries, which are embedded in
        // the binary rather than downloaded.
        for entry in crate::inference::CATALOG_MODELS {
            if entry.sha256 != UNVERIFIED_SHA256 && !entry.id.starts_with("builtin-") {
                assert!(
                    download_spec(entry.id).is_some(),
                    "{} is pinned but not downloadable",
                    entry.id
                );
            }
        }
    }

    #[test]
    fn unknown_model_has_no_spec() {
        assert!(download_spec("no-such-model").is_none());
    }

    #[test]
    fn constant_time_eq_behaves() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
    }
}
