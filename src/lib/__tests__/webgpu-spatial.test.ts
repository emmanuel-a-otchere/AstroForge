// CR-07 §29.3b.2: behavioural tests for `webgpu-spatial.ts`.
//
// Pins the contract that `WebGpuSpatialCompute` makes
// the right WebGPU API calls for each SpatialMode:
// - Loads the matching WGSL shader source.
// - Allocates the right buffers (input, output,
//   readback, uniform) at the right sizes.
// - Dispatches `ceil(pixel_count / 64)` workgroups.
// - Writes the uniform buffer with the right params.
// - Returns a Float32Array of the right length on
//   success.
//
// Behavioural coverage is via a mock GPUDevice that
// records every call. Tests also pin the host-side
// finalize helpers (`finalizeLuminanceNoise`,
// `finalizeChromaticNoise`, `finalizeLocalContrast`)
// which are pure JS functions.

import { describe, it, expect, beforeEach } from "vitest";
import {
  WebGpuSpatialCompute,
  finalizeLuminanceNoise,
  finalizeChromaticNoise,
  finalizeLocalContrast,
  finalizeBackgroundGradient,
} from "../webgpu-spatial";
import { MockGpuDevice, makeF32Image } from "./mock-gpu";

describe("WebGpuSpatialCompute", () => {
  let mock: MockGpuDevice;
  let compute: WebGpuSpatialCompute;

  beforeEach(() => {
    mock = new MockGpuDevice();
    compute = new WebGpuSpatialCompute(mock as unknown as GPUDevice);
  });

  it("loads the luminance_noise shader on first call", async () => {
    const image = makeF32Image(8, 8, 1);
    mock.setReadbackData(new ArrayBuffer(image.byteLength));
    await compute.compute("luminance_noise", image, 8, 8, 1);
    expect(mock.shaders).toHaveLength(1);
    // The shader source contains the per-pixel residual
    // logic; check for a unique signature string.
    expect(mock.shaders[0]!.code).toContain("let median = max_f32(s18, min_f32(s23, min_f32(s25, s20)))");
  });

  it("loads the chromatic_noise shader on first call", async () => {
    const image = makeF32Image(8, 8, 3);
    mock.setReadbackData(new ArrayBuffer(image.byteLength));
    await compute.compute("chromatic_noise", image, 8, 8, 3);
    expect(mock.shaders).toHaveLength(1);
    // The chromatic shader uses workgroup-shared sums
    // + atomicAdd for the count.
    expect(mock.shaders[0]!.code).toContain("atomicAdd(&wg_count");
  });

  it("loads the local_contrast shader on first call", async () => {
    const image = makeF32Image(8, 8, 1);
    mock.setReadbackData(new ArrayBuffer(image.byteLength));
    await compute.compute("local_contrast", image, 8, 8, 1);
    expect(mock.shaders).toHaveLength(1);
    // The local_contrast shader computes the 3x3 sum and
    // divides by 9 for the mean.
    expect(mock.shaders[0]!.code).toContain("let mean = sum / 9.0");
  });

  it("loads the background_gradient shader on first call", async () => {
    // Use an image at least 64x64 so the shader dispatches
    // at least one tile. Set readback data large enough
    // to hold one tile record (4 floats * 1 tile = 4 floats).
    const image = makeF32Image(64, 64, 1);
    mock.setReadbackData(new ArrayBuffer(4 * 4));
    await compute.compute("background_gradient", image, 64, 64, 1);
    expect(mock.shaders).toHaveLength(1);
    // The shader's source contains the unique TILE_SIZE
    // constant declaration.
    expect(mock.shaders[0]!.code).toContain("const TILE_SIZE: u32 = 64u");
  });

  it("dispatches one workgroup per 64x64 tile for background_gradient", async () => {
    // 128x128 image = 2x2 = 4 tiles.
    const image = makeF32Image(128, 128, 1);
    mock.setReadbackData(new ArrayBuffer(4 * 4 * 4));
    await compute.compute("background_gradient", image, 128, 128, 1);
    const pass = mock.lastComputePass();
    expect(pass).toBeDefined();
    expect(pass!.dispatchedWorkgroups).toHaveLength(1);
    expect(pass!.dispatchedWorkgroups[0]!.x).toBe(4);
  });

  it("caches compiled pipelines across calls", async () => {
    const image = makeF32Image(8, 8, 1);
    mock.setReadbackData(new ArrayBuffer(image.byteLength));
    await compute.compute("luminance_noise", image, 8, 8, 1);
    await compute.compute("luminance_noise", image, 8, 8, 1);
    expect(mock.shaders).toHaveLength(1);
    expect(mock.pipelines).toHaveLength(1);
  });

  it("dispatches ceil(pixel_count / 64) workgroups", async () => {
    const image = makeF32Image(8, 8, 1); // 64 pixels
    mock.setReadbackData(new ArrayBuffer(image.byteLength));
    await compute.compute("luminance_noise", image, 8, 8, 1);
    const pass = mock.lastComputePass();
    expect(pass).toBeDefined();
    expect(pass!.dispatchedWorkgroups).toHaveLength(1);
    expect(pass!.dispatchedWorkgroups[0]!.x).toBe(1);
  });

  it("dispatches ceil(65 / 64) = 2 workgroups for 65 pixels", async () => {
    const image = makeF32Image(65, 1, 1); // 65 pixels
    mock.setReadbackData(new ArrayBuffer(image.byteLength));
    await compute.compute("luminance_noise", image, 65, 1, 1);
    const pass = mock.lastComputePass();
    expect(pass!.dispatchedWorkgroups[0]!.x).toBe(2);
  });

  it("writes the uniform buffer with the right params", async () => {
    const image = makeF32Image(8, 8, 1); // 64 pixels
    mock.setReadbackData(new ArrayBuffer(image.byteLength));
    await compute.compute("local_contrast", image, 8, 8, 1);
    mock.syncRecordedWrites();
    const uniformBuf = mock.buffers.find((b) => (b.usage & 64) !== 0);
    expect(uniformBuf).toBeDefined();
    expect(uniformBuf!.lastWritten).toBeDefined();
    const dv = new DataView(uniformBuf!.lastWritten!);
    expect(dv.getUint32(0, true)).toBe(64); // pixel_count
    expect(dv.getUint32(4, true)).toBe(8); // width
    expect(dv.getUint32(8, true)).toBe(8); // height
    expect(dv.getUint32(12, true)).toBe(1); // channels
  });

  it("throws when image length does not match width*height*channels", async () => {
    const image = makeF32Image(8, 8, 1); // 64 floats
    await expect(
      compute.compute("luminance_noise", image, 16, 16, 1),
    ).rejects.toThrow(/does not match width\*height\*channels/);
  });

  it("destroys transient buffers after dispatch", async () => {
    const image = makeF32Image(8, 8, 1);
    mock.setReadbackData(new ArrayBuffer(image.byteLength));
    await compute.compute("luminance_noise", image, 8, 8, 1);
    // The input + output buffers should be destroyed.
    const inputBuf = mock.buffers[0]!;
    expect(inputBuf.destroyed).toBe(true);
    // The uniform buffer (last created) should still be alive.
    const uniformBuf = mock.buffers.find((b) => (b.usage & 64) !== 0);
    expect(uniformBuf!.destroyed).toBe(false);
  });

  it("returns a Float32Array of the right size for per-pixel modes", async () => {
    const image = makeF32Image(8, 8, 1); // 64 floats = 256 bytes
    mock.setReadbackData(new ArrayBuffer(image.byteLength));
    const result = await compute.compute("luminance_noise", image, 8, 8, 1);
    expect(result).toBeInstanceOf(Float32Array);
    // spatialOutputSize for per-pixel modes = pixelCount.
    expect(result!.length).toBe(64);
  });

  it("returns a Float32Array of the right size for chromatic_noise", async () => {
    const image = makeF32Image(64, 1, 3); // 64 pixels = 192 floats
    // Output is 4 floats per workgroup = 1 workgroup * 4 = 4 floats.
    mock.setReadbackData(new ArrayBuffer(4 * 4));
    const result = await compute.compute("chromatic_noise", image, 64, 1, 3);
    expect(result).toBeInstanceOf(Float32Array);
    expect(result!.length).toBe(4);
  });

  it("dispose() clears cached pipelines and destroys uniform buffer", async () => {
    const image = makeF32Image(8, 8, 1);
    mock.setReadbackData(new ArrayBuffer(image.byteLength));
    await compute.compute("luminance_noise", image, 8, 8, 1);
    const uniformBuf = mock.buffers.find((b) => (b.usage & 64) !== 0);
    expect(uniformBuf!.destroyed).toBe(false);
    compute.dispose();
    expect(uniformBuf!.destroyed).toBe(true);
  });
});

