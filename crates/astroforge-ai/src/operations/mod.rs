//! CR-06 P4 — AI Enhancement Operations registry.
//!
//! P4 ships the operation layer that:
//!
//! - defines the registry of supported operations
//!   (`denoise_luminance`, `denoise_chrominance`,
//!   `deconv`, `star_refine`, `star_reduce`, `detail_enhance`,
//!   `super_resolution`, `inpaint`, `background_cleanup`,
//!   `hot_pixel_clean`, `trail_clean`);
//! - pins the canonical `OperationParameter` schema for
//!   each;
//! - carries the `SafetyClassification` and human-readable
//!   metadata the Studio UI surfaces;
//! - exposes a pure `OperationOutcome` type the apply
//!   round persists into the `AiOperation` row.
//!
//! The actual pixel transformation for the model-backed
//! operations (`denoise`, `sr`, `inpaint`, etc.) lands in
//! P5 with the real ONNX dispatch. P4 deliberately
//! exposes a `dispatch()` function whose body returns a
//! `PassthroughOutcome` for every operation, so the
//! end-to-end apply + branch + stack-reorder flow is
//! exercisable on a CI runner without a GPU. The
//! substitution is local: P5 replaces the body, the
//! signatures + state machine stay.
//!
//! This module does NOT live in `astroforge-core` because
//! the operations registry is the AI crate's domain —
//! `astroforge-core` carries the data model + stack engine
//! but the operations themselves are AI-side.

use serde::{Deserialize, Serialize};

use astroforge_core::image::F32Image;
use astroforge_core::masks::Mask;

use crate::hardware::HardwareProbe;
use crate::inference::OnnxEngine;

/// One operation's canonical metadata. The Studio UI
/// reads this list to render the per-operation card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationInfo {
    pub operation_id: String,
    pub display_name: String,
    pub category: OperationCategory,
    pub safety_classification: SafetyClassification,
    pub description: String,
    pub default_parameters_json: String,
    /// Operations marked `requires_region` cannot run
    /// without a mask (`inpaint` is the canonical
    /// example). The Studio UI greys the Apply button
    /// out when no mask is selected.
    #[serde(default)]
    pub requires_region: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationCategory {
    /// Pre-denoise cleanups (deterministic).
    Cleanup,
    /// Noise / chromatic reduction.
    Denoise,
    /// Spatial / deconvolution restoration.
    Restoration,
    /// Star / astronomical structure refinement.
    Star,
    /// Detail / structure recovery.
    Detail,
    /// Super-resolution / generative upscale.
    Upscale,
    /// Inpainting / region-restricted replacement.
    Inpaint,
    /// Background / sky gradient operations.
    Background,
}

impl OperationCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationCategory::Cleanup => "cleanup",
            OperationCategory::Denoise => "denoise",
            OperationCategory::Restoration => "restoration",
            OperationCategory::Star => "star",
            OperationCategory::Detail => "detail",
            OperationCategory::Upscale => "upscale",
            OperationCategory::Inpaint => "inpaint",
            OperationCategory::Background => "background",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyClassification {
    /// No generative step; output is reproducible.
    Deterministic,
    /// Perceptual enhancement (sharpening, denoise);
    /// pixel values are modified but no new structure
    /// is invented.
    Perceptual,
    /// Generative step (SR, inpaint, trail fill);
    /// output may include synthesised structure.
    Generative,
}

impl SafetyClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyClassification::Deterministic => "deterministic",
            SafetyClassification::Perceptual => "perceptual",
            SafetyClassification::Generative => "generative",
        }
    }
}

