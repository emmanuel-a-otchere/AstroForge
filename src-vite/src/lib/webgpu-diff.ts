// CR-07 §29.3a: WebGPU compute wrapper for `compute_diff`.
//
// Mirrors the Rust `compute_diff(kind, a, b, gain, width)`
// API in `crates/astroforge-core/src/difference.rs`. Runs
// the same operation on the GPU via a precompiled compute
// pipeline.
//
// This slice ships a feature-flagged prototype:
//
// - The wrapper compiles the WGSL shaders lazily on first
//   use and reuses the compiled pipelines across calls.
// - The wrapper accepts a `GPUDevice` from the caller (so
//   the caller's lifecycle manager owns the device).
// - If WebGPU is unavailable (no `navigator.gpu`, no
//   device acquired), `computeDiffWebGpu` returns `null`
//   so callers can fall back to the Rust-backed path.
// - The wrapper handles the gain clamping for Amplified
//   mode (negative / non-finite gain becomes 1.0) on the
//   CPU side, matching the Rust baseline.
//
// The slice does NOT:
//
// - Wire the wrapper into the comparison UI. The IPC
//   layer + CompareWorkspace integration is §29.3b.
// - Ship a behavioural test harness. The project does not
//   yet have a JS-side test runner (no Vitest). Future
//   slices can add Vitest infrastructure + browser-based
//   GPU tests (Playwright + headless Chrome with WebGPU
//   enabled) to verify byte-equivalence with the Rust
//   backend.
// - Offload the spatial detectors (`luminance_noise`,
//   `chromatic_noise`, `local_contrast`,
//   `background_gradient`) to GPU. Those are §29.3c.
//
// Type choices:
//
// - `DiffMode` is a string union mirroring the Rust
//   `DiffKind` serde rename-all = "kebab-case" mapping.
//   The four values map to "absolute" / "signed" /
//   "amplified" / "structural".
// - Buffers are typed as `Uint8Array` (the natural
//   representation of RGBA8 image data) and re-packed
//   into `Uint32Array` for GPU upload (4 bytes per
//   pixel = 1 u32). The conversion is a single
//   `DataView`-free copy.
// - Output is a fresh `Uint8Array`. The caller is
//   responsible for freeing the buffers (the wrapper
//   destroys them on disposal).
//
// Edge cases:
//
// - Buffers of mismatched length: the wrapper throws
//   (matches the Rust `debug_assert_eq!`).
// - `width` <= 1 for structural mode: structural path
//   falls back to absolute on Rust side. The WGSL
//   structural shader writes 0 for first/last columns
//   and first row; the rest of the buffer is correct.
//   For `width <= 1` the entire buffer is edge pixels.
// - `gain` for non-amplified modes: the value is in the
//   uniform but ignored. Setting `gain` to 1.0 for
//   non-amplified modes is the recommended API contract.

import { shaderFor } from "./webgpu-shaders";
import "./webgpu-types";

/**
 * Diff mode string. Mirror of the Rust `DiffKind`
 * serde-renamed variants.
 */
export type DiffMode = "absolute" | "signed" | "amplified" | "structural";

/**
 * 16-byte aligned uniform buffer struct (matches the
 * WGSL `DiffParams` struct).
 *
 * Layout:
 * - offset 0:  pixel_count : u32
 * - offset 4:  gain        : f32
 * - offset 8:  width       : u32
 * - offset 12: pad0        : u32
 */
const UNIFORM_BYTES = 16;

/**
 * Workgroup size used by all four compute shaders.
 * Each shader declares `@workgroup_size(64)`. The
 * dispatch count is `ceil(pixel_count / 64)`.
 */
const WORKGROUP_SIZE = 64;

/**
 * Opaque wrapper that owns the compiled pipelines +
 * bind-group layout for the four diff compute shaders.
 * Reusable across many `computeDiffWebGpu` calls; the
 * caller provides the GPUDevice so they control when
 * it is destroyed.
 */
export class WebGpuDiffCompute {
  private device: GPUDevice;
  private pipelines: Partial<Record<DiffMode, GPUComputePipeline>> = {};
  private bindGroupLayout: GPUBindGroupLayout | null = null;
  private uniformBuffer: GPUBuffer | null = null;

