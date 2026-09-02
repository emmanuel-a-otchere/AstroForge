use astroforge_core::image::F32Image;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterminismRecord {
    pub model_name: String,
    pub model_sha256: String,
    pub backend: String,
    pub tile_size: u32,
    pub overlap: u32,
    pub seed: Option<u64>,
    pub timestamp: String,
}

impl DeterminismRecord {
    pub fn new(
        model_name: String,
        model_sha256: String,
        backend: String,
        tile_size: u32,
        overlap: u32,
        seed: Option<u64>,
    ) -> Self {
        Self {
            model_name,
            model_sha256,
            backend,
            tile_size,
            overlap,
            seed,
            timestamp: now_iso(),
        }
    }
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("1970-01-01T00:00:{}Z", secs)
}

pub struct InferenceContext {
    pub model_name: String,
    pub backend: String,
    pub tile_size: u32,
    pub overlap: u32,
    pub seed: Option<u64>,
}

impl InferenceContext {
    pub fn record(&self, sha256: &str) -> DeterminismRecord {
        DeterminismRecord::new(
            self.model_name.clone(),
            sha256.to_string(),
            self.backend.clone(),
            self.tile_size,
            self.overlap,
            self.seed,
        )
    }
}

pub trait AiModel: Send + Sync {
    fn name(&self) -> &str;
    fn stage(&self) -> &str;
    fn run(&self, image: &F32Image) -> F32Image;
    fn determinism(&self) -> DeterminismRecord;
}

pub struct SwinIrDenoise {
    ctx: InferenceContext,
}

impl SwinIrDenoise {
    pub fn new(ctx: InferenceContext) -> Self {
        Self { ctx }
    }
}

impl AiModel for SwinIrDenoise {
    fn name(&self) -> &str {
        "swinir-denoise-astro"
    }
    fn stage(&self) -> &str {
        "noise_reduction"
    }
    fn run(&self, image: &F32Image) -> F32Image {
        image.clone()
    }
    fn determinism(&self) -> DeterminismRecord {
        self.ctx.record("placeholder")
    }
}

pub struct SwinIrSuperRes {
    ctx: InferenceContext,
    scale: u32,
}

impl SwinIrSuperRes {
    pub fn new(ctx: InferenceContext, scale: u32) -> Self {
        Self { ctx, scale }
    }
}

impl AiModel for SwinIrSuperRes {
    fn name(&self) -> &str {
        "swinir-sr-astro-2x"
    }
    fn stage(&self) -> &str {
        "ai_super_resolution"
    }
    fn run(&self, image: &F32Image) -> F32Image {
        let new_w = image.width() * self.scale as usize;
        let new_h = image.height() * self.scale as usize;
        let mut result = F32Image::new(new_w, new_h, image.channels());
        for c in 0..image.channels() {
            for y in 0..new_h {
                for x in 0..new_w {
                    let src_x = x / self.scale as usize;
                    let src_y = y / self.scale as usize;
                    result[(c, y, x)] = image[(c, src_y, src_x)];
                }
            }
        }
        result
    }
    fn determinism(&self) -> DeterminismRecord {
        self.ctx.record("placeholder")
    }
}

pub struct Swin2SrDejpeg {
    ctx: InferenceContext,
}

impl Swin2SrDejpeg {
    pub fn new(ctx: InferenceContext) -> Self {
        Self { ctx }
    }
}

impl AiModel for Swin2SrDejpeg {
    fn name(&self) -> &str {
        "swin2sr-dejpeg"
    }
    fn stage(&self) -> &str {
        "compression_cleanup"
    }
    fn run(&self, image: &F32Image) -> F32Image {
        image.clone()
    }
    fn determinism(&self) -> DeterminismRecord {
        self.ctx.record("placeholder")
    }
}

pub struct StarSegV1 {
    ctx: InferenceContext,
}

impl StarSegV1 {
    pub fn new(ctx: InferenceContext) -> Self {
        Self { ctx }
    }
}

