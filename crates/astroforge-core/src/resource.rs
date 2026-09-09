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

// ─── P5 slice 1 — backend enumeration + execution budget (D-CR05-8) ─────────
//
// Placement note: D-CR05-8 names `execution/resource.rs`; the repository
// convention is flat crate-root modules (`pipeline/` and `pipeline_plan/`
// are the only directories), so the budget lives here in `resource.rs`
// alongside the snapshot it derives from. Same ownership, less churn.

/// One backend's advertised capability at snapshot time (§21 + D-CR05-8:
/// "backends advertise capability at startup").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendCapability {
    pub backend: ExecutionBackend,
    pub available: bool,
    /// Human-readable device name when available (e.g. the GPU model).
    pub device: Option<String>,
    /// Why the backend is unavailable — surfaced verbatim in Expert mode
    /// so the user never has to guess why e.g. CUDA is greyed out.
    pub unavailable_reason: Option<String>,
}

/// Pure enumeration from an existing snapshot — deterministic for a given
/// snapshot shape, which is what the P5 determinism tests pin down.
pub fn enumerate_backends_from(snapshot: &ResourceSnapshot) -> Vec<BackendCapability> {
    let gpu_for = |backend: ExecutionBackend| {
        snapshot
            .gpus
            .iter()
            .find(|g| g.backend == backend)
            .map(|g| g.name.clone())
    };
    let capability = |backend: ExecutionBackend, platform_ok: bool, missing: &str| {
        let device = gpu_for(backend);
        let available = platform_ok && device.is_some();
        BackendCapability {
            backend,
            available,
            device,
            unavailable_reason: if available {
                None
            } else if !platform_ok {
                Some(format!(
                    "{} is not supported on this platform",
                    backend.label()
                ))
            } else {
                Some(missing.to_string())
            },
        }
    };

    vec![
        BackendCapability {
            backend: ExecutionBackend::Cpu,
            available: true, // CPU execution is always available.
            device: Some(snapshot.cpu_model.clone()),
            unavailable_reason: None,
        },
        capability(
            ExecutionBackend::Cuda,
            cfg!(any(target_os = "linux", target_os = "windows")),
            "no NVIDIA GPU detected",
        ),
        capability(
            ExecutionBackend::DirectMl,
            cfg!(target_os = "windows"),
            "no DirectML-capable GPU detected",
        ),
        capability(
            ExecutionBackend::CoreMl,
            cfg!(target_os = "macos"),
            "no CoreML-capable GPU detected",
        ),
        // OpenVINO is a *runtime*, not just hardware: zero-dependency
        // detection of the runtime is out of scope for this slice, so we
        // report honestly rather than guess.
        BackendCapability {
            backend: ExecutionBackend::OpenVino,
            available: false,
            device: None,
            unavailable_reason: Some("OpenVINO runtime detection not implemented yet".to_string()),
        },
    ]
}

/// Detect the device and enumerate backends. Thin wrapper so callers who
/// already hold a snapshot can use the pure variant instead.
pub fn enumerate_backends() -> Vec<BackendCapability> {
    enumerate_backends_from(&ResourceSnapshot::detect())
}

/// What the runner is allowed to spend on one stage, derived from the
/// dataset and the memory actually available right now (§22).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ExecutionBudget {
    /// Ceiling for the stage's working set (50% of available RAM, 512 MiB
    /// floor — same rule as `RecommendedExecution.memory_budget_bytes`).
    #[serde(default)]
    pub memory_budget_bytes: u64,
    /// True when the dataset cannot fit in the budget whole and the stage
    /// must run tiled / streamed (§22 "process it in smaller tiles").
    #[serde(default)]
    pub requires_tiling: bool,
    /// Tile edge length in pixels when `requires_tiling` (still meaningful
    /// otherwise — the runner may always run tiled for uniformity).
    #[serde(default)]
    pub tile_size: u32,
    #[serde(default)]
    pub thread_count: u32,
    /// §22 pre-flight warning copy when memory is tight; `None` when the
    /// stage fits comfortably. Rendered verbatim by the UI.
    #[serde(default)]
    pub warning: Option<String>,
}

/// Assumed working-set multiplier: an RGB f32 tile is 12 bytes/px, and a
/// stage typically needs input + output + two intermediate buffers.
/// Documented so the estimate is auditable, not magic.
const WORKING_SET_BUFFERS: u64 = 4;
const BYTES_PER_PIXEL_F32_RGB: u64 = 12;

