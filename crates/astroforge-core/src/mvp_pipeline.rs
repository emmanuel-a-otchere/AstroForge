use crate::background;
use crate::calibration::StreamingCalibrator;
use crate::color_calibration;
use crate::crop::{self, CropRegion};
use crate::curves::{self, CurvesParams};
use crate::export::{self, FrameStats, ProcessingReport, StageParams};
use crate::image::F32Image;
use crate::ingest::{FrameType, SessionManifest};
use crate::registration::{self, AffineTransform};
use crate::stacking::{self, StreamingStacker};
use crate::star_segmentation;
use crate::stretching;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub verbosity: Verbosity,
    pub lights_only: bool,
    pub kappa: f64,
    pub max_iterations: u32,
    pub reject_percentile: f64,
    /// Optional crop applied after stacking (P1.5-M6-T2). When `None`,
    /// no crop is performed — the pipeline never silently auto-crops.
    pub crop: Option<CropRegion>,
    /// Enable background extraction (P1.5-M6-T3). When true, a sample
    /// grid is taken from the stacked image and the gradient is
    /// subtracted. Defaults to false (backward-compatible).
    pub background_enabled: bool,
    /// Enable colour calibration (P1.5-M6-T4). When true, the stacked
    /// image goes through `apply_color_calibration`. Defaults to false.
    pub color_calibration_enabled: bool,
    /// Enable star segmentation (P1.5-M6-T8). When true, the post-stretch
    /// image is split into starless + stars layers; both are returned on
    /// `PipelineResult`. Defaults to false.
    pub star_segmentation_enabled: bool,
    /// Optional curves applied after stretching (P1.5-M6-T9). When
    /// `None`, the curves stage is skipped entirely.
    pub curves: Option<CurvesParams>,
    /// If set, write the final processed image to this path as JPEG
    /// (P1.5-M6-T10). Defaults to None (no file written).
    pub export_path: Option<std::path::PathBuf>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            verbosity: Verbosity::Beginner,
            lights_only: false,
            kappa: 3.0,
            max_iterations: 5,
            reject_percentile: 15.0,
            crop: None,
            background_enabled: false,
            color_calibration_enabled: false,
            star_segmentation_enabled: false,
            curves: None,
            export_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum Verbosity {
    #[default]
    Beginner,
    Intermediate,
    Expert,
}

