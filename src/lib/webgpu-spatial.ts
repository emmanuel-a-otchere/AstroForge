// CR-07 §29.3b.1: WebGPU spatial detector wrapper.
//
// Mirrors the Rust detector functions
// (`luminance_noise`, `chromatic_noise`,
// `local_contrast`) in
// `crates/astroforge-core/src/image_analysis/metrics.rs`
// by running the per-pixel work on the GPU.
//
// The wrapper accepts a single f32 image (one channel
// of pixel data, in `[0, 1]`) and dispatches the
// appropriate compute shader. The output is a partial
// result: the per-pixel residual for
// `luminance_noise` / `local_contrast`, or the per-
// workgroup partial sums for `chromatic_noise`. The
// host finishes the calc (sort + sigma for noise;
// ratio + stddev for chromatic; global mean for local
// contrast). This split is deliberate: the per-pixel
// work is the expensive part, and the host-side
// reduction is trivial.
//
// This slice ships the GPU primitive only. There is
// no IPC layer wiring and no UI consumer. The wrapper
// is opt-in: callers instantiate `WebGpuSpatialCompute`
// explicitly and pass a `GPUDevice` they own.
//
// The 4th spatial detector (`background_gradient`)
// requires 64x64 tile-strided reduction + a host-side
// plane-fit solve. It is NOT shipped in this slice;
// that lands in a follow-on sub-slice (§29.3b.1a) or
// folds into the §29.3b production rollout.
//
// Type choices:
//
// - `SpatialMode` is a string union of the three
//   detectors that the slice supports.
// - Input image is `Float32Array` with row-major layout
//   over (y, x), channels flattened as separate rows
//   in the buffer (matches Rust `image[(c, y, x)]`
//   indexing).
// - Output is a fresh `Float32Array`. For
//   `luminance_noise` and `local_contrast`, the output
//   has one entry per pixel (the per-pixel residual).
//   For `chromatic_noise`, the output has 4 entries per
//   workgroup (3 partial sums + 1 pixel count). The
//   host is responsible for the final reduction.
//
// Edge cases:
//
// - Image too small for 3x3 neighbourhood (width < 3
//   or height < 3): the shader writes 0 for every
//   pixel. Matches the Rust baseline (which returns
//   `MetricsSample { value: 0.0, ... }` when w < 3 or
//   h < 3).
// - For `chromatic_noise` with channels < 3: the shader
//   writes 0 for the missing channels + a count that
//   reflects only the present channels. Matches the
//   Rust baseline (which returns 0.0 when channels <
//   3, with `Confidence::Low`).
// - The host post-processing for `chromatic_noise`
//   needs to handle the "total mean sum < 1e-12" guard
//   from the Rust baseline. The wrapper documents this.

import {
  shaderForSpatial,
  SPATIAL_PARAMS_BYTES,
  spatialOutputSize,
  type SpatialMode,
} from "./webgpu-spatial-shaders";
import "./webgpu-types";

/**
 * Result returned by `computeSpatialWebGpu`.
 *
 * For `luminance_noise` / `local_contrast`: an array of
 * per-pixel residuals. The host computes the global
 * stat (sigma = 1.4826 * MAD for noise; mean for local
 * contrast).
 *
 * For `chromatic_noise`: a flat array of length
 * `workgroupCount * 4`. Each workgroup contributes 4
 * entries: 3 partial per-channel sums + 1 pixel count.
 * The host sums the partials across workgroups then
 * computes the ratios + stddev.
 */
export type SpatialPartialResult = Float32Array;

const WORKGROUP_SIZE = 64;

/**
 * Compute the chromatic noise value from the GPU's
 * partial-sum output. The caller passes the result
 * buffer from `WebGpuSpatialCompute.compute(mode, ...)`
 * and gets back the chromatic noise value (channel-
 * ratio stddev). Returns 0.0 if the total mean is
 * below 1e-12 (matching the Rust baseline).
 */
export function finalizeChromaticNoise(partials: Float32Array): number {
  if (partials.length === 0) return 0.0;
  const stride = 4;
  const workgroupCount = partials.length / stride;
  let sR = 0.0;
  let sG = 0.0;
  let sB = 0.0;
  let count = 0;
  for (let g = 0; g < workgroupCount; g++) {
    sR += partials[g * stride + 0]!;
    sG += partials[g * stride + 1]!;
    sB += partials[g * stride + 2]!;
    count += partials[g * stride + 3]!;
  }
  if (count === 0) return 0.0;
  const mR = sR / count;
  const mG = sG / count;
  const mB = sB / count;
  const total = mR + mG + mB;
  if (total < 1e-12) return 0.0;
  const rR = mR / total;
  const rG = mG / total;
  const rB = mB / total;
  const m = (rR + rG + rB) / 3.0;
  const v = ((rR - m) ** 2 + (rG - m) ** 2 + (rB - m) ** 2) / 3.0;
  return Math.sqrt(v);
}