impl AiModel for StarSegV1 {
    fn name(&self) -> &str {
        "star-seg-v1"
    }
    fn stage(&self) -> &str {
        "star_segmentation"
    }
    fn run(&self, image: &F32Image) -> F32Image {
        let mut result = F32Image::new(image.width(), image.height(), 2);
        for y in 0..image.height() {
            for x in 0..image.width() {
                let val = image[(0, y, x)];
                result[(0, y, x)] = if val > 100.0 { 1.0 } else { 0.0 };
                result[(1, y, x)] = 1.0 - result[(0, y, x)];
            }
        }
        result
    }
    fn determinism(&self) -> DeterminismRecord {
        self.ctx.record("placeholder")
    }
}

pub struct CloudScoreV1 {
    ctx: InferenceContext,
}

impl CloudScoreV1 {
    pub fn new(ctx: InferenceContext) -> Self {
        Self { ctx }
    }
}

impl AiModel for CloudScoreV1 {
    fn name(&self) -> &str {
        "cloud-score-v1"
    }
    fn stage(&self) -> &str {
        "quality_filter"
    }
    fn run(&self, image: &F32Image) -> F32Image {
        let mut result = F32Image::new(1, 1, 1);
        let mean = image.iter().sum::<f32>() / image.len() as f32;
        let var = image.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / image.len() as f32;
        let cloud_score = (var.sqrt() / mean.max(1e-10)).min(1.0);
        result[(0, 0, 0)] = cloud_score;
        result
    }
    fn determinism(&self) -> DeterminismRecord {
        self.ctx.record("placeholder")
    }
}

pub struct ColorCalNet {
    ctx: InferenceContext,
}

impl ColorCalNet {
    pub fn new(ctx: InferenceContext) -> Self {
        Self { ctx }
    }
}

impl AiModel for ColorCalNet {
    fn name(&self) -> &str {
        "color-cal-net"
    }
    fn stage(&self) -> &str {
        "color_calibration"
    }
    fn run(&self, image: &F32Image) -> F32Image {
        image.clone()
    }
    fn determinism(&self) -> DeterminismRecord {
        self.ctx.record("placeholder")
    }
}

pub struct TrailLamaTiny {
    ctx: InferenceContext,
}

impl TrailLamaTiny {
    pub fn new(ctx: InferenceContext) -> Self {
        Self { ctx }
    }
}

impl AiModel for TrailLamaTiny {
    fn name(&self) -> &str {
        "trail-lama-tiny"
    }
    fn stage(&self) -> &str {
        "trail_inpaint"
    }
    fn run(&self, image: &F32Image) -> F32Image {
        image.clone()
    }
    fn determinism(&self) -> DeterminismRecord {
        self.ctx.record("placeholder")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> InferenceContext {
        InferenceContext {
            model_name: "test".into(),
            backend: "CPU".into(),
            tile_size: 512,
            overlap: 64,
            seed: Some(42),
        }
    }

    fn make_test_image() -> F32Image {
        let mut img = F32Image::new(8, 8, 1);
        for y in 0..8 {
            for x in 0..8 {
                img[(0, y, x)] = (x + y) as f32;
            }
        }
        img
    }

    #[test]
    fn test_swinir_denoise() {
        let model = SwinIrDenoise::new(make_ctx());
        let img = make_test_image();
        let result = model.run(&img);
        assert_eq!(result.width(), 8);
        assert_eq!(result.height(), 8);
        let record = model.determinism();
        assert_eq!(record.backend, "CPU");
        assert_eq!(record.seed, Some(42));
    }

    #[test]
    fn test_swinir_sr_2x() {
        let mut ctx = make_ctx();
        ctx.model_name = "swinir-sr-astro-2x".into();
        let model = SwinIrSuperRes::new(ctx, 2);
        let img = make_test_image();
        let result = model.run(&img);
        assert_eq!(result.width(), 16);
        assert_eq!(result.height(), 16);
    }

    #[test]
    fn test_star_seg_v1() {
        let model = StarSegV1::new(make_ctx());
        let img = make_test_image();
        let result = model.run(&img);
        assert_eq!(result.channels(), 2);
    }

    #[test]
    fn test_cloud_score() {
        let model = CloudScoreV1::new(make_ctx());
        let img = make_test_image();
        let result = model.run(&img);
        let score = result[(0, 0, 0)];
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn test_determinism_record() {
        let model = SwinIrDenoise::new(make_ctx());
        let record = model.determinism();
        assert_eq!(record.model_name, "test");
        assert_eq!(record.tile_size, 512);
        assert_eq!(record.overlap, 64);
    }
}
