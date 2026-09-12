//! CR-06 P5.2 — GPU execution-provider selection.
//!
//! Maps a [`HardwareProbe`] onto the set of [`ExecutionProvider`]
//! entries that the [`ort`] runtime should attempt. Each provider is
//! compile-time gated by the matching `gpu-*` Cargo feature in
//! `Cargo.toml`; when the feature is off, that provider is simply
//! absent from the [`build_selection`] output.
//!
//! # Design
//!
//! The slice intentionally *does not* call into `ort` itself. The
//! selection logic is pure data: given a probe, return the ordered
//! preference list of providers the dispatcher would pass to
//! `ort::Session::builder().with_execution_providers(...)`. The actual
//! `ort` invocation lives in `Onnx_runtime::open_*` and is exercised
//! end-to-end by per-platform CI runners with the matching feature
//! enabled.
//!
//! Default `cargo build` (CPU-only) builds and tests cleanly without
//! any GPU SDK present. CI matrix expansion to GPU runners is a
//! follow-up tracked in CHANGELOG (slice #315).

use serde::{Deserialize, Serialize};

use crate::hardware::{GpuBackend, HardwareProbe};

/// One entry in the execution-provider preference list.
///
/// Order matters: `ort` tries providers in the order they're passed;
/// the first one that initializes successfully wins. We always lead
/// with the GPU provider (when available) and fall back to CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionProvider {
    /// CUDA 13+ execution provider (NVIDIA GPUs).
    /// Compile-time gated by the `gpu-cuda` Cargo feature.
    Cuda,
    /// TensorRT execution provider (NVIDIA GPUs).
    /// Compile-time gated by the `gpu-tensorrt` Cargo feature.
    TensorRt,
    /// DirectML execution provider (Windows + DX12 GPUs).
    /// Compile-time gated by the `gpu-directml` Cargo feature.
    DirectMl,
    /// CoreML / Metal execution provider (macOS).
    /// Compile-time gated by the `gpu-coreml` Cargo feature.
    CoreMl,
    /// OpenVINO execution provider (Intel CPUs and iGPUs).
    /// Compile-time gated by the `gpu-openvino` Cargo feature.
    OpenVino,
    /// CPU execution provider. Always available regardless of
    /// feature flags.
    Cpu,
}

impl ExecutionProvider {
    pub fn label(self) -> &'static str {
        match self {
            ExecutionProvider::Cuda => "CUDA",
            ExecutionProvider::TensorRt => "TensorRT",
            ExecutionProvider::DirectMl => "DirectML",
            ExecutionProvider::CoreMl => "CoreML",
            ExecutionProvider::OpenVino => "OpenVINO",
            ExecutionProvider::Cpu => "CPU",
        }
    }

    /// Which [`GpuBackend`] this provider maps to. CPU is its own
    /// category (no GPU).
    pub fn backend(self) -> Option<GpuBackend> {
        match self {
            ExecutionProvider::Cuda | ExecutionProvider::TensorRt => Some(GpuBackend::Cuda),
            ExecutionProvider::DirectMl => Some(GpuBackend::DirectMl),
            ExecutionProvider::CoreMl => Some(GpuBackend::Metal),
            ExecutionProvider::OpenVino => Some(GpuBackend::OpenVino),
            ExecutionProvider::Cpu => None,
        }
    }

    /// True if the matching Cargo feature is enabled in this build.
    /// Pure-function — does *not* check whether the platform SDK is
    /// actually installed (that's a runtime concern handled by `ort`).
    pub fn is_compiled(self) -> bool {
        match self {
            #[cfg(feature = "gpu-cuda")]
            ExecutionProvider::Cuda => true,
            #[cfg(not(feature = "gpu-cuda"))]
            ExecutionProvider::Cuda => false,
            #[cfg(feature = "gpu-tensorrt")]
            ExecutionProvider::TensorRt => true,
            #[cfg(not(feature = "gpu-tensorrt"))]
            ExecutionProvider::TensorRt => false,
            #[cfg(feature = "gpu-directml")]
            ExecutionProvider::DirectMl => true,
            #[cfg(not(feature = "gpu-directml"))]
            ExecutionProvider::DirectMl => false,
            #[cfg(feature = "gpu-coreml")]
            ExecutionProvider::CoreMl => true,
            #[cfg(not(feature = "gpu-coreml"))]
            ExecutionProvider::CoreMl => false,
            #[cfg(feature = "gpu-openvino")]
            ExecutionProvider::OpenVino => true,
            #[cfg(not(feature = "gpu-openvino"))]
            ExecutionProvider::OpenVino => false,
            ExecutionProvider::Cpu => true,
        }
    }
}

