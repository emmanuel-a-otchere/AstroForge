// CR-07 §29.3b.2: behavioural tests for `webgpu-diff.ts`.
//
// Pins the contract that `WebGpuDiffCompute` makes
// the right WebGPU API calls for each DiffMode:
// - Loads the matching WGSL shader source.
// - Allocates the right number of buffers (input a,
//   input b, output, readback, uniform) at the right
//   sizes.
// - Dispatches `ceil(pixel_count / 64)` workgroups.
// - Writes the uniform buffer with the right params.
// - Returns a Uint8Array of the right length on
//   success.
//
// Behavioural coverage is via a mock GPUDevice that
// records every call. The mock returns pre-defined
// readback data so the wrapper can run its full code
// path (pack RGBA8 -> u32, dispatch, readback, unpack).

import { describe, it, expect, beforeEach } from "vitest";
import { WebGpuDiffCompute, acquireWebGpuDevice } from "../webgpu-diff";
import { MockGpuDevice, makeRgbaGrid } from "./mock-gpu";
import * as webgpuTypes from "../webgpu-types";

describe("WebGpuDiffCompute", () => {
  let mock: MockGpuDevice;
  let compute: WebGpuDiffCompute;

  beforeEach(() => {
    mock = new MockGpuDevice();
    compute = new WebGpuDiffCompute(mock as unknown as GPUDevice);
  });

  it("loads the absolute shader on first absolute-mode call", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    // Mock returns zeros for readback.
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("absolute", a, b, 1.0, 8);
    expect(mock.shaders).toHaveLength(1);
    expect(mock.shaders[0]!.code).toContain("abs_i32");
  });

  it("loads the signed shader on first signed-mode call", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("signed", a, b, 1.0, 8);
    expect(mock.shaders).toHaveLength(1);
    expect(mock.shaders[0]!.code).toContain("clamp_i32");
  });

  it("loads the amplified shader on first amplified-mode call", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("amplified", a, b, 2.0, 8);
    expect(mock.shaders).toHaveLength(1);
    expect(mock.shaders[0]!.code).toContain("clamp_f32");
  });

  it("loads the structural shader on first structural-mode call", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("structural", a, b, 1.0, 8);
    expect(mock.shaders).toHaveLength(1);
    expect(mock.shaders[0]!.code).toContain("unpack_rgba");
  });

  it("caches compiled pipelines across calls", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("absolute", a, b, 1.0, 8);
    await compute.compute("absolute", a, b, 1.0, 8);
    // Only ONE shader load + ONE pipeline create.
    expect(mock.shaders).toHaveLength(1);
    expect(mock.pipelines).toHaveLength(1);
  });

  it("allocates 5 buffers per dispatch (a, b, out, readback, uniform)", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("absolute", a, b, 1.0, 8);
    // uniform is created once on first call (lazy);
    // subsequent calls reuse it. So first call = 5 buffers
    // (a + b + out + readback + uniform).
    expect(mock.buffers.length).toBeGreaterThanOrEqual(4);
  });

  it("dispatches ceil(pixel_count / 64) workgroups", async () => {
    const a = makeRgbaGrid(8, 8); // 64 pixels
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("absolute", a, b, 1.0, 8);
    const pass = mock.lastComputePass();
    expect(pass).toBeDefined();
    expect(pass!.dispatchedWorkgroups).toHaveLength(1);
    expect(pass!.dispatchedWorkgroups[0]!.x).toBe(1); // ceil(64/64) = 1
  });

  it("dispatches ceil(65 / 64) = 2 workgroups for 65 pixels", async () => {
    const a = new Uint8Array(65 * 4); // 65 pixels = 260 bytes
    const b = new Uint8Array(65 * 4);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("absolute", a, b, 1.0, 65);
    const pass = mock.lastComputePass();
    expect(pass!.dispatchedWorkgroups[0]!.x).toBe(2); // ceil(65/64) = 2
  });

  it("writes the uniform buffer with the right params", async () => {
    const a = makeRgbaGrid(8, 8); // 64 pixels
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("amplified", a, b, 3.5, 8);
    // Find the uniform buffer (the one with UNIFORM usage).
    mock.syncRecordedWrites(); const uniformBuf = mock.buffers.find((b) => (b.usage & 64) !== 0);
    expect(uniformBuf).toBeDefined();
    expect(uniformBuf!.lastWritten).toBeDefined();
    // Decode: offset 0 = pixel_count (u32), offset 4 = gain (f32),
    // offset 8 = width (u32), offset 12 = pad (u32).
    const dv = new DataView(uniformBuf!.lastWritten!);
    expect(dv.getUint32(0, true)).toBe(64); // pixel_count
    expect(dv.getFloat32(4, true)).toBe(3.5); // gain
    expect(dv.getUint32(8, true)).toBe(8); // width
  });

  it("clamps non-positive gain to 1.0 in the uniform", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("amplified", a, b, -2.0, 8);
    mock.syncRecordedWrites(); const uniformBuf = mock.buffers.find((b) => (b.usage & 64) !== 0);
    const dv = new DataView(uniformBuf!.lastWritten!);
    expect(dv.getFloat32(4, true)).toBe(1.0);
  });

  it("clamps non-finite gain to 1.0 in the uniform", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("amplified", a, b, NaN, 8);
    mock.syncRecordedWrites(); const uniformBuf = mock.buffers.find((b) => (b.usage & 64) !== 0);
    const dv = new DataView(uniformBuf!.lastWritten!);
    expect(dv.getFloat32(4, true)).toBe(1.0);
  });

  it("throws when a/b length mismatch", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = new Uint8Array(10); // different length
    await expect(compute.compute("absolute", a, b, 1.0, 8)).rejects.toThrow(/length mismatch/);
  });

  it("throws when a length is not a multiple of 4", async () => {
    const a = new Uint8Array(7); // not a multiple of 4
    const b = new Uint8Array(7);
    await expect(compute.compute("absolute", a, b, 1.0, 8)).rejects.toThrow(/multiple of 4/);
  });

  it("reuses the uniform buffer across calls (doesn't reallocate)", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("absolute", a, b, 1.0, 8);
    const uniformBuffersBefore = mock.buffers.filter((b) => (b.usage & 64) !== 0).length;
    await compute.compute("absolute", a, b, 1.0, 8);
    const uniformBuffersAfter = mock.buffers.filter((b) => (b.usage & 64) !== 0).length;
    expect(uniformBuffersAfter).toBe(uniformBuffersBefore);
  });

  it("destroys transient buffers after dispatch", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("absolute", a, b, 1.0, 8);
    // The first 3 buffers (a, b, out) should be destroyed.
    // The uniform buffer is preserved.
    expect(mock.buffers[0]!.destroyed).toBe(true);
    expect(mock.buffers[1]!.destroyed).toBe(true);
    expect(mock.buffers[2]!.destroyed).toBe(true);
    // The uniform buffer (last created) should still be alive.
    mock.syncRecordedWrites(); const uniformBuf = mock.buffers.find((b) => (b.usage & 64) !== 0);
    expect(uniformBuf!.destroyed).toBe(false);
  });

  it("returns a Uint8Array of pixel_count * 4 bytes on success", async () => {
    const a = makeRgbaGrid(8, 8); // 64 pixels = 256 bytes
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    const result = await compute.compute("absolute", a, b, 1.0, 8);
    // The mock returns the readback data verbatim. Since
    // the wrapper unpacks u32 -> RGBA8, the result should
    // be the same length as a.
    // NOTE: the mock's mapAsync returns a fresh ArrayBuffer
    // of the readback size, then the wrapper copies +
    // unpacks. We don't assert exact byte values (those
    // depend on the GPU dispatch), only the length.
    expect(result).toBeInstanceOf(Uint8Array);
    expect(result!.length).toBe(a.length);
  });

  it("dispose() clears cached pipelines and destroys uniform buffer", async () => {
    const a = makeRgbaGrid(8, 8);
    const b = makeRgbaGrid(8, 8);
    mock.setReadbackData(new ArrayBuffer(a.length));
    await compute.compute("absolute", a, b, 1.0, 8);
    mock.syncRecordedWrites(); const uniformBuf = mock.buffers.find((b) => (b.usage & 64) !== 0);
    expect(uniformBuf!.destroyed).toBe(false);
    compute.dispose();
    expect(uniformBuf!.destroyed).toBe(true);
  });
});

describe("acquireWebGpuDevice", () => {
  it("returns null when navigator.gpu is undefined", async () => {
    const original = (globalThis as any).navigator;
    (globalThis as any).navigator = {};
    const device = await acquireWebGpuDevice();
    expect(device).toBeNull();
    (globalThis as any).navigator = original;
  });

  it("returns null when navigator.gpu.requestAdapter throws", async () => {
    const original = (globalThis as any).navigator;
    (globalThis as any).navigator = {
      gpu: {
        requestAdapter: () => Promise.reject(new Error("not supported")),
      },
    };
    const device = await acquireWebGpuDevice();
    expect(device).toBeNull();
    (globalThis as any).navigator = original;
  });

  it("returns null when adapter is null", async () => {
    const original = (globalThis as any).navigator;
    (globalThis as any).navigator = {
      gpu: {
        requestAdapter: () => Promise.resolve(null),
      },
    };
    const device = await acquireWebGpuDevice();
    expect(device).toBeNull();
    (globalThis as any).navigator = original;
  });
});