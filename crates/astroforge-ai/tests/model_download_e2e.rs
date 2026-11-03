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

/// RT-7 slice 2 acceptance: the downloaded SwinIR model serves a real
/// `denoise_luminance` dispatch end-to-end — model resolution, NCHW
/// tiling at the fixed 128 edge, tile stitch, provenance row.
#[test]
fn denoise_dispatch_routes_through_downloaded_swinir() {
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
    let _ = path;

    // 128x128 RGB scene: single tile, no padding needed beyond the
    // exact edge.
    let mut src = astroforge_core::image::F32Image::new(128, 128, 3);
    for c in 0..3 {
        for y in 0..128 {
            for x in 0..128 {
                src[(c, y, x)] = 0.5 + 0.1 * ((x * 7 + y * 13) % 11) as f32 / 55.0;
            }
        }
    }
    let inputs = astroforge_ai::operations::DispatchInputs {
        source: &src,
        mask: None,
        probe: &astroforge_ai::hardware::HardwareProbe::detect(),
        models_dir: Some(&dir),
    };
    let out = astroforge_ai::operations::dispatch_operation(
        "denoise_luminance",
        r#"{"strength":0.8}"#,
        "v-e2e-src",
        "v-e2e-dst".into(),
        "pv-e2e".into(),
        inputs,
    )
    .expect("dispatch through the downloaded model");

    // Provenance: the catalog model served the request, not the
    // builtin.
    assert_eq!(out.model_id, "swinir-denoise-astro");
    assert_eq!(out.model_hash, spec.sha256);
    assert_eq!(out.outcome.source_image_version_id, "v-e2e-src");
    let tiles: serde_json::Value =
        serde_json::from_str(&out.tile_configuration_json).expect("tile json");
    assert_eq!(tiles["tile_size"], 128, "fixed SwinIR tile forced");
    // "tiled" records that the tiled code path served the op
    // (spatial_scale == 1); with overlap 64 the tiler emits
    // overlapping 128-edge tiles even for an exactly-128 source.
    assert_eq!(tiles["tiled"], true);
    assert_eq!(
        (
            out.result_image.width(),
            out.result_image.height(),
            out.result_image.channels()
        ),
        (128, 128, 3)
    );
    let mut all_finite = true;
    for c in 0..3 {
        for y in 0..128 {
            for x in 0..128 {
                all_finite &= out.result_image[(c, y, x)].is_finite();
            }
        }
    }
    assert!(all_finite, "output must be finite");

    // Multi-tile path: 200x150 forces 128-edge tiling + edge padding
    // (tiles at the right/bottom edges are smaller than 128).
    let mut big = astroforge_core::image::F32Image::new(200, 150, 3);
    for c in 0..3 {
        for y in 0..150 {
            for x in 0..200 {
                big[(c, y, x)] = ((x * 3 + y * 5) % 17) as f32 / 170.0 + 0.2;
            }
        }
    }
    let inputs = astroforge_ai::operations::DispatchInputs {
        source: &big,
        mask: None,
        probe: &astroforge_ai::hardware::HardwareProbe::detect(),
        models_dir: Some(&dir),
    };
    let out = astroforge_ai::operations::dispatch_operation(
        "denoise_luminance",
        r#"{"strength":0.8}"#,
        "v-e2e-src",
        "v-e2e-dst".into(),
        "pv-e2e".into(),
        inputs,
    )
    .expect("multi-tile dispatch");
    assert_eq!(out.model_id, "swinir-denoise-astro");
    let tiles: serde_json::Value =
        serde_json::from_str(&out.tile_configuration_json).expect("tile json");
    assert_eq!(tiles["tiled"], true, "200x150 source must tile");
    assert_eq!(
        (out.result_image.width(), out.result_image.height()),
        (200, 150)
    );
    let mut all_finite = true;
    for c in 0..3 {
        for y in 0..150 {
            for x in 0..200 {
                all_finite &= out.result_image[(c, y, x)].is_finite();
            }
        }
    }
    assert!(all_finite, "output must be finite");
    let _ = std::fs::remove_dir_all(&dir);
}
