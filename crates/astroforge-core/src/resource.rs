//! CR-05 P4 slice 7 — `ResourceSnapshot` detection per §21 (Resource-Aware
//! Execution).
//!
//! §21 requires that the user never has to choose CPU / CUDA / DirectML /
//! CoreML / OpenVINO, tile sizes, or thread counts manually. Instead the
//! application detects the device once and derives a recommended execution
//! profile from the snapshot. Advanced users can inspect the result
//! (CPU / GPU / Memory / Tile size / Precision / Backend).
//!
//! Detection is best-effort and dependency-light: `sysinfo` covers CPU and
//! memory on all three desktop platforms; GPU probing shells out to
//! platform inventory tools (`nvidia-smi`, `system_profiler`,
//! `Get-CimInstance`) and degrades to an empty GPU list when none are
//! present. The derived recommendation is a pure function so it can be
//! tested deterministically — detection drift across CI runners never
//! changes the recommendation semantics for a given snapshot shape.

use serde::{Deserialize, Serialize};

/// §21 backend vocabulary: CPU; CUDA; DirectML; CoreML; OpenVINO.
///
/// Note: `astroforge-ai::hardware::GpuBackend` predates this enum and uses
/// `Metal` where §21 says `CoreML`. This enum follows the spec; the mapping
/// (macOS → CoreML execution backed by Metal) is documented here rather
/// than renaming the older crate in this slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionBackend {
    Cpu,
    Cuda,
    DirectMl,
    CoreMl,
    OpenVino,
}

impl ExecutionBackend {
    pub fn label(&self) -> &'static str {
        match self {
            ExecutionBackend::Cpu => "CPU",
            ExecutionBackend::Cuda => "CUDA",
            ExecutionBackend::DirectMl => "DirectML",
            ExecutionBackend::CoreMl => "CoreML",
            ExecutionBackend::OpenVino => "OpenVINO",
        }
    }
}

/// Working precision for pixel buffers. Astro work is `F32` by default;
/// `F16` is only recommended when a GPU with ample VRAM is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Precision {
    F16,
    F32,
}

impl Precision {
    pub fn label(&self) -> &'static str {
        match self {
            Precision::F16 => "float16",
            Precision::F32 => "float32",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub backend: ExecutionBackend,
    pub vram_bytes: Option<u64>,
}

/// What §21 asks the application to pick on the user's behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecommendedExecution {
    pub backend: ExecutionBackend,
    pub tile_size: u32,
    pub thread_count: u32,
    pub precision: Precision,
    /// §22 memory budgeting: the ceiling a single stage should plan its
    /// working set against. Derived from *available* (not total) memory.
    pub memory_budget_bytes: u64,
}

/// Point-in-time hardware snapshot. Not persisted in this slice — it is
/// recomputed on demand so it always reflects current memory pressure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub cpu_model: String,
    pub logical_cores: u32,
    pub physical_cores: Option<u32>,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub gpus: Vec<GpuInfo>,
    pub recommended: RecommendedExecution,
}

impl ResourceSnapshot {
    /// Detect the current device. Never fails hard: individual probes that
    /// error are skipped, and the recommendation is always derivable from
    /// whatever was found (worst case: CPU-only with conservative memory).
    pub fn detect() -> Self {
        use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_cpu_all();
        sys.refresh_memory();

        let cpu_model = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown CPU".to_string());
        let logical_cores = sys.cpus().len().try_into().unwrap_or(0u32).max(1);
        let physical_cores = System::physical_core_count().and_then(|n| u32::try_from(n).ok());
        let total_memory_bytes = sys.total_memory();
        let available_memory_bytes = sys.available_memory();

        let gpus = detect_gpus();
        let backend = gpus
            .first()
            .map(|g| g.backend)
            .unwrap_or(ExecutionBackend::Cpu);
        let best_vram = gpus.iter().filter_map(|g| g.vram_bytes).max();
        let recommended =
            recommend_execution(backend, logical_cores, available_memory_bytes, best_vram);

        Self {
            cpu_model,
            logical_cores,
            physical_cores,
            total_memory_bytes,
            available_memory_bytes,
            gpus,
            recommended,
        }
    }
}