/// Derive the per-stage execution budget.
///
/// - `dataset_size_bytes`: uncompressed size of the stage's input
///   (width × height × channels × bytes-per-sample × frames for stacks).
///   Pass [`UNKNOWN_DATASET_SIZE_BYTES`] (or any value larger than
///   `available_memory_bytes`) when the size isn't known — the budget
///   stays conservative (no tiling warning) so §22 never refuses to
///   make progress.
/// - `available_memory_bytes`: current *available* RAM, not total.
/// - `logical_cores`: for the thread-count headroom rule.
pub fn derive_budget(
    dataset_size_bytes: u64,
    available_memory_bytes: u64,
    logical_cores: u32,
) -> ExecutionBudget {
    let memory_budget_bytes = (available_memory_bytes / 2).max(512 * 1024 * 1024);
    let thread_count = logical_cores.saturating_sub(2).max(1);

    // §22 promise: when the dataset size is unknown (sentinel), report a
    // conservative budget that does NOT require tiling or warn. Real
    // oversized datasets take the regular path.
    let real_size_known = dataset_size_bytes < UNKNOWN_DATASET_SIZE_BYTES;
    let requires_tiling = real_size_known && dataset_size_bytes > memory_budget_bytes;
    let tile_size = tile_size_for(
        if real_size_known {
            dataset_size_bytes
        } else {
            memory_budget_bytes
        },
        memory_budget_bytes,
    );

    let warning = if requires_tiling {
        let tiles = tile_count(dataset_size_bytes, tile_size);
        Some(format!(
            "This operation requires more memory than is currently available. \
             AstroForge will process it in {tiles} smaller tiles."
        ))
    } else {
        None
    };

    ExecutionBudget {
        memory_budget_bytes,
        requires_tiling,
        tile_size,
        thread_count,
        warning,
    }
}

/// Number of `tile_size`² tiles needed to cover an image whose total
/// uncompressed size is `dataset_size_bytes` (RGB f32 assumed, matching
/// `BYTES_PER_PIXEL_F32_RGB`). Used for the §22 warning copy.
fn tile_count(dataset_size_bytes: u64, tile_size: u32) -> u64 {
    let pixels = dataset_size_bytes / BYTES_PER_PIXEL_F32_RGB;
    let tile_pixels = (tile_size as u64).saturating_pow(2).max(1);
    pixels.div_ceil(tile_pixels).max(1)
}

/// Default dataset-size fallback when a stage doesn't report one.
/// §22 says we should never refuse to make progress — so when the size
/// is unknown we assume the dataset fits the memory budget and no
/// tiling warning is needed. Handlers that know better can override by
/// stamping `dataset_size_bytes` on the stage's `parameters_json`.
pub const UNKNOWN_DATASET_SIZE_BYTES: u64 = u64::MAX;

/// Pure wrapper around `derive_budget` for callers that already hold a
/// snapshot (typically handlers inside the runner). Avoids the cost of
/// re-detecting the device for every stage.
pub fn derive_stage_budget(
    snapshot: &ResourceSnapshot,
    dataset_size_bytes: u64,
) -> ExecutionBudget {
    derive_budget(
        dataset_size_bytes,
        snapshot.available_memory_bytes,
        snapshot.logical_cores,
    )
}

/// Optional explicit dataset size encoded by a stage's
/// `parameters_json.dataset_size_bytes` (D-CR05-8 follow-up: an upstream
/// handler can stamp the dataset size so the runner can budget against
/// it without re-loading source assets). Returns `None` when the
/// parameter is missing or not a non-negative integer, so callers must
/// treat the absence as "unknown — use conservative defaults".
pub fn parse_dataset_size_bytes(parameters_json: Option<&str>) -> Option<u64> {
    let raw = parameters_json?;
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let n = v.get("dataset_size_bytes")?.as_u64()?;
    Some(n)
}

/// Convenience for the common call-site shape where the JSON string
/// is `Option<String>`. Returns `None` for both "absent" and
/// "not-a-positive-integer" cases (same semantics as the `Option<&str>`
/// variant).
pub fn parse_dataset_size_bytes_opt(parameters_json: Option<&String>) -> Option<u64> {
    parse_dataset_size_bytes(parameters_json.map(String::as_str))
}