/**
 * Compute the local contrast value (global mean of the
 * per-pixel residuals) from the GPU's output. Returns
 * 0.0 if the buffer is empty.
 */
export function finalizeLocalContrast(residuals: Float32Array): number {
  if (residuals.length === 0) return 0.0;
  let sum = 0.0;
  for (let i = 0; i < residuals.length; i++) {
    sum += residuals[i]!;
  }
  return sum / residuals.length;
}

/**
 * Compute the background gradient magnitude from the
 * GPU's per-tile output. Mirrors the Rust baseline:
 * fit plane z = a*x + b*y + c via least-squares over the
 * tile medians, return sqrt(a² + b²) * 100 (so a flat
 * image returns ~0 and a typical gradient returns ~5-50).
 *
 * The GPU shader writes one record per tile:
 * (x_center, y_center, median, has_samples_flag).
 * Tiles with `has_samples_flag = 0` are skipped (empty
 * tiles can occur when the image is smaller than the
 * tile size).
 *
 * Mirrors the Rust baseline's:
 * - returns 0.0 when n < 2 valid tiles
 * - returns 0.0 when denom < 1e-12 (degenerate fit)
 * - multiplies the per-pixel magnitude by 100 to report
 *   the gradient "per 100 px" (matching the original
 *   unit).
 */
export function finalizeBackgroundGradient(partials: Float32Array): number {
  const STRIDE = 4;
  if (partials.length < STRIDE * 2) return 0.0;
  // First pass: collect valid (x, y, v) tuples.
  const xs: number[] = [];
  const ys: number[] = [];
  const vs: number[] = [];
  for (let i = 0; i + STRIDE <= partials.length; i += STRIDE) {
    const hasSamples = partials[i + 3]!;
    if (hasSamples > 0.5) {
      xs.push(partials[i + 0]!);
      ys.push(partials[i + 1]!);
      vs.push(partials[i + 2]!);
    }
  }
  const n = xs.length;
  if (n < 2) return 0.0;
  // Second pass: least-squares normal-equation
  // accumulators. Mirrors the Rust baseline
  // character-for-character.
  let sX = 0, sY = 0, sZ = 0;
  let sXX = 0, sXY = 0, sXZ = 0;
  let sYY = 0, sYZ = 0;
  for (let i = 0; i < n; i++) {
    const x = xs[i]!, y = ys[i]!, v = vs[i]!;
    sX += x;
    sY += y;
    sZ += v;
    sXX += x * x;
    sXY += x * y;
    sXZ += x * v;
    sYY += y * y;
    sYZ += y * v;
  }
  const meanX = sX / n;
  const meanY = sY / n;
  const meanZ = sZ / n;
  const sXXc = sXX / n - meanX * meanX;
  const sXYc = sXY / n - meanX * meanY;
  const sXZc = sXZ / n - meanX * meanZ;
  const sYYc = sYY / n - meanY * meanY;
  const sYZc = sYZ / n - meanY * meanZ;
  const denom = sXXc * sYYc - sXYc * sXYc;
  if (Math.abs(denom) < 1e-12) return 0.0;
  const a = (sYYc * sXZc - sXYc * sYZc) / denom;
  const b = (sXXc * sYZc - sXYc * sXZc) / denom;
  const magPerPx = Math.sqrt(a * a + b * b);
  return magPerPx * 100.0;
}

/**
 * Compute the luminance noise sigma from the GPU's
 * per-pixel residual output. Mirrors the Rust baseline:
 * sigma = 1.4826 * MAD(median(|residuals|)).
 *
 * NOTE: the MAD computation requires sorting the
 * residuals. On the GPU, this would require a parallel
 * sort (radix sort, bitonic merge, etc.) which is a
 * non-trivial kernel. The slice keeps the sort on the
 * host because the residual array is small (the
 * detector operates on a 25% downsample, so a 4K input
 * yields ~520K residuals = 2 MB of f32).
 */
export function finalizeLuminanceNoise(residuals: Float32Array): number {
  if (residuals.length === 0) return 0.0;
  const sorted = residuals.slice();
  sorted.sort();
  const mad = sorted[Math.floor(sorted.length / 2)]!;
  return 1.4826 * mad;
}

/**
 * Opaque wrapper that owns the compiled pipelines +
 * bind-group layout for the three spatial compute
 * shaders. Reusable across many `computeSpatialWebGpu`
 * calls; the caller provides the GPUDevice so they
 * control when it is destroyed.
 */
export class WebGpuSpatialCompute {
  private device: GPUDevice;
  private pipelines: Partial<Record<SpatialMode, GPUComputePipeline>> = {};
  private bindGroupLayout: GPUBindGroupLayout | null = null;
  private uniformBuffer: GPUBuffer | null = null;