impl Verbosity {
    pub fn dialog_mode(&self) -> DialogMode {
        match self {
            Verbosity::Beginner => DialogMode::Auto,
            Verbosity::Intermediate => DialogMode::Confirm,
            Verbosity::Expert => DialogMode::Manual,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DialogMode {
    Auto,
    Confirm,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub success: bool,
    pub report: ProcessingReport,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub preview: Option<PreviewImage>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub stretched: Option<crate::image::F32Image>,
    /// Starless layer (P1.5-M6-T8). Only populated when
    /// `PipelineConfig.star_segmentation_enabled` is true.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub starless: Option<crate::image::F32Image>,
    /// Stars-only layer (P1.5-M6-T8). Only populated when
    /// `PipelineConfig.star_segmentation_enabled` is true.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub stars: Option<crate::image::F32Image>,
    /// Path the final image was exported to (P1.5-M6-T10). Only
    /// populated when `PipelineConfig.export_path` was set.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exported_to: Option<std::path::PathBuf>,
    pub error: Option<String>,
}

/// RGBA preview bitmap produced by the MVP pipeline. Width × height
/// pixels, each 4 bytes (R, G, B, A). Sent to the UI for live preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewImage {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

pub fn run_pipeline(
    manifest: &SessionManifest,
    calibrated_frames: Vec<F32Image>,
    config: &PipelineConfig,
) -> PipelineResult {
    let mut stage_params: Vec<StageParams> = Vec::new();

    if calibrated_frames.is_empty() {
        return PipelineResult {
            success: false,
            report: ProcessingReport {
                session_id: manifest.session_id.clone(),
                frame_stats: compute_frame_stats(manifest),
                rejected_frames: vec![],
                stage_parameters: stage_params,
                export_path: None,
            },
            preview: None,
            stretched: None,
            starless: None,
            stars: None,
            exported_to: None,
            error: Some("No calibrated frames provided".into()),
        };
    }

    let ref_stars: Vec<Vec<registration::Star>> = calibrated_frames
        .iter()
        .map(|f| registration::extract_stars(f, 3.0))
        .collect();

    let ref_idx = registration::select_reference_frame(&ref_stars);

    let aligned_frames: Vec<F32Image> = calibrated_frames
        .iter()
        .enumerate()
        .map(|(i, frame)| {
            if i == ref_idx {
                frame.clone()
            } else {
                let transform = registration::compute_transform(&ref_stars[ref_idx], &ref_stars[i])
                    .unwrap_or(AffineTransform {
                        dx: 0.0,
                        dy: 0.0,
                        rotation: 0.0,
                        scale: 1.0,
                    });
                registration::apply_transform(frame, &transform)
            }
        })
        .collect();

    stage_params.push(StageParams {
        stage_id: "registration".into(),
        params: [("reference_frame".into(), ref_idx.to_string())]
            .into_iter()
            .collect(),
    });

    let stack_result =
        stacking::kappa_sigma_stack(&aligned_frames, config.kappa, config.max_iterations)
            .unwrap_or_else(|_| stacking::StackResult {
                image: F32Image::new(1, 1, 1),
                weight_map: F32Image::new(1, 1, 1),
                frame_count: 0,
                rejected_count: 0,
            });

    stage_params.push(StageParams {
        stage_id: "stacking".into(),
        params: [
            ("kappa".into(), config.kappa.to_string()),
            ("iterations".into(), config.max_iterations.to_string()),
            ("rejected".into(), stack_result.rejected_count.to_string()),
        ]
        .into_iter()
        .collect(),
    });

    // ── P1.5-M6-T2 — Crop ───
    let post_crop: F32Image = if let Some(region) = &config.crop {
        let cropped = crop::crop(&stack_result.image, region);
        stage_params.push(StageParams {
            stage_id: "crop".into(),
            params: [
                ("x".into(), region.x.to_string()),
                ("y".into(), region.y.to_string()),
                ("width".into(), region.width.to_string()),
                ("height".into(), region.height.to_string()),
            ]
            .into_iter()
            .collect(),
        });
        cropped
    } else {
        // Pipeline never silently auto-crops; the option is explicit.
        stack_result.image
    };

    // ── P1.5-M6-T3 — Background Extraction ───
    let post_background: F32Image = if config.background_enabled {
        let h = post_crop.height();
        let w = post_crop.width();
        // Sample a 5×5 grid of evenly-distributed points across the
        // image. Real implementations would let the user place these
        // points manually; the 5×5 grid is the same default most
        // tools ship with for unguided background extraction.
        let mut samples: Vec<(f64, f64)> = Vec::with_capacity(25);
        for i in 0..5 {
            for j in 0..5 {
                let x = (w as f64) * (i as f64 + 0.5) / 5.0;
                let y = (h as f64) * (j as f64 + 0.5) / 5.0;
                samples.push((x, y));
            }
        }
        let gradient = background::extract_background(&post_crop, &samples);
        let cleaned = background::subtract_gradient(&post_crop, &gradient);
        stage_params.push(StageParams {
            stage_id: "background_extraction".into(),
            params: [("samples".into(), samples.len().to_string())]
                .into_iter()
                .collect(),
        });
        cleaned
    } else {
        post_crop
    };

    // ── P1.5-M6-T4 — Colour Calibration ───
    let post_calibration: F32Image = if config.color_calibration_enabled {
        let calibration = color_calibration::calibrate_color(&post_background);
        let calibrated = color_calibration::apply_color_calibration(&post_background, &calibration);
        stage_params.push(StageParams {
            stage_id: "color_calibration".into(),
            params: [
                ("method".into(), "auto".into()),
                ("bounded".into(), "true".into()),
            ]
            .into_iter()
            .collect(),
        });
        calibrated
    } else {
        post_background
    };

    let stretched = stretching::auto_stretch(&post_calibration);

    stage_params.push(StageParams {
        stage_id: "stretching".into(),
        params: [("method".into(), "arcsinh_auto".into())]
            .into_iter()
            .collect(),
    });

    // ── P1.5-M6-T9 — Creative / Curves ───
    let post_curves: F32Image = if let Some(params) = &config.curves {
        let out = curves::apply_curves(&stretched, params);
        stage_params.push(StageParams {
            stage_id: "curves".into(),
            params: [
                ("saturation".into(), params.saturation.to_string()),
                ("r_gamma".into(), params.r.gamma.to_string()),
                ("g_gamma".into(), params.g.gamma.to_string()),
                ("b_gamma".into(), params.b.gamma.to_string()),
            ]
            .into_iter()
            .collect(),
        });
        out
    } else {
        stretched.clone()
    };

    // ── P1.5-M6-T8 — Star Handling ───
    let (starless, stars) = if config.star_segmentation_enabled {
        let segmentation = star_segmentation::segment_stars(&post_curves, 3.0);
        let starless_layer =
            star_segmentation::enhance_background_layer(&segmentation.background_layer, 1.0, 1.0);
        let stars_layer = star_segmentation::enhance_star_layer(&segmentation.star_layer, 1.0, 1.0);
        stage_params.push(StageParams {
            stage_id: "star_segmentation".into(),
            params: [("threshold_sigma".into(), "3.0".into())]
                .into_iter()
                .collect(),
        });
        (Some(starless_layer), Some(stars_layer))
    } else {
        (None, None)
    };

    // ── P1.5-M6-T10 — Export ───
    let exported_to: Option<std::path::PathBuf> = if let Some(path) = &config.export_path {
        let file_result = std::fs::File::create(path);
        match file_result {
            Ok(mut file) => {
                if export::export_jpeg_8bit(&post_curves, 90, &mut file).is_ok() {
                    stage_params.push(StageParams {
                        stage_id: "export".into(),
                        params: [
                            ("format".into(), "jpeg".into()),
                            ("quality".into(), "90".into()),
                            ("path".into(), path.display().to_string()),
                        ]
                        .into_iter()
                        .collect(),
                    });
                    Some(path.clone())
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    } else {
        None
    };

    let frame_stats = compute_frame_stats(manifest);

    let preview = f32_image_to_rgba(&post_curves);

    PipelineResult {
        success: true,
        report: ProcessingReport {
            session_id: manifest.session_id.clone(),
            frame_stats,
            rejected_frames: vec![],
            stage_parameters: stage_params,
            export_path: exported_to.as_ref().map(|p| p.display().to_string()),
        },
        preview: Some(preview),
        stretched: Some(stretched),
        starless,
        stars,
        exported_to,
        error: None,
    }
}

/// Convert a 3-channel float image to an 8-bit RGBA byte vector with
/// percentile-based normalization. Per-channel normalisation handles
/// narrowband stacks where one channel may dominate; the alpha channel
/// is set to 255 everywhere so the result paints opaquely.
///
/// Used by the Tauri bridge to feed the WebGL renderer / preview UI.
pub fn f32_image_to_rgba(img: &F32Image) -> PreviewImage {
    let width = img.width();
    let height = img.height();
    let channels = img.channels().min(3);

    // 1st / 99.5th percentile per channel — robust against hot pixels
    // and cosmic ray hits that would otherwise compress the dynamic
    // range during normalisation.
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for c in 0..channels {
        let mut values: Vec<f32> = img
            .as_standard_layout()
            .slice(ndarray::s![c, .., ..])
            .iter()
            .copied()
            .collect();
        if values.is_empty() {
            continue;
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = values.len();
        let p_lo = values[(n as f64 * 0.005) as usize];
        let p_hi = values[((n as f64 * 0.995) as usize).min(n - 1)];
        lo[c] = p_lo;
        hi[c] = p_hi;
    }

    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            for c in 0..3 {
                let raw = if c < channels { img[(c, y, x)] } else { 0.0 };
                let range = hi[c] - lo[c];
                let normalized = if range > 1e-6 {
                    ((raw - lo[c]) / range).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                rgba.push((normalized * 255.0) as u8);
            }
            rgba.push(255);
        }
    }

    PreviewImage {
        width,
        height,
        rgba,
    }
}

fn compute_frame_stats(manifest: &SessionManifest) -> FrameStats {
    let mut lights = 0;
    let mut darks = 0;
    let mut flats = 0;
    let mut biases = 0;
    let mut total_exposure = 0.0;

    for frame in &manifest.frames {
        match frame.frame_type {
            FrameType::Light => {
                lights += 1;
                total_exposure += frame.exptime.unwrap_or(0.0);
            }
            FrameType::Dark => darks += 1,
            FrameType::Flat => flats += 1,
            FrameType::Bias => biases += 1,
        }
    }

    FrameStats {
        total_frames: manifest.frames.len(),
        lights,
        darks,
        flats,
        biases,
        total_exposure,
    }
}

pub fn run_streaming_pipeline(
    frames: impl Iterator<Item = F32Image>,
    width: usize,
    height: usize,
    channels: usize,
    calibrator: &StreamingCalibrator,
) -> stacking::StackResult {
    let mut stacker = StreamingStacker::new(width, height, channels);
    for frame in frames {
        let calibrated = calibrator.calibrate_frame(&frame);
        stacker.add_frame(&calibrated);
    }
    stacker.result()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{FrameInfo, FrameType, LightGroup};
    use std::path::PathBuf;

    fn make_manifest(n_lights: usize) -> SessionManifest {
        let frames: Vec<FrameInfo> = (0..n_lights)
            .map(|i| FrameInfo {
                path: PathBuf::from(format!("light_{:03}.fits", i)),
                frame_type: FrameType::Light,
                exptime: Some(120.0),
                filter: Some("Ha".into()),
                date_obs: None,
                ccd_temp: Some(-10.0),
                width: Some(64),
                height: Some(64),
                binning: Some(1),
                anomalies: vec![],
            })
            .collect();

        SessionManifest {
            session_id: "test-session".into(),
            source_dir: "/test".into(),
            frames,
            light_groups: vec![LightGroup {
                filter: "Ha".into(),
                binning: 1,
                frame_paths: vec![],
            }],
        }
    }

    fn make_test_frame(width: usize, height: usize) -> F32Image {
        let mut img = F32Image::new(width, height, 1);
        for y in 0..height {
            for x in 0..width {
                img[(0, y, x)] = (x + y) as f32;
            }
        }
        img
    }

    #[test]
    fn test_pipeline_config_defaults() {
        let config = PipelineConfig::default();
        assert_eq!(config.verbosity, Verbosity::Beginner);
        assert!(!config.lights_only);
        assert!((config.kappa - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_verbosity_dialog_mode() {
        assert_eq!(Verbosity::Beginner.dialog_mode(), DialogMode::Auto);
        assert_eq!(Verbosity::Intermediate.dialog_mode(), DialogMode::Confirm);
        assert_eq!(Verbosity::Expert.dialog_mode(), DialogMode::Manual);
    }

    #[test]
    fn test_run_pipeline() {
        let manifest = make_manifest(5);
        let frames: Vec<F32Image> = (0..5).map(|_| make_test_frame(8, 8)).collect();
        let config = PipelineConfig::default();
        let result = run_pipeline(&manifest, frames, &config);
        assert!(result.success);
        assert_eq!(result.report.frame_stats.lights, 5);
        assert_eq!(result.report.frame_stats.total_exposure, 600.0);
        assert!(!result.report.stage_parameters.is_empty());
    }

    #[test]
    fn test_streaming_pipeline() {
        let frames = vec![
            make_test_frame(8, 8),
            make_test_frame(8, 8),
            make_test_frame(8, 8),
        ];
        let calibrator = StreamingCalibrator::new(None, None, None);
        let result = run_streaming_pipeline(frames.into_iter(), 8, 8, 1, &calibrator);
        assert_eq!(result.frame_count, 3);
    }

    #[test]
    fn test_streaming_pipeline_30_frames() {
        let frames: Vec<F32Image> = (0..30).map(|_| make_test_frame(8, 8)).collect();
        let calibrator = StreamingCalibrator::new(None, None, None);
        let result = run_streaming_pipeline(frames.into_iter(), 8, 8, 1, &calibrator);
        assert_eq!(result.frame_count, 30);
        assert!((result.weight_map[(0, 0, 0)] - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_pipeline() {
        let manifest = make_manifest(0);
        let config = PipelineConfig::default();
        let result = run_pipeline(&manifest, vec![], &config);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_run_pipeline_emits_preview() {
        let manifest = make_manifest(3);
        let frames: Vec<F32Image> = (0..3)
            .map(|i| {
                let mut img = F32Image::new(16, 16, 3);
                for y in 0..16 {
                    for x in 0..16 {
                        img[(0, y, x)] = (x + y + i) as f32;
                        img[(1, y, x)] = (x + y + i) as f32 * 0.5;
                        img[(2, y, x)] = (x + y + i) as f32 * 0.25;
                    }
                }
                img
            })
            .collect();
        let config = PipelineConfig::default();
        let result = run_pipeline(&manifest, frames, &config);
        assert!(result.success);
        let preview = result.preview.expect("preview present");
        assert_eq!(preview.width, 16);
        assert_eq!(preview.height, 16);
        assert_eq!(preview.rgba.len(), 16 * 16 * 4);
        // Alpha must be opaque everywhere.
        assert!(preview.rgba.iter().skip(3).step_by(4).all(|&a| a == 255));
    }

    #[test]
    fn test_f32_image_to_rgba_percentiles() {
        // Linear ramp 0..1 — percentiles should land inside the
        // distribution so the middle value maps to roughly the
        // middle byte (~128).
        let mut img = F32Image::new(64, 64, 1);
        for y in 0..64 {
            for x in 0..64 {
                img[(0, y, x)] = (x + y * 64) as f32 / (64.0 * 64.0 - 1.0);
            }
        }
        let preview = f32_image_to_rgba(&img);
        assert_eq!(preview.width, 64);
        assert_eq!(preview.height, 64);
        // Pixel at (32, 32) ≈ middle of the ramp should map to
        // mid-range, not blow out to either end.
        let middle = preview.rgba[(32 * 64 + 32) * 4];
        assert!(middle > 64 && middle < 192, "got middle={}", middle);
        // First pixel ≈ 0 should map near black.
        let first = preview.rgba[0];
        assert!(first < 16, "first byte={} should be near black", first);
        // Last pixel ≈ 1 should map near white.
        let last = preview.rgba[(63 * 64 + 63) * 4];
        assert!(last > 192, "last byte={} should be near white", last);
    }
}