/// Pure recommendation derivation, split from detection so it is
/// deterministically testable (CI hardware drift cannot change semantics).
///
/// Tiers mirror `astroforge-ai::hardware::HardwareProbe` tile sizes so the
/// two resource views agree:
/// - available < 4 GiB  → 256px tiles
/// - available < 8 GiB  → 384px tiles
/// - otherwise          → 512px tiles
pub fn recommend_execution(
    backend: ExecutionBackend,
    logical_cores: u32,
    available_memory_bytes: u64,
    best_vram_bytes: Option<u64>,
) -> RecommendedExecution {
    const GIB: u64 = 1024 * 1024 * 1024;

    let tile_size = if available_memory_bytes < 4 * GIB {
        256
    } else if available_memory_bytes < 8 * GIB {
        384
    } else {
        512
    };

    // Leave two cores of headroom for the UI / OS so processing never
    // freezes the workspace (§22 bounded concurrency).
    let thread_count = logical_cores.saturating_sub(2).max(1);

    // F16 only when a GPU has enough VRAM to hold full working sets;
    // astrophotography precision defaults to F32 everywhere else.
    let precision = match (backend, best_vram_bytes) {
        (ExecutionBackend::Cpu, _) => Precision::F32,
        (_, Some(vram)) if vram >= 8 * GIB => Precision::F16,
        _ => Precision::F32,
    };

    // §22 memory budgeting: a stage may plan against at most half of what
    // is currently available, with a 512 MiB floor so tiny systems still
    // make progress (in smaller tiles, with a time warning per §22 UX).
    let memory_budget_bytes = (available_memory_bytes / 2).max(512 * 1024 * 1024);

    RecommendedExecution {
        backend,
        tile_size,
        thread_count,
        precision,
        memory_budget_bytes,
    }
}

// ─── GPU probing ─────────────────────────────────────────────────────────────

/// Best-effort GPU inventory. Returns an empty vec when nothing is
/// detected — that is a valid answer (CPU-only device), not an error.
fn detect_gpus() -> Vec<GpuInfo> {
    let mut gpus = probe_nvidia_smi();
    if gpus.is_empty() {
        gpus = platform_gpu_probe();
    }
    gpus
}

/// `nvidia-smi` covers NVIDIA cards on both Linux and Windows.
fn probe_nvidia_smi() -> Vec<GpuInfo> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            parse_nvidia_smi_csv(&String::from_utf8_lossy(&out.stdout))
        }
        _ => Vec::new(),
    }
}

/// Parse `nvidia-smi --query-gpu=name,memory.total --format=csv,noheader,nounits`
/// output: one `Name, MiB` line per GPU.
fn parse_nvidia_smi_csv(text: &str) -> Vec<GpuInfo> {
    text.lines()
        .filter_map(|line| {
            let (name, mib) = line.split_once(',')?;
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            let vram_bytes = mib.trim().parse::<u64>().ok().map(|m| m * 1024 * 1024);
            Some(GpuInfo {
                name: name.to_string(),
                backend: ExecutionBackend::Cuda,
                vram_bytes,
            })
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn platform_gpu_probe() -> Vec<GpuInfo> {
    let output = std::process::Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .output();
    match output {
        Ok(out) if out.status.success() => {
            parse_macos_system_profiler(&String::from_utf8_lossy(&out.stdout))
        }
        _ => Vec::new(),
    }
}

/// Parse the `system_profiler SPDisplaysDataType` plain-text report.
/// Integrated Apple-silicon GPUs report "Total Number of Cores" instead of
/// VRAM (unified memory) — `vram_bytes` stays `None` there, which the
/// recommendation correctly treats as "unknown, be conservative".
#[cfg(target_os = "macos")]
fn parse_macos_system_profiler(text: &str) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let mut current: Option<GpuInfo> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("Chipset Model:") {
            if let Some(g) = current.take() {
                gpus.push(g);
            }
            current = Some(GpuInfo {
                name: name.trim().to_string(),
                backend: ExecutionBackend::CoreMl,
                vram_bytes: None,
            });
        } else if let Some(vram) = trimmed.strip_prefix("VRAM (Total):") {
            if let Some(g) = current.as_mut() {
                g.vram_bytes = parse_size_to_bytes(vram.trim());
            }
        }
    }
    if let Some(g) = current.take() {
        gpus.push(g);
    }
    gpus
}

#[cfg(target_os = "windows")]
fn platform_gpu_probe() -> Vec<GpuInfo> {
    // CSV via ConvertTo-Csv keeps parsing trivial and locale-stable enough
    // for a best-effort probe (Name is a display string; AdapterRAM bytes).
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object Name,AdapterRAM | ConvertTo-Csv -NoTypeInformation",
        ])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            parse_windows_video_controller_csv(&String::from_utf8_lossy(&out.stdout))
        }
        _ => Vec::new(),
    }
}