  constructor(device: GPUDevice) {
    this.device = device;
  }

  private pipelineFor(mode: SpatialMode): GPUComputePipeline {
    const cached = this.pipelines[mode];
    if (cached) return cached;
    if (!this.bindGroupLayout) {
      this.bindGroupLayout = this.device.createBindGroupLayout({
        entries: [
          { binding: 0, visibility: GPUShaderStage.COMPUTE, buffer: { type: "read-only-storage" } },
          { binding: 1, visibility: GPUShaderStage.COMPUTE, buffer: { type: "uniform" } },
          { binding: 2, visibility: GPUShaderStage.COMPUTE, buffer: { type: "storage" } },
        ],
      });
    }
    if (!this.uniformBuffer) {
      this.uniformBuffer = this.device.createBuffer({
        size: SPATIAL_PARAMS_BYTES,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      });
    }
    const module = this.device.createShaderModule({ code: shaderForSpatial(mode) });
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
   * Run the spatial detector on the GPU. Returns a
   * `SpatialPartialResult` (Float32Array) that the
   * caller post-processes via the `finalize*` helpers
   * above. Returns `null` on GPU failure so the caller
   * can fall back to the Rust-backed path.
   *
   * Async: GPU readback is asynchronous in real WebGPU;
   * the mock is synchronous but we await anyway to
   * match the production semantics.
   *
   * Parameters:
   * - `mode`: which detector to run.
   * - `image`: f32 image data, row-major over (y, x),
   *   channels flattened as separate rows in the buffer.
   *   Matches Rust `image[(c, y, x)]` indexing.
   * - `width`, `height`, `channels`: image dimensions.
   */
  async compute(
    mode: SpatialMode,
    image: Float32Array,
    width: number,
    height: number,
    channels: number,
  ): Promise<SpatialPartialResult | null> {
    if (image.length !== width * height * channels) {
      throw new Error(
        `spatial compute: image length ${image.length} does not match width*height*channels = ${width * height * channels}`,
      );
    }
    const pixelCount = width * height;
    try {
      return await this.dispatch(mode, image, width, height, channels, pixelCount);
    } catch (err) {
      console.warn("[webgpu-spatial] dispatch failed; falling back", err);
      return null;
    }
  }

  private async dispatch(
    mode: SpatialMode,
    image: Float32Array,
    width: number,
    height: number,
    channels: number,
    pixelCount: number,
  ): Promise<Float32Array<ArrayBufferLike>> {
    const device = this.device;
    const inputBuf = device.createBuffer({
      size: image.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      mappedAtCreation: true,
    });
    new Float32Array(inputBuf.getMappedRange()).set(image);
    inputBuf.unmap();
    // `background_gradient` is dispatched by tile count
    // (one workgroup per 64x64 tile) instead of by pixel
    // count. The output buffer holds 4 f32s per tile.
    const TILE_SIZE = 64;
    const tileCount =
      mode === "background_gradient"
        ? Math.ceil(width / TILE_SIZE) * Math.ceil(height / TILE_SIZE)
        : 0;
    const outputSize =
      mode === "background_gradient"
        ? tileCount * 4
        : spatialOutputSize(mode, pixelCount, WORKGROUP_SIZE);
    const outputBuf = device.createBuffer({
      size: Math.max(outputSize, 1) * 4,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    });
    // Update the uniform buffer with the per-call params.
    const uniformData = new ArrayBuffer(SPATIAL_PARAMS_BYTES);
    const u32 = new Uint32Array(uniformData);
    u32[0] = pixelCount;
    u32[1] = width;
    u32[2] = height;
    u32[3] = channels;
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
        { binding: 0, resource: { buffer: inputBuf } },
        { binding: 1, resource: { buffer: this.uniformBuffer! } },
        { binding: 2, resource: { buffer: outputBuf } },
      ],
    });
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    const workgroups =
      mode === "background_gradient"
        ? tileCount
        : Math.ceil(pixelCount / WORKGROUP_SIZE);
    pass.dispatchWorkgroups(Math.max(workgroups, 1));
    pass.end();
    // Read back the output buffer.
    const readback = device.createBuffer({
      size: Math.max(outputSize, 1) * 4,
      usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
    });
    encoder.copyBufferToBuffer(outputBuf, readback, 0, Math.max(outputSize, 1) * 4);
    const commandBuffer = encoder.finish();
    device.queue.submit([commandBuffer]);
    inputBuf.destroy();
    outputBuf.destroy();
    return new Promise<Float32Array<ArrayBufferLike>>((resolve, reject) => {
      readback.mapAsync(GPUMapMode.READ).then(
        () => {
          const copy = new Float32Array(readback.getMappedRange()).slice();
          readback.unmap();
          readback.destroy();
          resolve(copy);
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