describe("finalizeLuminanceNoise", () => {
  it("returns 0 for empty residuals", () => {
    expect(finalizeLuminanceNoise(new Float32Array(0))).toBe(0.0);
  });

  it("computes sigma = 1.4826 * median(residuals)", () => {
    // Rust baseline:
    //   sorted.sort();
    //   let mad = sorted[sorted.len() / 2];
    //   let sigma = 1.4826 * mad;
    // [1,2,3,4,5,6,7,8,9] sorted, median = sorted[4] = 5.
    // sigma = 1.4826 * 5 = 7.413.
    const residuals = new Float32Array([9, 1, 5, 3, 7, 2, 8, 6, 4]);
    expect(finalizeLuminanceNoise(residuals)).toBeCloseTo(7.413, 2);
  });

  it("returns 0 for uniform residuals (all equal to zero)", () => {
    const residuals = new Float32Array([0, 0, 0, 0, 0, 0, 0, 0]);
    // All values are 0; median = 0; sigma = 0.
    expect(finalizeLuminanceNoise(residuals)).toBe(0.0);
  });
});

describe("finalizeChromaticNoise", () => {
  it("returns 0 for empty partials", () => {
    expect(finalizeChromaticNoise(new Float32Array(0))).toBe(0.0);
  });

  it("returns 0 when all partial counts are zero", () => {
    // 2 workgroups, each with count=0.
    const partials = new Float32Array([0, 0, 0, 0, 0, 0, 0, 0]);
    expect(finalizeChromaticNoise(partials)).toBe(0.0);
  });

  it("returns 0 when total mean sum is below 1e-12", () => {
    // 1 workgroup, partials = (0, 0, 0, 10). Total = 0 < 1e-12.
    const partials = new Float32Array([0, 0, 0, 10]);
    expect(finalizeChromaticNoise(partials)).toBe(0.0);
  });

  it("computes stddev of channel ratios", () => {
    // 1 workgroup: R/G/B means = (1, 2, 3), count = 6.
    // Total = 6. Ratios = (1/6, 2/6, 3/6) = (0.1667, 0.3333, 0.5).
    // Mean of ratios = 1/3 ≈ 0.3333.
    // Variance = ((1/6-1/3)^2 + (2/6-1/3)^2 + (3/6-1/3)^2) / 3
    //          = (1/36 + 0 + 1/36) / 3 = 2/108 = 1/54.
    // Stddev = sqrt(1/54) ≈ 0.1361.
    const partials = new Float32Array([1, 2, 3, 6]);
    expect(finalizeChromaticNoise(partials)).toBeCloseTo(Math.sqrt(1 / 54), 4);
  });

  it("aggregates partials across multiple workgroups", () => {
    // 2 workgroups:
    //   WG1: R/G/B sums = (1, 2, 3), count = 6.
    //   WG2: R/G/B sums = (2, 4, 6), count = 12.
    // Totals: R=3, G=6, B=9, count=18. Means = (1/6, 2/6, 3/6).
    // Same as the single-workgroup case above.
    const partials = new Float32Array([1, 2, 3, 6, 2, 4, 6, 12]);
    expect(finalizeChromaticNoise(partials)).toBeCloseTo(Math.sqrt(1 / 54), 4);
  });
});

