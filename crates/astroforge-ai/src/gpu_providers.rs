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
//! The selection logic is pure data: given a probe, return the
//! ordered preference list of providers for the runtime. RT-8 (#427)
//! adds the `ort` bridge: [`ort_gpu_dispatches`] materializes the
//! compiled preference list into `ort::ep::ExecutionProviderDispatch`
//! values for `Session::builder().with_execution_providers(...)` —
//! consumed by `inference::OnnxEngine::open_catalog_on` and exercised
//! on the CI `ai-gpu-coreml` job (macOS runners register the real
//! CoreML provider).
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

/// Materialize the compiled GPU preference list into `ort`
/// execution-provider dispatches for
/// `Session::builder().with_execution_providers(...)`. Each provider
/// is gated by its `gpu-*` Cargo feature (which enables the matching
/// `ort` feature); the CPU fallback is intentionally absent — ONNX
/// Runtime always falls back to CPU internally, and registering it
/// explicitly pins the order needlessly.
///
/// This is the only place that touches `ort` EP types; everything
/// above stays pure so selection logic tests run in CPU-only builds.
pub fn ort_gpu_dispatches(
    probe: &HardwareProbe,
) -> Vec<ort::execution_providers::ExecutionProviderDispatch> {
    compiled_selection(probe)
        .into_iter()
        .filter_map(|ep| match ep {
            #[cfg(feature = "gpu-cuda")]
            ExecutionProvider::Cuda => Some(ort::execution_providers::CUDA::default().build()),
            #[cfg(feature = "gpu-tensorrt")]
            ExecutionProvider::TensorRt => {
                Some(ort::execution_providers::TensorRT::default().build())
            }
            #[cfg(feature = "gpu-directml")]
            ExecutionProvider::DirectMl => {
                Some(ort::execution_providers::DirectML::default().build())
            }
            #[cfg(feature = "gpu-coreml")]
            ExecutionProvider::CoreMl => Some(ort::execution_providers::CoreML::default().build()),
            #[cfg(feature = "gpu-openvino")]
            ExecutionProvider::OpenVino => {
                Some(ort::execution_providers::OpenVINO::default().build())
            }
            // CPU never registers (implicit fallback); uncompiled GPU
            // providers were already filtered out by compiled_selection.
            ExecutionProvider::Cpu => None,
            #[allow(unreachable_patterns)]
            _ => None,
        })
        .collect()
}

/// The user-facing backend label for a probe: the first compiled GPU
/// provider's label, or `"cpu"`. This is the *requested* backend —
/// ONNX Runtime may still fall back internally (e.g. no Metal
/// device); surfacing the definitive provider requires session
/// metadata parsing (follow-up).
pub fn backend_label(probe: &HardwareProbe) -> String {
    compiled_selection(probe)
        .into_iter()
        .find(|ep| ep.backend().is_some())
        .map(|ep| ep.label().to_lowercase())
        .unwrap_or_else(|| "cpu".to_string())
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
    fn gpu_providers_compile_only_with_their_feature() {
        // Each GPU provider is compiled iff its gpu-* feature is on —
        // this test passes under the default CPU-only build AND under
        // feature-enabled runs (the ai-gpu-coreml CI job, RT-8 #427).
        assert_eq!(
            ExecutionProvider::CoreMl.is_compiled(),
            cfg!(feature = "gpu-coreml")
        );
        assert_eq!(
            ExecutionProvider::Cuda.is_compiled(),
            cfg!(feature = "gpu-cuda")
        );
        assert_eq!(
            ExecutionProvider::DirectMl.is_compiled(),
            cfg!(feature = "gpu-directml")
        );
        assert_eq!(
            ExecutionProvider::TensorRt.is_compiled(),
            cfg!(feature = "gpu-tensorrt")
        );
        assert_eq!(
            ExecutionProvider::OpenVino.is_compiled(),
            cfg!(feature = "gpu-openvino")
        );
        assert!(ExecutionProvider::Cpu.is_compiled());
    }

    #[test]
    fn has_compiled_gpu_returns_false_for_cpu_probe() {
        assert!(!has_compiled_gpu(&probe(GpuBackend::Cpu)));
    }

    #[test]
    fn backend_label_reflects_compiled_providers() {
        let p = probe(GpuBackend::Metal);
        if cfg!(feature = "gpu-coreml") {
            assert_eq!(backend_label(&p), "coreml");
            assert!(has_compiled_gpu(&p));
        } else {
            // Default CPU-only build: the Metal probe finds no
            // compiled provider and the label stays honest.
            assert_eq!(backend_label(&p), "cpu");
            assert!(!has_compiled_gpu(&p));
        }
        assert_eq!(backend_label(&probe(GpuBackend::Cpu)), "cpu");
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