/// Pick the tile edge length for a stage. Candidate ladder mirrors the
/// `HardwareProbe` tiers and extends upward for well-resourced machines:
/// the largest tile whose *full working set* fits within half the budget
/// (the other half is reserved for the streamed dataset + OS). Falls
/// back to 256 with the understanding that the budget warning already
/// fired — we never return a tile of 0.
pub fn tile_size_for(dataset_size_bytes: u64, memory_budget_bytes: u64) -> u32 {
    const CANDIDATES: [u32; 5] = [1024, 768, 512, 384, 256];
    let per_tile_ceiling = memory_budget_bytes / 2;
    let dataset_pixels = dataset_size_bytes / BYTES_PER_PIXEL_F32_RGB;
    for &tile in &CANDIDATES {
        let tile_working_set = (tile as u64).pow(2) * BYTES_PER_PIXEL_F32_RGB * WORKING_SET_BUFFERS;
        // Fit within the ceiling, and don't return a tile larger than the
        // dataset itself (a 1024px tile for a 400px image is wasteful).
        // The smallest candidate is always allowed so we never return 0.
        let covers_dataset = (tile as u64).pow(2) <= dataset_pixels || tile == 256;
        if tile_working_set <= per_tile_ceiling && covers_dataset {
            return tile;
        }
    }
    256
}

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

    // ── P5 slice 1: backend enumeration ──

    fn snapshot_with_gpus(gpus: Vec<GpuInfo>) -> ResourceSnapshot {
        let mut snap = ResourceSnapshot {
            cpu_model: "Test CPU".to_string(),
            logical_cores: 8,
            physical_cores: Some(8),
            total_memory_bytes: 16 * GIB,
            available_memory_bytes: 12 * GIB,
            gpus,
            recommended: recommend_execution(ExecutionBackend::Cpu, 8, 12 * GIB, None),
        };
        snap.recommended = recommend_execution(
            snap.gpus
                .first()
                .map(|g| g.backend)
                .unwrap_or(ExecutionBackend::Cpu),
            8,
            12 * GIB,
            snap.gpus.iter().filter_map(|g| g.vram_bytes).max(),
        );
        snap
    }

    #[test]
    fn enumerate_backends_cpu_only_snapshot() {
        let caps = enumerate_backends_from(&snapshot_with_gpus(Vec::new()));
        assert_eq!(caps.len(), 5); // §21 vocabulary, always complete
        let cpu = &caps[0];
        assert_eq!(cpu.backend, ExecutionBackend::Cpu);
        assert!(cpu.available);
        assert_eq!(cpu.device.as_deref(), Some("Test CPU"));
        // No GPU anywhere → every GPU backend unavailable with a reason.
        for cap in &caps[1..] {
            assert!(!cap.available);
            assert!(cap.unavailable_reason.is_some());
        }
        // Determinism: same snapshot → identical enumeration.
        assert_eq!(
            caps,
            enumerate_backends_from(&snapshot_with_gpus(Vec::new()))
        );
    }

    #[test]
    fn enumerate_backends_with_matching_gpu_is_platform_gated() {
        let cuda_gpu = GpuInfo {
            name: "NVIDIA RTX 3060".to_string(),
            backend: ExecutionBackend::Cuda,
            vram_bytes: Some(12 * GIB),
        };
        let caps = enumerate_backends_from(&snapshot_with_gpus(vec![cuda_gpu]));
        let cuda = caps
            .iter()
            .find(|c| c.backend == ExecutionBackend::Cuda)
            .unwrap();
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            assert!(cuda.available);
            assert_eq!(cuda.device.as_deref(), Some("NVIDIA RTX 3060"));
        }
        #[cfg(target_os = "macos")]
        {
            assert!(!cuda.available);
            assert_eq!(
                cuda.unavailable_reason.as_deref(),
                Some("CUDA is not supported on this platform")
            );
        }
    }

    // ── P5 slice 1: budget derivation ──

    #[test]
    fn derive_budget_fits_whole_dataset_no_tiling() {
        // 100 MP RGB f32 ≈ 1.2 GB; 12 GB available → budget 6 GB, fits.
        let dataset = 100_000_000 * BYTES_PER_PIXEL_F32_RGB;
        let b = derive_budget(dataset, 12 * GIB, 8);
        assert_eq!(b.memory_budget_bytes, 6 * GIB);
        assert!(!b.requires_tiling);
        assert!(b.warning.is_none());
        assert_eq!(b.thread_count, 6);
    }

    #[test]
    fn derive_budget_oversized_dataset_requires_tiling_with_s22_copy() {
        // 500 MP RGB f32 ≈ 6 GB; 4 GB available → budget 2 GB, must tile.
        let dataset = 500_000_000 * BYTES_PER_PIXEL_F32_RGB;
        let b = derive_budget(dataset, 4 * GIB, 8);
        assert!(b.requires_tiling);
        let warning = b.warning.expect("tiling must warn per §22");
        assert!(
            warning.starts_with("This operation requires more memory than is currently available.")
        );
        assert!(warning.contains("smaller tiles"));
    }

    #[test]
    fn derive_budget_tiny_memory_still_makes_progress() {
        let b = derive_budget(50 * GIB, 600 * 1024 * 1024, 2);
        assert_eq!(b.memory_budget_bytes, 512 * 1024 * 1024); // floor
        assert_eq!(b.thread_count, 1);
        assert!(b.requires_tiling);
        // 1024² working set (48 MiB) still fits the 256 MiB per-tile
        // ceiling even on a starved machine — the floor budget keeps
        // tiles large enough to make real progress.
        assert_eq!(b.tile_size, 1024);
    }

    // ── P5 slice 1: tile-size table ──

    #[test]
    fn tile_size_for_table() {
        // Working set of tile N = N² × 12 B/px × 4 buffers.
        // Ceiling = budget / 2.
        let big_image = 2000 * 2000 * BYTES_PER_PIXEL_F32_RGB; // 2000² image

        // 1024² ws = 48 MiB → needs ceiling ≥ 48 MiB → budget ≥ 96 MiB.
        assert_eq!(tile_size_for(big_image, 512 * 1024 * 1024), 1024);
        // Budget 32 MiB → ceiling 16 MiB → 512² ws = 12 MiB fits, 768² doesn't.
        assert_eq!(tile_size_for(big_image, 32 * 1024 * 1024), 512);
        // Tiny image (400²) never gets a tile bigger than itself.
        let small_image = 400 * 400 * BYTES_PER_PIXEL_F32_RGB;
        assert_eq!(tile_size_for(small_image, 512 * 1024 * 1024), 384);
        // Starved budget → 256 floor, never zero.
        assert_eq!(tile_size_for(big_image, 1024), 256);
    }

    #[test]
    fn tile_count_covers_dataset() {
        // 2000² image with 512px tiles → ceil(4M / 262144) = 16 tiles.
        let dataset = 2000 * 2000 * BYTES_PER_PIXEL_F32_RGB;
        assert_eq!(tile_count(dataset, 512), 16);
        assert_eq!(tile_count(dataset, 1024), 4);
        // Degenerate input still reports one tile, never zero.
        assert_eq!(tile_count(0, 256), 1);
    }

    // ── P5 slice 2: dataset-size parsing + budget derivation ──

    #[test]
    fn parse_dataset_size_bytes_round_trip() {
        assert_eq!(
            parse_dataset_size_bytes(Some(r#"{"dataset_size_bytes": 12345}"#)),
            Some(12345)
        );
        // Missing field → None (caller falls back to UNKNOWN).
        assert_eq!(parse_dataset_size_bytes(Some("{}")), None);
        // Wrong type → None, not a panic.
        assert_eq!(
            parse_dataset_size_bytes(Some(r#"{"dataset_size_bytes": "nope"}"#)),
            None
        );
        // Not JSON → None.
        assert_eq!(parse_dataset_size_bytes(Some("not json")), None);
        // No parameters at all → None.
        assert_eq!(parse_dataset_size_bytes(None), None);
    }

    #[test]
    fn parse_dataset_size_bytes_opt_handles_option_string() {
        let json = r#"{"dataset_size_bytes": 4096}"#.to_string();
        assert_eq!(parse_dataset_size_bytes_opt(Some(&json)), Some(4096));
        assert_eq!(parse_dataset_size_bytes_opt(None), None);
    }

    #[test]
    fn unknown_dataset_size_never_triggers_tiling_warning() {
        // §22 says we should never refuse to make progress — unknown size
        // must report no warning even when "size" is u64::MAX.
        let snap = ResourceSnapshot::detect();
        let budget = derive_stage_budget(&snap, UNKNOWN_DATASET_SIZE_BYTES);
        assert!(
            budget.warning.is_none(),
            "unknown size must not produce a §22 warning"
        );
    }
}