/// Build the preference-ordered execution-provider list for the given
/// hardware probe. CPU is always appended as the final fallback.
///
/// The list is empty when the probe reports no GPU *and* the build is
/// CPU-only — but `ExecutionProvider::Cpu` is unconditionally compiled,
/// so in practice the list is always non-empty.
pub fn build_selection(probe: &HardwareProbe) -> Vec<ExecutionProvider> {
    let mut out: Vec<ExecutionProvider> = Vec::new();
    match probe.gpu_backend {
        GpuBackend::Cuda => {
            // CUDA preferred; TensorRT second (also CUDA, but more
            // aggressive optimization — try CUDA first to fail fast on
            // a missing driver, then TensorRT for the workload that
            // supports it).
            out.push(ExecutionProvider::Cuda);
            out.push(ExecutionProvider::TensorRt);
        }
        GpuBackend::DirectMl => out.push(ExecutionProvider::DirectMl),
        GpuBackend::Metal => out.push(ExecutionProvider::CoreMl),
        GpuBackend::OpenVino => out.push(ExecutionProvider::OpenVino),
        GpuBackend::Cpu => {}
    }
    out.push(ExecutionProvider::Cpu);
    out
}

/// Convenience: return only the providers that are actually compiled
/// into this build. The probe-driven list minus the uncompiled ones
/// — useful for surfacing in startup logs so users can see why a
/// particular GPU isn't being used.
pub fn compiled_selection(probe: &HardwareProbe) -> Vec<ExecutionProvider> {
    build_selection(probe)
        .into_iter()
        .filter(|ep| ep.is_compiled())
        .collect()
}

/// True if the probe's GPU backend has any compiled execution
/// provider. Returns false when the probe reports a GPU but the build
/// is CPU-only — the caller should fall back to CPU.
pub fn has_compiled_gpu(probe: &HardwareProbe) -> bool {
    compiled_selection(probe)
        .iter()
        .any(|ep| ep.backend().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(backend: GpuBackend) -> HardwareProbe {
        HardwareProbe {
            ram_mb: 16_384,
            gpu_backend: backend,
            vram_mb: 8_192,
            cpu_cores: 8,
        }
    }

    #[test]
    fn cpu_probe_returns_cpu_only() {
        let list = build_selection(&probe(GpuBackend::Cpu));
        assert_eq!(list, vec![ExecutionProvider::Cpu]);
    }

    #[test]
    fn cuda_probe_orders_cuda_before_tensorrt_before_fallback() {
        let list = build_selection(&probe(GpuBackend::Cuda));
        assert_eq!(
            list,
            vec![
                ExecutionProvider::Cuda,
                ExecutionProvider::TensorRt,
                ExecutionProvider::Cpu,
            ]
        );
    }

    #[test]
    fn metal_probe_maps_to_coreml() {
        let list = build_selection(&probe(GpuBackend::Metal));
        assert_eq!(
            list,
            vec![ExecutionProvider::CoreMl, ExecutionProvider::Cpu]
        );
        assert_eq!(ExecutionProvider::CoreMl.backend(), Some(GpuBackend::Metal));
    }

    #[test]
    fn directml_probe_maps_to_directml() {
        let list = build_selection(&probe(GpuBackend::DirectMl));
        assert_eq!(
            list,
            vec![ExecutionProvider::DirectMl, ExecutionProvider::Cpu]
        );
    }

    #[test]
    fn openvino_probe_maps_to_openvino() {
        let list = build_selection(&probe(GpuBackend::OpenVino));
        assert_eq!(
            list,
            vec![ExecutionProvider::OpenVino, ExecutionProvider::Cpu]
        );
    }

    #[test]
    fn cpu_is_always_compiled() {
        assert!(ExecutionProvider::Cpu.is_compiled());
    }

    #[test]
    fn default_build_has_no_compiled_gpu() {
        // Without any gpu-* feature flag, the GPU providers are not
        // compiled; only CPU is.
        assert!(!ExecutionProvider::Cuda.is_compiled());
        assert!(!ExecutionProvider::DirectMl.is_compiled());
        assert!(!ExecutionProvider::CoreMl.is_compiled());
        assert!(!ExecutionProvider::OpenVino.is_compiled());
        assert!(!ExecutionProvider::TensorRt.is_compiled());
    }

    #[test]
    fn has_compiled_gpu_returns_false_for_cpu_probe() {
        assert!(!has_compiled_gpu(&probe(GpuBackend::Cpu)));
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(ExecutionProvider::Cuda.label(), "CUDA");
        assert_eq!(ExecutionProvider::TensorRt.label(), "TensorRT");
        assert_eq!(ExecutionProvider::DirectMl.label(), "DirectML");
        assert_eq!(ExecutionProvider::CoreMl.label(), "CoreML");
        assert_eq!(ExecutionProvider::OpenVino.label(), "OpenVINO");
        assert_eq!(ExecutionProvider::Cpu.label(), "CPU");
    }
}
