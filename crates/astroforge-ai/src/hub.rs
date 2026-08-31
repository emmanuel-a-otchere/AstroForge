use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub stage: String,
    pub license: String,
    pub input_channels: u32,
    pub input_tile_size: u32,
    pub output_channels: u32,
    pub scale_factor: u32,
}

pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            name: "swinir-denoise-astro".into(),
            version: "1.0.0".into(),
            sha256: "placeholder".into(),
            size_bytes: 45_000_000,
            stage: "noise_reduction".into(),
            license: "MIT".into(),
            input_channels: 3,
            input_tile_size: 512,
            output_channels: 3,
            scale_factor: 1,
        },
        ModelInfo {
            name: "swinir-sr-astro-2x".into(),
            version: "1.0.0".into(),
            sha256: "placeholder".into(),
            size_bytes: 50_000_000,
            stage: "ai_super_resolution".into(),
            license: "MIT".into(),
            input_channels: 3,
            input_tile_size: 512,
            output_channels: 3,
            scale_factor: 2,
        },
        ModelInfo {
            name: "swin2sr-dejpeg".into(),
            version: "1.0.0".into(),
            sha256: "placeholder".into(),
            size_bytes: 40_000_000,
            stage: "compression_cleanup".into(),
            license: "MIT".into(),
            input_channels: 3,
            input_tile_size: 512,
            output_channels: 3,
            scale_factor: 1,
        },
        ModelInfo {
            name: "star-seg-v1".into(),
            version: "1.0.0".into(),
            sha256: "placeholder".into(),
            size_bytes: 35_000_000,
            stage: "star_segmentation".into(),
            license: "MIT".into(),
            input_channels: 3,
            input_tile_size: 512,
            output_channels: 2,
            scale_factor: 1,
        },
        ModelInfo {
            name: "cloud-score-v1".into(),
            version: "1.0.0".into(),
            sha256: "placeholder".into(),
            size_bytes: 20_000_000,
            stage: "quality_filter".into(),
            license: "MIT".into(),
            input_channels: 3,
            input_tile_size: 256,
            output_channels: 1,
            scale_factor: 1,
        },
        ModelInfo {
            name: "color-cal-net".into(),
            version: "1.0.0".into(),
            sha256: "placeholder".into(),
            size_bytes: 15_000_000,
            stage: "color_calibration".into(),
            license: "MIT".into(),
            input_channels: 3,
            input_tile_size: 512,
            output_channels: 3,
            scale_factor: 1,
        },
        ModelInfo {
            name: "trail-lama-tiny".into(),
            version: "1.0.0".into(),
            sha256: "placeholder".into(),
            size_bytes: 30_000_000,
            stage: "trail_inpaint".into(),
            license: "MIT".into(),
            input_channels: 4,
            input_tile_size: 512,
            output_channels: 3,
            scale_factor: 1,
        },
    ]
}

pub fn get_model(name: &str) -> Option<ModelInfo> {
    model_catalog().into_iter().find(|m| m.name == name)
}

pub fn get_models_for_stage(stage: &str) -> Vec<ModelInfo> {
    model_catalog()
        .into_iter()
        .filter(|m| m.stage == stage)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_catalog_has_7_models() {
        let catalog = model_catalog();
        assert_eq!(catalog.len(), 7);
    }

    #[test]
    fn test_get_model_by_name() {
        let model = get_model("swinir-denoise-astro").unwrap();
        assert_eq!(model.stage, "noise_reduction");
        assert_eq!(model.scale_factor, 1);
    }

    #[test]
    fn test_get_models_for_stage() {
        let models = get_models_for_stage("noise_reduction");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "swinir-denoise-astro");
    }

    #[test]
    fn test_all_models_have_permissive_license() {
        for m in model_catalog() {
            assert!(matches!(m.license.as_str(), "MIT" | "Apache-2.0"));
        }
    }
}