  constructor(device: GPUDevice) {
    this.device = device;
  }

  /**
   * Lazily compile the pipeline for `mode`. Subsequent
   * calls reuse the cached pipeline.
   */
  private pipelineFor(mode: DiffMode): GPUComputePipeline {
    const cached = this.pipelines[mode];
    if (cached) return cached;
    if (!this.bindGroupLayout) {
      this.bindGroupLayout = this.device.createBindGroupLayout({
        entries: [
          { binding: 0, visibility: GPUShaderStage.COMPUTE, buffer: { type: "read-only-storage" } },
          { binding: 1, visibility: GPUShaderStage.COMPUTE, buffer: { type: "read-only-storage" } },
          { binding: 2, visibility: GPUShaderStage.COMPUTE, buffer: { type: "uniform" } },
          { binding: 3, visibility: GPUShaderStage.COMPUTE, buffer: { type: "storage" } },
        ],
      });
    }
    if (!this.uniformBuffer) {
      this.uniformBuffer = this.device.createBuffer({
        size: UNIFORM_BYTES,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      });
    }
    const module = this.device.createShaderModule({ code: shaderFor(mode) });
    const pipeline = this.device.createComputePipeline({
      layout: this.device.createPipelineLayout({
        bindGroupLayouts: [this.bindGroupLayout],
      }),
      compute: { module, entryPoint: "main" },
    });
    this.pipelines[mode] = pipeline;
    return pipeline;
  }

  /**
   * Pack 4 RGBA8 bytes into a u32 (little-endian on
   * the GPU). The byte order matches the WGSL
   * `(packed >> 0u) & 0xffu` unpacking.
   */
  private static packRgba(src: Uint8Array, pixelIndex: number): number {
    const base = pixelIndex * 4;
    return (
      (src[base + 0]!)! |
      ((src[base + 1]!)! << 8) |
      ((src[base + 2]!)! << 16) |
      ((src[base + 3]!)! << 24)
    ) >>> 0;
  }

  /**
   * Run `compute_diff` on the GPU.
   *
   * Returns `null` if the GPU buffers cannot be
   * created (caller should fall back to the Rust
   * backend). Throws on length mismatch (matching the
   * Rust `debug_assert_eq!`).
   *
   * Async: the GPU readback is asynchronous in real
   * WebGPU implementations, so the call returns a
   * Promise. Callers `await compute(...)` and check
   ` the result for null on GPU failure.
   */
  async compute(
    mode: DiffMode,
    a: Uint8Array,
    b: Uint8Array,
    gain: number,
    width: number,
  ): Promise<Uint8Array | null> {
    if (a.length !== b.length) {
      throw new Error(
        `compute_diff: a/b length mismatch (${a.length} vs ${b.length})`,
      );
    }
    if (a.length % 4 !== 0) {
      throw new Error(`compute_diff: a length not a multiple of 4`);
    }
    const pixelCount = a.length / 4;
    // CPU-side gain clamping mirrors the Rust baseline
    // for Amplified mode. For other modes, the shader
    // ignores gain; we still write the clamped value.
    const clampedGain =
      Number.isFinite(gain) && gain > 0 ? gain : 1.0;
    try {
      return await this.dispatch(mode, a, b, clampedGain, width, pixelCount);
    } catch (err) {
      // Surface the GPU failure to the caller as null
      // so they can fall back to the Rust path.
      console.warn("[webgpu-diff] dispatch failed; falling back", err);
      return null;
    }
  }