describe("finalizeLocalContrast", () => {
  it("returns 0 for empty residuals", () => {
    expect(finalizeLocalContrast(new Float32Array(0))).toBe(0.0);
  });

  it("computes the global mean of the residuals", () => {
    // [2, 4, 6, 8] -> mean = 5.
    expect(finalizeLocalContrast(new Float32Array([2, 4, 6, 8]))).toBe(5.0);
  });

  it("returns 0 for all-zero residuals", () => {
    expect(finalizeLocalContrast(new Float32Array([0, 0, 0, 0]))).toBe(0.0);
  });
});
describe("finalizeBackgroundGradient", () => {
  it("returns 0 for empty partials", () => {
    expect(finalizeBackgroundGradient(new Float32Array(0))).toBe(0.0);
  });

  it("returns 0 when fewer than 2 tiles have samples", () => {
    // 1 tile with samples + 1 tile without = 1 valid tile < 2.
    const partials = new Float32Array([
      32, 32, 0.5, 1.0, // x=32, y=32, median=0.5, has_samples=1
      96, 32, 0.0, 0.0, // has_samples=0, skipped
    ]);
    expect(finalizeBackgroundGradient(partials)).toBe(0.0);
  });

  it("returns 0 when denom is degenerate (all x equal)", () => {
    // All tiles have x=64 (constant x). The x-variance
    // is zero, so the plane-fit is degenerate.
    const partials = new Float32Array([
      64, 32, 0.5, 1.0,
      64, 96, 0.7, 1.0,
    ]);
    expect(finalizeBackgroundGradient(partials)).toBe(0.0);
  });

  it("computes gradient magnitude from 4 tiles", () => {
    // 4 tiles at the corners of a 128x128 image:
    //   (32, 32)  -> 0.0
    //   (96, 32)  -> 0.5
    //   (32, 96)  -> 1.0
    //   (96, 96)  -> 1.5
    // This is a plane z = 0 + 0.5*(x-32)/64 + 1.0*(y-32)/64.
    // a = 0.5/64 = 0.0078125, b = 1.0/64 = 0.015625.
    // mag = sqrt(0.0078125^2 + 0.015625^2) = 0.01736...
    // per 100 px = 1.736...
    const partials = new Float32Array([
      32, 32, 0.0, 1.0,
      96, 32, 0.5, 1.0,
      32, 96, 1.0, 1.0,
      96, 96, 1.5, 1.0,
    ]);
    const expected = Math.sqrt(0.5 * 0.5 + 1.0 * 1.0) / 64 * 100;
    expect(finalizeBackgroundGradient(partials)).toBeCloseTo(expected, 2);
  });

  it("returns 0 for a uniform image (all medians equal)", () => {
    // 4 tiles, all with median = 0.5. The plane is flat.
    const partials = new Float32Array([
      32, 32, 0.5, 1.0,
      96, 32, 0.5, 1.0,
      32, 96, 0.5, 1.0,
      96, 96, 0.5, 1.0,
    ]);
    expect(finalizeBackgroundGradient(partials)).toBe(0.0);
  });
});