/// Return the full operations registry. The list is the
/// authoritative one — every UI card, every apply
/// round, every recommendation-to-operation mapping
/// reads from it.
pub fn registry() -> Vec<OperationInfo> {
    vec![
        OperationInfo {
            operation_id: "background_cleanup".into(),
            display_name: "Background Cleanup".into(),
            category: OperationCategory::Background,
            safety_classification: SafetyClassification::Deterministic,
            description: "Remove the sky gradient so subsequent operations read a flat background.".into(),
            default_parameters_json: r#"{"strength":1.0}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "hot_pixel_clean".into(),
            display_name: "Hot Pixel Cleanup".into(),
            category: OperationCategory::Cleanup,
            safety_classification: SafetyClassification::Deterministic,
            description: "Remove single-pixel defects detected by the analyzer.".into(),
            default_parameters_json: r#"{"threshold":0.95}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "trail_clean".into(),
            display_name: "Trail Cleanup".into(),
            category: OperationCategory::Inpaint,
            safety_classification: SafetyClassification::Generative,
            description: "Inpaint satellite / aeroplane trails. Generative — the trail pixels are synthesised.".into(),
            default_parameters_json: r#"{"strength":1.0}"#.into(),
            requires_region: true,
        },
        OperationInfo {
            operation_id: "denoise_luminance".into(),
            display_name: "Luminance Denoise".into(),
            category: OperationCategory::Denoise,
            safety_classification: SafetyClassification::Perceptual,
            description: "Reduce luminance noise while preserving fine structure.".into(),
            default_parameters_json: r#"{"strength":0.7,"detail_preservation":0.6,"tile_size":512}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "denoise_chrominance".into(),
            display_name: "Chrominance Denoise".into(),
            category: OperationCategory::Denoise,
            safety_classification: SafetyClassification::Perceptual,
            description: "Reduce color speckle while preserving saturation on stars.".into(),
            default_parameters_json: r#"{"strength":0.6,"tile_size":512}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "deconv".into(),
            display_name: "Deconvolution".into(),
            category: OperationCategory::Restoration,
            safety_classification: SafetyClassification::Deterministic,
            description: "Restore spatial detail via iterative PSF-aware deconvolution.".into(),
            default_parameters_json: r#"{"iterations":20,"psf_kernels":3}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "star_refine".into(),
            display_name: "Star Refinement".into(),
            category: OperationCategory::Star,
            safety_classification: SafetyClassification::Perceptual,
            description: "Sharpen star profiles and tighten FWHM without amplifying noise.".into(),
            default_parameters_json: r#"{"strength":0.5,"protect_color":true}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "star_reduce".into(),
            display_name: "Star Reduction".into(),
            category: OperationCategory::Star,
            safety_classification: SafetyClassification::Perceptual,
            description: "Reduce star size and brightness to expose faint nebular structure.".into(),
            default_parameters_json: r#"{"strength":0.4}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "detail_enhance".into(),
            display_name: "Detail Enhancement".into(),
            category: OperationCategory::Detail,
            safety_classification: SafetyClassification::Perceptual,
            description: "Recover local contrast in faint structures. Should run after denoise.".into(),
            default_parameters_json: r#"{"strength":0.5,"tile_size":512}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "super_resolution".into(),
            display_name: "2× Super Resolution".into(),
            category: OperationCategory::Upscale,
            safety_classification: SafetyClassification::Generative,
            description: "2× linear super-resolution. Perceptual — syntheses high-frequency detail.".into(),
            default_parameters_json: r#"{"scale":2}"#.into(),
            requires_region: false,
        },
        OperationInfo {
            operation_id: "inpaint".into(),
            display_name: "Region Inpaint".into(),
            category: OperationCategory::Inpaint,
            safety_classification: SafetyClassification::Generative,
            description: "Replace pixels inside a region-restricted mask with synthesised content.".into(),
            default_parameters_json: r#"{"region_mask_required":true}"#.into(),
            requires_region: true,
        },
    ]
}

pub fn get_operation(operation_id: &str) -> Option<OperationInfo> {
    registry()
        .into_iter()
        .find(|op| op.operation_id == operation_id)
}

/// The outcome of dispatching an operation. P4 ships a
/// `Passthrough` variant for every operation; the
/// pixels are unchanged (the source version is returned
/// verbatim). P5 replaces this with model-backed
/// variants (`Denoise`, `SuperResolution`, etc.) per
/// the per-operation handlers in `astroforge-ai`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationOutcome {
    pub operation_id: String,
    pub safety_classification: SafetyClassification,
    pub parameters_hash: String,
    pub source_image_version_id: String,
    pub result_image_version_id: String,
    pub preview_id: String,
    pub note: String,
}

/// CR-06 P5.1 — the full result of a real dispatch:
/// the stable [`OperationOutcome`] metadata plus the
/// produced pixels and the execution provenance the
/// apply round persists onto the `AiOperation` row.
#[derive(Debug, Clone)]
pub struct DispatchResult {
    pub outcome: OperationOutcome,
    /// The pixels the model produced (post tile-stitch,
    /// post mask-composite, clamped to `[0, 1]`).
    pub result_image: F32Image,
    /// Model that ran: the catalog model name when a
    /// downloaded artifact served the request, else the
    /// builtin id (e.g. `builtin-blur-blend`).
    pub model_id: String,
    pub model_version: String,
    /// SHA-256 of the model bytes that ran.
    pub model_hash: String,
    /// Execution backend label (`"cpu"` in P5.1; GPU
    /// providers land in P5.2).
    pub backend: String,
    /// Runtime label, e.g. `onnxruntime-1.28`.
    pub runtime: String,
    /// JSON: `{tile_size, overlap, tiles, tiled}`.
    pub tile_configuration_json: String,
    pub duration_ms: u64,
}