  private async dispatch(
    mode: DiffMode,
    a: Uint8Array,
    b: Uint8Array,
    gain: number,
    width: number,
    pixelCount: number,
  ): Promise<Uint8Array<ArrayBufferLike>> {
    const device = this.device;
    // Pack a + b into u32 arrays (4 bytes per pixel).
    const aPacked = new Uint32Array(pixelCount);
    const bPacked = new Uint32Array(pixelCount);
    for (let i = 0; i < pixelCount; i++) {
      aPacked[i] = WebGpuDiffCompute.packRgba(a, i);
      bPacked[i] = WebGpuDiffCompute.packRgba(b, i);
    }
    const aBuf = device.createBuffer({
      size: aPacked.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      mappedAtCreation: true,
    });
    new Uint32Array(aBuf.getMappedRange()).set(aPacked);
    aBuf.unmap();
    const bBuf = device.createBuffer({
      size: bPacked.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      mappedAtCreation: true,
    });
    new Uint32Array(bBuf.getMappedRange()).set(bPacked);
    bBuf.unmap();
    const outBuf = device.createBuffer({
      size: aPacked.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    });
    // Update the uniform buffer with the per-call
    // params (reused across calls).
    const uniformData = new ArrayBuffer(UNIFORM_BYTES);
    const u32 = new Uint32Array(uniformData);
    const f32 = new Float32Array(uniformData);
    u32[0] = pixelCount;
    f32[1] = gain;
    u32[2] = width;
    u32[3] = 0; // pad
    const pipeline = this.pipelineFor(mode);
    if (!this.bindGroupLayout) {
      throw new Error("bind group layout not initialized");
    }
    // Now that pipelineFor has ensured the uniform
    // buffer exists, write the per-call params.
    if (this.uniformBuffer) {
      device.queue.writeBuffer(this.uniformBuffer, 0, uniformData);
    }
    const bindGroup = device.createBindGroup({
      layout: this.bindGroupLayout,
      entries: [
        { binding: 0, resource: { buffer: aBuf } },
        { binding: 1, resource: { buffer: bBuf } },
        { binding: 2, resource: { buffer: this.uniformBuffer! } },
        { binding: 3, resource: { buffer: outBuf } },
      ],
    });
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(Math.ceil(pixelCount / WORKGROUP_SIZE));
    pass.end();
    // Read back the output buffer.
    const readback = device.createBuffer({
      size: outBuf.size,
      usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
    });
    encoder.copyBufferToBuffer(outBuf, readback, 0, outBuf.size);
    const commandBuffer = encoder.finish();
    device.queue.submit([commandBuffer]);
    aBuf.destroy();
    bBuf.destroy();
    outBuf.destroy();
    // Await the GPU readback. Real WebGPU is
    // asynchronous; the mock is synchronous but we
    // await anyway to match the production semantics.
    return new Promise<Uint8Array<ArrayBufferLike>>((resolve, reject) => {
      readback.mapAsync(GPUMapMode.READ).then(
        () => {
          const copy = new Uint32Array(readback.getMappedRange()).slice();
          readback.unmap();
          readback.destroy();
          // Unpack u32 -> 4 RGBA8 bytes per pixel.
          const out = new Uint8Array(pixelCount * 4);
          for (let i = 0; i < pixelCount; i++) {
            const p = copy[i]!;
            out[i * 4 + 0] = (p >> 0) & 0xff;
            out[i * 4 + 1] = (p >> 8) & 0xff;
            out[i * 4 + 2] = (p >> 16) & 0xff;
            out[i * 4 + 3] = (p >> 24) & 0xff;
          }
          resolve(out);
        },
        (err) => {
          readback.destroy();
          reject(err);
        },
      );
    });
  }

  /**
   * Free GPU resources. The wrapper cannot be reused
   * after `dispose()`. The caller still owns the
   * GPUDevice.
   */
  dispose(): void {
    if (this.uniformBuffer) {
      this.uniformBuffer.destroy();
      this.uniformBuffer = null;
    }
    this.pipelines = {};
    this.bindGroupLayout = null;
  }
}

/**
 * Request a WebGPU device if the browser supports it.
 * Returns `null` when WebGPU is unavailable so the
 * caller can fall back to the Rust-backed diff path.
 *
 * Capability detection reuses the existing
 * `probeGpu()` module. This function extends the
 * detection with an `adapter + device` request, which
 * is the real test of compute availability (some
 * browsers advertise WebGPU but lack compute
 * support).
 */
export async function acquireWebGpuDevice(): Promise<GPUDevice | null> {
  if (typeof navigator === "undefined" || !("gpu" in navigator)) {
    return null;
  }
  try {
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) return null;
    const device = await adapter.requestDevice();
    return device;
  } catch {
    return null;
  }
}