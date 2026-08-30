use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub stage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProbe {
    pub ram_mb: u64,
    pub gpu_backend: GpuBackend,
    pub vram_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GpuBackend {
    Cpu,
    Cuda,
    DirectMl,
    Metal,
    OpenVino,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QualityTier {
    Fast,
    Balanced,
    Research,
    Perceptual,
}

impl HardwareProbe {
    pub fn select_tier(&self) -> QualityTier {
        match (self.ram_mb, &self.gpu_backend, self.vram_mb) {
            (ram, GpuBackend::Cpu, _) if ram <= 4096 => QualityTier::Fast,
            (ram, GpuBackend::Cpu, _) if ram <= 8192 => QualityTier::Balanced,
            (ram, _, vram) if ram >= 8192 && vram >= 8192 => QualityTier::Perceptual,
            _ => QualityTier::Research,
        }
    }
}