/// CR-06 P5.1 — the inputs the dispatcher needs from
/// the apply round. Bundled into a struct so the
/// dispatcher's signature stays under clippy's
/// `too_many_arguments` threshold and so future apply
/// round knobs (e.g. backend preference, seed) ship
/// without breaking the call site.
#[derive(Debug, Clone)]
pub struct DispatchInputs<'a> {
    pub source: &'a F32Image,
    pub mask: Option<&'a Mask>,
    pub probe: &'a HardwareProbe,
    pub models_dir: Option<&'a std::path::Path>,
}

/// CR-06 P5.1 — how an operation maps onto a model.
/// The catalog model (downloaded artifact) takes
/// precedence when present in `models_dir`; the
/// builtin classical-kernel graph is the
/// always-available floor.
pub struct OperationModelBinding {
    /// Builtin graph that serves this operation.
    pub builtin_model_id: &'static str,
    /// Catalog model that supersedes the builtin when
    /// its artifact is downloaded (`<models_dir>/<name>.onnx`).
    pub catalog_model_name: Option<&'static str>,
    /// `parameters_json` key that feeds the graph's
    /// scalar input. `None` for graphs without a scalar.
    pub scalar_param_key: Option<&'static str>,
    pub scalar_default: f32,
    /// When true the graph input is `1.0 + value` (the
    /// hot-pixel graph compares against `mean * t`, so
    /// the user-facing `threshold` 0.95 maps to a 1.95x
    /// local-mean trip point).
    pub scalar_offset_one: bool,
    /// Output spatial multiplier. 1 for same-size
    /// graphs; 2 for the 2x super-resolution graph
    /// (which runs untiled — the tiler's blend assumes
    /// input-sized outputs).
    pub spatial_scale: u32,
}

/// CR-06 P5.1 — the operation-to-model binding table.
/// Every registered operation resolves to exactly one
/// binding; an unknown id yields `None` (and the
/// dispatcher raises `UnknownOperation`).
pub fn model_binding(operation_id: &str) -> Option<OperationModelBinding> {
    let binding = match operation_id {
        "background_cleanup" => OperationModelBinding {
            builtin_model_id: "builtin-blur-blend",
            catalog_model_name: None,
            scalar_param_key: Some("strength"),
            scalar_default: 1.0,
            scalar_offset_one: false,
            spatial_scale: 1,
        },
        "hot_pixel_clean" => OperationModelBinding {
            builtin_model_id: "builtin-hotpixel",
            catalog_model_name: None,
            scalar_param_key: Some("threshold"),
            scalar_default: 0.95,
            scalar_offset_one: true,
            spatial_scale: 1,
        },
        "trail_clean" => OperationModelBinding {
            builtin_model_id: "builtin-masked-fill",
            catalog_model_name: Some("trail-lama-tiny"),
            scalar_param_key: None,
            scalar_default: 0.0,
            scalar_offset_one: false,
            spatial_scale: 1,
        },
        "denoise_luminance" => OperationModelBinding {
            builtin_model_id: "builtin-blur-blend",
            catalog_model_name: Some("swinir-denoise-astro"),
            scalar_param_key: Some("strength"),
            scalar_default: 0.7,
            scalar_offset_one: false,
            spatial_scale: 1,
        },
        "denoise_chrominance" => OperationModelBinding {
            builtin_model_id: "builtin-blur-blend",
            catalog_model_name: Some("swinir-denoise-astro"),
            scalar_param_key: Some("strength"),
            scalar_default: 0.6,
            scalar_offset_one: false,
            spatial_scale: 1,
        },
        // Deconvolution approximates to an unsharp mask
        // at fixed strength until a real PSF-aware model
        // lands; `iterations` / `psf_kernels` params are
        // recorded in provenance but do not steer the
        // classical kernel.
        "deconv" => OperationModelBinding {
            builtin_model_id: "builtin-sharpen-blend",
            catalog_model_name: None,
            scalar_param_key: None,
            scalar_default: 0.5,
            scalar_offset_one: false,
            spatial_scale: 1,
        },
        "star_refine" => OperationModelBinding {
            builtin_model_id: "builtin-sharpen-blend",
            catalog_model_name: None,
            scalar_param_key: Some("strength"),
            scalar_default: 0.5,
            scalar_offset_one: false,
            spatial_scale: 1,
        },
        "star_reduce" => OperationModelBinding {
            builtin_model_id: "builtin-blur-blend",
            catalog_model_name: None,
            scalar_param_key: Some("strength"),
            scalar_default: 0.4,
            scalar_offset_one: false,
            spatial_scale: 1,
        },
        "detail_enhance" => OperationModelBinding {
            builtin_model_id: "builtin-sharpen-blend",
            catalog_model_name: None,
            scalar_param_key: Some("strength"),
            scalar_default: 0.5,
            scalar_offset_one: false,
            spatial_scale: 1,
        },
        "super_resolution" => OperationModelBinding {
            builtin_model_id: "builtin-upscale-2x",
            catalog_model_name: Some("swinir-sr-astro-2x"),
            scalar_param_key: None,
            scalar_default: 0.0,
            scalar_offset_one: false,
            spatial_scale: 2,
        },
        "inpaint" => OperationModelBinding {
            builtin_model_id: "builtin-masked-fill",
            catalog_model_name: None,
            scalar_param_key: None,
            scalar_default: 0.0,
            scalar_offset_one: false,
            spatial_scale: 1,
        },
        _ => return None,
    };
    Some(binding)
}

