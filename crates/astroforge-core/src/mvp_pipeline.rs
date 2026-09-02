use crate::calibration::StreamingCalibrator;
use crate::export::{FrameStats, ProcessingReport, StageParams};
use crate::image::F32Image;
use crate::ingest::{FrameType, SessionManifest};
use crate::registration::{self, AffineTransform};
use crate::stacking::{self, StreamingStacker};
use crate::stretching;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub verbosity: Verbosity,
    pub lights_only: bool,
    pub kappa: f64,
    pub max_iterations: u32,
    pub reject_percentile: f64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            verbosity: Verbosity::Beginner,
            lights_only: false,
            kappa: 3.0,
            max_iterations: 5,
            reject_percentile: 15.0,
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
    pub error: Option<String>,
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

    let _stretched = stretching::auto_stretch(&stack_result.image);

    stage_params.push(StageParams {
        stage_id: "stretching".into(),
        params: [("method".into(), "arcsinh_auto".into())]
            .into_iter()
            .collect(),
    });

    let frame_stats = compute_frame_stats(manifest);

    PipelineResult {
        success: true,
        report: ProcessingReport {
            session_id: manifest.session_id.clone(),
            frame_stats,
            rejected_frames: vec![],
            stage_parameters: stage_params,
            export_path: None,
        },
        error: None,
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
}
