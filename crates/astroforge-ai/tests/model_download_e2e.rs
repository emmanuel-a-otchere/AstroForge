//! RT-7 (#426) — network-gated end-to-end download + load test.
//!
//! Opt-in: runs only when `ASTROFORGE_MODEL_E2E=1` is set, because it
//! downloads real multi-MB model artifacts. The normal CI suite skips
//! every test here (they print one line and return). The
//! `.github/workflows/model-e2e.yml` workflow runs this file on
//! demand (workflow_dispatch) where network is available.
//!
//! What it proves, per the pinned specs:
//! 1. The published artifact still matches the pinned SHA-256 (the
//!    fail-closed contract would otherwise make every install fail).
//! 2. The bytes load as an ONNX session under the pinned `ort`.
//! 3. The documented tensor contract holds (input/output names and
//!    shapes from the publisher's model card) — pinned with a raw
//!    NCHW run so the slice-2 dispatcher adapter has a verified
//!    target.

use astroforge_ai::downloader::{install_model, DownloadError, HttpByteSource, DOWNLOAD_SPECS};

fn enabled() -> bool {
    std::env::var("ASTROFORGE_MODEL_E2E").ok().as_deref() == Some("1")
}

fn tmp_dir() -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    p.push(format!("astroforge-model-e2e-{stamp}"));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("mkdir");
    p
}

#[test]
fn swinir_denoise_downloads_pins_and_loads() {
    if !enabled() {
        eprintln!("skipping: set ASTROFORGE_MODEL_E2E=1 to run the network e2e");
        return;
    }
    let spec = DOWNLOAD_SPECS
        .iter()
        .find(|s| s.id == "swinir-denoise-astro")
        .expect("denoise spec");
    let dir = tmp_dir();
    let path = install_model(&dir, spec, &HttpByteSource).expect("download + verify + install");
    assert!(path.is_file());

    let bytes = std::fs::read(&path).expect("read installed model");

    // The session builder re-verifies the digest, then loads.
    astroforge_ai::inference::OnnxEngine::open_verified(
        &bytes,
        spec.sha256,
        astroforge_ai::inference::BuiltinInputKind::ImageOnly,
    )
    .expect("verified session opens");

    // Tensor contract from the model card: input `image`
    // [1,3,128,128] float32 RGB in [0,1] → output `denoised`
    // [1,3,128,128]. Raw ort run with the documented input name —
    // the run itself is the contract check: a renamed input or an
    // unexpected shape fails here (the per-plane builtin engine is
    // single-channel; the NCHW adapter lands in the next slice).
    let mut session = ort::session::Session::builder()
        .and_then(|mut b| b.commit_from_memory(&bytes))
        .expect("raw session");
    let input = ort::value::Tensor::from_array((
        vec![1usize, 3, 128, 128],
        vec![0.5_f32; 3 * 128 * 128].into_boxed_slice(),
    ))
    .expect("input tensor");
    let outputs = session
        .run(ort::inputs![spec.input_name => input])
        .expect("inference runs");
    let (shape, data) = outputs[0]
        .try_extract_tensor::<f32>()
        .expect("output tensor");
    let dims: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
    assert_eq!(dims, vec![1, 3, 128, 128], "denoise preserves dims");
    assert!(data.iter().all(|v| v.is_finite()), "output must be finite");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn swin2sr_superres_downloads_pins_and_loads() {
    if !enabled() {
        eprintln!("skipping: set ASTROFORGE_MODEL_E2E=1 to run the network e2e");
        return;
    }
    let spec = DOWNLOAD_SPECS
        .iter()
        .find(|s| s.id == "swin2sr-realworld-x4")
        .expect("superres spec");
    let dir = tmp_dir();
    let result = install_model(&dir, spec, &HttpByteSource);
    match result {
        Ok(path) => {
            assert!(path.is_file());
            let bytes = std::fs::read(&path).expect("read installed model");
            // Session load proves ort accepts the int8-quantized graph
            // and the pin matches; the SR tensor contract is asserted
            // in the adapter slice (transformers.js export has dynamic
            // spatial dims — pinned there once, not guessed here).
            astroforge_ai::inference::OnnxEngine::open_verified(
                &bytes,
                spec.sha256,
                astroforge_ai::inference::BuiltinInputKind::ImageOnly,
            )
            .expect("verified session opens");
        }
        Err(DownloadError::ChecksumMismatch { .. }) => {
            panic!("upstream artifact changed; re-pin the digest after review");
        }
        Err(e) => panic!("unexpected download failure: {e}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}