/// FNV-1a 64-bit hex. Stable across runs; cheap to
/// compute; sufficient as a UI digest.
fn short_hash(input: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut h = FNV_OFFSET;
    for byte in input.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    format!("{:016x}", h)
}

/// Read the scalar parameter for a binding out of the
/// operation's `parameters_json`, falling back to the
/// binding default when the key is absent.
fn scalar_for(
    binding: &OperationModelBinding,
    parameters_json: &str,
) -> Result<f32, OperationError> {
    let Some(key) = binding.scalar_param_key else {
        return Ok(binding.scalar_default);
    };
    let value: serde_json::Value = serde_json::from_str(parameters_json)
        .map_err(|e| OperationError::BadParameters(format!("invalid parameters_json: {e}")))?;
    let raw = value
        .get(key)
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or(binding.scalar_default);
    let raw = if binding.scalar_offset_one {
        1.0 + raw
    } else {
        raw
    };
    Ok(raw)
}

/// Extract the tile-region of a full-frame mask as its
/// own `Mask`, so the masked-fill graph receives a mask
/// tile matching the image tile.
fn mask_tile(mask: &Mask, tile: &crate::tiling::Tile) -> Mask {
    let mut out = Mask::zeros(
        tile.width,
        tile.height,
        mask.kind,
        format!("{} (tile {},{})", mask.provenance, tile.x, tile.y),
    );
    for y in 0..tile.height {
        for x in 0..tile.width {
            out.set(x, y, mask.get(tile.x + x, tile.y + y));
        }
    }
    out
}

/// Mask-weighted composite: `mask * inferred +
/// (1 - mask) * source`. Applied for region-restricted
/// blending when the graph itself does not consume the
/// mask (the masked-fill graph already honours it).
fn composite_with_mask(source: &F32Image, inferred: &F32Image, mask: &Mask) -> F32Image {
    let mut out = F32Image::new(source.width(), source.height(), source.channels());
    for c in 0..source.channels() {
        for y in 0..source.height() {
            for x in 0..source.width() {
                let m = mask.get(x as u32, y as u32);
                out[(c, y, x)] = m * inferred[(c, y, x)] + (1.0 - m) * source[(c, y, x)];
            }
        }
    }
    out
}

fn clamp_unit(img: &mut F32Image) {
    for v in img.iter_mut() {
        *v = v.clamp(0.0, 1.0);
    }
}

