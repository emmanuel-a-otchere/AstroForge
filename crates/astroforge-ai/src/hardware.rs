use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProbe {
    pub ram_mb: u64,
    pub gpu_backend: GpuBackend,
    pub vram_mb: u64,
    pub cpu_cores: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuBackend {
    Cpu,
    Cuda,
    DirectMl,
    Metal,
    OpenVino,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QualityTier {
    Fast,
    Balanced,
    Research,
    Perceptual,
}

impl QualityTier {
    pub fn label(&self) -> &'static str {
        match self {
            QualityTier::Fast => "Fast",
            QualityTier::Balanced => "Balanced",
            QualityTier::Research => "Research",
            QualityTier::Perceptual => "Perceptual",
        }
    }

    pub fn allows_perceptual(&self) -> bool {
        matches!(self, QualityTier::Perceptual)
    }
}

impl HardwareProbe {
    pub fn detect() -> Self {
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4);

        Self {
            ram_mb: estimate_ram(),
            gpu_backend: detect_gpu_backend(),
            vram_mb: estimate_vram(),
            cpu_cores,
        }
    }

    pub fn select_tier(&self) -> QualityTier {
        match (self.ram_mb, &self.gpu_backend, self.vram_mb) {
            (ram, GpuBackend::Cpu, _) if ram <= 4096 => QualityTier::Fast,
            (ram, GpuBackend::Cpu, _) if ram <= 8192 => QualityTier::Balanced,
            (ram, _, vram) if ram >= 8192 && vram >= 8192 => QualityTier::Perceptual,
            (ram, _, _) if ram >= 4096 => QualityTier::Research,
            _ => QualityTier::Fast,
        }
    }

    pub fn max_tile_size(&self) -> u32 {
        match self.select_tier() {
            QualityTier::Fast => 256,
            QualityTier::Balanced => 384,
            QualityTier::Research => 512,
            QualityTier::Perceptual => 512,
        }
    }

    pub fn supports_backend(&self, backend: &GpuBackend) -> bool {
        self.gpu_backend == *backend
    }
}

fn estimate_ram() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return kb / 1024;
                        }
                    }
                }
            }
        }
        8192
    }
    #[cfg(not(target_os = "linux"))]
    {
        8192
    }
}

fn detect_gpu_backend() -> GpuBackend {
    #[cfg(target_os = "macos")]
    {
        GpuBackend::Metal
    }
    #[cfg(target_os = "windows")]
    {
        GpuBackend::DirectMl
    }
    #[cfg(target_os = "linux")]
    {
        GpuBackend::Cpu
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        GpuBackend::Cpu
    }
}

fn estimate_vram() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_tier_fast() {
        let probe = HardwareProbe {
            ram_mb: 2048,
            gpu_backend: GpuBackend::Cpu,
            vram_mb: 0,
            cpu_cores: 2,
        };
        assert_eq!(probe.select_tier(), QualityTier::Fast);
    }

    #[test]
    fn test_select_tier_balanced() {
        let probe = HardwareProbe {
            ram_mb: 6144,
            gpu_backend: GpuBackend::Cpu,
            vram_mb: 0,
            cpu_cores: 4,
        };
        assert_eq!(probe.select_tier(), QualityTier::Balanced);
    }

    #[test]
    fn test_select_tier_perceptual() {
        let probe = HardwareProbe {
            ram_mb: 16384,
            gpu_backend: GpuBackend::Cuda,
            vram_mb: 12288,
            cpu_cores: 8,
        };
        assert_eq!(probe.select_tier(), QualityTier::Perceptual);
    }

    #[test]
    fn test_max_tile_size() {
        let probe = HardwareProbe {
            ram_mb: 2048,
            gpu_backend: GpuBackend::Cpu,
            vram_mb: 0,
            cpu_cores: 2,
        };
        assert_eq!(probe.max_tile_size(), 256);

        let probe = HardwareProbe {
            ram_mb: 16384,
            gpu_backend: GpuBackend::Cuda,
            vram_mb: 12288,
            cpu_cores: 8,
        };
        assert_eq!(probe.max_tile_size(), 512);
    }

    #[test]
    fn test_quality_tier_label() {
        assert_eq!(QualityTier::Fast.label(), "Fast");
        assert_eq!(QualityTier::Perceptual.label(), "Perceptual");
    }
}