#[cfg(target_os = "windows")]
fn parse_windows_video_controller_csv(text: &str) -> Vec<GpuInfo> {
    text.lines()
        .skip(1) // header row
        .filter_map(|line| {
            // "Name","12345678" — both fields quoted by ConvertTo-Csv.
            let cols: Vec<&str> = line.split("\",\"").collect();
            if cols.len() != 2 {
                return None;
            }
            let name = cols[0].trim_matches('"').trim();
            if name.is_empty() {
                return None;
            }
            let vram_bytes = cols[1]
                .trim_matches(|c| c == '"' || c == '\r')
                .trim()
                .parse::<u64>()
                .ok();
            Some(GpuInfo {
                name: name.to_string(),
                backend: ExecutionBackend::DirectMl,
                vram_bytes,
            })
        })
        .collect()
}

/// Linux without NVIDIA: no zero-dependency inventory that beats an honest
/// empty list. ROCm / OpenVINO probing is tracked for P5.
#[cfg(all(unix, not(target_os = "macos")))]
fn platform_gpu_probe() -> Vec<GpuInfo> {
    Vec::new()
}

/// Parse human sizes like "8 GB" / "8192 MB" into bytes.
#[cfg(target_os = "macos")]
fn parse_size_to_bytes(text: &str) -> Option<u64> {
    let mut parts = text.split_whitespace();
    let value: f64 = parts.next()?.parse().ok()?;
    let unit = parts.next().unwrap_or("").to_ascii_lowercase();
    let factor: f64 = match unit.as_str() {
        "kb" => 1024.0,
        "mb" => 1024.0 * 1024.0,
        "gb" => 1024.0 * 1024.0 * 1024.0,
        "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((value * factor) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    // ── recommendation derivation (pure, deterministic) ──

    #[test]
    fn recommend_cpu_only_low_memory_is_conservative() {
        let r = recommend_execution(ExecutionBackend::Cpu, 4, 3 * GIB, None);
        assert_eq!(r.backend, ExecutionBackend::Cpu);
        assert_eq!(r.tile_size, 256);
        assert_eq!(r.thread_count, 2);
        assert_eq!(r.precision, Precision::F32);
        assert_eq!(r.memory_budget_bytes, 3 * GIB / 2);
    }

    #[test]
    fn recommend_mid_memory_takes_384_tiles() {
        let r = recommend_execution(ExecutionBackend::Cpu, 8, 6 * GIB, None);
        assert_eq!(r.tile_size, 384);
        assert_eq!(r.thread_count, 6);
    }

    #[test]
    fn recommend_cuda_with_ample_vram_allows_f16_and_large_tiles() {
        let r = recommend_execution(ExecutionBackend::Cuda, 16, 32 * GIB, Some(12 * GIB));
        assert_eq!(r.backend, ExecutionBackend::Cuda);
        assert_eq!(r.tile_size, 512);
        assert_eq!(r.thread_count, 14);
        assert_eq!(r.precision, Precision::F16);
    }

    #[test]
    fn recommend_gpu_without_vram_reading_stays_f32() {
        // Apple-silicon unified memory: no discrete VRAM figure → conservative.
        let r = recommend_execution(ExecutionBackend::CoreMl, 10, 16 * GIB, None);
        assert_eq!(r.precision, Precision::F32);
        assert_eq!(r.tile_size, 512);
    }

    #[test]
    fn recommend_memory_budget_has_floor() {
        let r = recommend_execution(ExecutionBackend::Cpu, 1, 600 * 1024 * 1024, None);
        assert_eq!(r.memory_budget_bytes, 512 * 1024 * 1024);
        assert_eq!(r.thread_count, 1); // single-core floor, never zero
    }

    // ── parsers ──

    #[test]
    fn parses_nvidia_smi_csv() {
        let text = "NVIDIA GeForce RTX 3060, 12288\nNVIDIA GeForce GT 710, 2048\n";
        let gpus = parse_nvidia_smi_csv(text);
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].name, "NVIDIA GeForce RTX 3060");
        assert_eq!(gpus[0].backend, ExecutionBackend::Cuda);
        assert_eq!(gpus[0].vram_bytes, Some(12288 * 1024 * 1024));
        assert_eq!(gpus[1].vram_bytes, Some(2048 * 1024 * 1024));
    }

    #[test]
    fn nvidia_smi_csv_skips_blank_and_malformed_lines() {
        assert!(parse_nvidia_smi_csv("").is_empty());
        assert!(parse_nvidia_smi_csv("no comma here").is_empty());
        assert!(parse_nvidia_smi_csv(", 4096").is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parses_macos_system_profiler() {
        let text = "Graphics/Displays:\n\n    Apple M1 Pro:\n\n      Chipset Model: Apple M1 Pro\n      Type: GPU\n      Total Number of Cores: 16\n\n    Radeon Pro:\n\n      Chipset Model: Radeon Pro 555X\n      VRAM (Total): 4 GB\n";
        let gpus = parse_macos_system_profiler(text);
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].name, "Apple M1 Pro");
        assert_eq!(gpus[0].backend, ExecutionBackend::CoreMl);
        assert_eq!(gpus[0].vram_bytes, None); // unified memory
        assert_eq!(gpus[1].vram_bytes, Some(4 * GIB));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parses_human_sizes() {
        assert_eq!(parse_size_to_bytes("8 GB"), Some(8 * GIB));
        assert_eq!(parse_size_to_bytes("8192 MB"), Some(8 * GIB));
        assert_eq!(parse_size_to_bytes("bogus"), None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn parses_windows_video_controller_csv() {
        let text = "\"Name\",\"AdapterRAM\"\r\n\"NVIDIA GeForce RTX 3060\",\"4293918720\"\r\n";
        let gpus = parse_windows_video_controller_csv(text);
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].name, "NVIDIA GeForce RTX 3060");
        assert_eq!(gpus[0].backend, ExecutionBackend::DirectMl);
        assert_eq!(gpus[0].vram_bytes, Some(4293918720));
    }

    // ── detection smoke (runs on linux / macos / windows CI) ──

    #[test]
    fn detect_returns_sane_snapshot_on_any_platform() {
        let snap = ResourceSnapshot::detect();
        assert!(snap.logical_cores >= 1);
        assert!(snap.total_memory_bytes > 0);
        assert!(snap.available_memory_bytes <= snap.total_memory_bytes);
        assert!(!snap.cpu_model.is_empty());
        // Recommendation is always derivable and internally consistent.
        assert!(snap.recommended.thread_count >= 1);
        assert!(snap.recommended.thread_count < snap.logical_cores.max(2));
        assert!(snap.recommended.memory_budget_bytes >= 512 * 1024 * 1024);
        assert!([256, 384, 512].contains(&snap.recommended.tile_size));
        // GPU inventory may legitimately be empty on CI runners; when a
        // GPU is found its backend must not be Cpu.
        for gpu in &snap.gpus {
            assert_ne!(gpu.backend, ExecutionBackend::Cpu);
            assert!(!gpu.name.is_empty());
        }
    }
}