/// Dispatch an operation. P5.1 replaces the P4
/// passthrough body with the real path:
///
/// 1. Resolve the operation's model binding (catalog
///    artifact when downloaded, builtin otherwise).
/// 2. Build a digest-verified ONNX session.
/// 3. Tile the source via [`crate::tiling`] using the
///    model's tile size clamped by the hardware probe;
///    the 2x upscale runs untiled (the tiler assumes
///    input-sized outputs).
/// 4. Run inference per tile, stitch, and composite
///    with the mask when one is attached.
///
/// The metadata contract ([`OperationOutcome`] +
/// [`OperationInfo`]) is unchanged from P4.
pub fn dispatch_operation(
    operation_id: &str,
    parameters_json: &str,
    source_image_version_id: &str,
    result_image_version_id: String,
    preview_id: String,
    inputs: DispatchInputs<'_>,
) -> Result<DispatchResult, OperationError> {
    let DispatchInputs {
        source,
        mask,
        probe,
        models_dir,
    } = inputs;
    let started = std::time::Instant::now();
    let info = get_operation(operation_id)
        .ok_or_else(|| OperationError::UnknownOperation(operation_id.into()))?;
    let binding = model_binding(operation_id)
        .ok_or_else(|| OperationError::UnknownOperation(operation_id.into()))?;
    if info.requires_region && mask.is_none() {
        return Err(OperationError::MissingMask(format!(
            "operation '{operation_id}' requires a region mask"
        )));
    }
    let scalar = scalar_for(&binding, parameters_json)?;

    // Model resolution: a downloaded catalog artifact
    // supersedes the builtin. Catalog digests are
    // placeholders pending DP#4, so catalog artifacts
    // skip digest pinning until the licensing decision
    // lands real hashes.
    let mut engine = resolve_engine(&binding, models_dir)?;
    let (model_id, model_version, model_hash) = engine_model_identity(&binding, models_dir);

    let tile_size = crate::tiling::DEFAULT_TILE_SIZE.min(probe.max_tile_size());
    let config = crate::tiling::TileConfig {
        tile_size,
        overlap: crate::tiling::DEFAULT_OVERLAP,
    };

    let inferred = if binding.spatial_scale != 1 {
        // Spatial-scale graphs run untiled (see
        // `spatial_scale` note above).
        engine
            .run(source, Some(scalar), mask)
            .map_err(|e| OperationError::Inference(e.to_string()))?
    } else {
        // The tiler's `infer_fn` is infallible (`Fn ->
        // F32Image`), so tile errors are captured
        // out-of-band and re-raised after the run; a
        // failed tile never silently passes through as
        // a clone of the input.
        let engine_cell = std::cell::RefCell::new(engine);
        let first_error: std::cell::RefCell<Option<crate::inference::InferenceError>> =
            std::cell::RefCell::new(None);
        let result = crate::tiling::run_tiled_inference(source, &config, |tile_img, tile_geom| {
            if first_error.borrow().is_some() {
                return tile_img.clone();
            }
            let tile_mask = mask.map(|m| mask_tile(m, tile_geom));
            match engine_cell
                .borrow_mut()
                .run(tile_img, Some(scalar), tile_mask.as_ref())
            {
                Ok(out) => out,
                Err(e) => {
                    *first_error.borrow_mut() = Some(e);
                    tile_img.clone()
                }
            }
        });
        engine = engine_cell.into_inner();
        if let Some(e) = first_error.into_inner() {
            return Err(OperationError::Inference(e.to_string()));
        }
        result
    };

    let mut result_image = match (mask, engine.kind()) {
        // The masked-fill graph already honours the
        // mask; compositing again would double-apply it.
        (Some(_), crate::inference::BuiltinInputKind::Mask) => inferred,
        (Some(m), _) => composite_with_mask(source, &inferred, m),
        (None, _) => inferred,
    };
    clamp_unit(&mut result_image);

    let tile_configuration_json = serde_json::json!({
        "tile_size": tile_size,
        "overlap": crate::tiling::DEFAULT_OVERLAP,
        "tiled": binding.spatial_scale == 1,
        "spatial_scale": binding.spatial_scale,
    })
    .to_string();

    Ok(DispatchResult {
        outcome: OperationOutcome {
            operation_id: info.operation_id,
            safety_classification: info.safety_classification,
            parameters_hash: short_hash(parameters_json),
            source_image_version_id: source_image_version_id.into(),
            result_image_version_id,
            preview_id,
            note: "P5.1: real ONNX inference (builtin classical-kernel graph unless a catalog model artifact is present).".into(),
        },
        result_image,
        model_id,
        model_version,
        model_hash,
        backend: "cpu".into(),
        runtime: "onnxruntime-1.28".into(),
        tile_configuration_json,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Build the engine for a binding: catalog artifact
/// when `<models_dir>/<catalog>.onnx` exists, builtin
/// otherwise. The returned engine borrows nothing; the
/// builtin path embeds its bytes.
fn resolve_engine(
    binding: &OperationModelBinding,
    models_dir: Option<&std::path::Path>,
) -> Result<OnnxEngine, OperationError> {
    if let (Some(dir), Some(name)) = (models_dir, binding.catalog_model_name) {
        let path = dir.join(format!("{name}.onnx"));
        if path.exists() {
            let bytes = std::fs::read(&path)
                .map_err(|e| OperationError::Io(format!("read {}: {e}", path.display())))?;
            // DP#4: open_catalog verifies the digest against the
            // pinned registry entry. Unknown ids fail closed;
            // unpinned entries (sha256 = "unverified") fail closed
            // until real hashes land.
            return OnnxEngine::open_catalog(name, bytes)
                .map_err(|e| OperationError::Inference(e.to_string()));
        }
    }
    let builtin = crate::inference::builtin_model(binding.builtin_model_id)
        .ok_or_else(|| OperationError::UnknownOperation(binding.builtin_model_id.into()))?;
    OnnxEngine::open_builtin(builtin).map_err(|e| OperationError::Inference(e.to_string()))
}

/// The (id, version, hash) identity of the model that
/// will run, for the provenance row.
fn engine_model_identity(
    binding: &OperationModelBinding,
    models_dir: Option<&std::path::Path>,
) -> (String, String, String) {
    if let (Some(dir), Some(name)) = (models_dir, binding.catalog_model_name) {
        let path = dir.join(format!("{name}.onnx"));
        if path.exists() {
            if let Ok(bytes) = std::fs::read(&path) {
                let hash = astroforge_core::artifact::ContentStore::sha256_hex(&bytes);
                let version = crate::hub::get_model(name)
                    .map(|m| m.version)
                    .unwrap_or_else(|| "0.0.0".into());
                return (name.to_string(), version, hash);
            }
        }
    }
    let builtin = crate::inference::builtin_model(binding.builtin_model_id);
    match builtin {
        Some(b) => (b.id.to_string(), "builtin".into(), b.sha256.to_string()),
        None => (
            binding.builtin_model_id.into(),
            "builtin".into(),
            String::new(),
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationError {
    UnknownOperation(String),
    /// A `requires_region` operation was dispatched
    /// without a mask.
    MissingMask(String),
    /// `parameters_json` did not parse.
    BadParameters(String),
    /// The inference engine failed (model load or run).
    Inference(String),
    /// Reading a catalog model artifact failed.
    Io(String),
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationError::UnknownOperation(id) => write!(f, "unknown operation: {id}"),
            OperationError::MissingMask(msg) => write!(f, "missing mask: {msg}"),
            OperationError::BadParameters(msg) => write!(f, "bad parameters: {msg}"),
            OperationError::Inference(msg) => write!(f, "inference failed: {msg}"),
            OperationError::Io(msg) => write!(f, "io: {msg}"),
        }
    }
}

impl std::error::Error for OperationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_11_operations() {
        assert_eq!(registry().len(), 11);
    }

    #[test]
    fn get_operation_round_trips() {
        let info = get_operation("denoise_luminance").expect("present");
        assert_eq!(info.category, OperationCategory::Denoise);
        assert_eq!(info.safety_classification, SafetyClassification::Perceptual);
    }

    #[test]
    fn get_operation_unknown_returns_none() {
        assert!(get_operation("not_an_op").is_none());
    }

    #[test]
    fn generative_operations_require_region_or_are_upscale() {
        let trail = get_operation("trail_clean").unwrap();
        let sr = get_operation("super_resolution").unwrap();
        let inpaint = get_operation("inpaint").unwrap();
        let denoise = get_operation("denoise_luminance").unwrap();
        assert_eq!(
            trail.safety_classification,
            SafetyClassification::Generative
        );
        assert_eq!(sr.safety_classification, SafetyClassification::Generative);
        assert_eq!(
            inpaint.safety_classification,
            SafetyClassification::Generative
        );
        assert_ne!(
            denoise.safety_classification,
            SafetyClassification::Generative
        );
        assert!(trail.requires_region);
        assert!(inpaint.requires_region);
        assert!(!sr.requires_region);
    }

    #[test]
    fn dispatch_returns_passthrough_outcome() {
        let src = flat(16, 16, 1, 0.4);
        let out = dispatch_operation(
            "denoise_luminance",
            r#"{"strength":0.7}"#,
            "v1",
            "v2".into(),
            "pv1".into(),
            DispatchInputs {
                source: &src,
                mask: None,
                probe: &test_probe(),
                models_dir: None,
            },
        )
        .unwrap();
        assert_eq!(out.outcome.operation_id, "denoise_luminance");
        assert_eq!(out.outcome.source_image_version_id, "v1");
        assert_eq!(out.outcome.result_image_version_id, "v2");
        assert_eq!(out.outcome.parameters_hash.len(), 16);
    }

    #[test]
    fn dispatch_unknown_errors() {
        let src = flat(8, 8, 1, 0.5);
        let err = dispatch_operation(
            "missing_op",
            "{}",
            "v1",
            "v2".into(),
            "pv1".into(),
            DispatchInputs {
                source: &src,
                mask: None,
                probe: &test_probe(),
                models_dir: None,
            },
        )
        .unwrap_err();
        assert_eq!(err, OperationError::UnknownOperation("missing_op".into()));
    }

    #[test]
    fn same_parameters_produce_same_hash() {
        let src = flat(8, 8, 1, 0.5);
        let a = dispatch_operation(
            "denoise_luminance",
            r#"{"strength":0.7}"#,
            "v1",
            "v2".into(),
            "p".into(),
            DispatchInputs {
                source: &src,
                mask: None,
                probe: &test_probe(),
                models_dir: None,
            },
        )
        .unwrap();
        let b = dispatch_operation(
            "denoise_luminance",
            r#"{"strength":0.7}"#,
            "v1",
            "v2".into(),
            "p".into(),
            DispatchInputs {
                source: &src,
                mask: None,
                probe: &test_probe(),
                models_dir: None,
            },
        )
        .unwrap();
        assert_eq!(a.outcome.parameters_hash, b.outcome.parameters_hash);
    }

    #[test]
    fn safety_classification_round_trips() {
        for c in [
            SafetyClassification::Deterministic,
            SafetyClassification::Perceptual,
            SafetyClassification::Generative,
        ] {
            assert_eq!(parse_classification(c.as_str()), Some(c));
        }
    }

    fn parse_classification(raw: &str) -> Option<SafetyClassification> {
        registry()
            .into_iter()
            .map(|o| o.safety_classification)
            .find(|c| c.as_str() == raw)
    }

    // ─── CR-06 P5.1 — real dispatch behaviour ────────────────────────

    fn flat(w: usize, h: usize, c: usize, value: f32) -> F32Image {
        F32Image::from(ndarray::Array3::from_elem((c, h, w), value))
    }

    fn checkerboard(w: usize, h: usize) -> F32Image {
        let mut arr = ndarray::Array3::<f32>::zeros((1, h, w));
        for y in 0..h {
            for x in 0..w {
                arr[(0, y, x)] = if (x + y) % 2 == 0 { 0.9 } else { 0.1 };
            }
        }
        F32Image::from(arr)
    }

    /// Probe pinned to the Research tier so tile size
    /// resolves deterministically (512) in tests.
    fn test_probe() -> HardwareProbe {
        HardwareProbe {
            ram_mb: 16384,
            gpu_backend: crate::hardware::GpuBackend::Cpu,
            vram_mb: 0,
            cpu_cores: 8,
        }
    }

    fn variance(img: &F32Image) -> f32 {
        let n = (img.width() * img.height()) as f32;
        let (mut sum, mut sq) = (0.0f32, 0.0f32);
        for y in 0..img.height() {
            for x in 0..img.width() {
                let v = img[(0, y, x)];
                sum += v;
                sq += v * v;
            }
        }
        (sq / n) - (sum / n).powi(2)
    }

    #[test]
    fn every_registered_operation_has_a_model_binding() {
        for op in registry() {
            assert!(
                model_binding(&op.operation_id).is_some(),
                "no binding for {}",
                op.operation_id
            );
        }
    }

    #[test]
    fn denoise_actually_smooths_the_pixels() {
        let src = checkerboard(48, 48);
        let out = dispatch_operation(
            "denoise_luminance",
            r#"{"strength":0.9}"#,
            "v1",
            "v2".into(),
            "p".into(),
            DispatchInputs {
                source: &src,
                mask: None,
                probe: &test_probe(),
                models_dir: None,
            },
        )
        .unwrap();
        assert!(
            variance(&out.result_image) < variance(&src) * 0.6,
            "denoise must reduce variance (src={}, out={})",
            variance(&src),
            variance(&out.result_image)
        );
        assert_eq!(out.model_id, "builtin-blur-blend");
        assert_eq!(out.backend, "cpu");
        assert_eq!(out.model_hash.len(), 64, "sha256 hex digest");
        let tile_cfg: serde_json::Value =
            serde_json::from_str(&out.tile_configuration_json).unwrap();
        assert_eq!(tile_cfg["tiled"], true);
        assert_eq!(tile_cfg["tile_size"], 512);
    }

    #[test]
    fn dispatch_is_deterministic() {
        let src = checkerboard(32, 32);
        let run = || {
            dispatch_operation(
                "detail_enhance",
                r#"{"strength":0.5}"#,
                "v1",
                "v2".into(),
                "p".into(),
                DispatchInputs {
                    source: &src,
                    mask: None,
                    probe: &test_probe(),
                    models_dir: None,
                },
            )
            .unwrap()
        };
        let a = run();
        let b = run();
        for y in 0..32 {
            for x in 0..32 {
                assert_eq!(a.result_image[(0, y, x)], b.result_image[(0, y, x)]);
            }
        }
    }

    #[test]
    fn masked_blend_leaves_excluded_region_untouched() {
        use astroforge_core::masks::MaskKind;
        let src = checkerboard(32, 32);
        let mut mask = Mask::zeros(32, 32, MaskKind::User, "test".into());
        for y in 0..32 {
            for x in 0..16 {
                mask.set(x, y, 1.0);
            }
        }
        let out = dispatch_operation(
            "denoise_luminance",
            r#"{"strength":1.0}"#,
            "v1",
            "v2".into(),
            "p".into(),
            DispatchInputs {
                source: &src,
                mask: Some(&mask),
                probe: &test_probe(),
                models_dir: None,
            },
        )
        .unwrap();
        // Excluded right half equals the source exactly.
        for y in 0..32 {
            for x in 16..32 {
                assert_eq!(out.result_image[(0, y, x)], src[(0, y, x)]);
            }
        }
        // Included left half was blurred (interior pixel).
        assert!((out.result_image[(0, 8, 8)] - src[(0, 8, 8)]).abs() > 1e-3);
    }

    #[test]
    fn requires_region_operation_without_mask_errors() {
        let src = flat(16, 16, 1, 0.5);
        let err = dispatch_operation(
            "inpaint",
            "{}",
            "v1",
            "v2".into(),
            "p".into(),
            DispatchInputs {
                source: &src,
                mask: None,
                probe: &test_probe(),
                models_dir: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, OperationError::MissingMask(_)));
    }

    #[test]
    fn inpaint_with_mask_fills_masked_region() {
        use astroforge_core::masks::MaskKind;
        let mut src = flat(24, 24, 1, 0.2);
        src[(0, 12, 12)] = 1.0; // the "defect"
        let mut mask = Mask::zeros(24, 24, MaskKind::User, "test".into());
        mask.set(12, 12, 1.0);
        let out = dispatch_operation(
            "inpaint",
            "{}",
            "v1",
            "v2".into(),
            "p".into(),
            DispatchInputs {
                source: &src,
                mask: Some(&mask),
                probe: &test_probe(),
                models_dir: None,
            },
        )
        .unwrap();
        assert!(
            out.result_image[(0, 12, 12)] < 0.9,
            "masked defect must be filled toward the local estimate"
        );
        assert_eq!(out.model_id, "builtin-masked-fill");
    }

    #[test]
    fn super_resolution_doubles_dims_untiled() {
        let src = flat(10, 8, 3, 0.4);
        let out = dispatch_operation(
            "super_resolution",
            r#"{"scale":2}"#,
            "v1",
            "v2".into(),
            "p".into(),
            DispatchInputs {
                source: &src,
                mask: None,
                probe: &test_probe(),
                models_dir: None,
            },
        )
        .unwrap();
        assert_eq!(out.result_image.width(), 20);
        assert_eq!(out.result_image.height(), 16);
        assert_eq!(out.result_image.channels(), 3);
        let tile_cfg: serde_json::Value =
            serde_json::from_str(&out.tile_configuration_json).unwrap();
        assert_eq!(tile_cfg["tiled"], false);
        assert_eq!(tile_cfg["spatial_scale"], 2);
    }

    #[test]
    fn probe_clamps_tile_size() {
        let low_probe = HardwareProbe {
            ram_mb: 2048,
            gpu_backend: crate::hardware::GpuBackend::Cpu,
            vram_mb: 0,
            cpu_cores: 2,
        };
        let src = checkerboard(24, 24);
        let out = dispatch_operation(
            "denoise_luminance",
            r#"{"strength":0.5}"#,
            "v1",
            "v2".into(),
            "p".into(),
            DispatchInputs {
                source: &src,
                mask: None,
                probe: &low_probe,
                models_dir: None,
            },
        )
        .unwrap();
        let tile_cfg: serde_json::Value =
            serde_json::from_str(&out.tile_configuration_json).unwrap();
        assert_eq!(tile_cfg["tile_size"], 256);
    }

    #[test]
    fn bad_parameters_json_errors() {
        let src = flat(8, 8, 1, 0.5);
        let err = dispatch_operation(
            "denoise_luminance",
            "not json",
            "v1",
            "v2".into(),
            "p".into(),
            DispatchInputs {
                source: &src,
                mask: None,
                probe: &test_probe(),
                models_dir: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, OperationError::BadParameters(_)));
    }
}
