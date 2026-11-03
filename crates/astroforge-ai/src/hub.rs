use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    /// Pinned content digest, or "unverified" for placeholder
    /// entries that fail closed at runtime (see
    /// `inference::UNVERIFIED_SHA256`).
    pub sha256: String,
    /// Exact artifact size; 0 for unannounced placeholders.
    pub size_bytes: u64,
    pub stage: String,
    pub license: String,
    pub input_channels: u32,
    pub input_tile_size: u32,
    pub output_channels: u32,
    pub scale_factor: u32,
    /// Runtime-download URL for installable models (`None` for
    /// unpinned placeholders). The installer verifies the fetched
    /// bytes against `sha256` before installing (fail-closed).
    pub download_url: Option<String>,
}

pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            name: "swinir-denoise-astro".into(),
            version: "1.0.0".into(),
            sha256: "5fb427215c6a893b4c4aa27b6ab0aad66742fb9cb7ab24f65eff27b09cc2ea74".into(),
            size_bytes: 54_860_607,
            stage: "noise_reduction".into(),
            license: "Apache-2.0".into(),
            input_channels: 3,
            // Fixed 128x128 tiles per the publisher's card (SwinIR
            // window attention is brittle under dynamic H/W).
            input_tile_size: 128,
            output_channels: 3,
            scale_factor: 1,
            download_url: Some("https://huggingface.co/Heliosoph/swinir-onnx/resolve/main/swinir_denoising_color_25.onnx".into()),
        },
        ModelInfo {
            name: "swin2sr-realworld-x4".into(),
            version: "1.0.0".into(),
            sha256: "13474507a83b3dc254df0fbf32f4a34ffa62070d62b33cc99eacef43df3a3692".into(),
            size_bytes: 18_935_979,
            stage: "ai_super_resolution".into(),
            license: "Apache-2.0".into(),
            input_channels: 3,
            input_tile_size: 64,
            output_channels: 3,
            scale_factor: 4,
            download_url: Some("https://huggingface.co/onnx-community/swin2SR-realworld-sr-x4-64-bsrgan-psnr-ONNX/resolve/main/onnx/model_int8.onnx".into()),
        },
        ModelInfo {
            name: "swin2sr-dejpeg".into(),
            version: "1.0.0".into(),
            sha256: "unverified".into(),
            size_bytes: 0,
            stage: "compression_cleanup".into(),
            license: "TBD".into(),
            input_channels: 3,
            input_tile_size: 512,
            output_channels: 3,
            download_url: None,
            scale_factor: 1,
        },
        ModelInfo {
            name: "star-seg-v1".into(),
            version: "1.0.0".into(),
            sha256: "unverified".into(),
            size_bytes: 0,
            stage: "star_segmentation".into(),
            license: "TBD".into(),
            input_channels: 3,
            input_tile_size: 512,
            output_channels: 2,
            download_url: None,
            scale_factor: 1,
        },
        ModelInfo {
            name: "cloud-score-v1".into(),
            version: "1.0.0".into(),
            sha256: "unverified".into(),
            size_bytes: 0,
            stage: "quality_filter".into(),
            license: "TBD".into(),
            input_channels: 3,
            input_tile_size: 256,
            output_channels: 1,
            download_url: None,
            scale_factor: 1,
        },
        ModelInfo {
            name: "color-cal-net".into(),
            version: "1.0.0".into(),
            sha256: "unverified".into(),
            size_bytes: 0,
            stage: "color_calibration".into(),
            license: "TBD".into(),
            input_channels: 3,
            input_tile_size: 512,
            output_channels: 3,
            download_url: None,
            scale_factor: 1,
        },
        ModelInfo {
            name: "trail-lama-tiny".into(),
            version: "1.0.0".into(),
            sha256: "unverified".into(),
            size_bytes: 0,
            stage: "trail_inpaint".into(),
            license: "TBD".into(),
            input_channels: 4,
            input_tile_size: 512,
            output_channels: 3,
            download_url: None,
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
    fn test_pinned_models_have_permissive_license() {
        // RT-7: pinned (downloadable) models must carry a vetted SPDX
        // license; unpinned placeholders honestly say "TBD" until
        // their DP#4 close-out.
        for m in model_catalog() {
            if m.download_url.is_some() {
                assert!(
                    matches!(m.license.as_str(), "MIT" | "Apache-2.0"),
                    "{} is downloadable but licensed {:?}",
                    m.name,
                    m.license
                );
                assert_ne!(m.sha256, "unverified");
                assert!(m.size_bytes > 0);
            } else {
                assert_eq!(m.license, "TBD");
                assert_eq!(m.sha256, "unverified");
            }
        }
    }

    #[test]
    fn test_pinned_hub_entries_match_download_specs() {
        for spec in crate::downloader::DOWNLOAD_SPECS {
            let info = get_model(spec.id)
                .unwrap_or_else(|| panic!("{} missing from hub catalog", spec.id));
            assert_eq!(info.sha256, spec.sha256, "{} digest drift", spec.id);
            assert!(
                info.download_url.is_some(),
                "{} lacks download url",
                spec.id
            );
        }
    }
}
